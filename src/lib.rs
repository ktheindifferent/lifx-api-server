// TODO - Impliment authentication header (DONE)
// TODO - Wrap as a rust library with configurable ports + authentication (DONE)
// TODO - Impliment LIFX Effects, Scenes, Clean, Cycle
// TODO - Impliment an extended API for changing device labels, wifi-config, etc.


use get_if_addrs::{get_if_addrs, IfAddr, Ifv4Addr};
use lifx_rs::lan::{get_product_info, BuildOptions, Message, PowerLevel, ProductInfo, RawMessage, HSBK};
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::sync::{Arc, Mutex};
use std::thread::{spawn};
use std::time::{Duration, Instant};
use rouille::try_or_400;
use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;
use std::thread;

use rouille::Response;
use rouille::post_input;


use serde::{Serialize, Deserialize};

use palette::{Hsv, Srgb, IntoColor};

use colors_transform::{Rgb, Color};



const HOUR: Duration = Duration::from_secs(60 * 60);

// Rate limiting configuration
const MAX_AUTH_ATTEMPTS: u32 = 5;
const AUTH_WINDOW_SECONDS: u64 = 60;

// Simple rate limiter for authentication attempts
#[derive(Debug, Clone)]
struct AuthAttempt {
    timestamp: Instant,
    count: u32,
}

struct RateLimiter {
    attempts: Arc<Mutex<HashMap<String, AuthAttempt>>>,
}

