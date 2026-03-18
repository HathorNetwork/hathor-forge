#!/usr/bin/env python3
"""Generate DMG background image for macOS installer."""
from PIL import Image, ImageDraw, ImageFont
import math

W, H = 660, 400

# Dark background matching app theme
img = Image.new('RGBA', (W, H), (9, 11, 16, 255))

# Subtle blue glow from center
for y in range(H):
    for x in range(W):
        dx = (x - W / 2) / (W / 2)
        dy = (y - H * 0.6) / (H * 0.5)
        d = math.sqrt(dx * dx + dy * dy)
        alpha = max(0, int(25 * (1 - min(d, 1))))
        r, g, b, a = img.getpixel((x, y))
        img.putpixel((x, y), (r, g + alpha // 4, b + alpha, 255))

draw = ImageDraw.Draw(img)

# Arrow in center pointing right
arrow_y = 195
arrow_x_start = 275
arrow_x_end = 385
arrow_color = (156, 243, 91, 160)

# Shaft
for y_off in range(-2, 3):
    draw.line(
        [(arrow_x_start, arrow_y + y_off), (arrow_x_end - 20, arrow_y + y_off)],
        fill=arrow_color,
    )

# Arrowhead
for i in range(22):
    half_h = int((22 - i) * 0.7)
    x = arrow_x_end - 22 + i
    for y_off in range(-half_h, half_h + 1):
        if 0 <= x < W and 0 <= arrow_y + y_off < H:
            draw.point((x, arrow_y + y_off), fill=arrow_color)

# Fonts
try:
    font = ImageFont.truetype("/System/Library/Fonts/Helvetica.ttc", 14)
    small_font = ImageFont.truetype("/System/Library/Fonts/Helvetica.ttc", 11)
except Exception:
    font = ImageFont.load_default()
    small_font = font

# "Drag to Applications" label
text = "Drag to Applications"
bbox = draw.textbbox((0, 0), text, font=font)
tw = bbox[2] - bbox[0]
draw.text(((W - tw) / 2, arrow_y + 35), text, fill=(255, 255, 255, 120), font=font)

# Subtle branding at bottom
ver = "Hathor Forge"
bbox = draw.textbbox((0, 0), ver, font=small_font)
tw = bbox[2] - bbox[0]
draw.text(((W - tw) / 2, H - 35), ver, fill=(255, 255, 255, 40), font=small_font)

img.save("src-tauri/dmg/background.png")
print(f"Saved DMG background: {W}x{H}")
