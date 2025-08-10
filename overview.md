# LIFX API Server - High Level Overview

## Project Purpose
A Rust-based server that provides a local HTTP API interface for controlling LIFX smart bulbs using the LAN protocol, eliminating the need for cloud connectivity while maintaining API compatibility with the official LIFX API.

## Architecture Overview

```
┌─────────────────┐         ┌──────────────────┐
│   HTTP Client   │────────▶│  HTTP API Server │
│  (curl, apps)   │         │   (Port 8000)    │
└─────────────────┘         └──────────────────┘
                                     │
                                     ▼
                            ┌──────────────────┐
                            │   Auth Layer     │
                            │  (Bearer Token)  │
                            └──────────────────┘
                                     │
                                     ▼
                            ┌──────────────────┐
                            │  Request Router  │
                            │  & State Manager │
                            └──────────────────┘
                                     │
                    ┌────────────────┼────────────────┐
                    ▼                                  ▼
         ┌──────────────────┐              ┌──────────────────┐
         │  Bulb Discovery  │              │  Bulb Control    │
         │    Thread        │              │    Thread        │
         └──────────────────┘              └──────────────────┘
                    │                                  │
                    └────────────────┬─────────────────┘
                                     ▼
                            ┌──────────────────┐
                            │   UDP Socket     │
                            │  (Port 56700)    │
                            └──────────────────┘
                                     │
                    ┌────────────────┼────────────────┐
                    ▼                ▼                ▼
              ┌──────────┐    ┌──────────┐    ┌──────────┐
              │  LIFX    │    │  LIFX    │    │  LIFX    │
              │  Bulb 1  │    │  Bulb 2  │    │  Bulb N  │
              └──────────┘    └──────────┘    └──────────┘
```

## Core Components

### 1. Manager Structure
- Central orchestrator for all bulb operations
- Maintains HashMap of discovered bulbs with their current state
- Handles UDP socket communication
- Coordinates discovery and refresh cycles

### 2. BulbInfo Structure
- Represents individual LIFX bulb state
- Tracks:
  - Identity (ID, UUID, MAC address)
  - State (power, color, brightness)
  - Metadata (label, group, location, product info)
  - Network info (IP address, last seen timestamp)
- Implements RefreshableData pattern for efficient caching

### 3. Threading Model
- **Main Thread**: Initializes manager and starts worker threads
- **UDP Receiver Thread**: Continuously listens for bulb responses
- **Refresh Thread**: Periodically queries bulbs for updated state
- **HTTP Server Thread**: Handles incoming API requests

### 4. Network Communication
- **Discovery**: Broadcast UDP packets to find bulbs on LAN
- **Control**: Targeted UDP messages to specific bulbs
- **State Queries**: Request current state from bulbs
- **Response Processing**: Parse and update internal state

## Data Flow

1. **Client Request** → HTTP API endpoint receives request
2. **Authentication** → Validates Bearer token
3. **Request Parsing** → Extracts selector and parameters
4. **Bulb Selection** → Filters bulbs based on selector
5. **Command Building** → Creates LIFX protocol message
6. **UDP Transmission** → Sends message to bulb(s)
7. **Response Collection** → Receives acknowledgments
8. **State Update** → Updates internal bulb state
9. **HTTP Response** → Returns result to client

## Key Design Patterns

### RefreshableData Pattern
- Caches bulb information with expiration times
- Reduces network traffic by avoiding unnecessary queries
- Automatically refreshes stale data when needed

### Message Building Pattern
- BuildOptions structure for consistent message construction
- Type-safe message creation using enum variants
- Automatic serialization to binary protocol format

### Selector Pattern
- Flexible bulb targeting (all, by ID, by group, by location)
- Consistent with official LIFX API selectors
- Extensible for future selector types

## Security Considerations
- Bearer token authentication required for all API calls
- Runs with elevated privileges for network operations
- No external network dependencies (fully local)
- Token stored in environment variable

## Performance Characteristics
- Sub-second bulb discovery on local network
- Millisecond response times for cached data
- Configurable refresh intervals
- Thread-safe concurrent operations
- Efficient binary protocol communication

## Extensibility Points
- Additional API endpoints can be added to router
- New color formats can be implemented
- Effect and scene support can be added
- Extended device configuration APIs possible
- Plugin architecture for custom behaviors