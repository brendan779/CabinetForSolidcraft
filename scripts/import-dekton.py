#!/usr/bin/env python3
"""Dekton (Cosentino, AU): the range from Cosentino's Australian colour pages (scripts/dekton-au.json)
and each colour's full-slab photo from Cosentino's image server. The photos stay on this Mac with the
other decors (owner's go-ahead to download them, 2026-11-27); not committed until the brand agrees.

  scripts/import-dekton.py            # download what's missing, write the catalog files
  scripts/import-dekton.py --offline  # catalog files from the photos already downloaded
"""
import argparse
import hashlib
import io
import json
from pathlib import Path
import subprocess

from PIL import Image, ImageStat

HERE = Path(__file__).resolve().parent
DECORS = Path.home() / 'Library' / 'Application Support' / 'CabinetStudio' / 'decors'
SLAB = 'https://assetstools.cosentino.com/api/v1/bynder/color/{code}/tablahd/{code}-fullslab.jpg?w=2200&h=2200&q=75&auto=format'
# Benchtop thicknesses: 8 and 4 mm are for cladding and furniture.
TOPS = [12, 20]
# Our visual estimates for rendering, not supplier measurements.
LOOK = {'Polished': (0.1, 0.0), 'Velvet': (0.38, 0.0), 'Matte': (0.42, 0.03)}


def fetch(url, dest):
    # Plain curl: no browser identity.
    subprocess.run(['curl', '-sfL', '--retry', '2', '-o', str(dest), url], check=True)


def main(catalog, offline):
    data = json.loads((HERE / 'dekton-au.json').read_text())
    raw = DECORS / 'dekton-originals'
    raw.mkdir(parents=True, exist_ok=True)
    boards, images, missing = [], {}, []
    for c in data['colours']:
        finish = next(f for f in c['finishes'] if f != 'Grip+')
        original = raw / f"{c['code']}-fullslab.jpg"
        if not original.exists() and not offline:
            fetch(SLAB.format(code=c['code']), original)
        # The photo shows the colour's largest slab.
        length, width = max(c['formats'], key=lambda f: f[0] * f[1])
        colour = None
        if original.exists():
            raw_bytes = original.read_bytes()
            im = Image.open(io.BytesIO(raw_bytes)).convert('RGB')
            if im.height > im.width:
                im = im.transpose(Image.Transpose.ROTATE_90)
            im.save(DECORS / f"dekton-{c['slug']}.jpg", quality=95, subsampling=0)
            sample = im.copy()
            sample.thumbnail((100, 100))
            colour = '#' + ''.join(f'{round(v):02x}' for v in ImageStat.Stat(sample).mean)
            roughness, micro = LOOK[finish]
            images[c['name']] = {
                'name': f"dekton-{c['slug']}", 'pixels': list(im.size), 'covers': [length, width],
                'url': None, 'normal': None, 'stone': True,
                'look': {'roughness': roughness, 'bump': 0, 'coat': 0, 'micro_bump': micro},
                'original_file': SLAB.format(code=c['code']).split('?')[0].rsplit('/', 1)[-1],
                'original_sha256': hashlib.sha256(raw_bytes).hexdigest(), 'discontinued': False,
            }
        else:
            missing.append(c['name'])
        for t in TOPS:
            if t not in c['thicknesses']:
                continue
            boards.append({
                'id': f"dekton.{c['collection'].lower().replace(' ', '-')}.{c['slug']}.{t}",
                'brand': 'Dekton', 'range': c['collection'].title().replace('Dk ', 'DK '),
                'decor': c['name'], 'finish': finish, 'kind': 'sintered_stone', 'substrate': 'other',
                'thickness': t, 'sheets': [{'length': l, 'width': w} for l, w in c['formats']],
                'grained': True, 'double_sided': False, 'preview_colour': colour or '#c8c4bc',
                'source': {'url': f"https://www.cosentino.com/en-au/colours/dekton/{c['slug']}/",
                           'document_date': 'live product page, checked 2026-11-27',
                           'retrieved': '2026-11-27', 'verified': False},
            })
    (catalog / 'boards' / 'dekton.json').write_text(json.dumps({
        'brand': 'Dekton', 'website': 'https://www.cosentino.com/en-au/colours/',
        'notes': ('Dekton by Cosentino, ultra-compact surface. The Australian range as shown on '
                  'cosentino.com/en-au, 2026-11-27: each colour at 12 and 20 mm (8 and 4 mm, for '
                  'cladding, left out), in every slab format its page lists. Grip+ (non-slip, for '
                  'floors) is not a benchtop finish. Kind sintered_stone is our slab kind for benchtop '
                  'pickers. Render roughness is our visual estimate. No prices inferred.'),
        'boards': sorted(boards, key=lambda b: b['id']), 'edge_tapes': []}, indent=2) + '\n')
    (catalog / 'textures' / 'dekton.json').write_text(json.dumps({
        'source': ('Cosentino full-slab photographs (each colour page\'s "detailed view of the full '
                   'slab"), 2200 px, downloaded 2026-11-27 with the owner\'s go-ahead.'),
        'notes': ('Each photo maps to its colour\'s largest slab format. Local only: the brand\'s images '
                  'live in ~/Library/Application Support/CabinetStudio/decors as <name>.jpg '
                  '(scripts/import-dekton.py), originals in decors/dekton-originals; never committed.'),
        'verified': False, 'images': dict(sorted(images.items()))}, indent=2) + '\n')
    print(f"{len(data['colours'])} colours, {len(boards)} slabs, {len(images)} photos"
          + (f'; missing {missing}' if missing else ''))


if __name__ == '__main__':
    p = argparse.ArgumentParser()
    p.add_argument('--catalog', type=Path, default=HERE.parent / 'catalog')
    p.add_argument('--offline', action='store_true')
    a = p.parse_args()
    main(a.catalog, a.offline)
