# LIFX API Server - API Documentation

## Overview
This server provides a local implementation of the LIFX HTTP API, allowing control of LIFX bulbs on your local network without requiring internet connectivity.

## Authentication
All API requests require a Bearer token in the Authorization header:
```
Authorization: Bearer YOUR_SECRET_KEY
```

## Base URL
The server runs on a configurable port (default: 8000 or 8089):
```
http://localhost:8000/v1/
```

## Endpoints

### 1. List Lights
**GET** `/v1/lights/:selector`

Returns a list of lights matching the selector.

#### Selectors
- `all` - All lights
- `id:<id>` - Specific light by ID
- `group_id:<group_id>` - All lights in a group
- `location_id:<location_id>` - All lights in a location
- `label:<label>` - Light with specific label
- `group:<name>` - All lights in a named group
- `location:<name>` - All lights in a named location

#### Response
```json
[
  {
    "id": "d073d5127219",
    "uuid": "028b4be5-23a6-48e7-9bb5-8d73a0e1c55a",
    "label": "Kitchen Light",
    "connected": true,
    "power": "on",
    "color": {
      "hue": 120,
      "saturation": 1.0,
      "kelvin": 3500
    },
    "brightness": 0.8,
    "group": {
      "id": "1c8de82b81f445e7cfaafae49b259c71",
      "name": "Kitchen"
    },
    "location": {
      "id": "1d6fe8ef0fde4c6d77b0012dc736662c",
      "name": "Home"
    },
    "product": {
      "name": "LIFX A19",
      "identifier": "lifx_a19",
      "company": "LIFX",
      "capabilities": {
        "has_color": true,
        "has_variable_color_temp": true,
        "has_ir": false,
        "has_chain": false,
        "has_multizone": false,
        "min_kelvin": 1500,
        "max_kelvin": 9000
      }
    },
    "last_seen": "2024-01-15T12:00:00Z",
    "seconds_since_seen": 0
  }
]
```

### 2. Set State
**PUT** `/v1/lights/:selector/state`

Sets the state of lights matching the selector.

#### Request Body
```json
{
  "power": "on",           // "on" or "off"
  "color": "red",          // Color specification (see Color Formats below)
  "brightness": 0.8,       // 0.0 to 1.0
  "duration": 2.0,         // Transition duration in seconds
  "infrared": 0.5,         // 0.0 to 1.0 (for IR-capable bulbs)
  "fast": false            // Skip response for faster execution
}
```

#### Color Formats
- **Named colors**: `"white"`, `"red"`, `"orange"`, `"yellow"`, `"cyan"`, `"green"`, `"blue"`, `"purple"`, `"pink"`
- **Kelvin**: `"kelvin:3500"` (1500-9000)
- **Hue**: `"hue:120"` (0-360 degrees)
- **Saturation**: `"saturation:0.5"` (0.0-1.0)
- **Brightness**: `"brightness:0.8"` (0.0-1.0)
- **RGB**: `"rgb:255,128,0"` (0-255 for each component)
- **Hex**: `"#FF8000"`
- **HSB combination**: `"hue:120 saturation:1.0 brightness:0.5"`

#### Response
```json
{
  "results": [
    {
      "id": "d073d5127219",
      "status": "ok",
      "label": "Kitchen Light"
    }
  ]
}
```

### 3. Set States (Bulk Operation)
**PUT** `/v1/lights/states`

Sets the state of multiple groups of lights in a single request. This endpoint supports bulk operations with retry logic and detailed error reporting.

#### Request Body
```json
{
  "states": [
    {
      "selector": "group_id:bedroom",
      "power": "off",
      "duration": 2.0
    },
    {
      "selector": "group_id:living_room",
      "power": "on",
      "color": "white",
      "brightness": 1.0
    },
    {
      "selector": "label:Kitchen",
      "color": "kelvin:2700",
      "brightness": 0.6
    }
  ],
  "defaults": {
    "power": "on",
    "brightness": 0.5,
    "duration": 1.0
  }
}
```

#### Features
- **Atomic Operations**: All state changes are validated before execution
- **Retry Logic**: Failed operations are automatically retried up to 3 times with exponential backoff
- **Request Validation**: All parameters are validated before execution
- **Defaults Support**: Common values can be specified in the `defaults` field
- **Detailed Error Reporting**: Each bulb operation returns individual status

#### Response
```json
{
  "results": [
    {
      "id": "d073d5127219",
      "label": "Bedroom Light",
      "status": "ok"
    },
    {
      "id": "d073d5127220",
      "label": "Living Room Light",
      "status": "ok"
    },
    {
      "id": "d073d5127221",
      "label": "Kitchen",
      "status": "error",
      "error": "Attempt 3: Failed to set color: Device not responding"
    }
  ]
}
```

#### Validation Rules
- `power` must be "on" or "off"
- `brightness` must be between 0.0 and 1.0
- `infrared` must be between 0.0 and 1.0
- `duration` must be between 0 and 3155760000 seconds
- `selector` must follow valid selector format
- `color` must follow valid color format

## Error Handling

### HTTP Status Codes
- `200 OK` - Request succeeded
- `400 Bad Request` - Invalid request format or parameters
- `401 Unauthorized` - Missing or invalid authorization token
- `404 Not Found` - No lights match the selector
- `500 Internal Server Error` - Server error

### Error Response Format
```json
{
  "error": "Validation failed",
  "message": "brightness must be between 0.0 and 1.0"
}
```

## Performance Considerations

### SetStates Endpoint
- Processes bulb updates sequentially to ensure stability
- Implements retry logic with exponential backoff (100ms, 200ms, 400ms)
- Validates all requests before execution
- Returns detailed status for each bulb operation

### Best Practices
1. Use `fast: true` for time-critical operations
2. Group related state changes in a single SetStates request
3. Use appropriate selectors to minimize affected bulbs
4. Set reasonable duration values for smooth transitions

## Example Usage

### Turn on all lights
```bash
curl -X PUT http://localhost:8000/v1/lights/all/state \
  -H "Authorization: Bearer YOUR_SECRET_KEY" \
  -H "Content-Type: application/json" \
  -d '{"power": "on"}'
```

### Set multiple light groups with different colors
```bash
curl -X PUT http://localhost:8000/v1/lights/states \
  -H "Authorization: Bearer YOUR_SECRET_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "states": [
      {
        "selector": "group:bedroom",
        "power": "on",
        "color": "kelvin:2700",
        "brightness": 0.3
      },
      {
        "selector": "group:living_room",
        "power": "on",
        "color": "white",
        "brightness": 0.8
      }
    ],
    "defaults": {
      "duration": 2.0
    }
  }'
```

### Fade lights to red over 5 seconds
```bash
curl -X PUT http://localhost:8000/v1/lights/all/state \
  -H "Authorization: Bearer YOUR_SECRET_KEY" \
  -H "Content-Type: application/json" \
  -d '{"color": "red", "duration": 5.0}'
```

## Limitations
- Maximum 100 bulbs per SetStates request (recommended)
- Concurrent requests are processed sequentially
- Some LIFX cloud features (scenes, effects) are not yet implemented