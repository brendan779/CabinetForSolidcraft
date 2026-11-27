#!/usr/bin/env python3
"""Laminate benchtops made to order, as products for the benchtop picker. The pictures are the
decors' own, from the board catalogue's textures/<brand>.json, where we have them.

polytec: every colour and finish its availability guide lists, at each thickness it is made in,
with the profiles it comes in (scripts/polytec-benchtops.json, parsed from the September 2026 guide).
Laminex: the Made to Measure colour range (scripts/laminex-benchtops.json) at 33 mm; each is also
made at 20, 25 and 38 mm.

  scripts/import-laminate-tops.py
"""
import json
from pathlib import Path
import re

HERE = Path(__file__).resolve().parent
CATALOG = HERE.parent / 'catalog'
# Tops are made to order from 33 mm and 21 mm board 3600 x 1800 (polytec's complete availability
# guide); a rolled edge limits the length to 3600 on one side, 3500 on two.
SHEET = {'length': 3600, 'width': 1800}
PROFILES = {'square_edge': '1 mm square edge', 'tight_form': 'tight form', 'radius_10': '10/10 radius'}


def slug(s):
    return re.sub(r'[^a-z0-9]+', '-', s.lower().replace("'", '')).strip('-')


def laminex():
    data = json.loads((HERE / 'laminex-benchtops.json').read_text())
    boards_file = json.loads((CATALOG / 'boards' / 'laminex.json').read_text())
    colour_of = {b['decor']: b.get('preview_colour') for b in boards_file['boards']}
    finish = {'Natural': 'Natural', 'AbsoluteGrain': 'AbsoluteGrain'}
    boards = [{
        'id': f"laminex.benchtop.{c['slug']}-{slug(c['finish'])}.33",
        'brand': 'Laminex', 'range': f"Made to Measure benchtops: {c['group']}",
        'decor': c['decor'], 'finish': finish.get(c['finish'], c['finish']), 'kind': 'laminate_top',
        'substrate': 'moisture_resistant_particleboard', 'thickness': 33,
        # Not stated for Made to Measure tops: the 3600 sheets the decors come in.
        'sheets': [{'length': 3600, 'width': 1200}],
        'grained': c['group'] == 'Woodgrains', 'double_sided': False,
        'preview_colour': colour_of.get(c['decor']) or '#b8b2a8',
        'source': {'url': f"https://www.laminex.com.au/products/{c['slug']}/{c['finish']}/p/{c['code']}",
                   'document_date': 'Made to Measure Benchtops colour range, live page checked 2026-11-27; National Availability Guide, October 2025',
                   'retrieved': '2026-11-27', 'verified': False},
    } for c in data['colours']]
    (CATALOG / 'boards' / 'laminex-benchtops.json').write_text(json.dumps({
        'brand': 'Laminex', 'website': 'https://www.laminex.com.au/brands/laminex/product-ranges/made-to-measure/benchtops',
        'notes': ('Laminex Made to Measure laminate benchtops: the colour range page\'s decors, each at '
                  '33 mm; the National Availability Guide (October 2025) gives "ALL OPTIONS AVAILABLE ON '
                  'PARTICLEBOARD MR IN 20, 25, 33 OR 38MM THICKNESS", and per colour the edges (self '
                  'edged, 1 or 2 mm edging, rolled 3, 8 or 10 mm, half or full bullnose) and finishes: '
                  'see the guide. Length: not stated; 3600 is our assumption. Kind laminate_top keeps '
                  'them out of cabinet board pickers. No prices inferred.'),
        'boards': sorted(boards, key=lambda b: b['id']), 'edge_tapes': []}, indent=2, ensure_ascii=False) + '\n')
    images = json.loads((CATALOG / 'textures' / 'laminex.json').read_text()).get('images', {})
    print(f"Laminex: {len(boards)} tops; {sum(1 for c in data['colours'] if c['decor'] in images)} with a picture")


def main():
    data = json.loads((HERE / 'polytec-benchtops.json').read_text())
    images = json.loads((CATALOG / 'textures' / 'polytec.json').read_text()).get('images', {})
    boards_file = json.loads((CATALOG / 'boards' / 'polytec.json').read_text())
    colour_of = {b['decor']: b.get('preview_colour') for b in boards_file['boards']}
    boards = []
    for c in data['colours']:
        made = {col: ok for col, ok in zip(data['columns'], c['available'])}
        for t in (21, 33):
            profiles = [PROFILES[col.rsplit('_', 1)[0]] for col, ok in made.items()
                        if ok and col.endswith(f'_{t}')]
            if not profiles:
                continue
            boards.append({
                'id': f"polytec.benchtop.{slug(c['colour'])}-{slug(c['finish'])}.{t}",
                'brand': 'Polytec', 'range': f"Benchtops: {', '.join(profiles)}",
                'decor': c['colour'], 'finish': c['finish'], 'kind': 'laminate_top',
                'substrate': 'particleboard', 'thickness': t, 'sheets': [SHEET],
                'grained': c['finish'] in ('Woodmatt', 'Ravine'), 'double_sided': False,
                'preview_colour': colour_of.get(c['colour']) or '#b8b2a8',
                'source': {'url': 'https://www.polytec.com.au/docs/technical/polytec-technical-benchtop-profiles-and-thickness-availability-by-colour.pdf',
                           'document_date': 'Availability Guide, Benchtops Profiles & Thicknesses, September 2026',
                           'retrieved': '2026-11-27', 'verified': False},
            })
    (CATALOG / 'boards' / 'polytec-benchtops.json').write_text(json.dumps({
        'brand': 'Polytec', 'website': 'https://www.polytec.com.au',
        'notes': ('polytec laminate benchtops, made to order: each colour and finish at 21 and 33 mm '
                  'where the September 2026 availability guide ticks a profile (1 mm square edge, '
                  'tight form, 10/10 radius; range names them). Sheet: the 3600 x 1800 board they are '
                  'made from; a rolled edge limits a top to 3600 (one side) or 3500 (two). Kind '
                  'laminate_top keeps them out of cabinet board pickers. No prices inferred.'),
        'boards': sorted(boards, key=lambda b: b['id']), 'edge_tapes': []}, indent=2, ensure_ascii=False) + '\n')
    pictured = sum(1 for c in data['colours'] if c['colour'] in images)
    print(f"polytec: {len(boards)} tops from {len(data['colours'])} colours; {pictured} colours have a picture")


if __name__ == '__main__':
    main()
    laminex()
