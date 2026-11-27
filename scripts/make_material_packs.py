#!/usr/bin/env python3
"""Builds the official material packs (docs/material-packs.md): one per brand that has pictures,
from catalog/textures/<brand>.json, the brand's catalog/boards files, and the pictures in the decors
folder at full size. Writes the zips and their index, packs.json, to the output folder; publish them
as the assets of the GitHub release `material-packs`.

    python3 scripts/make_material_packs.py [--out target/material-packs] [--version 2026.11.27]
"""
import argparse
import datetime
import hashlib
import json
import os
import re
import sys
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CATALOG = ROOT / 'catalog'
DECORS = Path.home() / 'Library' / 'Application Support' / 'CabinetStudio' / 'decors'
SAFE = re.compile(r'^[A-Za-z0-9._-]+$')


def sha(data):
    return hashlib.sha256(data).hexdigest()


def picture(name):
    for ext in ('.jpg', '.png'):
        p = DECORS / f'{name}{ext}'
        if p.exists():
            return p
    return None


def build(textures, out, version):
    stem = textures.stem
    doc = json.loads(textures.read_text())
    images = doc.get('images') or {}
    if not images:
        return None
    files, missing = {}, []
    for decor, rec in sorted(images.items()):
        for key in ('name', 'normal'):
            name = rec.get(key)
            if not name:
                continue
            p = picture(name)
            if p is None:
                missing.append(name)
                continue
            if not SAFE.match(p.name):
                sys.exit(f'{stem}: unsafe file name {p.name}')
            files[f'images/{p.name}'] = p.read_bytes()
    files[f'catalog/textures/{textures.name}'] = textures.read_bytes()
    # The brand's board records: the files whose brand names this textures file.
    brand = None
    for b in sorted((CATALOG / 'boards').glob('*.json')):
        rec = json.loads(b.read_text())
        if rec.get('brand', '').lower().replace(' ', '-') == stem:
            brand = rec['brand']
            files[f'catalog/boards/{b.name}'] = b.read_bytes()
    manifest = {
        'format': 'cabinet-studio-material-pack', 'format_version': 1,
        'id': stem, 'name': f'{brand or stem} materials', 'brand': brand or stem, 'version': version,
        'description': f'The pictures of {brand or stem}\'s decors and slabs at full size, with how each maps.',
        'source': doc.get('source') or 'Cabinet Studio catalog',
        'files': {k: sha(v) for k, v in sorted(files.items())},
    }
    out.mkdir(parents=True, exist_ok=True)
    path = out / f'{stem}-{version}.zip'
    # Pictures are compressed already: stored, as they are.
    with zipfile.ZipFile(path, 'w', zipfile.ZIP_STORED) as z:
        z.writestr('pack.json', json.dumps(manifest, indent=2, ensure_ascii=False))
        for k, v in sorted(files.items()):
            z.writestr(k, v)
    data = path.read_bytes()
    return {'id': stem, 'name': manifest['name'], 'brand': manifest['brand'], 'version': version,
            'file': path.name, 'size': len(data), 'sha256': sha(data),
            'images': sum(1 for k in files if k.startswith('images/')), 'missing': missing}


def main():
    p = argparse.ArgumentParser()
    p.add_argument('--out', type=Path, default=ROOT / 'target' / 'material-packs')
    p.add_argument('--version', default=datetime.date.today().strftime('%Y.%m.%d'))
    a = p.parse_args()
    packs = []
    for t in sorted((CATALOG / 'textures').glob('*.json')):
        made = build(t, a.out, a.version)
        if made:
            missing = made.pop('missing')
            print(f"{made['file']}: {made['images']} pictures, {made['size'] / 1e6:.0f} MB"
                  + (f'; missing {missing}' if missing else ''))
            packs.append(made)
    index = {'format': 'cabinet-studio-material-packs', 'format_version': 1, 'packs': packs}
    (a.out / 'packs.json').write_text(json.dumps(index, indent=2, ensure_ascii=False) + '\n')
    print(f"{len(packs)} packs, {sum(p['size'] for p in packs) / 1e6:.0f} MB")


if __name__ == '__main__':
    main()
