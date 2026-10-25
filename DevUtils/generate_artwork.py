#!/usr/bin/env python3
"""Generate the Basalt cover and icon from code, with no external assets."""

from __future__ import annotations

import math
import random
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "app" / "basalt"
FONT = "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf"
FONT_REGULAR = "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"
SCALE = 2

VOID = (6, 9, 16, 255)
INK = (10, 16, 26, 255)
PANEL = (13, 21, 33, 255)
GRID = (27, 42, 59, 255)
DIM = (70, 96, 116, 255)
MUTED = (128, 158, 178, 255)
WHITE = (232, 243, 245, 255)
CYAN = (91, 235, 214, 255)
CORAL = (255, 142, 112, 255)
AMBER = (255, 205, 105, 255)


def font(size: int, regular: bool = False) -> ImageFont.FreeTypeFont:
    return ImageFont.truetype(FONT_REGULAR if regular else FONT, size * SCALE)


def xy(values):
    return tuple(int(value * SCALE) for value in values)


def line(draw: ImageDraw.ImageDraw, a, b, width, color):
    draw.line([xy(a), xy(b)], fill=color, width=max(1, int(width * SCALE)))


def rect(draw, box, color, outline=None, width=1):
    x, y, w, h = box
    draw.rectangle([xy((x, y)), xy((x + w, y + h))], fill=color, outline=outline, width=max(1, int(width * SCALE)))


def text(draw, position, value, size, color, regular=False, spacing=0):
    current = position[0]
    f = font(size, regular)
    for char in value:
        draw.text(xy((current, position[1])), char, font=f, fill=color)
        current += (draw.textlength(char, font=f) / SCALE) + spacing


def text_width(draw, value, size, spacing=0, regular=False):
    f = font(size, regular)
    return sum(draw.textlength(char, font=f) / SCALE for char in value) + spacing * max(0, len(value) - 1)


def background(draw, width, height):
    for y in range(height):
        blend = y / height
        color = (
            int(6 + blend * 4),
            int(9 + blend * 6),
            int(16 + blend * 9),
            255,
        )
        draw.rectangle([0, int(y * SCALE), int(width * SCALE), int((y + 1) * SCALE)], fill=color)
    random.seed(20261025)
    for _ in range(260):
        x = random.uniform(0, width)
        y = random.uniform(0, height)
        size = 2 if random.random() < 0.06 else 1
        shade = random.choice([(32, 43, 55, 255), (44, 60, 72, 255), (59, 78, 89, 255)])
        draw.rectangle([xy((x, y)), xy((x + size, y + size))], fill=shade)
    for x in range(0, width, 75):
        line(draw, (x, 0), (x, height), 0.7, (17, 26, 39, 255))
    for y in range(0, height, 75):
        line(draw, (0, y), (width, y), 0.7, (17, 26, 39, 255))


def pong_motif(draw, cx, cy, accent):
    line(draw, (cx - 56, cy - 46), (cx + 56, cy - 46), 1.2, (34, 55, 69, 255))
    line(draw, (cx - 56, cy + 46), (cx + 56, cy + 46), 1.2, (34, 55, 69, 255))
    line(draw, (cx - 56, cy - 46), (cx - 56, cy + 46), 1.2, (34, 55, 69, 255))
    line(draw, (cx + 56, cy - 46), (cx + 56, cy + 46), 1.2, (34, 55, 69, 255))
    for y in range(int(cy - 40), int(cy + 40), 14):
        line(draw, (cx, y), (cx, y + 7), 1.0, (41, 66, 78, 255))
    line(draw, (cx - 47, cy - 18), (cx - 47, cy + 18), 8, CYAN)
    line(draw, (cx + 47, cy - 13), (cx + 47, cy + 14), 8, CORAL)
    draw.ellipse([xy((cx + 12, cy - 8)), xy((cx + 24, cy + 4))], fill=WHITE)
    line(draw, (cx - 12, cy + 13), (cx + 9, cy - 7), 1.4, accent)


