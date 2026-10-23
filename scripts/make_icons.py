"""Renders the app icons in icons/ from tomato.svg.

Run from the repo root: python scripts/make_icons.py (needs Pillow).
The SVG only uses absolute M/L/C/Z, so the paths are flattened here instead of
pulling in an SVG renderer.
"""

import re
from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parent.parent
SVG = ROOT / "tomato.svg"
OUT = ROOT / "icons"

PAPER = (0xF2, 0xEE, 0xE6, 255)
BIG = 2048  # drawn large, then downsampled for antialiasing
STEPS = 24


def parse_paths(svg):
    paths = []
    for tag in re.findall(r"<path\b[^>]*>", svg):
        d = re.search(r'\bd="([^"]*)"', tag).group(1)
        fill = re.search(r'\bfill="#([0-9a-fA-F]{6})"', tag).group(1)
        dx, dy = 0.0, 0.0
        t = re.search(r'translate\(\s*([-\d.]+)[\s,]+([-\d.]+)\s*\)', tag)
        if t:
            dx, dy = float(t.group(1)), float(t.group(2))
        paths.append((flatten(d, dx, dy), tuple(int(fill[i:i + 2], 16) for i in (0, 2, 4)) + (255,)))
    return paths


def flatten(d, dx, dy):
    """Returns the subpaths of `d` as point lists."""
    tokens = re.findall(r"[MLCZ]|-?\d*\.?\d+(?:e-?\d+)?", d)
    subpaths, pts, i, cmd = [], [], 0, None
    nums = lambda n: [float(tokens[i + k]) for k in range(n)]
    while i < len(tokens):
        if tokens[i] in "MLCZ":
            cmd = tokens[i]
            i += 1
            if cmd == "Z":
                if pts:
                    subpaths.append(pts)
                pts = []
                continue
        if cmd == "M":
            if pts:
                subpaths.append(pts)
            x, y = nums(2)
            pts = [(x + dx, y + dy)]
            i += 2
            cmd = "L"
        elif cmd == "L":
            x, y = nums(2)
            pts.append((x + dx, y + dy))
            i += 2
        elif cmd == "C":
            x1, y1, x2, y2, x, y = nums(6)
            (x0, y0) = pts[-1]
            c = [(x0, y0), (x1 + dx, y1 + dy), (x2 + dx, y2 + dy), (x + dx, y + dy)]
            for s in range(1, STEPS + 1):
                t = s / STEPS
                u = 1 - t
                pts.append(tuple(
                    u**3 * c[0][k] + 3 * u * u * t * c[1][k] + 3 * u * t * t * c[2][k] + t**3 * c[3][k]
                    for k in range(2)))
            i += 6
    if pts:
        subpaths.append(pts)
    return subpaths


def render(paths):
    """The tomato on a rounded paper tile, like the app window."""
    img = Image.new("RGBA", (BIG, BIG), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)
    margin = BIG * 0.04
    draw.rounded_rectangle((margin, margin, BIG - margin, BIG - margin), radius=BIG * 0.2, fill=PAPER)

    xs = [x for sub, _ in paths for pts in sub for x, _ in pts]
    ys = [y for sub, _ in paths for pts in sub for _, y in pts]
    size = max(max(xs) - min(xs), max(ys) - min(ys))
    scale = BIG * 0.68 / size
    ox = BIG / 2 - (min(xs) + max(xs)) / 2 * scale
    oy = BIG / 2 - (min(ys) + max(ys)) / 2 * scale
    for sub, fill in paths:
        for pts in sub:
            draw.polygon([(ox + x * scale, oy + y * scale) for x, y in pts], fill=fill)
    return img


def main():
    OUT.mkdir(exist_ok=True)
    img = render(parse_paths(SVG.read_text(encoding="utf-8")))
    for px in (32, 128, 256, 512):
        img.resize((px, px), Image.LANCZOS).save(OUT / f"{px}x{px}.png")
    full = img.resize((1024, 1024), Image.LANCZOS)
    full.save(OUT / "icon.ico", sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)])
    full.save(OUT / "icon.icns")


if __name__ == "__main__":
    main()
