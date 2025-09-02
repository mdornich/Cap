#!/usr/bin/env python3
"""
Convert blue Klip icons to purple by shifting the hue.
This script processes PNG images and shifts blue colors to purple.
"""

import os
import sys
from PIL import Image
import numpy as np
import colorsys

def rgb_to_hsv(rgb):
    """Convert RGB to HSV color space."""
    return colorsys.rgb_to_hsv(rgb[0]/255.0, rgb[1]/255.0, rgb[2]/255.0)

def hsv_to_rgb(hsv):
    """Convert HSV to RGB color space."""
    rgb = colorsys.hsv_to_rgb(hsv[0], hsv[1], hsv[2])
    return tuple(int(c * 255) for c in rgb)

def shift_blue_to_purple(image_path, output_path):
    """
    Shift blue colors in an image to purple.
    Blue hue is around 210-240 degrees, purple is around 270-300 degrees.
    """
    # Open the image
    img = Image.open(image_path).convert('RGBA')
    pixels = img.load()
    
    # Process each pixel
    for y in range(img.height):
        for x in range(img.width):
            r, g, b, a = pixels[x, y]
            
            # Skip transparent pixels
            if a == 0:
                continue
            
            # Convert to HSV
            h, s, v = rgb_to_hsv((r, g, b))
            
            # Check if the color is in the blue range (approximately 200-250 degrees in 360 scale)
            # In Python's colorsys, hue is 0-1, so blue is around 0.55-0.69
            if 0.50 <= h <= 0.70:  # Blue range
                # Shift to purple (add about 60 degrees, or 0.167 in 0-1 scale)
                # Blue (240°) -> Purple (280°)
                h = h + 0.111  # Shift from blue to purple
                if h > 1.0:
                    h -= 1.0
                
                # Convert back to RGB
                new_rgb = hsv_to_rgb((h, s, v))
                pixels[x, y] = (*new_rgb, a)
            # Also handle light blue colors
            elif 0.45 <= h <= 0.50 and s > 0.2:  # Light blue range
                # Shift to light purple
                h = h + 0.139  # Slightly more shift for light blues
                if h > 1.0:
                    h -= 1.0
                
                # Convert back to RGB
                new_rgb = hsv_to_rgb((h, s, v))
                pixels[x, y] = (*new_rgb, a)
    
    # Save the modified image
    img.save(output_path, 'PNG')
    print(f"Converted: {os.path.basename(image_path)} -> {os.path.basename(output_path)}")

def main():
    icons_dir = "/Users/mitchdornich/Documents/GitHub/Cap/apps/desktop/src-tauri/icons"
    backup_dir = "/Users/mitchdornich/Documents/GitHub/Cap/apps/desktop/src-tauri/icons_blue_backup"
    
    # Create backup directory
    if not os.path.exists(backup_dir):
        os.makedirs(backup_dir)
        print(f"Created backup directory: {backup_dir}")
    
    # Process PNG files
    png_files = [
        "32x32.png",
        "128x128.png", 
        "128x128@2x.png",
        "icon.png",
        "Square30x30Logo.png",
        "Square44x44Logo.png",
        "Square71x71Logo.png",
        "Square89x89Logo.png",
        "Square107x107Logo.png",
        "Square142x142Logo.png",
        "Square150x150Logo.png",
        "Square284x284Logo.png",
        "Square310x310Logo.png",
        "StoreLogo.png"
    ]
    
    for filename in png_files:
        src_path = os.path.join(icons_dir, filename)
        if os.path.exists(src_path):
            # Backup original
            backup_path = os.path.join(backup_dir, filename)
            if not os.path.exists(backup_path):
                os.system(f'cp "{src_path}" "{backup_path}"')
                print(f"Backed up: {filename}")
            
            # Convert to purple
            shift_blue_to_purple(src_path, src_path)
    
    print("\n✅ All PNG icons have been converted from blue to purple!")
    print(f"Original blue icons backed up to: {backup_dir}")
    print("\nNote: .ico and .icns files need to be regenerated from the PNG files.")

if __name__ == "__main__":
    main()