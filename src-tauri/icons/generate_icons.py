"""
Generate 3 simple 32x32 PNG tray icons using only struct and zlib.
No PIL/Pillow needed - writes raw PNG bytes directly.

Icons:
  icon-idle.png       - white/gray circle on transparent background
  icon-recording.png  - red circle on transparent background
  icon-processing.png - yellow/amber circle on transparent background
"""

import struct
import zlib
import math
import os

WIDTH = 32
HEIGHT = 32
CENTER = (WIDTH / 2, HEIGHT / 2)
RADIUS = 13.0  # slightly smaller than half to leave margin
EDGE_WIDTH = 1.5  # anti-aliasing edge


def make_png(pixels: list[list[tuple[int, int, int, int]]]) -> bytes:
    """Create a PNG file from RGBA pixel data."""

    def chunk(chunk_type: bytes, data: bytes) -> bytes:
        c = chunk_type + data
        crc = zlib.crc32(c) & 0xFFFFFFFF
        return struct.pack(">I", len(data)) + c + struct.pack(">I", crc)

    # PNG signature
    sig = b"\x89PNG\r\n\x1a\n"

    # IHDR: width, height, bit_depth=8, color_type=6 (RGBA), compression=0, filter=0, interlace=0
    ihdr_data = struct.pack(">IIBBBBB", WIDTH, HEIGHT, 8, 6, 0, 0, 0)
    ihdr = chunk(b"IHDR", ihdr_data)

    # IDAT: image data
    raw_data = bytearray()
    for row in pixels:
        raw_data.append(0)  # filter byte: None
        for r, g, b, a in row:
            raw_data.extend([r, g, b, a])

    compressed = zlib.compress(bytes(raw_data), 9)
    idat = chunk(b"IDAT", compressed)

    # IEND
    iend = chunk(b"IEND", b"")

    return sig + ihdr + idat + iend


def circle_alpha(x: int, y: int) -> float:
    """Return alpha [0.0, 1.0] for anti-aliased circle."""
    dx = x + 0.5 - CENTER[0]
    dy = y + 0.5 - CENTER[1]
    dist = math.sqrt(dx * dx + dy * dy)

    if dist <= RADIUS - EDGE_WIDTH:
        return 1.0
    elif dist >= RADIUS + EDGE_WIDTH:
        return 0.0
    else:
        # Smooth transition in edge zone
        t = (RADIUS + EDGE_WIDTH - dist) / (2 * EDGE_WIDTH)
        return max(0.0, min(1.0, t))


def generate_circle_icon(color: tuple[int, int, int]) -> list[list[tuple[int, int, int, int]]]:
    """Generate a 32x32 RGBA pixel grid with a colored circle."""
    pixels = []
    for y in range(HEIGHT):
        row = []
        for x in range(WIDTH):
            alpha = circle_alpha(x, y)
            if alpha <= 0:
                row.append((0, 0, 0, 0))
            else:
                a = int(alpha * 255)
                row.append((color[0], color[1], color[2], a))
        pixels.append(row)
    return pixels


def main():
    script_dir = os.path.dirname(os.path.abspath(__file__))

    icons = {
        "icon-idle.png": (200, 200, 200),       # Light gray
        "icon-recording.png": (220, 40, 40),     # Red
        "icon-processing.png": (230, 180, 30),   # Yellow/amber
    }

    for filename, color in icons.items():
        pixels = generate_circle_icon(color)
        png_data = make_png(pixels)
        path = os.path.join(script_dir, filename)
        with open(path, "wb") as f:
            f.write(png_data)
        print(f"Generated {path} ({len(png_data)} bytes)")


if __name__ == "__main__":
    main()
