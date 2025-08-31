# Extended Device Management API Documentation

## Overview
This document describes the extended device management API endpoints for LIFX smart lighting control. These endpoints provide advanced configuration and management capabilities for LIFX devices.

## Authentication
All endpoints require authentication via Bearer token in the Authorization header:
```
Authorization: Bearer <your-secret-key>
```

### Elevated Permissions
Some sensitive operations (WiFi configuration, device reboot) require elevated permissions. These endpoints require an additional header:
```
X-LIFX-Elevated-Token: ELEVATED-<token>
```

## Rate Limiting
- **Authentication failures**: Maximum 5 attempts per 60 seconds per IP
- **Configuration changes**: Maximum 5 changes per 5 minutes per IP
- Rate limit exceeded returns HTTP 429 with `Retry-After` header

## API Endpoints

### 1. Change Device Label
**Endpoint**: `PUT /v1/lights/:selector/label`

**Description**: Changes the label (name) of one or more LIFX devices.

**Request Body**:
```json
{
  "label": "Living Room Light"
}
```

**Constraints**:
- Label must be 32 characters or less
- Subject to configuration change rate limiting

**Response**:
```json
{
  "results": [
    {
      "id": "d073d5123456",
      "label": "Living Room Light",
      "status": "ok",
      "message": null
    }
  ]
}
```

**Example**:
```bash
curl -X PUT "http://localhost:8089/v1/lights/all/label" \
  -H "Authorization: Bearer your-secret-key" \
  -H "Content-Type: application/json" \
  -d '{"label": "Office Lamp"}'
```

### 2. Get Device Configuration
**Endpoint**: `GET /v1/lights/:selector/config`

**Description**: Retrieves detailed configuration information for one or more LIFX devices.

**Response**:
```json
[
  {
    "id": "d073d5123456",
    "label": "Living Room Light",
    "group": "Living Room",
    "location": "Home",
    "product": {
      "name": "LIFX A19",
      "vendor": "LIFX",
      "product_id": 49,
      "capabilities": {
        "has_color": true,
        "has_variable_color_temp": true,
        "has_ir": false,
        "has_chain": false,
        "has_matrix": false,
        "has_multizone": false,
        "min_kelvin": 2500,
        "max_kelvin": 9000
      }
    },
    "version": {
      "major": 3,
      "minor": 70,
      "build": 0
    },
    "wifi": {
      "ssid": "HomeNetwork",
      "signal_strength": -45,
      "rssi": -45,
      "security_type": "WPA2",
      "ipv4_address": "192.168.1.100",
      "ipv6_address": null
    },
    "uptime": 3600,
    "host_info": {
      "uptime_seconds": 3600,
      "downtime_seconds": 0,
      "last_seen": "2024-01-15T10:30:00Z"
    }
  }
]
```

**Example**:
```bash
curl -X GET "http://localhost:8089/v1/lights/all/config" \
  -H "Authorization: Bearer your-secret-key"
```

### 3. Update WiFi Settings (Elevated Permissions Required)
**Endpoint**: `PUT /v1/lights/:selector/wifi`

**Description**: Updates WiFi configuration for one or more LIFX devices. Requires elevated permissions.

**Request Body**:
```json
{
  "ssid": "NewNetwork",
  "pass": "SecurePassword123",
  "security": 3
}
```

**Security Types**:
- 0: Open (no security)
- 1: WEP
- 2: WPA
- 3: WPA2
- 4: WPA/WPA2

**Constraints**:
- SSID must be 1-32 characters
- Password must be 64 characters or less
- Requires elevated permissions header
- Subject to configuration change rate limiting

**Response**:
```json
{
  "results": [
    {
      "id": "d073d5123456",
      "label": "Living Room Light",
      "status": "ok",
      "message": null
    }
  ]
}
```

**Example**:
```bash
curl -X PUT "http://localhost:8089/v1/lights/all/wifi" \
  -H "Authorization: Bearer your-secret-key" \
  -H "X-LIFX-Elevated-Token: ELEVATED-admin-token" \
  -H "Content-Type: application/json" \
  -d '{"ssid": "NewNetwork", "pass": "SecurePass", "security": 3}'
```

### 4. Reboot Device (Elevated Permissions Required)
**Endpoint**: `POST /v1/lights/:selector/reboot`

**Description**: Reboots one or more LIFX devices. Requires elevated permissions.

**Request Body** (optional):
```json
{
  "delay": 30
}
```

**Parameters**:
- `delay`: Optional delay in seconds before reboot (default: 0)

**Response**:
```json
{
  "results": [
    {
      "id": "d073d5123456",
      "label": "Living Room Light",
      "status": "rebooting",
      "message": null
    }
  ]
}
```

**Example**:
```bash
curl -X POST "http://localhost:8089/v1/lights/all/reboot" \
  -H "Authorization: Bearer your-secret-key" \
  -H "X-LIFX-Elevated-Token: ELEVATED-admin-token" \
  -H "Content-Type: application/json" \
  -d '{"delay": 10}'
```

### 5. Get Extended Device Information
**Endpoint**: `GET /v1/lights/:selector/info`

**Description**: Retrieves comprehensive information about one or more LIFX devices, including configuration, capabilities, network status, and firmware details.