impl RateLimiter {
    fn new() -> Self {
        RateLimiter {
            attempts: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn check_and_update(&self, client_ip: String) -> bool {
        let mut attempts = self.attempts.lock().unwrap();
        let now = Instant::now();
        let window = Duration::from_secs(AUTH_WINDOW_SECONDS);
        
        match attempts.get_mut(&client_ip) {
            Some(attempt) => {
                if now.duration_since(attempt.timestamp) > window {
                    // Reset window
                    attempt.timestamp = now;
                    attempt.count = 1;
                    true
                } else if attempt.count >= MAX_AUTH_ATTEMPTS {
                    // Too many attempts
                    false
                } else {
                    // Increment counter
                    attempt.count += 1;
                    true
                }
            }
            None => {
                // First attempt
                attempts.insert(client_ip, AuthAttempt {
                    timestamp: now,
                    count: 1,
                });
                true
            }
        }
    }

    fn cleanup_old_entries(&self) {
        let mut attempts = self.attempts.lock().unwrap();
        let now = Instant::now();
        let window = Duration::from_secs(AUTH_WINDOW_SECONDS * 2);
        
        attempts.retain(|_, attempt| {
            now.duration_since(attempt.timestamp) <= window
        });
    }
}

// Authentication middleware result
enum AuthResult {
    Authorized,
    Unauthorized(Response),
}

// Centralized authentication middleware
fn authenticate_request(
    request: &rouille::Request,
    secret_key: &str,
    rate_limiter: &Arc<RateLimiter>,
) -> AuthResult {
    // Extract client IP for rate limiting
    let client_ip = request.remote_addr().ip().to_string();
    
    // Get authorization header
    let auth_header = request.header("Authorization");
    
    match auth_header {
        None => {
            // Check rate limit for failed auth attempts
            if !rate_limiter.check_and_update(client_ip) {
                return AuthResult::Unauthorized(
                    Response::text("Too many authentication attempts. Please try again later.")
                        .with_status_code(429)
                        .with_additional_header("Retry-After", "60")
                );
            }
            
            // Return 401 Unauthorized when no auth header is present
            AuthResult::Unauthorized(
                Response::text("Unauthorized: Missing Authorization header")
                    .with_status_code(401)
                    .with_additional_header("WWW-Authenticate", "Bearer realm=\"LIFX API\"")
            )
        }
        Some(auth_value) => {
            // Validate the token
            let expected_token = format!("Bearer {}", secret_key);
            if auth_value != &expected_token {
                // Check rate limit for failed auth attempts
                if !rate_limiter.check_and_update(client_ip) {
                    return AuthResult::Unauthorized(
                        Response::text("Too many authentication attempts. Please try again later.")
                            .with_status_code(429)
                            .with_additional_header("Retry-After", "60")
                    );
                }
                
                // Return 401 Unauthorized for invalid token
                AuthResult::Unauthorized(
                    Response::text("Unauthorized: Invalid token")
                        .with_status_code(401)
                        .with_additional_header("WWW-Authenticate", "Bearer realm=\"LIFX API\"")
                )
            } else {
                AuthResult::Authorized
            }
        }
    }
}


#[derive(Debug)]
struct RefreshableData<T> {
    data: Option<T>,
    max_age: Duration,
    last_updated: Instant,
    refresh_msg: Message,
}

impl<T> RefreshableData<T> {
    fn empty(max_age: Duration, refresh_msg: Message) -> RefreshableData<T> {
        RefreshableData {
            data: None,
            max_age,
            last_updated: Instant::now(),
            refresh_msg,
        }
    }
    fn update(&mut self, data: T) {
        self.data = Some(data);
        self.last_updated = Instant::now();


    }
    fn needs_refresh(&self) -> bool {
        self.data.is_none() || self.last_updated.elapsed() > self.max_age
    }
    fn as_ref(&self) -> Option<&T> {
        self.data.as_ref()
    }
}

#[derive(Debug, Serialize)]
struct BulbInfo {
    pub id: String,
    pub uuid: String,
    pub label: String,
    pub connected: bool,
    pub power: String,
    #[serde(rename = "color")]
    pub lifx_color: Option<LifxColor>,
    pub brightness: f64,
    #[serde(rename = "group")]
    pub lifx_group: Option<LifxGroup>,
    #[serde(rename = "location")]
    pub lifx_location: Option<LifxLocation>,
    pub product: Option<ProductInfo>,
    #[serde(rename = "last_seen")]
    pub lifx_last_seen: String,
    #[serde(rename = "seconds_since_seen")]
    pub seconds_since_seen: i64,
    // pub error: Option<String>,
    // pub errors: Option<Vec<Error>>,

    #[serde(skip_serializing)]
    last_seen: Instant,

    source: u32,

    target: u64,

    addr: SocketAddr,

    #[serde(skip_serializing)]
    group: RefreshableData<LifxGroup>,


    #[serde(skip_serializing)]
    name: RefreshableData<String>,
    #[serde(skip_serializing)]
    model: RefreshableData<(u32, u32)>,
    #[serde(skip_serializing)]
    location: RefreshableData<String>,
    #[serde(skip_serializing)]
    host_firmware: RefreshableData<u32>,
    #[serde(skip_serializing)]
    wifi_firmware: RefreshableData<u32>,
    #[serde(skip_serializing)]
    power_level: RefreshableData<PowerLevel>,
    #[serde(skip_serializing)]
    color: LiColor,
}

#[derive(Debug)]
enum LiColor {
    Unknown,
    Single(RefreshableData<HSBK>),
    Multi(RefreshableData<Vec<Option<HSBK>>>),
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[doc(hidden)]
pub struct LifxLocation {
    pub id: String,
    pub name: String,
}

/// Represents an LIFX Color
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LifxColor {
    pub hue: u16,
    pub saturation: u16,
    pub kelvin: u16,
    pub brightness: u16,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[doc(hidden)]
pub struct LifxGroup {
    pub id: String,
    pub name: String,
}

impl BulbInfo {
    fn new(source: u32, target: u64, addr: SocketAddr) -> BulbInfo {
        let id: String = thread_rng().sample_iter(&Alphanumeric).take(12).map(char::from).collect();
        let uuid: String = thread_rng().sample_iter(&Alphanumeric).take(30).map(char::from).collect();
        BulbInfo {
            id: id.to_string(),
            uuid: uuid.to_string(),
            label: format!(""),
            connected: true,
            power: format!("off"),
            lifx_color: None,
            brightness: 0.0,
            lifx_group: None,
            lifx_location: None,
            product: None,
            lifx_last_seen: format!(""),
            seconds_since_seen: 0,
            last_seen: Instant::now(),
            source,
            target,
            addr,
            group: RefreshableData::empty(HOUR, Message::GetGroup),
            location: RefreshableData::empty(HOUR, Message::GetLocation),
            name: RefreshableData::empty(HOUR, Message::GetLabel),
            model: RefreshableData::empty(HOUR, Message::GetVersion),
            host_firmware: RefreshableData::empty(HOUR, Message::GetHostFirmware),
            wifi_firmware: RefreshableData::empty(HOUR, Message::GetWifiFirmware),
            power_level: RefreshableData::empty(Duration::from_millis(500), Message::GetPower),
            color: LiColor::Unknown,
        }
    }

    fn update(&mut self, addr: SocketAddr) {
        self.last_seen = Instant::now();
        self.addr = addr;
    }

    fn refresh_if_needed<T>(
        &self,
        sock: &UdpSocket,
        data: &RefreshableData<T>,
    ) -> Result<(), failure::Error> {
        if data.needs_refresh() {
            let options = BuildOptions {
                target: Some(self.target),
                res_required: true,
                source: self.source,
                ..Default::default()
            };
            let message = RawMessage::build(&options, data.refresh_msg.clone())?;
            sock.send_to(&message.pack()?, self.addr)?;
        }
        Ok(())
    }

    fn set_power(
        &self,
        sock: &UdpSocket,
        power_level: PowerLevel,
    ) -> Result<(), failure::Error> {
        
        let options = BuildOptions {
            target: Some(self.target),
            res_required: true,
            source: self.source,
            ..Default::default()
        };
        let message = RawMessage::build(&options, Message::SetPower{level: power_level})?;
        sock.send_to(&message.pack()?, self.addr)?;
  
        Ok(())
    }

    fn set_infrared(
        &self,
        sock: &UdpSocket,
        brightness: u16,
    ) -> Result<(), failure::Error> {
        
        let options = BuildOptions {
            target: Some(self.target),
            res_required: true,
            source: self.source,
            ..Default::default()
        };
        let message = RawMessage::build(&options, Message::LightSetInfrared{brightness: brightness})?;
        sock.send_to(&message.pack()?, self.addr)?;
  
        Ok(())
    }


    fn set_color(
        &self,
        sock: &UdpSocket,
        color: HSBK,
        duration: u32
    ) -> Result<(), failure::Error> {
        
        let options = BuildOptions {
            target: Some(self.target),
            res_required: true,
            source: self.source,
            ..Default::default()
        };
        let message = RawMessage::build(&options, Message::LightSetColor{reserved: 0, color: color, duration: duration})?;
        sock.send_to(&message.pack()?, self.addr)?;
  
        Ok(())
    }




    fn query_for_missing_info(&self, sock: &UdpSocket) -> Result<(), failure::Error> {
        self.refresh_if_needed(sock, &self.name)?;
        self.refresh_if_needed(sock, &self.model)?;
        self.refresh_if_needed(sock, &self.location)?;
        self.refresh_if_needed(sock, &self.host_firmware)?;
        self.refresh_if_needed(sock, &self.wifi_firmware)?;
        self.refresh_if_needed(sock, &self.power_level)?;
        self.refresh_if_needed(sock, &self.group)?;
        match &self.color {
            LiColor::Unknown => (), // we'll need to wait to get info about this bulb's model, so we'll know if it's multizone or not
            LiColor::Single(d) => self.refresh_if_needed(sock, d)?,
            LiColor::Multi(d) => self.refresh_if_needed(sock, d)?,
        }

    

        Ok(())
    }
}

// impl std::fmt::Debug for BulbInfo {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(f, "BulbInfo({:0>16X} - {}  ", self.target, self.addr)?;

//         if let Some(name) = self.name.as_ref() {
//             write!(f, "{}", name)?;
//         }
//         if let Some(location) = self.location.as_ref() {
//             write!(f, "/{}", location)?;
//         }
//         if let Some((vendor, product)) = self.model.as_ref() {
//             if let Some(info) = get_product_info(*vendor, *product) {
//                 write!(f, " - {} ", info.name)?;
//             } else {
//                 write!(
//                     f,
//                     " - Unknown model (vendor={}, product={}) ",
//                     vendor, product
//                 )?;
//             }
//         }
//         if let Some(fw_version) = self.host_firmware.as_ref() {
//             write!(f, " McuFW:{:x}", fw_version)?;
//         }
//         if let Some(fw_version) = self.wifi_firmware.as_ref() {
//             write!(f, " WifiFW:{:x}", fw_version)?;
//         }
//         if let Some(level) = self.power_level.as_ref() {
//             if *level == PowerLevel::Enabled {
//                 write!(f, "  Powered On(")?;
//                 match self.color {
//                     Color::Unknown => write!(f, "??")?,
//                     Color::Single(ref color) => {
//                         f.write_str(
//                             &color
//                                 .as_ref()
//                                 .map(|c| c.describe(false))
//                                 .unwrap_or_else(|| "??".to_owned()),
//                         )?;
//                     }
//                     Color::Multi(ref color) => {
//                         if let Some(vec) = color.as_ref() {
//                             write!(f, "Zones: ")?;
//                             for zone in vec {
//                                 if let Some(color) = zone {
//                                     write!(f, "{} ", color.describe(true))?;
//                                 } else {
//                                     write!(f, "?? ")?;
//                                 }
//                             }
//                         }
//                     }
//                 }
//                 write!(f, ")")?;
//             } else {
//                 write!(f, "  Powered Off")?;
//             }
//         }
//         write!(f, ")")
//     }
// }

struct Manager {
    bulbs: Arc<Mutex<HashMap<u64, BulbInfo>>>,
    last_discovery: Instant,
    sock: UdpSocket,
    source: u32,
}

impl Manager {
    fn new() -> Result<Manager, failure::Error> {
        let sock = UdpSocket::bind("0.0.0.0:56700")?;
        sock.set_broadcast(true)?;

        // spawn a thread that can send to our socket
        let recv_sock = sock.try_clone()?;

        let bulbs = Arc::new(Mutex::new(HashMap::new()));
        let receiver_bulbs = bulbs.clone();
        let source = 0x72757374;

        // spawn a thread that will receive data from our socket and update our internal data structures
        spawn(move || Self::worker(recv_sock, source, receiver_bulbs));

        let mut mgr = Manager {
            bulbs,
            last_discovery: Instant::now(),
            sock,
            source,
        };
        mgr.discover()?;
        Ok(mgr)
    }

    fn handle_message(raw: RawMessage, bulb: &mut BulbInfo) -> Result<(), lifx_rs::lan::Error> {
        match Message::from_raw(&raw)? {
            Message::StateService { port: _, service: _ } => {
                // if port != bulb.addr.port() as u32 || service != Service::UDP {
                //     println!("Unsupported service: {:?}/{}", service, port);
                // }
            }
            Message::StateLabel { label } => {
                bulb.name.update(label.0);
                bulb.label = bulb.name.data.as_ref().unwrap().to_string();

            },

  
            Message::StateLocation { location, label, updated_at: _ } => {

                let lab = label.0;

                bulb.location.update(lab.clone());


          
                let group_two = LifxLocation{id: format!("{:?}", location.0).replace(", ", "").replace("[", "").replace("]", ""), name: lab};
                bulb.lifx_location = Some(group_two);

            },
            Message::StateVersion {
                vendor, product, ..
            } => {
                bulb.model.update((vendor, product));
                if let Some(info) = get_product_info(vendor, product) {
                    // println!("{:?}", info.clone());

                    bulb.product = Some(info.clone());

                    if info.capabilities.has_multizone {
                        bulb.color = LiColor::Multi(RefreshableData::empty(
                            Duration::from_secs(15),
                            Message::GetColorZones {
                                start_index: 0,
                                end_index: 255,
                            },
                        ))
                    } else {
                        bulb.color = LiColor::Single(RefreshableData::empty(
                            Duration::from_secs(15),
                            Message::LightGet,
                        ))
                    }
                }
            }
            Message::StatePower { level } => {
                bulb.power_level.update(level);

                if bulb.power_level.data.as_ref().unwrap() ==  &PowerLevel::Enabled{
                    bulb.power = format!("on");
                } else {
                    bulb.power = format!("off");
                }

               
            },

            Message::StateGroup { group, label, updated_at: _ } => {

                let group_one = LifxGroup{id: format!("{:?}", group.0), name: label.to_string()};
                
                let group_two = LifxGroup{id: format!("{:?}", group.0).replace(", ", "").replace("[", "").replace("]", ""), name: label.to_string()};
                bulb.group.update(group_one);
                bulb.lifx_group = Some(group_two);
            },



            Message::StateHostFirmware { version, .. } => bulb.host_firmware.update(version),
            Message::StateWifiFirmware { version, .. } => bulb.wifi_firmware.update(version),
            Message::LightState {
                color,
                power,
                label,
                ..
            } => {
                if let LiColor::Single(ref mut d) = bulb.color {
                    d.update(color);

                    let bc = color;


                    bulb.lifx_color = Some(LifxColor{
                        hue: bc.hue,
                        saturation: bc.saturation,
                        kelvin: bc.kelvin,
                        brightness: bc.brightness,
                    });

                    bulb.brightness = (bc.brightness / 65535) as f64;


                    bulb.power_level.update(power);
                }
                bulb.name.update(label.0);
            }
            Message::StateZone {
                count,
                index,
                color,
            } => {
                if let LiColor::Multi(ref mut d) = bulb.color {
                    d.data.get_or_insert_with(|| {
                        let mut v = Vec::with_capacity(count as usize);
                        v.resize(count as usize, None);
                        assert!(index <= count);
                        v
                    })[index as usize] = Some(color);
                }
            }
            Message::StateMultiZone {
                count,
                index,
                color0,
                color1,
                color2,
                color3,
                color4,
                color5,
                color6,
                color7,
            } => {
                if let LiColor::Multi(ref mut d) = bulb.color {
                    let v = d.data.get_or_insert_with(|| {
                        let mut v = Vec::with_capacity(count as usize);
                        v.resize(count as usize, None);
                        assert!(index + 7 <= count);
                        v
                    });

                    v[index as usize + 0] = Some(color0);
                    v[index as usize + 1] = Some(color1);
                    v[index as usize + 2] = Some(color2);
                    v[index as usize + 3] = Some(color3);
                    v[index as usize + 4] = Some(color4);
                    v[index as usize + 5] = Some(color5);
                    v[index as usize + 6] = Some(color6);
                    v[index as usize + 7] = Some(color7);
                }
            }
            unknown => {
                println!("Received, but ignored {:?}", unknown);
            }
        }
        Ok(())
    }

    fn worker(
        recv_sock: UdpSocket,
        source: u32,
        receiver_bulbs: Arc<Mutex<HashMap<u64, BulbInfo>>>,
    ) {
        let mut buf = [0; 1024];
        loop {
            match recv_sock.recv_from(&mut buf) {
                Ok((0, addr)) => println!("Received a zero-byte datagram from {:?}", addr),
                Ok((nbytes, addr)) => match RawMessage::unpack(&buf[0..nbytes]) {
                    Ok(raw) => {
                        if raw.frame_addr.target == 0 {
                            continue;
                        }
                        if let Ok(mut bulbs) = receiver_bulbs.lock() {
                            let bulb = bulbs
                                .entry(raw.frame_addr.target)
                                .and_modify(|bulb| bulb.update(addr))
                                .or_insert_with(|| {
                                    BulbInfo::new(source, raw.frame_addr.target, addr)
                                });
                            if let Err(e) = Self::handle_message(raw, bulb) {
                                println!("Error handling message from {}: {}", addr, e)
                            }
                        }
                    }
                    Err(e) => println!("Error unpacking raw message from {}: {}", addr, e),
                },
                Err(e) => panic!("recv_from err {:?}", e),
            }
        }
    }

    fn discover(&mut self) -> Result<(), failure::Error> {
        println!("Doing discovery");

        let opts = BuildOptions {
            source: self.source,
            ..Default::default()
        };
        let rawmsg = RawMessage::build(&opts, Message::GetService).unwrap();
        let bytes = rawmsg.pack().unwrap();

        for addr in get_if_addrs().unwrap() {
            match addr.addr {
                IfAddr::V4(Ifv4Addr {
                    broadcast: Some(bcast),
                    ..
                }) => {
                    if addr.ip().is_loopback() {
                        continue;
                    }
                    let addr = SocketAddr::new(IpAddr::V4(bcast), 56700);
                    println!("Discovering bulbs on LAN {:?}", addr);
                    self.sock.send_to(&bytes, &addr)?;
                }
                _ => {}
            }
        }

        self.last_discovery = Instant::now();

        Ok(())
    }

    fn refresh(&self) {
        if let Ok(bulbs) = self.bulbs.lock() {
            for bulb in bulbs.values() {
                match bulb.query_for_missing_info(&self.sock){
                    Ok(_missing_info) => {
                    },
                    Err(e) => {
                        println!("Error querying for missing info: {:?}", e);
                    }
                }
            }
        }
    }
}

/// Used to set the params when posting a FlameEffect event
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub secret_key: String,
    pub port: u16,
}

pub fn start(config: Config) {


    sudo::with_env(&["SECRET_KEY"]).unwrap();
    sudo::escalate_if_needed().unwrap();


    let mgr = Manager::new();

    match mgr {
        Ok(mgr) => {
            let mgr_arc = Arc::new(Mutex::new(mgr));

            let th_arc_mgr = Arc::clone(&mgr_arc);

            thread::spawn(move || {
                loop{
                    let mut lock = th_arc_mgr.lock().unwrap();
                    let mgr = &mut *lock;  
                
                    // if Instant::now() - mgr.last_discovery > Duration::from_secs(300) {
                    //     mgr.discover().unwrap();
                    // }
            
                    mgr.refresh();
                    thread::sleep(Duration::from_millis(1000));
                }
        
            });
        
        
            let th2_arc_mgr = Arc::clone(&mgr_arc);
            
            // Initialize rate limiter
            let rate_limiter = Arc::new(RateLimiter::new());
            
            // Spawn cleanup thread for rate limiter
            let cleanup_limiter = Arc::clone(&rate_limiter);
            thread::spawn(move || {
                loop {
                    thread::sleep(Duration::from_secs(120));
                    cleanup_limiter.cleanup_old_entries();
                }
            });
        
            thread::spawn(move || {
                rouille::start_server(format!("0.0.0.0:{}", config.port).as_str(), move |request| {
        
                    // Use centralized authentication middleware
                    match authenticate_request(request, &config.secret_key, &rate_limiter) {
                        AuthResult::Unauthorized(response) => return response,
                        AuthResult::Authorized => {
                            // Continue with request processing
                        }
                    }
        
        
        
        
                    let mut response = Response::text("hello world");
        
                    let mut lock = th2_arc_mgr.lock().unwrap();
                    let mgr = &mut *lock;  
        
                
                    mgr.refresh();
        
        
                    let urls = request.url().to_string();
                    let split = urls.split("/");
                    let vec: Vec<&str> = split.collect();
        
                    let mut selector = "";
        
                    if vec.len() >= 3 {
                        selector = vec[3];
                    }
            
        
        
                    let mut bulbs_vec: Vec<&BulbInfo> = Vec::new();
        
                    let bulbs = mgr.bulbs.lock().unwrap();
                    
                        
                    for bulb in bulbs.values() {
                        println!("{:?}", *bulb);
                        bulbs_vec.push(bulb);
                    }
        
                    if selector == "all"{
                    
                    }
        
                    if selector.contains("group_id:"){
                        bulbs_vec = bulbs_vec
                        .into_iter()
                        .filter(|b| b.lifx_group.as_ref().unwrap().id.contains(&selector.replace("group_id:", "")))
                        .collect();
                    }
        
                    if selector.contains("location_id:"){
                        bulbs_vec = bulbs_vec
                        .into_iter()
                        .filter(|b| b.lifx_location.as_ref().unwrap().id.contains(&selector.replace("location_id:", "")))
                        .collect();
                    }
        
                    if selector.contains("id:"){
                        bulbs_vec = bulbs_vec
                        .into_iter()
                        .filter(|b| b.id.contains(&selector.replace("id:", "")))
                        .collect();
                    }
        
        
                    // (PUT) SetStates
                    // https://api.lifx.com/v1/lights/states
                    if request.url().contains("/lights/states") && request.method() == "PUT" {
                        #[derive(Deserialize)]
                        struct StateUpdate {
                            selector: String,
                            power: Option<String>,
                            color: Option<String>,
                            brightness: Option<f64>,
                            duration: Option<f64>,
                            infrared: Option<f64>,
                            fast: Option<bool>,
                        }

                        #[derive(Deserialize)]
                        struct StatesRequest {
                            states: Vec<StateUpdate>,
                            defaults: Option<StateUpdate>,
                        }

                        #[derive(Serialize)]
                        struct StateResult {
                            id: String,
                            label: String,
                            status: String,
                        }

                        #[derive(Serialize)]
                        struct StatesResponse {
                            results: Vec<StateResult>,
                        }

                        let body = try_or_400!(rouille::input::plain_text_body(request));
                        let input: StatesRequest = try_or_400!(serde_json::from_str(&body));
                        let mut results = Vec::new();

                        for state_update in input.states {
                            // Filter bulbs based on selector
                            let mut filtered_bulbs: Vec<&BulbInfo> = Vec::new();
                            
                            for bulb in bulbs.values() {
                                let mut matches = false;
                                
                                if state_update.selector == "all" {
                                    matches = true;
                                } else if state_update.selector.starts_with("id:") {
                                    let id = state_update.selector.strip_prefix("id:").unwrap_or("");
                                    matches = bulb.id.contains(id);
                                } else if state_update.selector.starts_with("group_id:") {
                                    let group_id = state_update.selector.strip_prefix("group_id:").unwrap_or("");
                                    matches = bulb.lifx_group.as_ref().map_or(false, |g| g.id.contains(group_id));
                                } else if state_update.selector.starts_with("location_id:") {
                                    let location_id = state_update.selector.strip_prefix("location_id:").unwrap_or("");
                                    matches = bulb.lifx_location.as_ref().map_or(false, |l| l.id.contains(location_id));
                                } else if state_update.selector.starts_with("label:") {
                                    let label = state_update.selector.strip_prefix("label:").unwrap_or("");
                                    matches = bulb.label.contains(label);
                                }
                                
                                if matches {
                                    filtered_bulbs.push(bulb);
                                }
                            }

                            // Apply state changes to filtered bulbs
                            for bulb in filtered_bulbs {
                                let mut status = "ok";
                                
                                // Apply power state
                                if let Some(ref power) = state_update.power {
                                    let result = if power == "on" {
                                        bulb.set_power(&mgr.sock, PowerLevel::Enabled)
                                    } else {
                                        bulb.set_power(&mgr.sock, PowerLevel::Standby)
                                    };
                                    
                                    if result.is_err() {
                                        status = "error";
                                    }
                                }
                                
                                // Apply color if specified
                                if let Some(ref color_str) = state_update.color {
                                    // Reuse existing color parsing logic
                                    let mut kelvin = bulb.lifx_color.as_ref().map_or(6500, |c| c.kelvin);
                                    let mut brightness = bulb.lifx_color.as_ref().map_or(65535, |c| c.brightness);
                                    let mut saturation = bulb.lifx_color.as_ref().map_or(0, |c| c.saturation);
                                    let mut hue = bulb.lifx_color.as_ref().map_or(0, |c| c.hue);
                                    let duration = state_update.duration.unwrap_or(0.0) as u32;
                                    
                                    // Parse color string and update HSBK values
                                    let hsbk = if color_str.starts_with("kelvin:") {
                                        let k = color_str.strip_prefix("kelvin:").unwrap_or("6500").parse::<u16>().unwrap_or(6500);
                                        HSBK { hue, saturation: 0, brightness, kelvin: k }
                                    } else if color_str.starts_with("hue:") {
                                        let h = color_str.strip_prefix("hue:").unwrap_or("0").parse::<u16>().unwrap_or(0);
                                        HSBK { hue: h, saturation, brightness, kelvin }
                                    } else if color_str.starts_with("brightness:") {
                                        let b = color_str.strip_prefix("brightness:").unwrap_or("1.0").parse::<f64>().unwrap_or(1.0);
                                        HSBK { hue, saturation, brightness: (b * 65535.0) as u16, kelvin }
                                    } else if color_str.starts_with("saturation:") {
                                        let s = color_str.strip_prefix("saturation:").unwrap_or("1.0").parse::<f64>().unwrap_or(1.0);
                                        HSBK { hue, saturation: (s * 65535.0) as u16, brightness, kelvin }
                                    } else {
                                        // Handle named colors
                                        match color_str.as_str() {
                                            "white" => HSBK { hue: 0, saturation: 0, brightness, kelvin },
                                            "red" => HSBK { hue: 0, saturation: 65535, brightness, kelvin },
                                            "orange" => HSBK { hue: 7098, saturation: 65535, brightness, kelvin },
                                            "yellow" => HSBK { hue: 10920, saturation: 65535, brightness, kelvin },
                                            "cyan" => HSBK { hue: 32760, saturation: 65535, brightness, kelvin },
                                            "green" => HSBK { hue: 21840, saturation: 65535, brightness, kelvin },
                                            "blue" => HSBK { hue: 43680, saturation: 65535, brightness, kelvin },
                                            "purple" => HSBK { hue: 50050, saturation: 65535, brightness, kelvin },
                                            "pink" => HSBK { hue: 63700, saturation: 25000, brightness, kelvin },
                                            _ => HSBK { hue, saturation, brightness, kelvin }
                                        }
                                    };
                                    
                                    if bulb.set_color(&mgr.sock, hsbk, duration).is_err() {
                                        status = "error";
                                    }
                                }
                                
                                // Apply brightness if specified independently
                                if state_update.color.is_none() && state_update.brightness.is_some() {
                                    let brightness_val = state_update.brightness.unwrap();
                                    let duration = state_update.duration.unwrap_or(0.0) as u32;
                                    
                                    let mut kelvin = bulb.lifx_color.as_ref().map_or(6500, |c| c.kelvin);
                                    let mut saturation = bulb.lifx_color.as_ref().map_or(0, |c| c.saturation);
                                    let mut hue = bulb.lifx_color.as_ref().map_or(0, |c| c.hue);
                                    
                                    let hsbk = HSBK {
                                        hue,
                                        saturation,
                                        brightness: (brightness_val * 65535.0) as u16,
                                        kelvin,
                                    };
                                    
                                    if bulb.set_color(&mgr.sock, hsbk, duration).is_err() {
                                        status = "error";
                                    }
                                }
                                
                                // Apply infrared if specified
                                if let Some(infrared) = state_update.infrared {
                                    let ir_brightness = (infrared * 65535.0) as u16;
                                    if bulb.set_infrared(&mgr.sock, ir_brightness).is_err() {
                                        status = "error";
                                    }
                                }
                                
                                results.push(StateResult {
                                    id: bulb.id.clone(),
                                    label: bulb.label.clone(),
                                    status: status.to_string(),
                                });
                            }
                        }

                        response = Response::json(&StatesResponse { results });
                    }
        
                    // (PUT) SetState
                    // https://api.lifx.com/v1/lights/:selector/state
                    if request.url().contains("/state"){
        
                        let input = try_or_400!(post_input!(request, {
                            power: Option<String>,
                            color: Option<String>,
                            brightness: Option<f64>,
                            duration: Option<f64>,
                            infrared: Option<f64>,
                            fast: Option<bool>
                        }));
        
        
                        // Power
                        if input.power.is_some() {
                            let power = input.power.unwrap();
                            if power == format!("on"){
                                for bulb in &bulbs_vec {
                                    bulb.set_power(&mgr.sock, PowerLevel::Enabled);
                                }
                            } 
                
                            if power == format!("off"){
                                for bulb in &bulbs_vec {
                                    bulb.set_power(&mgr.sock, PowerLevel::Standby);
                                }
                            } 
                        }
        
                        // Color
                        if input.color.is_some() {
                            let cc = input.color.unwrap();
        
        
        
                            for bulb in &bulbs_vec {
        
        
                                let mut kelvin = 6500;
                                let mut brightness = 65535;
                                let mut saturation = 0;
                                let mut hue = 0;
        
                                let mut duration = 0;
                                if input.duration.is_some(){
                                    duration = input.duration.unwrap() as u32;
                                }
        
                                if bulb.lifx_color.is_some() {
                                    let lifxc = bulb.lifx_color.as_ref().unwrap();
                                    kelvin = lifxc.kelvin;
                                    brightness = lifxc.brightness;
                                    saturation = lifxc.saturation;
                                    hue = lifxc.hue;
                                }
                            
                                if cc.contains("white"){
                                    let hbsk_set = HSBK {
                                        hue: 0,
                                        saturation: 0,
                                        brightness: brightness,
                                        kelvin: kelvin,
                                    };
                                    bulb.set_color(&mgr.sock, hbsk_set, duration);
                                }
        
                                if cc.contains("red"){
                                    let hbsk_set = HSBK {
                                        hue: 0,
                                        saturation: 65535,
                                        brightness: brightness,
                                        kelvin: kelvin,
                                    };
                                    bulb.set_color(&mgr.sock, hbsk_set, duration);
                                }
        
                                if cc.contains("orange"){
                                    let hbsk_set = HSBK {
                                        hue: 7098,
                                        saturation: 65535,
                                        brightness: brightness,
                                        kelvin: kelvin,
                                    };
                                    bulb.set_color(&mgr.sock, hbsk_set, duration);
                                }
        
                                if cc.contains("yellow"){
                                    let hbsk_set = HSBK {
                                        hue: 10920,
                                        saturation: 65535,
                                        brightness: brightness,
                                        kelvin: kelvin,
                                    };
                                    bulb.set_color(&mgr.sock, hbsk_set, duration);
                                }
        
                                if cc.contains("cyan"){
                                    let hbsk_set = HSBK {
                                        hue: 32760,
                                        saturation: 65535,
                                        brightness: brightness,
                                        kelvin: kelvin,
                                    };
                                    bulb.set_color(&mgr.sock, hbsk_set, duration);
                                }
        
                                if cc.contains("green"){
                                    let hbsk_set = HSBK {
                                        hue: 21840,
                                        saturation: 65535,
                                        brightness: brightness,
                                        kelvin: kelvin,
                                    };
                                    bulb.set_color(&mgr.sock, hbsk_set, duration);
                                }
        
                                if cc.contains("blue"){
                                    let hbsk_set = HSBK {
                                        hue: 43680,
                                        saturation: 65535,
                                        brightness: brightness,
                                        kelvin: kelvin,
                                    };
                                    bulb.set_color(&mgr.sock, hbsk_set, duration);
                                }
        
                                if cc.contains("purple"){
                                    let hbsk_set = HSBK {
                                        hue: 50050,
                                        saturation: 65535,
                                        brightness: brightness,
                                        kelvin: kelvin,
                                    };
                                    bulb.set_color(&mgr.sock, hbsk_set, duration);
                                }
        
                                if cc.contains("pink"){
                                    let hbsk_set = HSBK {
                                        hue: 63700,
                                        saturation: 25000,
                                        brightness: brightness,
                                        kelvin: kelvin,
                                    };
                                    bulb.set_color(&mgr.sock, hbsk_set, duration);
                                }
        
        
                                if cc.contains("hue:"){
        
                                    let hue_split = cc.split("hue:");
                                    let hue_vec: Vec<&str> = hue_split.collect();
                                    let new_hue = hue_vec[1].to_string().parse::<u16>().unwrap(); 
                                    let hbsk_set = HSBK {
                                        hue: new_hue,
                                        saturation: saturation,
                                        brightness: brightness,
                                        kelvin: kelvin,
                                    };
                                    bulb.set_color(&mgr.sock, hbsk_set, duration);
                                }
        
                                if cc.contains("saturation:"){
                                    let saturation_split = cc.split("saturation:");
                                    let saturation_vec: Vec<&str> = saturation_split.collect();
                                    let new_saturation_float = saturation_vec[1].to_string().parse::<f64>().unwrap(); 
                                    let new_saturation: u16 = (f64::from(100) * new_saturation_float) as u16;
                                    let hbsk_set = HSBK {
                                        hue: hue,
                                        saturation: new_saturation,
                                        brightness: brightness,
                                        kelvin: kelvin,
                                    };
                                    bulb.set_color(&mgr.sock, hbsk_set, duration);
                                }
        
                                if cc.contains("brightness:"){
                                    let brightness_split = cc.split("brightness:");
                                    let brightness_vec: Vec<&str> = brightness_split.collect();
                                    let new_brightness_float = brightness_vec[1].to_string().parse::<f64>().unwrap(); 
                                    let new_brightness: u16 = (f64::from(65535) * new_brightness_float) as u16;
                                    let hbsk_set = HSBK {
                                        hue: hue,
                                        saturation: saturation,
                                        brightness: new_brightness,
                                        kelvin: kelvin,
                                    };
                                    bulb.set_color(&mgr.sock, hbsk_set, duration);
                                }
        
                                if cc.contains("kelvin:"){
                                    let kelvin_split = cc.split("kelvin:");
                                    let kelvin_vec: Vec<&str> = kelvin_split.collect();
                                    let new_kelvin = kelvin_vec[1].to_string().parse::<u16>().unwrap(); 
                                    let hbsk_set = HSBK {
                                        hue: hue,
                                        saturation: 0,
                                        brightness: brightness,
                                        kelvin: new_kelvin,
                                    };
                                    bulb.set_color(&mgr.sock, hbsk_set, duration);
                                }
        
                                if cc.contains("rgb:"){
        
        
                                    let rgb_split = cc.split("rgb:");
                                    let rgb_vec: Vec<&str> = rgb_split.collect();
                                    let rgb_parts = rgb_vec[1].to_string();
        
                                    let rgb_part_split = rgb_parts.split(",");
                                    let rgb_parts_vec: Vec<&str> = rgb_part_split.collect();
        
                                    let red_int = rgb_parts_vec[0].to_string().parse::<i64>().unwrap(); 
                                    let red_float: f32 = (red_int) as f32;
        
                                    let green_int = rgb_parts_vec[1].to_string().parse::<i64>().unwrap(); 
                                    let green_float: f32 = (green_int) as f32;
        
                                    let blue_int = rgb_parts_vec[2].to_string().parse::<i64>().unwrap(); 
                                    let blue_float: f32 = (blue_int) as f32;
        
                                    let rgb = Srgb::new(red_float / 255.0, green_float / 255.0, blue_float / 255.0);
                                    let hcc: Hsv = rgb.into_color();
        
                                    // TODO: Why does this ugly hack work? Why is lifx api so differ
                                    let hbsk_set = HSBK {
                                        hue: (hcc.hue.into_positive_degrees() * 182.0) as u16,
                                        saturation: (hcc.saturation * 65535.0) as u16,
                                        brightness: brightness,
                                        kelvin: kelvin,
                                    };

        
                                    bulb.set_color(&mgr.sock, hbsk_set, duration);
        
                                }
        
                                if cc.contains("#"){
                                    println!("!CC!");
                                    let hex_split = cc.split("#");
                                    let hex_vec: Vec<&str> = hex_split.collect();
                                    let hex = hex_vec[1].to_string();
        
                                    let rgb2 = Rgb::from_hex_str(format!("#{}", hex).as_str()).unwrap();
                                    // Rgb { r: 255.0, g: 204.0, b: 0.0 }
        
                                    println!("{:?}", rgb2);
        
                                    let red_int = rgb2.get_red().to_string().parse::<i64>().unwrap(); 
                                    let red_float: f32 = (red_int) as f32;
        
                                    let green_int = rgb2.get_green().to_string().parse::<i64>().unwrap(); 
                                    let green_float: f32 = (green_int) as f32;
        
                                    let blue_int = rgb2.get_blue().to_string().parse::<i64>().unwrap(); 
                                    let blue_float: f32 = (blue_int) as f32;
        
        
                                    println!("red_float: {:?}", red_float);
                                    println!("green_float: {:?}", green_float);
                                    println!("blue_float: {:?}", blue_float);
        
                    
                                    let rgb = Srgb::new(red_float / 255.0, green_float / 255.0, blue_float / 255.0);
                                    let hcc: Hsv = rgb.into_color();

                                    println!("hcc: {:?}", hcc);
        
                                    // TODO: Why does this ugly hack work? Why is lifx api so differ
                                    let hbsk_set = HSBK {
                                        hue: (hcc.hue.into_positive_degrees() * 182.0) as u16,
                                        saturation: (hcc.saturation * 65535.0) as u16,
                                        brightness: brightness,
                                        kelvin: kelvin,
                                    };

                                    println!("hbsk_set: {:?}", hbsk_set);
        
        
        
                                    bulb.set_color(&mgr.sock, hbsk_set, duration);
        
                                }
        
                            }
                        }
        
        
                        // Brightness
                        if input.brightness.is_some() {
                            let brightness = input.brightness.unwrap();
        
                            for bulb in &bulbs_vec {
        
        
                                let mut kelvin = 6500;
                                let mut saturation = 0;
                                let mut hue = 0;
        
                                let mut duration = 0;
                                if input.duration.is_some(){
                                    duration = input.duration.unwrap() as u32;
                                }
        
                                if bulb.lifx_color.is_some() {
                                    let lifxc = bulb.lifx_color.as_ref().unwrap();
                                    kelvin = lifxc.kelvin;
                                    saturation = lifxc.saturation;
                                    hue = lifxc.hue;
                                }
                                
                                let new_brightness_float = brightness.to_string().parse::<f64>().unwrap(); 
                                let new_brightness: u16 = (f64::from(65535) * new_brightness_float) as u16;
                                let hbsk_set = HSBK {
                                    hue: hue,
                                    saturation: saturation,
                                    brightness: new_brightness,
                                    kelvin: kelvin,
                                };
                                bulb.set_color(&mgr.sock, hbsk_set, duration);
        
                            }
        
                        }
        
                        // Infrared
                        if input.infrared.is_some() {
                            let new_brightness: u16 = (f64::from(65535) * input.infrared.unwrap()) as u16;
        
                            for bulb in &bulbs_vec {
                                bulb.set_infrared(&mgr.sock, new_brightness);
                            }
                        }
        
        
                        // Return results in proper format
                        #[derive(Serialize)]
                        struct SingleStateResult {
                            id: String,
                            label: String,
                            status: String,
                        }

                        #[derive(Serialize)]
                        struct SingleStateResponse {
                            results: Vec<SingleStateResult>,
                        }

                        let mut results = Vec::new();
                        for bulb in &bulbs_vec {
                            results.push(SingleStateResult {
                                id: bulb.id.clone(),
                                label: bulb.label.clone(),
                                status: "ok".to_string(),
                            });
                        }

                        response = Response::json(&SingleStateResponse { results });
        
                    }
        
        
                    // ListLights
                    // https://api.lifx.com/v1/lights/:selector
                    if request.url().contains("/v1/lights/") && !request.url().contains("/state"){
                        response = Response::json(&bulbs_vec.clone());
                    }
        
        
                    // Mutex locks will be automatically dropped when they go out of scope
        
                    return response;
                });
            });


        },
        Err(e) => {
            println!("{:?}", e);
        }
    }










}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    #[test]
    fn test_bulb_info_new() {
        let source = 0x12345678;
        let target = 0xABCDEF123456;
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)), 56700);
        
