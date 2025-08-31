# lifx-api-server

## Description
A library/server that mimicks the official LIFX API using the local LAN protocol.

## Offline API Supported Methods:
* List Lights
* Set State
* Set States

## How to use library

Add the following line to your cargo.toml:
```
lifx-api-server = "0.1.15"
```

### Authentication

The server supports flexible authentication configuration:

1. **With Authentication (Production)**: Set the `SECRET_KEY` environment variable
   ```bash
   SECRET_KEY=your-secret-key cargo run
   ```

2. **Development Mode**: Auto-generates a random key
   ```bash
   LIFX_API_MODE=development cargo run
   ```

3. **Without Authentication**: Don't set `SECRET_KEY` (use only in trusted environments)
   ```bash
   cargo run
   ```

### Example:
```rust
extern crate lifx_api_server;

fn main() {
    // Authentication is now optional
    let config = lifx_api_server::Config { 
        secret_key: Some("xxx".to_string()),  // Or None to disable auth
        port: 8089,
        auth_required: true,  // Set to false for public access
    };

    lifx_api_server::start(config);

    println!("sync");

    loop {
        
    }
}


```
### Now you can use curl to access the api:
```curl -X PUT "http://localhost:8089/v1/lights/all/state"      -H "Authorization: Bearer xxx"      -d "color=kelvin:9000"```
### Or with rust using [lifx-rs](https://crates.io/crates/lifx-rs):
```rust
extern crate lifx_rs as lifx;

fn main() {

    let key = "xxx".to_string();
    let mut api_endpoints: Vec<String> = Vec::new();

    api_endpoints.push(format!("http://localhost:8089"));

    let config = lifx::LifxConfig{
        access_token: key.clone(),
        api_endpoints: api_endpoints
    };
    
    let mut off_state = lifx::State::new();
    off_state.power = Some(format!("off"));

    // Turn off all lights
    lifx::Light::set_state_by_selector(config.clone(), format!("all"), off_state);

}
```


## License

Released under Apache 2.0 or MIT.

# Support and follow my work by:

#### Buying my dope NTFs:
 * https://opensea.io/accounts/PixelCoda

#### Checking out my Github:
 * https://github.com/PixelCoda

#### Following my facebook page:
 * https://www.facebook.com/pixelcoda/

#### Subscribing to my Patreon:
 * https://www.patreon.com/calebsmith_pixelcoda

#### Or donating crypto:
 * ADA: addr1qyp299a45tgvveh83tcxlf7ds3yaeh969yt3v882lvxfkkv4e0f46qvr4wzj8ty5c05jyffzq8a9pfwz9dl6m0raac7s4rac48
 * ALGO: VQ5EK4GA3IUTGSPNGV64UANBUVFAIVBXVL5UUCNZSDH544XIMF7BAHEDM4
 * ATOM: cosmos1wm7lummcealk0fxn3x9tm8hg7xsyuz06ul5fw9
 * BTC: bc1qh5p3rff4vxnv23vg0hw8pf3gmz3qgc029cekxz
 * ETH: 0x7A66beaebF7D0d17598d37525e63f524CfD23452
 * ERC20: 0x7A66beaebF7D0d17598d37525e63f524CfD23452
 * XLM: GCJAUMCO2L7PTYMXELQ6GHBTF25MCQKEBNSND2C4QMUPTSVCPEN3LCOG
 * XTZ: tz1SgJppPn56whprsDDGcqR4fxqCr2PXvg1R


#### TODO:
- Server Application Release for debian linux
- Ability to automatically update
- Easy Installer
- Move to the opensam foundation project