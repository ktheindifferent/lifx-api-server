# Device Discovery Refresh Features

## Overview
This document describes the enhanced device discovery refresh functionality added to the LIFX API Server.

## Features

### 1. Automatic Discovery Refresh
The server now automatically refreshes device discovery at configurable intervals to detect new devices on the network.

**Configuration:**
- `DISCOVERY_REFRESH_INTERVAL`: Set the interval in seconds (default: 300 seconds / 5 minutes)
- `AUTO_DISCOVERY_ENABLED`: Enable/disable automatic discovery (default: true)

**Example:**
```bash
export DISCOVERY_REFRESH_INTERVAL=300
export AUTO_DISCOVERY_ENABLED=true
cargo run
```

### 2. Manual Discovery Endpoint
Trigger device discovery manually via the API.

**Endpoint:** `POST /v1/discover`

**Example:**
```bash
curl -X POST "http://localhost:8000/v1/discover" \
     -H "Authorization: Bearer YOUR_SECRET_KEY" \
     -H "Content-Type: application/json"
```

**Response:**
```json
{
  "status": "success",
  "message": "Device discovery completed successfully"
}
```

### 3. Discovery Metrics Endpoint
Monitor discovery statistics and health.

**Endpoint:** `GET /v1/discover/metrics`

**Example:**
```bash
curl -X GET "http://localhost:8000/v1/discover/metrics" \
     -H "Authorization: Bearer YOUR_SECRET_KEY"
```

**Response:**
```json
{
  "total_discoveries": 10,
  "successful_discoveries": 9,
  "failed_discoveries": 1,
  "last_discovery_time": "2025-08-31T12:34:56Z",
  "last_discovery_status": "success",
  "devices_discovered": 5
}
```

## Error Handling
- Discovery failures are logged but don't interrupt service operation
- The server continues to function normally even if discovery fails
- Failed discoveries are tracked in metrics for monitoring

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `DISCOVERY_REFRESH_INTERVAL` | Seconds between automatic discovery refreshes | 300 |
| `AUTO_DISCOVERY_ENABLED` | Enable/disable automatic discovery | true |
| `SECRET_KEY` | API authentication key | (required) |

## Logging
Discovery events are logged at the INFO level:
- "Device discovery refreshed successfully" - Successful auto-refresh
- "Manual device discovery requested" - Manual discovery triggered
- "Discovery refresh failed: ..." - Discovery failure (with reason)

Enable logging with:
```bash
export RUST_LOG=info
```

## Testing
Use the provided test script to verify the discovery features:
```bash
./test_discovery.sh
```

## Implementation Details

### Code Structure
- **Manager struct**: Extended with `discovery_metrics` field for tracking
- **DiscoveryMetrics struct**: Contains discovery statistics
- **discover() method**: Updated to track metrics
- **Auto-refresh loop**: Runs in background thread with error handling
- **API endpoints**: Added `/v1/discover` and `/v1/discover/metrics`

### Safety Features
- Mutex protection for concurrent access
- Graceful error handling prevents service disruption
- Configurable intervals prevent excessive network traffic
- Metrics tracking for monitoring and alerting

## Future Enhancements
The following features are planned but not yet implemented:
- WebSocket notifications for device changes
- Incremental discovery to minimize network impact
- Device change events (added/removed/updated)
- Discovery rate limiting
- Per-device discovery history