def lunar_motif(draw, cx, cy, accent):
    draw.arc([xy((cx + 20, cy - 52)), xy((cx + 62, cy - 10))], 0, 360, fill=(55, 65, 77, 255), width=int(1.5 * SCALE))
    terrain = [(cx - 70, cy + 42), (cx - 42, cy + 18), (cx - 14, cy + 35), (cx + 12, cy + 42), (cx + 38, cy + 42), (cx + 60, cy + 20), (cx + 78, cy + 36)]
    for a, b in zip(terrain, terrain[1:]):
        line(draw, a, b, 1.8, (118, 99, 55, 255))
    line(draw, (cx + 12, cy + 42), (cx + 38, cy + 42), 4, AMBER)
    body = [(cx - 8, cy - 10), (cx + 8, cy - 10), (cx + 12, cy + 7), (cx + 6, cy + 14), (cx - 6, cy + 14), (cx - 12, cy + 7)]
    for a, b in zip(body, body[1:] + body[:1]):
        line(draw, a, b, 1.7, accent)
    line(draw, (cx - 6, cy + 14), (cx - 13, cy + 23), 1.5, accent)
    line(draw, (cx + 6, cy + 14), (cx + 13, cy + 23), 1.5, accent)
    line(draw, (cx - 13, cy + 23), (cx - 8, cy + 23), 2, accent)
    line(draw, (cx + 13, cy + 23), (cx + 8, cy + 23), 2, accent)
    line(draw, (cx - 4, cy + 15), (cx - 2, cy + 26), 1.2, (255, 220, 140, 255))
    line(draw, (cx + 4, cy + 15), (cx + 2, cy + 30), 1.2, (255, 220, 140, 255))


def asteroids_motif(draw, cx, cy, accent):
    random.seed(7)
    for (x, y, radius, sides) in ((cx - 38, cy - 16, 22, 9), (cx + 32, cy + 15, 30, 10), (cx + 8, cy - 40, 13, 8)):
        points = []
        for index in range(sides):
            angle = index / sides * math.tau
            wobble = 0.75 + random.random() * 0.28
            points.append((x + math.cos(angle) * radius * wobble, y + math.sin(angle) * radius * wobble))
        for a, b in zip(points, points[1:] + points[:1]):
            line(draw, a, b, 1.5, (174, 121, 103, 255))
    ship = [(cx + 2, cy + 22), (cx + 13, cy + 43), (cx + 2, cy + 38), (cx - 9, cy + 43)]
    for a, b in zip(ship, ship[1:] + ship[:1]):
        line(draw, a, b, 1.7, accent)
    line(draw, (cx + 2, cy + 38), (cx - 1, cy + 51), 1.1, (255, 220, 140, 255))
    line(draw, (cx + 2, cy + 38), (cx + 5, cy + 51), 1.1, (255, 220, 140, 255))
    line(draw, (cx - 40, cy + 22), (cx - 24, cy + 20), 2.2, WHITE)
    line(draw, (cx - 48, cy + 20), (cx - 35, cy + 18), 2.2, WHITE)


