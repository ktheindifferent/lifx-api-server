# LIFX API Server - Codebase Documentation

## Project Overview
**Name:** lifx-api-server  
**Version:** 0.1.15  
**Language:** Rust (Edition 2018)  
**Purpose:** A local HTTP API server that mimics the official LIFX API using the LAN protocol, enabling control of LIFX smart bulbs without cloud connectivity.

## Repository Structure
```
/root/repo/
├── Cargo.toml          # Rust package manifest with dependencies
├── Cargo.lock          # Dependency lock file
├── Dockerfile          # Container configuration
├── README.md           # User-facing documentation
├── overview.md         # High-level architecture documentation
├── project_description.md  # Project details
├── todo.md             # Development tasks
└── src/
    ├── main.rs         # Binary entry point (server executable)
    └── lib.rs          # Library implementation with core logic
```

## Core Dependencies
- **lifx-rs** (0.1.30): LIFX protocol implementation library
- **rouille** (3.6.2): HTTP server framework
- **serde** (1.0): Serialization/deserialization with derive features
- **serde_json** (1.0.96): JSON parsing and generation
- **palette** (0.7): Color space conversions
- **colors-transform** (0.2.11): Additional color transformations
- **get_if_addrs** (0.5.3): Network interface discovery
- **rand** (0.8.4): Random number generation
- **failure** (0.1.8): Error handling
- **sudo** (0.6.0): Privilege escalation for network operations

## Architecture Components

### Main Entry Point (`src/main.rs`)
- Handles environment variable loading for SECRET_KEY
- Requests elevated privileges for network operations
- Initializes server configuration on port 8000
- Starts the API server and maintains infinite loop

### Core Library (`src/lib.rs`)

#### Key Data Structures
1. **RefreshableData<T>** (lines 35-63)
   - Generic caching mechanism with expiration
   - Tracks last update time and max age
   - Automatically triggers refresh when stale

2. **BulbInfo** (lines 66-124)
   - Complete representation of a LIFX bulb state
   - Includes identity, state, network, and metadata
   - Serializable for API responses

3. **Manager** (lines 345-611)
   - Central coordinator for all bulb operations
   - Maintains HashMap of discovered bulbs
   - Handles UDP socket communication
   - Thread-safe with Arc<Mutex> pattern

4. **Config** (lines 613-617)
   - Server configuration structure
   - Contains secret_key for authentication
   - Configurable port number

#### Supporting Structures
- **LifxLocation** (line 126): Location grouping information
- **LifxColor** (line 134): Color representation with HSB values
- **LifxGroup** (line 144): Group metadata for bulbs

### API Endpoints

#### Implemented Endpoints
1. **GET /v1/lights/:selector**
   - Lists lights matching the selector
   - Returns array of BulbInfo objects

2. **PUT /v1/lights/:selector/state**
   - Sets state for matching lights
   - Supports power, color, brightness, duration parameters

3. **PUT /v1/lights/states**
   - Batch operation for multiple bulb state changes
   - Accepts array of state changes with selectors

#### Authentication
- Bearer token authentication required
- Token passed via `Authorization: Bearer <token>` header
- Configured through SECRET_KEY environment variable

### Threading Model
1. **Main Thread**: Server initialization and coordination
2. **UDP Receiver Thread**: Continuous listening for bulb responses
3. **Refresh Thread**: Periodic state updates from bulbs
4. **HTTP Server Thread**: Request handling via Rouille

### Network Communication
- **Protocol**: LIFX LAN protocol over UDP
- **Discovery Port**: 56700 (standard LIFX port)
- **HTTP API Port**: 8000 (configurable)
- **Broadcast**: Used for bulb discovery
- **Unicast**: Direct communication with known bulbs

## Color Support
- Kelvin temperature (2500-9000K)
- RGB hex codes (#RRGGBB)
- HSB (Hue, Saturation, Brightness) values
- Named colors via color transformation libraries

## Selectors
- `all`: Target all discovered bulbs
- `id:<bulb_id>`: Target specific bulb by ID
- `label:<name>`: Target bulb by label
- `group_id:<id>`: Target bulbs in group
- `location_id:<id>`: Target bulbs in location

## Build & Run Instructions

### Development
```bash
# Set environment variable
export SECRET_KEY="your-secret-key"

# Build the project
cargo build

# Run the server
cargo run
```

### Production
```bash
# Build release version
cargo build --release

# Run with elevated privileges
sudo SECRET_KEY="your-secret-key" ./target/release/lifx-api-server
```

### Docker
```bash
# Build image
docker build -t lifx-api-server .

# Run container
docker run -e SECRET_KEY="your-secret-key" --network=host lifx-api-server
```

## Testing
```bash
# Test API with curl
curl -X GET "http://localhost:8000/v1/lights/all" \
     -H "Authorization: Bearer your-secret-key"

curl -X PUT "http://localhost:8000/v1/lights/all/state" \
     -H "Authorization: Bearer your-secret-key" \
     -d "power=on&color=kelvin:5000&brightness=0.8"
```

## Future Enhancements (from TODO)
- [ ] Implement LIFX Effects API
- [ ] Implement Scenes API
- [ ] Implement Clean API
- [ ] Implement Cycle API
- [ ] Extended API for device configuration (labels, WiFi config)
- [ ] Debian package release
- [ ] Auto-update capability
- [ ] Easy installer script
- [ ] Migration to OpenSAM foundation

## Security Notes
- Requires elevated privileges for network operations
- Authentication via environment variable
- No external network dependencies (fully local)
- Runs on local network only

## Performance Characteristics
- Sub-second bulb discovery
- Millisecond response times for cached data
- Thread-safe concurrent operations
- Efficient binary protocol communication
- Configurable cache expiration (default: 1 hour)

## Error Handling
- Uses `failure` crate for error management
- HTTP 400 for bad requests
- HTTP 401 for authentication failures
- Graceful handling of network timeouts
- Automatic retry for failed bulb communications

## License
Dual-licensed under MIT OR Apache-2.0