        let bulb = BulbInfo::new(source, target, addr);
        
        assert_eq!(bulb.source, source);
        assert_eq!(bulb.target, target);
        assert_eq!(bulb.addr, addr);
        assert_eq!(bulb.connected, true);
        assert_eq!(bulb.power, "off");
        assert_eq!(bulb.brightness, 0.0);
        assert!(bulb.lifx_color.is_none());
        assert!(bulb.lifx_group.is_none());
        assert!(bulb.lifx_location.is_none());
        assert!(bulb.product.is_none());
    }

    #[test]
    fn test_bulb_info_update() {
        let source = 0x12345678;
        let target = 0xABCDEF123456;
        let addr1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)), 56700);
        let addr2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 101)), 56700);
        
        let mut bulb = BulbInfo::new(source, target, addr1);
        let initial_last_seen = bulb.last_seen;
        
        // Sleep briefly to ensure time difference
        std::thread::sleep(Duration::from_millis(10));
        
        bulb.update(addr2);
        
        assert_eq!(bulb.addr, addr2);
        assert!(bulb.last_seen > initial_last_seen);
    }

    #[test]
    fn test_refreshable_data_empty() {
        let data: RefreshableData<String> = RefreshableData::empty(
            Duration::from_secs(60),
            Message::GetLabel
        );
        
        assert!(data.data.is_none());
        assert!(data.needs_refresh());
        assert!(data.as_ref().is_none());
    }

    #[test]
    fn test_refreshable_data_update() {
        let mut data: RefreshableData<String> = RefreshableData::empty(
            Duration::from_secs(60),
            Message::GetLabel
        );
        
        data.update("Test Label".to_string());
        
        assert!(data.data.is_some());
        assert!(!data.needs_refresh());
        assert_eq!(data.as_ref().unwrap(), "Test Label");
    }

    #[test]
    fn test_refreshable_data_expiration() {
        let mut data: RefreshableData<String> = RefreshableData::empty(
            Duration::from_millis(10),
            Message::GetLabel
        );
        
        data.update("Test Label".to_string());
        assert!(!data.needs_refresh());
        
        // Wait for expiration
        std::thread::sleep(Duration::from_millis(15));
        assert!(data.needs_refresh());
    }

    #[test]
    fn test_lifx_color_creation() {
        let color = LifxColor {
            hue: 32768,
            saturation: 65535,
            kelvin: 3500,
            brightness: 32768,
        };
        
        assert_eq!(color.hue, 32768);
        assert_eq!(color.saturation, 65535);
        assert_eq!(color.kelvin, 3500);
        assert_eq!(color.brightness, 32768);
    }

    #[test]
    fn test_lifx_group_creation() {
        let group = LifxGroup {
            id: "group123".to_string(),
            name: "Living Room".to_string(),
        };
        
        assert_eq!(group.id, "group123");
        assert_eq!(group.name, "Living Room");
    }

    #[test]
    fn test_lifx_location_creation() {
        let location = LifxLocation {
            id: "loc456".to_string(),
            name: "Home".to_string(),
        };
        
        assert_eq!(location.id, "loc456");
        assert_eq!(location.name, "Home");
    }

    #[test]
    fn test_config_creation() {
        let config = Config {
            secret_key: "test_secret".to_string(),
            port: 8080,
        };
        
        assert_eq!(config.secret_key, "test_secret");
        assert_eq!(config.port, 8080);
    }

    // Color conversion helper function tests
    fn convert_rgb_to_hsbk(red: f32, green: f32, blue: f32) -> (u16, u16) {
        let rgb = Srgb::new(red / 255.0, green / 255.0, blue / 255.0);
        let hcc: Hsv = rgb.into_color();
        
        let hue = (hcc.hue.into_positive_degrees() * 182.0) as u16;
        let saturation = (hcc.saturation * 65535.0) as u16;
        
        (hue, saturation)
    }

    #[test]
    fn test_color_conversion_red() {
        let (hue, saturation) = convert_rgb_to_hsbk(255.0, 0.0, 0.0);
        assert_eq!(hue, 0);
        assert_eq!(saturation, 65535);
    }

    #[test]
    fn test_color_conversion_green() {
        let (hue, saturation) = convert_rgb_to_hsbk(0.0, 255.0, 0.0);
        // Green is at 120 degrees, which maps to approximately 21845 in LIFX scale
        assert!((hue as i32 - 21845).abs() < 100);
        assert_eq!(saturation, 65535);
    }

    #[test]
    fn test_color_conversion_blue() {
        let (hue, saturation) = convert_rgb_to_hsbk(0.0, 0.0, 255.0);
        // Blue is at 240 degrees, which maps to approximately 43690 in LIFX scale
        assert!((hue as i32 - 43690).abs() < 100);
        assert_eq!(saturation, 65535);
    }

    #[test]
    fn test_color_conversion_white() {
        let (_, saturation) = convert_rgb_to_hsbk(255.0, 255.0, 255.0);
        assert_eq!(saturation, 0); // White has no saturation
    }

    // Security tests for authentication
    #[test]
    fn test_rate_limiter_basic() {
        let limiter = RateLimiter::new();
        let client_ip = "192.168.1.1".to_string();
        
        // First attempt should succeed
        assert!(limiter.check_and_update(client_ip.clone()));
        
        // Subsequent attempts within limit should succeed
        for _ in 1..MAX_AUTH_ATTEMPTS {
            assert!(limiter.check_and_update(client_ip.clone()));
        }
        
        // Exceeding limit should fail
        assert!(!limiter.check_and_update(client_ip.clone()));
    }

    #[test]
    fn test_rate_limiter_window_reset() {
        let limiter = RateLimiter::new();
        let client_ip = "192.168.1.2".to_string();
        
        // Fill up the attempts
        for _ in 0..MAX_AUTH_ATTEMPTS {
            assert!(limiter.check_and_update(client_ip.clone()));
        }
        
        // Should be blocked now
        assert!(!limiter.check_and_update(client_ip.clone()));
        
        // Simulate waiting for window to expire
        // Note: In a real test, we'd need to mock time or use a configurable duration
        // For now, we'll test with a different IP
        let client_ip2 = "192.168.1.3".to_string();
        assert!(limiter.check_and_update(client_ip2));
    }

    #[test]
    fn test_rate_limiter_different_ips() {
        let limiter = RateLimiter::new();
        
        // Different IPs should have independent limits
        for i in 0..10 {
            let ip = format!("192.168.1.{}", i);
            assert!(limiter.check_and_update(ip));
        }
    }

    #[test]
    fn test_rate_limiter_cleanup() {
        let limiter = RateLimiter::new();
        
        // Add some entries
        for i in 0..5 {
            let ip = format!("192.168.1.{}", i);
            limiter.check_and_update(ip);
        }
        
        // Cleanup should not affect recent entries
        limiter.cleanup_old_entries();
        
        // Recent entries should still be tracked
        let test_ip = "192.168.1.0".to_string();
        for _ in 1..MAX_AUTH_ATTEMPTS {
            assert!(limiter.check_and_update(test_ip.clone()));
        }
        assert!(!limiter.check_and_update(test_ip));
    }

    #[test]
    fn test_auth_result_enum() {
        // Test that AuthResult can properly hold responses
        let unauth = AuthResult::Unauthorized(Response::text("test"));
        match unauth {
            AuthResult::Unauthorized(_) => assert!(true),
            AuthResult::Authorized => assert!(false, "Should be unauthorized"),
        }
        
        let auth = AuthResult::Authorized;
        match auth {
            AuthResult::Authorized => assert!(true),
            AuthResult::Unauthorized(_) => assert!(false, "Should be authorized"),
        }
    }
}