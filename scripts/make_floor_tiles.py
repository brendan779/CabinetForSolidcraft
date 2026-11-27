#!/usr/bin/env python3
"""Makes a seamless tiled floor for the Blender scene from photographs of a tile's faces: a grid of
tiles (6 x 6 by default), each a face picked at random and turned a random quarter, never the same
face as the tile beside or above it, with grout joints half a joint wide at every edge so the
picture repeats without a seam. Writes three pictures into the decors folder:

  <id>-colour.jpg   the tiles, grout a neutral grey (the scene mixes the grout colour chosen over it)
  <id>-height.png   16-bit bump: each face's fine relief (its pits and texture, from the photo's
                    detail), the joints recessed and the tile edges eased
  <id>-grout.png    where the grout is (white)

and prints the numbers for its record in catalog/floors/floors.json.

    python3 -I scripts/make_floor_tiles.py FACES_DIR --id floor-beaumont-pompeii-travertine-crema \
        --tile 600 --grid 6 --grout 3 --px-per-tile 1024
"""
import argparse
import hashlib
import json
import random
from pathlib import Path

import numpy as np
from PIL import Image, ImageFilter

DECORS = Path.home() / 'Library' / 'Application Support' / 'CabinetStudio' / 'decors'


def faces(folder):
    """The distinct faces in the folder (a download often holds copies)."""
    seen, out = set(), []
    for f in sorted(Path(folder).iterdir()):
        if f.suffix.lower() not in ('.webp', '.jpg', '.jpeg', '.png'):
            continue
        h = hashlib.sha256(f.read_bytes()).hexdigest()
        if h not in seen:
            seen.add(h)
            out.append(f)
    return out


def relief(rgb):
    """A face's relief from its photograph: fine detail over two scales, the pits dark."""
    lum = np.asarray(rgb.convert('L'), dtype=np.float32) / 255
    def blur(r):
        return np.asarray(rgb.convert('L').filter(ImageFilter.GaussianBlur(r)), dtype=np.float32) / 255
    fine = lum - blur(3)
    mid = lum - blur(14)
    h = 0.6 * fine + 0.4 * mid
    h = (h - h.mean()) / (h.std() + 1e-6)
    return np.clip(0.5 + 0.12 * h, 0.05, 1.0)


def main():
    p = argparse.ArgumentParser()
    p.add_argument('faces')
    p.add_argument('--id', required=True)
    p.add_argument('--tile', type=float, default=600)
    p.add_argument('--grid', type=int, default=6)
    p.add_argument('--grout', type=float, default=3)
    p.add_argument('--px-per-tile', type=int, default=1024)
    p.add_argument('--seed', type=int, default=1245446)
    p.add_argument('--out', type=Path, default=DECORS)
    a = p.parse_args()

    found = faces(a.faces)
    if len(found) < 2:
        raise SystemExit('need at least two faces')
    scale = a.px_per_tile / a.tile                       # pixels per mm
    pitch = a.tile + a.grout                              # one tile and one joint, mm
    size = round(a.grid * pitch * scale)                  # the picture's side, px
    joint = a.grout * scale
    rng = random.Random(a.seed)
    tiles = [Image.open(f).convert('RGB').resize((a.px_per_tile, a.px_per_tile), Image.LANCZOS) for f in found]
    reliefs = [relief(t) for t in tiles]

    colour = np.full((size, size, 3), 0.55, dtype=np.float32)
    height = np.zeros((size, size), dtype=np.float32)
    grout = np.ones((size, size), dtype=np.float32)
    pick = {}
    # The edge ease: a tile's last couple of millimetres roll down towards the joint.
    ease_px = max(2, round(1.5 * scale))
    ramp = np.minimum(np.arange(a.px_per_tile), np.arange(a.px_per_tile)[::-1]).astype(np.float32)
    ramp = np.clip(ramp / ease_px, 0, 1)
    ease = np.minimum.outer(ramp, ramp) ** 0.5
    for row in range(a.grid):
        for col in range(a.grid):
            # A face unlike its neighbours to the left and above (wrapping, as the floor repeats).
            near = {pick.get((row, (col - 1) % a.grid)), pick.get(((row - 1) % a.grid, col))}
            if col == a.grid - 1:
                near.add(pick.get((row, 0)))
            if row == a.grid - 1:
                near.add(pick.get((0, col)))
            choices = [i for i in range(len(tiles)) if i not in near] or list(range(len(tiles)))
            i = rng.choice(choices)
            pick[(row, col)] = i
            turns = rng.randrange(4)
            shade = 1 + rng.uniform(-0.035, 0.035)
            face = np.rot90(np.asarray(tiles[i], dtype=np.float32) / 255, turns) * shade
            rel = np.rot90(reliefs[i], turns) * ease
            x0 = round((col * pitch + a.grout / 2) * scale)
            y0 = round((row * pitch + a.grout / 2) * scale)
            n = a.px_per_tile
            colour[y0:y0 + n, x0:x0 + n] = np.clip(face, 0, 1)
            height[y0:y0 + n, x0:x0 + n] = rel
            grout[y0:y0 + n, x0:x0 + n] = 0

    a.out.mkdir(parents=True, exist_ok=True)
    name = a.id
    Image.fromarray((colour * 255).round().astype(np.uint8)).save(a.out / f'{name}-colour.jpg', quality=93, subsampling=0)
    Image.fromarray((height * 65535).round().astype(np.uint16)).save(a.out / f'{name}-height.png')
    Image.fromarray((grout * 255).round().astype(np.uint8)).save(a.out / f'{name}-grout.png')
    side = a.grid * pitch
    record = {
        'id': name, 'tile': [a.tile, a.tile], 'grid': a.grid, 'grout_mm': a.grout,
        'covers': [side, side], 'pixels': [size, size],
        'colour': f'{name}-colour', 'height': f'{name}-height', 'grout': f'{name}-grout',
        'faces': len(tiles), 'seed': a.seed, 'joint_px': round(joint, 2),
    }
    print(json.dumps(record, indent=2))


if __name__ == '__main__':
    main()
