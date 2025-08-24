# Project Description - LIFX API Server

## Current Work Summary

### Project Status
This is a Rust library/server that emulates the official LIFX API using the local LAN protocol. The project provides a local HTTP API server that can control LIFX smart bulbs on the local network without requiring internet connectivity.

### Key Components Implemented
1. **LAN Protocol Communication** - UDP socket communication with LIFX bulbs on port 56700
2. **Bulb Discovery** - Automatic discovery of LIFX bulbs on the local network via broadcast messages
3. **HTTP API Server** - RESTful API server running on configurable port (default 8000/8089)
4. **Authentication** - Bearer token authentication for API requests
5. **State Management** - Tracking and caching bulb information including color, brightness, power state, groups, and locations

### API Endpoints Implemented
- **GET /v1/lights/:selector** - List lights matching selector (all, id:xxx, group_id:xxx, location_id:xxx)
- **PUT /v1/lights/:selector/state** - Set state of lights (power, color, brightness, duration, infrared)
- **PUT /v1/lights/states** - Set states for multiple lights (fully implemented with retry logic and validation)

### Color Control Features
- Named colors (white, red, orange, yellow, cyan, green, blue, purple, pink)
- HSB values (hue, saturation, brightness)
- Kelvin temperature
- RGB values
- Hex color codes
- Infrared brightness control

### Technical Implementation Details
- Uses `lifx-rs` library for LIFX protocol messages
- Multi-threaded architecture with separate threads for:
  - UDP message receiving and processing
  - Periodic bulb refresh/discovery
  - HTTP API server
- Mutex-protected shared state for thread-safe bulb information access
- Automatic refresh of bulb information with configurable intervals
- Product information retrieval including multizone support detection

### Dependencies
- lifx-rs (LIFX protocol implementation)
- rouille (HTTP server)
- serde/serde_json (JSON serialization)
- palette/colors-transform (Color space conversions)
- get_if_addrs (Network interface discovery)
- sudo (Privilege escalation for network operations)

### Current Limitations
- Requires sudo privileges for network operations
- No support for LIFX Effects, Scenes, Clean, Cycle operations yet
- No extended API for device label changes or WiFi configuration

### Recent Work
- **SetStates Endpoint Implementation** - Completed full implementation with:
  - Request validation for all parameters
  - Retry logic with exponential backoff (up to 3 attempts)
  - Support for defaults field to apply common settings
  - Detailed per-bulb status reporting
  - Support for all color formats (RGB, hex, HSB, kelvin, named colors)
  - Comprehensive error handling and reporting
- **Test Suite** - Added comprehensive test coverage:
  - 10 unit tests for state validation and parsing
  - 9 integration tests for complex scenarios
  - Tests cover validation, defaults, multiple selectors, and color formats
- **API Documentation** - Created detailed API documentation with examples
- **Code Refactoring** - Modularized SetStates logic into separate module for maintainability