#!/usr/bin/env python3
"""
Verify LIFX color conversion factors.
This script demonstrates why the magic number 182.0 was used
and confirms the correct conversion factor.
"""

def main():
    print("LIFX Color Conversion Factor Analysis")
    print("=" * 50)
    
    # The original magic number
    original_factor = 182.0
    
    # Calculate the correct factors
    factor_65535 = 65535 / 360  # Using 0xFFFF
    factor_65536 = 65536 / 360  # Using 0x10000 (LIFX recommended)
    
    print(f"Original magic number: {original_factor}")
    print(f"65535 / 360 = {factor_65535:.10f}")
    print(f"65536 / 360 = {factor_65536:.10f}")
    print()
    
    # Show the difference in conversion
    test_degrees = [0, 60, 120, 180, 240, 300, 360]
    
    print("Conversion comparison for various hue values:")
    print("-" * 50)
    print(f"{'Degrees':<10} {'Old (182.0)':<15} {'Correct (65536/360)':<20} {'Difference'}")
    print("-" * 50)
    
    for deg in test_degrees:
        old_value = int(deg * original_factor)
        correct_value = int((deg * factor_65536) % 65536)
        diff = correct_value - old_value
        
        print(f"{deg:<10} {old_value:<15} {correct_value:<20} {diff:+d}")
    
    print()
    print("Explanation:")
    print("- The magic number 182.0 was an approximation of 65535/360 = 182.04166...")
    print("- However, LIFX documentation recommends using 65536/360 = 182.04444...")
    print("- This provides better rounding behavior and wrapping at 360 degrees")
    print("- The difference is small but can accumulate in color transitions")
    
    # Show named colors in LIFX format
    print()
    print("Named colors in LIFX u16 format:")
    print("-" * 50)
    
    colors = {
        'Red': 0,
        'Orange': 39,
        'Yellow': 60,
        'Green': 120,
        'Cyan': 180,
        'Blue': 240,
        'Purple': 275,
        'Pink': 350
    }
    
    for name, degrees in colors.items():
        lifx_value = int((degrees * factor_65536) % 65536)
        print(f"{name:<10} {degrees:>3}° → {lifx_value:>5} (0x{lifx_value:04X})")

if __name__ == "__main__":
    main()