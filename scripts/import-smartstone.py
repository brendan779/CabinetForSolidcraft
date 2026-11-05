#!/usr/bin/env python3
"""Import the owner's Smartstone slab archive, retaining full images and their provenance.
Product sizes/finishes checked against smartstone.com.au/stone-benchtops/<slug>, 2026-11-05.
No image-derived relief: the stone's veins are colour, not grooves.
"""
import argparse
import hashlib
import io
import json
from pathlib import Path
import re
from zipfile import ZipFile
from PIL import Image, ImageStat


def import_archive(archive, catalog):
    images = {}
    boards = []
    dest = catalog / 'textures' / 'smartstone'
    dest.mkdir(parents=True, exist_ok=True)
    with ZipFile(archive) as z:
        for member in z.namelist():
            if member.startswith('__MACOSX/') or member.endswith('/'):
                continue
            data = z.read(member)
            name = Path(member).stem.replace('smartstone_full-slab_', '').replace('smartstone_', '')
            name = re.sub(r'(_slab_16x9|_16x9|-slab-16x9|-full-slab|_1)$', '', name)
            slug = name
            decor = slug.replace('-', ' ').title()
            finish = 'Suede' if slug in ['pietra-scuro', 'grafite-titanio'] else 'Matt'
            im = Image.open(io.BytesIO(data)).convert('RGB')
            filename = f'{slug}.jpg'
            if Path(member).suffix.lower() == '.jpg':
                (dest / filename).write_bytes(data)
            else:
                im.save(dest / filename, quality=96, subsampling=0)
            sample = im.copy()
            sample.thumbnail((100, 100))
            colour = '#' + ''.join(f'{round(c):02x}' for c in ImageStat.Stat(sample).mean)
            images[decor] = {
                'name': f'smartstone-{slug}', 'file': f'smartstone/{filename}',
                'pixels': list(im.size), 'covers': [3200, 1600],
                'url': None, 'normal': None, 'stone': True,
                'look': {'roughness': 0.48 if finish == 'Suede' else 0.32, 'bump': 0, 'coat': 0, 'micro_bump': 0.08 if finish == 'Suede' else 0},
                'original_file': Path(member).name, 'original_sha256': hashlib.sha256(data).hexdigest(),
                'discontinued': slug == 'super-white',
            }
            boards.append({
                'id': f'smartstone.sintered.{slug}.20', 'brand': 'Smartstone',
                'range': 'Sintered Collection' + (' (discontinued)' if slug == 'super-white' else ''),
                'decor': decor, 'finish': finish, 'kind': 'sintered_stone', 'substrate': 'other',
                'thickness': 20, 'sheets': [{'length': 3200, 'width': 1600}],
                'grained': True, 'double_sided': False, 'preview_colour': colour,
                'source': {'url': f'https://www.smartstone.com.au/stone-benchtops/{slug}',
                    'document_date': 'live product page, checked 2026-11-05', 'retrieved': '2026-11-05', 'verified': False},
            })
    (catalog / 'boards' / 'smartstone.json').write_text(json.dumps({
        'brand': 'Smartstone', 'website': 'https://www.smartstone.com.au',
        'notes': 'Owner supplied full slab images. 3200 x 1600 x 20 mm, Matt except Pietra Scuro/Grafite Titanio Suede. Super White discontinued. Render roughness is our visual estimate, not a supplier measurement. No prices or structural allowances inferred.',
        'boards': sorted(boards, key=lambda b: b['id']), 'edge_tapes': []}, indent=2) + '\n')
    (catalog / 'textures' / 'smartstone.json').write_text(json.dumps({
        'source': 'Owner supplied SmartstoneSinteredStone.zip, 2026-11-05; product metadata from the official pages linked in boards/smartstone.json.',
        'notes': 'Images map to the stated physical slab size, independently of pixel aspect. Originals with 16:9 filenames retained in full. JPEG originals copied unchanged; WebP decoded to JPEG for native and Blender compatibility. No relief inferred from veins. Local images; no runtime download.',
        'verified': False, 'images': dict(sorted(images.items()))}, indent=2) + '\n')
    print(f'Imported {len(boards)} slabs')


if __name__ == '__main__':
    p = argparse.ArgumentParser()
    p.add_argument('archive', type=Path)
    p.add_argument('--catalog', type=Path, default=Path(__file__).resolve().parents[1] / 'catalog')
    a = p.parse_args()
    import_archive(a.archive, a.catalog)
