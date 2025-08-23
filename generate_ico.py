#!/usr/bin/env python3
"""
Generate Windows .ico file from PNG icons
"""

import os
from PIL import Image

def create_ico():
    icons_dir = "/Users/mitchdornich/Documents/GitHub/Cap/apps/desktop/src-tauri/icons"
    
    # Load the purple icon
    source_icon = os.path.join(icons_dir, "icon.png")
    img = Image.open(source_icon)
    
    # Create multiple sizes for the ICO file
    icon_sizes = [(16, 16), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)]
    
    # Backup old ico
    old_ico = os.path.join(icons_dir, "icon.ico")
    if os.path.exists(old_ico):
        backup_path = os.path.join(icons_dir, "icons_blue_backup", "icon.ico")
        os.system(f'cp "{old_ico}" "{backup_path}"')
        print("Backed up old icon.ico")
    
    # Generate ico with multiple sizes
    img.save(
        old_ico,
        format='ICO',
        sizes=icon_sizes
    )
    
    print("✅ Generated purple icon.ico for Windows")

if __name__ == "__main__":
    create_ico()