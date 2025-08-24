# Color Conversion Magic Number Fix

## Summary
Fixed the undocumented magic number `182.0` used in LIFX color conversions by replacing it with properly documented constants based on the LIFX protocol specification.

## Changes Made

### 1. Added Color Conversion Constants
```rust
// LIFX Protocol Color Conversion Constants
const LIFX_HUE_MAX: f32 = 65536.0; // 0x10000 for consistent rounding
const LIFX_HUE_DEGREE_FACTOR: f32 = LIFX_HUE_MAX / 360.0; // Converts degrees to u16
const LIFX_SATURATION_MAX: f32 = 65535.0; // 0xFFFF
const LIFX_BRIGHTNESS_MAX: f32 = 65535.0; // 0xFFFF

// Pre-calculated LIFX hue values for named colors
const HUE_RED: u16 = 0;        // 0°
const HUE_ORANGE: u16 = 7099;  // ~39°
const HUE_YELLOW: u16 = 10922; // 60°
const HUE_GREEN: u16 = 21845;  // 120°
const HUE_CYAN: u16 = 32768;   // 180°
const HUE_BLUE: u16 = 43690;   // 240°
const HUE_PURPLE: u16 = 50062; // ~275°
const HUE_PINK: u16 = 63715;   // ~350°
```

### 2. Updated Color Conversion Formula
**Before:**
```rust
hue: (hcc.hue.into_positive_degrees() * 182.0) as u16
```

**After:**
```rust
hue: ((hcc.hue.into_positive_degrees() * LIFX_HUE_DEGREE_FACTOR) as u32 % 0x10000) as u16
```

### 3. Key Improvements
- **Documentation**: Clear explanation of why the conversion factor exists
- **Accuracy**: Using 65536/360 instead of approximation (182.0)
- **Consistency**: All color conversions now use named constants
- **Correctness**: Added modulo operation to properly handle 360° wrapping
- **Testing**: Added comprehensive unit tests for color conversions

## Technical Details

### Why 182.0?
The magic number `182.0` was an approximation of `65535/360 = 182.04166...`

### LIFX Protocol Specification
- LIFX represents HSBK values as 16-bit unsigned integers (0-65535)
- Standard HSB uses: Hue (0-360°), Saturation (0-100%), Brightness (0-100%)
- LIFX recommends using 0x10000 (65536) for hue conversion for better rounding

### Conversion Formulas
- **Hue**: `degrees * 65536 / 360` (with modulo 65536 for wrapping)
- **Saturation**: `percentage * 65535`
- **Brightness**: `percentage * 65535`

## Testing
All 23 unit tests pass, including new comprehensive color conversion tests that verify:
- Primary colors (RGB)
- Secondary colors (CMY)
- Named colors (orange, purple, pink)
- Boundary conditions (0°, 360°, grayscale)
- Conversion accuracy

## Impact
- No breaking changes to the API
- More accurate color representation
- Better alignment with LIFX protocol specification
- Improved code maintainability