def make_cover():
    width, height = 900, 1200
    image = Image.new("RGBA", (width * SCALE, height * SCALE), VOID)
    draw = ImageDraw.Draw(image)
    background(draw, width, height)

    line(draw, (60, 62), (840, 62), 1.6, CYAN)
    line(draw, (60, 67), (220, 67), 1.0, (36, 94, 90, 255))
    text(draw, (60, 90), "A QUASAR ARCADE COLLECTION", 17, MUTED, spacing=2.3)
    text(draw, (60, 138), "CLASSIC", 128, WHITE, spacing=7)
    text(draw, (60, 276), "CLASSICS", 128, WHITE, spacing=7)
    line(draw, (62, 430), (838, 430), 1.4, GRID)
    text(draw, (62, 446), "THREE CLASSICS. ONE RUNTIME.", 19, CYAN, spacing=1.4)

    strips = [
        (62, 500, "01", "PONG", "TWO PADDLES. ONE BALL.", CYAN, pong_motif),
        (62, 706, "02", "LUNAR LANDER", "GRAVITY IS PATIENT.", AMBER, lunar_motif),
        (62, 912, "03", "ASTEROIDS", "EVERY ROCK SPLITS.", CORAL, asteroids_motif),
    ]
    for x, y, number, name, tagline, accent, motif in strips:
        rect(draw, (x, y, 776, 178), PANEL)
        rect(draw, (x, y, 776, 178), None, outline=GRID, width=1.2)
        line(draw, (x, y), (x + 86, y), 3, accent)
        rect(draw, (x + 18, y + 18, 142, 142), (8, 12, 20, 255))
        rect(draw, (x + 18, y + 18, 142, 142), None, outline=(24, 38, 52, 255), width=1)
        motif(draw, x + 89, y + 89, accent)
        text(draw, (x + 190, y + 22), number, 24, accent, spacing=2)
        text(draw, (x + 190, y + 62), name, 42, WHITE, spacing=2.2)
        text(draw, (x + 190, y + 120), tagline, 16, MUTED, spacing=1.6)

    line(draw, (62, 1128), (838, 1128), 1.2, GRID)
    text(draw, (62, 1144), "QUASAR / INPUT / RENDER-2D / AUDIO / STORAGE / CONFIG / CORE", 12, (66, 88, 103, 255), spacing=0.8)
    text(draw, (838 - text_width(draw, "900 x 1200", 12, 0.8), 1144), "900 x 1200", 12, (66, 88, 103, 255), spacing=0.8)

    OUT.mkdir(parents=True, exist_ok=True)
    image.resize((width, height), Image.Resampling.LANCZOS).convert("RGB").save(
        OUT / "cover.png", optimize=True
    )


def make_icon():
    size = 512
    image = Image.new("RGBA", (size * SCALE, size * SCALE), (0, 0, 0, 0))
    draw = ImageDraw.Draw(image)
    center = size / 2
    # Opaque token with a transparent outside.
    draw.ellipse(xy((12, 12, size - 12, size - 12)), fill=(9, 14, 23, 255), outline=CYAN, width=int(5 * SCALE))
    draw.ellipse(xy((30, 30, size - 30, size - 30)), outline=(30, 72, 70, 255), width=int(2 * SCALE))
    # Pong paddle + ball at upper left.
    line(draw, (center - 128, center - 124), (center - 128, center - 26), 18, CYAN)
    draw.ellipse(xy((center - 62, center - 94, center - 28, center - 60)), fill=WHITE)
    # Asteroid upper right.
    points = []
    for index in range(9):
        angle = index / 9 * math.tau
        radius = 62 * (0.78 + (index * 37 % 11) * 0.02)
        points.append((center + 82 + math.cos(angle) * radius, center - 76 + math.sin(angle) * radius))
    for a, b in zip(points, points[1:] + points[:1]):
        line(draw, a, b, 5, CORAL)
    # Lander at bottom centre.
    lander = [
        (center - 70, center + 92),
        (center + 70, center + 92),
        (center + 52, center + 142),
        (center, center + 166),
        (center - 52, center + 142),
    ]
    for a, b in zip(lander, lander[1:] + lander[:1]):
        line(draw, a, b, 6, AMBER)
    line(draw, (center - 52, center + 142), (center - 102, center + 192), 5, AMBER)
    line(draw, (center + 52, center + 142), (center + 102, center + 192), 5, AMBER)
    line(draw, (center - 102, center + 192), (center - 68, center + 192), 6, AMBER)
    line(draw, (center + 102, center + 192), (center + 68, center + 192), 6, AMBER)
    line(draw, (center - 18, center + 166), (center - 8, center + 222), 4, (255, 220, 140, 255))
    line(draw, (center + 18, center + 166), (center + 8, center + 222), 4, (255, 220, 140, 255))
    image.resize((size, size), Image.Resampling.LANCZOS).save(OUT / "icon.png", optimize=True)


if __name__ == "__main__":
    make_cover()
    make_icon()
    print(OUT / "cover.png")
    print(OUT / "icon.png")
