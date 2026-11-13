"""Make every icon the packages need from the mark, at build time.

    python3 -I make-icons.py MARK_SVG OUT_DIR

MARK_SVG is docs/design/brand/vessel-mark.svg (the owner's paths on a
square view box; its README says how the mark is used). Nothing made here
is committed: build.sh runs this into its build directory.

Writes into OUT_DIR:
  tile.svg                   the mark in white on a brand-colour tile with
                             rounded corners (the app icon's design)
  tile-<n>.png               that tile at n px, for n in TILE_SIZES
  vessel.ico                 Windows: 16, 20, 24, 32, 40, 48, 64, 256 in one
                             file, each frame PNG-compressed
  installer-header.bmp       NSIS's page header (150 x 57): the mark in the
                             brand colour on white, as on light surfaces
  installer-side.bmp         NSIS's finish page (164 x 314): the white mark
                             on the brand colour

Each size is rasterised from the vector at that size (not scaled down
from a large bitmap), by resvg: ImageMagick 6's own SVG renderer (MSVG,
what `convert` uses here) draws the nested lines with visibly heavier
antialiasing at 16 to 32 px; docs/plans/backend-m12-release.md shows both.

Decision Log
- Purpose: one source for every icon, so a change of the mark is a
  rebuild, never a hand-edited bitmap.
- Choice: compose SVGs here (the mark's paths inside a tile), rasterise
  with resvg, pack the .ico in this script (its format is a header, a
  directory and the PNG frames; Windows Vista and later read PNG frames
  at every size), convert to BMP with ImageMagick only from PNG.
- Cost: resvg is one more build tool (cargo install, MPL-2.0, not linked
  into anything shipped).
"""

import os
import re
import struct
import subprocess
import sys

BRAND = "#1a5868"
TILE_SIZES = (16, 20, 24, 32, 40, 48, 64, 128, 256, 512)
ICO_SIZES = (16, 20, 24, 32, 40, 48, 64, 256)
# The tile's corner radius and the mark's share of the tile. At 16 to 24
# px the nested lines merge into one stroke whatever is done (the brand
# README says so); a larger share keeps the V and the star readable there.
RADIUS = 0.1875
MARK_SHARE = 0.86


def mark_paths(svg_text):
    """The mark's <path> elements, with their fill taken out."""
    paths = re.findall(r"<path\b[^>]*/>", svg_text, flags=re.S)
    if not paths:
        raise SystemExit("no <path> in the mark")
    return [re.sub(r'\sfill="[^"]*"', "", path) for path in paths]


def view_box(svg_text):
    match = re.search(r'viewBox="([^"]+)"', svg_text)
    if not match:
        raise SystemExit("the mark has no viewBox")
    return match.group(1)


def tile_svg(paths, box):
    size = 1024
    inner = size * MARK_SHARE
    offset = (size - inner) / 2
    radius = size * RADIUS
    body = "\n".join(paths)
    return f"""<svg xmlns="http://www.w3.org/2000/svg" width="{size}" height="{size}" viewBox="0 0 {size} {size}">
  <rect width="{size}" height="{size}" rx="{radius}" ry="{radius}" fill="{BRAND}"/>
  <svg x="{offset}" y="{offset}" width="{inner}" height="{inner}" viewBox="{box}">
    <g fill="#ffffff">{body}</g>
  </svg>
</svg>
"""


def banner_svg(paths, box, width, height, background, mark, mark_height, right):
    """A bitmap of width x height with the mark mark_height tall, at the
    right edge (right=True) or centred."""
    x = width - mark_height - (height - mark_height) / 2 if right else (width - mark_height) / 2
    y = (height - mark_height) / 2
    body = "\n".join(paths)
    return f"""<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">
  <rect width="{width}" height="{height}" fill="{background}"/>
  <svg x="{x}" y="{y}" width="{mark_height}" height="{mark_height}" viewBox="{box}">
    <g fill="{mark}">{body}</g>
  </svg>
</svg>
"""


def render(svg_path, png_path, width, height):
    subprocess.run(
        ["resvg", "--width", str(width), "--height", str(height), svg_path, png_path],
        check=True,
    )


def png_size(data):
    if data[:8] != b"\x89PNG\r\n\x1a\n":
        raise SystemExit("not a PNG")
    return struct.unpack(">II", data[16:24])


def write_ico(frames, path):
    """An .ico of PNG frames: ICONDIR, one ICONDIRENTRY each, the data."""
    header = struct.pack("<HHH", 0, 1, len(frames))
    offset = 6 + 16 * len(frames)
    entries = b""
    for data in frames:
        width, height = png_size(data)
        entries += struct.pack(
            "<BBBBHHII",
            width if width < 256 else 0,
            height if height < 256 else 0,
            0, 0, 1, 32, len(data), offset,
        )
        offset += len(data)
    with open(path, "wb") as out:
        out.write(header + entries + b"".join(frames))


def to_bmp(png_path, bmp_path):
    # NSIS takes 24-bit BMP only; the banner has no transparency to lose.
    subprocess.run(["convert", png_path, "-alpha", "off", "-type", "TrueColor", f"BMP3:{bmp_path}"], check=True)


def main():
    if len(sys.argv) != 3:
        raise SystemExit("usage: python3 -I make-icons.py MARK_SVG OUT_DIR")
    mark_path, out = sys.argv[1], sys.argv[2]
    os.makedirs(out, exist_ok=True)
    text = open(mark_path, encoding="utf-8").read()
    paths, box = mark_paths(text), view_box(text)

    tile = os.path.join(out, "tile.svg")
    open(tile, "w", encoding="utf-8").write(tile_svg(paths, box))
    for size in TILE_SIZES:
        render(tile, os.path.join(out, f"tile-{size}.png"), size, size)
    frames = [open(os.path.join(out, f"tile-{size}.png"), "rb").read() for size in ICO_SIZES]
    write_ico(frames, os.path.join(out, "vessel.ico"))

    for name, width, height, background, mark, mark_height, right in (
        ("installer-header", 150, 57, "#ffffff", BRAND, 45, True),
        ("installer-side", 164, 314, BRAND, "#ffffff", 120, False),
    ):
        svg = os.path.join(out, f"{name}.svg")
        open(svg, "w", encoding="utf-8").write(
            banner_svg(paths, box, width, height, background, mark, mark_height, right)
        )
        png = os.path.join(out, f"{name}.png")
        render(svg, png, width, height)
        to_bmp(png, os.path.join(out, f"{name}.bmp"))
    print(f"icons in {out}: vessel.ico ({', '.join(map(str, ICO_SIZES))}), tiles, installer bitmaps")


main()
