#!/usr/bin/env python3
import io
import sys
import urllib.request
import zipfile
from pathlib import Path

import numpy as np
from PIL import Image

SIZE = 1024
CELLS = 4
OUT = Path(__file__).resolve().parents[2] / "groundstation" / "src" / "synthetic" / "facades"

MATERIALS = [
    ("PaintedPlaster017", "wall"),
    ("Plaster003", "wall"),
    ("Bricks085", "wall"),
    ("Bricks066", "wall"),
    ("Concrete034", "wall"),
    ("Facade004", "curtain"),
    ("Facade001", "curtain"),
    ("Facade018A", "windows"),
    ("Facade020A", "windows"),
]


def maps(asset: str, cache: Path) -> dict[str, Image.Image]:
    archive = cache / f"{asset}_2K-JPG.zip"
    if not archive.exists():
        url = f"https://ambientcg.com/get?file={asset}_2K-JPG.zip"
        request = urllib.request.Request(url, headers={"User-Agent": "aircast-facade-atlas"})
        archive.write_bytes(urllib.request.urlopen(request).read())
    with zipfile.ZipFile(archive) as bundle:
        return {
            kind: Image.open(io.BytesIO(bundle.read(f"{asset}_2K-JPG_{kind}.jpg"))).convert(
                "RGB" if kind == "Color" else "L"
            )
            for kind in ("Color", "Metalness", "Roughness")
            if f"{asset}_2K-JPG_{kind}.jpg" in bundle.namelist()
        }


def square(image: Image.Image) -> Image.Image:
    width, height = image.size
    tiled = Image.new(image.mode, (width, width))
    for top in range(0, width, height):
        tiled.paste(image, (0, top))
    return tiled.resize((SIZE, SIZE), Image.LANCZOS)


def runs(fraction: np.ndarray) -> list[tuple[int, int]]:
    edges = np.flatnonzero(np.diff(np.concatenate([[0], (fraction > 0.2).astype(int), [0]])))
    return [
        (start, end)
        for start, end in zip(edges[::2], edges[1::2], strict=True)
        if start > 0 and end < len(fraction)
    ]


def wall(found: dict[str, Image.Image]) -> tuple[Image.Image, Image.Image]:
    return square(found["Color"]), Image.new("L", (SIZE, SIZE), 0)


def curtain(found: dict[str, Image.Image]) -> tuple[Image.Image, Image.Image]:
    dark = np.asarray(found["Color"].convert("L")) < 110
    glass = ((np.asarray(found["Metalness"]) > 128) & dark).astype(np.uint8) * 255
    return square(found["Color"]), square(Image.fromarray(glass)).point(
        lambda v: 255 if v > 128 else 0
    )


def windows(found: dict[str, Image.Image]) -> tuple[Image.Image, Image.Image]:
    opening = np.asarray(found["Metalness"]) > 128
    columns, rows = runs(opening.mean(axis=0)), runs(opening.mean(axis=1))
    rects = [(left, top, right, bottom) for top, bottom in rows for left, right in columns][
        : CELLS * CELLS
    ]
    cell = SIZE // CELLS
    colour, mask = Image.new("RGB", (SIZE, SIZE)), Image.new("L", (SIZE, SIZE))
    glossy = np.asarray(found["Roughness"]) < 90
    for index, rect in enumerate(rects):
        at = ((index % CELLS) * cell, (index // CELLS) * cell)
        colour.paste(found["Color"].crop(rect).resize((cell, cell), Image.LANCZOS), at)
        left, top, right, bottom = rect
        glass = np.where(glossy[top:bottom, left:right], 255, 128).astype(np.uint8)
        mask.paste(Image.fromarray(glass).resize((cell, cell), Image.NEAREST), at)
    return colour, mask


def main() -> None:
    cache = Path(sys.argv[1])
    cache.mkdir(parents=True, exist_ok=True)
    build = {"wall": wall, "curtain": curtain, "windows": windows}
    layers = [build[kind](maps(asset, cache)) for asset, kind in MATERIALS]
    colour = Image.new("RGB", (SIZE, SIZE * len(layers)))
    mask = Image.new("L", (SIZE, SIZE * len(layers)))
    for index, (layer_colour, layer_mask) in enumerate(layers):
        colour.paste(layer_colour, (0, index * SIZE))
        mask.paste(layer_mask, (0, index * SIZE))
    OUT.mkdir(parents=True, exist_ok=True)
    colour.save(OUT / "colour.jpg", quality=86, optimize=True, progressive=False)
    mask.save(OUT / "mask.png", optimize=True)


if __name__ == "__main__":
    main()
