# TODO List - LIFX API Server

## High Priority - Core Functionality

### API Endpoints
- [ ] **Implement SetStates endpoint** - Multiple bulb state changes in one request (partially started at line 728)
- [ ] **Add response formatting** - Return proper JSON responses for state changes (line 1080-1089)
- [ ] **Implement error handling** - Return proper error responses instead of empty 404s

### Missing LIFX Features (from comments in code)
- [ ] **Implement LIFX Effects** - Pulse, breathe, etc.
- [ ] **Implement Scenes support** - Save and recall lighting scenes
- [ ] **Implement Clean operation** - LIFX clean cycle for antibacterial lights
- [ ] **Implement Cycle operation** - Cycle through colors/states

### Extended API Features
- [ ] **Device label modification** - API to change bulb names
- [ ] **WiFi configuration API** - Configure bulb network settings
- [ ] **Firmware info endpoint** - Expose firmware version information

## Testing Infrastructure

### Unit Tests Needed
- [ ] Test `BulbInfo::new()` - Verify proper initialization
- [ ] Test `BulbInfo::update()` - Verify state updates
- [ ] Test `BulbInfo::refresh_if_needed()` - Test refresh logic
- [ ] Test `BulbInfo::set_power()` - Power control testing
- [ ] Test `BulbInfo::set_color()` - Color control testing
- [ ] Test `BulbInfo::set_infrared()` - Infrared control testing
- [ ] Test `RefreshableData` - Cache expiration and refresh
- [ ] Test color conversion functions - RGB/HSB/Hex conversions
- [ ] Test selector parsing - Verify all selector types work

### Integration Tests Needed
- [ ] Test bulb discovery process
- [ ] Test HTTP API authentication
- [ ] Test concurrent requests handling
- [ ] Test state persistence across requests
- [ ] Test error scenarios (network failures, invalid bulbs)

## Code Quality Improvements

### Refactoring
- [ ] **Extract color parsing logic** - Move color parsing to separate module (lines 766-1027)
- [ ] **Improve error handling** - Replace unwrap() calls with proper error handling
- [ ] **Reduce code duplication** - Color setting code is repetitive
- [ ] **Extract HTTP routing** - Move route handling to separate functions

### Documentation
- [ ] Add module-level documentation
- [ ] Document public API functions
- [ ] Add examples for each endpoint
- [ ] Create API usage guide
- [ ] Document configuration options

### Performance
- [ ] **Optimize mutex usage** - Reduce lock contention in hot paths
- [ ] **Implement connection pooling** - Reuse UDP sockets efficiently
- [ ] **Add request caching** - Cache frequent requests
- [ ] **Batch UDP messages** - Send multiple messages in one packet where possible

## Bug Fixes

### Known Issues
- [ ] **Memory management** - Unnecessary drops in HTTP handler (lines 1104-1107)
- [ ] **Magic number in RGB conversion** - Investigate why * 182.0 works (line 966)
- [ ] **Saturation calculation issue** - Why * 1000.0 for saturation (line 967)
- [ ] **Discovery timer** - Re-enable periodic discovery (commented out at line 638)

### Security
- [ ] **Validate input data** - Add input validation for all parameters
- [ ] **Rate limiting** - Prevent API abuse
- [ ] **Sanitize bulb labels** - Prevent injection attacks
- [ ] **Secure token storage** - Improve secret key management

## Deployment & Distribution

### From README TODO section
- [ ] **Server Application Release** - Create debian package
- [ ] **Auto-update capability** - Implement self-updating mechanism
- [ ] **Easy installer** - Create installation script
- [ ] **OpenSAM Foundation migration** - Move to foundation project

## Future Enhancements

### Advanced Features
- [ ] WebSocket support for real-time updates
- [ ] GraphQL API alternative
- [ ] Prometheus metrics endpoint
- [ ] Admin dashboard UI
- [ ] Bulb grouping management
- [ ] Scheduling system
- [ ] Home Assistant integration
- [ ] MQTT bridge
- [ ] Multi-user support
- [ ] Backup/restore configuration

## Current Sprint Focus

1. **Add comprehensive test suite** - No tests currently exist
2. **Complete SetStates endpoint** - Partially implemented
3. **Fix response formatting** - Return proper API responses
4. **Refactor color parsing** - Extract to dedicated module
5. **Add input validation** - Security and stability

## Completed Tasks

- [x] Analyze project structure
- [x] Create project_description.md
- [x] Create overview.md
- [x] Create todo.md
- [x] Identify functions needing tests
- [x] Document partially implemented features