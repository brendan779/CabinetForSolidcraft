#!/usr/bin/env python3
"""Builds the official material packs (docs/material-packs.md): one per brand that has pictures,
from catalog/textures/<brand>.json, the brand's catalog/boards files, and the pictures in the decors
folder at full size; and one per brand of tile floors in catalog/floors, holding just its pictures
(the floor records ship with the app). Writes the zips and their index, packs.json, to the output
folder; publish them as the assets of the GitHub release `material-packs`.

    python3 scripts/make_material_packs.py [--out target/material-packs] [--version 2026.11.27]
        [--only beaumont-tiles] [--index published-packs.json]

With --only, just those packs are built, and they replace their entries in the --index given (the
release's current packs.json), so the other packs keep their versions and aren't downloaded again.
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


def build_floors(out, version):
    """A pack per brand in catalog/floors: the colour, height and grout pictures of its floors."""
    doc = json.loads((CATALOG / 'floors' / 'floors.json').read_text())
    brands = {}
    for f in doc['floors']:
        brands.setdefault(f['brand'], []).append(f)
    made = []
    for brand, floors in sorted(brands.items()):
        stem = brand.lower().replace(' ', '-')
        files, missing = {}, []
        for f in floors:
            for key in ('colour', 'height', 'grout'):
                p = picture(f[key])
                if p is None:
                    missing.append(f[key])
                elif not SAFE.match(p.name):
                    sys.exit(f'{stem}: unsafe file name {p.name}')
                else:
                    files[f'images/{p.name}'] = p.read_bytes()
        if missing:
            sys.exit(f'{stem}: missing {missing}')
        manifest = {
            'format': 'cabinet-studio-material-pack', 'format_version': 1,
            'id': stem, 'name': f'{brand} floor tiles', 'brand': brand, 'version': version,
            'description': f"{brand}'s tiles as Blender scene floors: a seamless grid of each with grout, a bump map and a grout mask.",
            'source': doc.get('source') or 'Cabinet Studio catalog',
            'files': {k: sha(v) for k, v in sorted(files.items())},
        }
        made.append(write_pack(out, stem, version, manifest, files))
    return made


def write_pack(out, stem, version, manifest, files):
    out.mkdir(parents=True, exist_ok=True)
    path = out / f'{stem}-{version}.zip'
    with zipfile.ZipFile(path, 'w', zipfile.ZIP_STORED) as z:
        z.writestr('pack.json', json.dumps(manifest, indent=2, ensure_ascii=False))
        for k, v in sorted(files.items()):
            z.writestr(k, v)
    data = path.read_bytes()
    return {'id': stem, 'name': manifest['name'], 'brand': manifest['brand'], 'version': version,
            'file': path.name, 'size': len(data), 'sha256': sha(data),
            'images': sum(1 for k in files if k.startswith('images/')), 'missing': []}


def main():
    p = argparse.ArgumentParser()
    p.add_argument('--out', type=Path, default=ROOT / 'target' / 'material-packs')
    p.add_argument('--version', default=datetime.date.today().strftime('%Y.%m.%d'))
    p.add_argument('--only', action='append')
    p.add_argument('--index', type=Path)
    a = p.parse_args()
    packs = []
    wanted = lambda stem: not a.only or stem in a.only
    built = [build(t, a.out, a.version) for t in sorted((CATALOG / 'textures').glob('*.json')) if wanted(t.stem)]
    built += [f for f in build_floors(a.out, a.version) if wanted(f['id'])]
    for made in built:
        if made:
            missing = made.pop('missing')
            print(f"{made['file']}: {made['images']} pictures, {made['size'] / 1e6:.0f} MB"
                  + (f'; missing {missing}' if missing else ''))
            packs.append(made)
    if a.only and a.index:
        kept = [p for p in json.loads(a.index.read_text())['packs'] if p['id'] not in {q['id'] for q in packs}]
        packs = sorted(kept + packs, key=lambda p: p['id'])
    index = {'format': 'cabinet-studio-material-packs', 'format_version': 1, 'packs': packs}
    (a.out / 'packs.json').write_text(json.dumps(index, indent=2, ensure_ascii=False) + '\n')
    print(f"{len(packs)} packs, {sum(p['size'] for p in packs) / 1e6:.0f} MB")


if __name__ == '__main__':
    main()