**Response**:
```json
[
  {
    "id": "d073d5123456",
    "uuid": "123e4567-e89b-12d3-a456-426614174000",
    "label": "Living Room Light",
    "connected": true,
    "power": "on",
    "color": {
      "hue": 120,
      "saturation": 100,
      "kelvin": 3500,
      "brightness": 80
    },
    "brightness": 0.8,
    "group": {
      "id": "group123",
      "name": "Living Room"
    },
    "location": {
      "id": "loc456",
      "name": "Home"
    },
    "product": {
      "name": "LIFX A19",
      "vendor": "LIFX",
      "product_id": 49,
      "capabilities": {
        "has_color": true,
        "has_variable_color_temp": true,
        "has_ir": false,
        "has_chain": false,
        "has_matrix": false,
        "has_multizone": false,
        "min_kelvin": 2500,
        "max_kelvin": 9000
      }
    },
    "last_seen": "2024-01-15T10:30:00Z",
    "config": {
      "id": "d073d5123456",
      "label": "Living Room Light",
      "group": "Living Room",
      "location": "Home",
      "product": {...},
      "version": {...},
      "wifi": {...},
      "uptime": 3600,
      "host_info": {...}
    },
    "capabilities": {
      "has_color": true,
      "has_variable_color_temp": true,
      "has_ir": false,
      "has_chain": false,
      "has_matrix": false,
      "has_multizone": false,
      "min_kelvin": 2500,
      "max_kelvin": 9000
    },
    "network": {
      "ssid": "HomeNetwork",
      "signal_strength": -45,
      "rssi": -45,
      "security_type": "WPA2",
      "ipv4_address": "192.168.1.100",
      "ipv6_address": null
    },
    "firmware": {
      "major": 3,
      "minor": 70,
      "build": 0
    }
  }
]
```

**Example**:
```bash
curl -X GET "http://localhost:8089/v1/lights/all/info" \
  -H "Authorization: Bearer your-secret-key"
```

## Selectors
All endpoints support LIFX selectors to target specific devices:
- `all` - All devices
- `label:<name>` - Device with specific label
- `id:<id>` - Device with specific ID
- `group_id:<id>` - All devices in a group
- `location_id:<id>` - All devices in a location

## Error Responses
All endpoints may return the following error responses:

### 400 Bad Request
```json
{
  "error": "Invalid request format or parameters"
}
```

### 401 Unauthorized
```json
{
  "error": "Unauthorized: Invalid token"
}
```

### 403 Forbidden
```json
{
  "error": "Elevated permissions required for this operation"
}
```

### 429 Too Many Requests
```json
{
  "error": "Too many requests. Please try again later."
}
```
Headers: `Retry-After: 60` or `Retry-After: 300`

### 500 Internal Server Error
```json
{
  "error": "Internal server error"
}
```

## Security Considerations

1. **Authentication**: Always use HTTPS in production to protect Bearer tokens
2. **Elevated Permissions**: Store elevated tokens securely and rotate regularly
3. **Rate Limiting**: Implement client-side retry logic with exponential backoff
4. **WiFi Credentials**: Never log or expose WiFi passwords in responses
5. **Audit Logging**: All configuration changes should be logged for security auditing

## Implementation Notes

1. **Label Changes**: Changes are sent via LIFX LAN protocol Message type 24 (SetLabel)
2. **WiFi Configuration**: Currently returns a placeholder error - requires full LIFX protocol implementation for Message type 305 (SetAccessPoint)
3. **Device Reboot**: Uses power cycling as a workaround - actual reboot message (type 38) needs implementation
4. **Configuration Data**: Some fields may return placeholder values if device querying is not fully implemented
5. **Network Information**: WiFi details require additional LIFX protocol messages to retrieve actual values

## Testing

### Unit Tests
Run the test suite with:
```bash
cargo test
```

Tests cover:
- Rate limiting for authentication and configuration changes
- Request structure validation
- Label and WiFi configuration validation
- Elevated permissions checking
- Cleanup routines

### Integration Testing
Example test sequence:
```bash
# 1. Get current device configuration
curl -X GET "http://localhost:8089/v1/lights/all/config" \
  -H "Authorization: Bearer test-key"

# 2. Change device label
curl -X PUT "http://localhost:8089/v1/lights/all/label" \
  -H "Authorization: Bearer test-key" \
  -H "Content-Type: application/json" \
  -d '{"label": "Test Light"}'

# 3. Get extended information
curl -X GET "http://localhost:8089/v1/lights/all/info" \
  -H "Authorization: Bearer test-key"

# 4. Test rate limiting (run 6 times quickly)
for i in {1..6}; do
  curl -X PUT "http://localhost:8089/v1/lights/all/label" \
    -H "Authorization: Bearer test-key" \
    -H "Content-Type: application/json" \
    -d '{"label": "Test '$i'"}'
done

# 5. Test elevated permissions requirement
curl -X POST "http://localhost:8089/v1/lights/all/reboot" \
  -H "Authorization: Bearer test-key" \
  -H "Content-Type: application/json" \
  -d '{"delay": 0}'
```

## Future Enhancements

1. **Full LIFX Protocol Support**: Implement complete message types for WiFi configuration and device reboot
2. **WebSocket Support**: Real-time device status updates
3. **Batch Operations**: Atomic updates across multiple devices
4. **Firmware Updates**: API endpoint for OTA firmware updates
5. **Scene Integration**: Integrate device configuration with scene management
6. **Metrics and Monitoring**: Prometheus-compatible metrics endpoint
7. **OpenAPI/Swagger**: Auto-generated API documentation
8. **GraphQL Interface**: Alternative query interface for complex device queries