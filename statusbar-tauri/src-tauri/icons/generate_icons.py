#!/usr/bin/env python3
"""
Generates placeholder tray/app icons for statusbar-tauri using only the
standard library (no Pillow/ImageMagick required).

Produces a simple filled circle (dark on transparent) at several sizes, plus
minimal-but-valid .icns (macOS) and .ico (Windows) containers wrapping PNG data.

Replace these with real artwork later via:
    npm run icon   (wraps `tauri icon icons/icon.png`)
"""

from __future__ import annotations

import os
import struct
import zlib

ICONS_DIR = os.path.join(os.path.dirname(__file__))


def _png_chunk(tag: bytes, data: bytes) -> bytes:
    chunk = tag + data
    return struct.pack(">I", len(data)) + chunk + struct.pack(">I", zlib.crc32(chunk) & 0xFFFFFFFF)


def _rgba_circle(size: int) -> bytearray:
    pixels = bytearray(size * size * 4)
    cx = cy = size / 2
    radius = size * 0.46
    for y in range(size):
        for x in range(size):
            dx, dy = x - cx + 0.5, y - cy + 0.5
            idx = (y * size + x) * 4
            if (dx * dx + dy * dy) ** 0.5 <= radius:
                pixels[idx : idx + 4] = bytes([20, 20, 20, 255])
            else:
                pixels[idx : idx + 4] = bytes([0, 0, 0, 0])
    return pixels


def write_png(path: str, size: int) -> bytes:
    pixels = _rgba_circle(size)
    ihdr = struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0)
    raw = bytearray()
    stride = size * 4
    for y in range(size):
        raw.append(0)  # filter type: none
        raw.extend(pixels[y * stride : (y + 1) * stride])
    idat = zlib.compress(bytes(raw), 9)

    data = b"\x89PNG\r\n\x1a\n" + _png_chunk(b"IHDR", ihdr) + _png_chunk(b"IDAT", idat) + _png_chunk(b"IEND", b"")
    with open(path, "wb") as f:
        f.write(data)
    return data


def write_icns(path: str, entries: list[tuple[bytes, bytes]]) -> None:
    body = b""
    for tag, png_data in entries:
        entry_len = 8 + len(png_data)
        body += tag + struct.pack(">I", entry_len) + png_data
    with open(path, "wb") as f:
        f.write(b"icns" + struct.pack(">I", 8 + len(body)) + body)


def write_ico(path: str, png_data: bytes, size: int) -> None:
    # Single-image "PNG-in-ICO" format, supported since Windows Vista.
    header = struct.pack("<HHH", 0, 1, 1)
    width_byte = 0 if size >= 256 else size
    height_byte = 0 if size >= 256 else size
    entry = struct.pack(
        "<BBBBHHII",
        width_byte,
        height_byte,
        0,
        0,
        1,
        32,
        len(png_data),
        6 + 16,
    )
    with open(path, "wb") as f:
        f.write(header + entry + png_data)


def main() -> None:
    png_16 = write_png(os.path.join(ICONS_DIR, "16x16.png"), 16)
    png_32 = write_png(os.path.join(ICONS_DIR, "32x32.png"), 32)
    png_128 = write_png(os.path.join(ICONS_DIR, "128x128.png"), 128)
    png_256 = write_png(os.path.join(ICONS_DIR, "128x128@2x.png"), 256)
    # Tray icon: small template-style image (macOS renders it as monochrome).
    with open(os.path.join(ICONS_DIR, "icon.png"), "wb") as f:
        f.write(png_128)

    write_icns(
        os.path.join(ICONS_DIR, "icon.icns"),
        [
            (b"icp4", png_16),
            (b"icp5", png_32),
            (b"ic07", png_128),
            (b"ic08", png_256),
        ],
    )
    write_ico(os.path.join(ICONS_DIR, "icon.ico"), png_256, 256)
    print("Generated placeholder icons in", ICONS_DIR)


if __name__ == "__main__":
    main()
