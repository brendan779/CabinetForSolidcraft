#!/usr/bin/env python3
"""Zenith Surfaces (Stone Ambassador, AU): the range, sizes and finishes from the June 2026 brochure,
and each design's slab photo from its product listing. The photos stay on this Mac with the other
decors (owner's go-ahead to download them, 2026-11-27); not committed until the brand agrees.

  scripts/import-zenith.py            # download what's missing, write the catalog files
  scripts/import-zenith.py --offline  # catalog files from the photos already downloaded
"""
import argparse
import hashlib
import io
import json
from pathlib import Path
import re
import subprocess

from PIL import Image, ImageStat

DECORS = Path.home() / 'Library' / 'Application Support' / 'CabinetStudio' / 'decors'
BROCHURE = ('https://stoneambassador.com.au/wp-content/uploads/2023/03/'
            'Zenith-Surfaces-Brochure-2026-June-Release-Colours-22-June-_compressed.pdf')
LISTING = 'https://stoneambassador.com.au/zenith-surfaces-vcs/'
UPLOADS = 'https://stoneambassador.com.au/wp-content/uploads/'

# Brochure page 12 (tiers) and pages 9-11 (finishes): polished unless M matte, T textured;
# PM pattern match. All 20 mm, 3200 x 1600. Dubai Drift: page 6 says (M), pages 11 and 12 say
# (T): taken as textured. Blanco Bliss and Albus Gris are WA only (listing).
RANGE = {
    'Essential': ['Sugar Rush', 'Sparkling White', 'Dapple White', 'Mottle Grey', 'Savannah',
                  'White Burst', 'Crystal'],
    'Deluxe': ['Pearl White', 'Mink Grey', 'Fossil', 'White Truffle', 'Mero', 'Kirec',
               'Blanco Bliss', 'Albus Gris'],
    'Premium': ['Carrara Mist', 'Lagos Grey', 'Casablanca', 'Graphene', 'Marrakesh', 'Suave',
                'Le Blanc', 'Firtina', 'Valour', 'Element Grey', 'Calacatta Signature',
                'Ottoman Grey', 'Rosé', 'Zayan', 'Vanilla Sky', 'Sarva White', 'Amina'],
    'Prestige': ['Marbella', 'Raiden', 'Whisper White', 'Allure Blue', 'Greylac', 'Naica',
                 'Bluetta', 'Gracious Grey', 'Bescato Bianco', 'Perla Venata', 'Perla Santana',
                 'Leylak', 'Sahara Swirl'],
    'Signature': ['Dulce De Leche', 'Alpine Drift', 'Le Taj', 'Dubai Drift', 'Lumira',
                  'Lumira Gold', 'Marmo Rosa'],
}
FINISH = {'Kirec': 'Matte', 'Firtina': 'Textured', 'Valour': 'Textured', 'Element Grey': 'Textured',
          'Amina': 'Textured', 'Dubai Drift': 'Textured'}
PATTERN_MATCH = {'Calacatta Signature', 'Zayan'}
WA_ONLY = {'Blanco Bliss', 'Albus Gris'}
NEW = {'Amina', 'Sarva White', 'Leylak', 'Sahara Swirl', 'Perla Santana', 'Dubai Drift', 'Lumira',
       'Lumira Gold', 'Marmo Rosa'}

# Each design's product page and photo, from the listing (read 2026-11-27). The listing's cards
# for Bluetta and Marbella show each other's file: each takes the file named for it.
PAGES = {
    'Sahara Swirl': ('sahara-swirl', '2026/04/Untitled-2-05-scaled.webp'),
    'Sarva White': ('sarva-white', '2026/04/Untitled-2-04-scaled.webp'),
    'Perla Santana': ('perla-santana', '2026/07/Zenith-new-colours-with-new-stickers-scaled.webp'),
    'Dubai Drift': ('dubai-drift', '2026/04/Untitled-2-02-scaled.webp'),
    'Lumira Gold': ('lumira-gold', '2026/04/Untitled-2-07-scaled.webp'),
    'Marmo Rosa': ('marmo-rosa', '2026/04/Untitled-2-09-scaled.webp'),
    'Amina': ('amina', '2026/06/22-Zenith-new-colours-with-new-stickers-scaled.webp'),
    'Lumira': ('lumira', '2026/04/Untitled-2-08-scaled.webp'),
    'Leylak': ('leylak', '2026/04/Untitled-2-03-scaled.webp'),
    'Blanco Bliss': ('blanco-bliss-wa-only', '2024/05/Blanco-Bliss-WA-Only.webp'),
    'Albus Gris': ('albus-gris-wa-only', '2024/05/Albus-Gris-WA-Only.webp'),
    'Alpine Drift': ('alpine-drift', '2025/08/Alpine_Drift_500x760-1.webp'),
    'Le Taj': ('le-taj', '2025/08/Le_Taj_500x760-2.webp'),
    'Dulce De Leche': ('dulce-de-leche', '2025/08/Dulce_de_Leche_500x760.webp'),
    'Naica': ('naica', '2025/08/Naica_500x760-e1755217579923.webp'),
    'Le Blanc': ('le-blanc', '2025/08/Le_Blanc_500x760-1.webp'),
    'Rosé': ('rose', '2025/08/rose-updated-460.png'),
    'Suave': ('suave', '2025/08/Suave_500x760.webp'),
    'Marrakesh': ('marrakesh', '2024/05/Marrakesh-img-01.webp'),
    'Vanilla Sky': ('vanilla-sky', '2024/05/Vanilla-Sky-img-01.webp'),
    'Whisper White': ('whisper-white', '2024/05/Whisper-White-img-01.webp'),
    'Greylac': ('greylac', '2024/05/Greylac-img-01.webp'),
    'Allure Blue': ('allure-blue', '2024/05/Allure-Blue-img-01.webp'),
    'Perla Venata': ('perla-venata', '2024/05/Perla-Venata-img-01.webp'),
    'Mink Grey': ('mink-grey', '2024/05/mink-grey-img-01.webp'),
    'Graphene': ('graphene', '2024/05/Graphene-img-01.webp'),
    'Bescato Bianco': ('bescato-bianco', '2024/05/Bescato-Bianco-img-01.webp'),
    'Mottle Grey': ('mottle-grey-2', '2023/02/Mottle-Grey.webp'),
    'Dapple White': ('dapple-white', '2023/02/Dapple-White.webp'),
    'Sparkling White': ('sparkling-white', '2023/02/Sparkling-White.webp'),
    'Sugar Rush': ('sugar-rush', '2023/02/Sugar-Rush.webp'),
    'White Burst': ('white-burst', '2023/02/White-Burst.webp'),
    'Crystal': ('crystal', '2023/02/Crystal.webp'),
    'Savannah': ('savannah', '2023/02/Savannah-Polished.webp'),
    'Fossil': ('fossil', '2023/02/Fossil.webp'),
    'Pearl White': ('pearl-white', '2023/02/Pearl-White.webp'),
    'Kirec': ('kirec', '2023/02/Kirec.webp'),
    'Mero': ('mero', '2023/02/Mero.webp'),
    'White Truffle': ('white-truffle', '2023/02/White-Truffle.webp'),
    'Carrara Mist': ('carrara-mist', '2023/02/Carrara-Mist.webp'),
    'Lagos Grey': ('lagos-grey', '2023/02/Lagos-Grey.webp'),
    'Element Grey': ('element-grey', '2023/02/Element-Grey.webp'),
    'Valour': ('valour', '2023/02/Valour.webp'),
    'Firtina': ('firtina', '2023/02/firtina.webp'),
    'Casablanca': ('casablanca', '2023/02/Casablanca.webp'),
    'Calacatta Signature': ('calacatta-signature', '2023/02/Calacatta-Signature.webp'),
    'Ottoman Grey': ('1938-ottoman-grey-s', '2023/02/sample-01-min.webp'),
    'Gracious Grey': ('1092-gracious-grey-s', '2023/02/Gracious-Grey.webp'),
    'Zayan': ('1176-zayan-s', '2023/02/ZAyan.webp'),
    'Bluetta': ('1522-bluetta-s', '2023/02/Bluetta.webp'),
    'Marbella': ('1482-marbella-pr', '2023/02/Marbella.webp'),
    'Raiden': ('1396-raiden-pr', '2023/02/Raiden.webp'),
}

# Our visual estimates for rendering, not supplier measurements.
LOOK = {'Polished': (0.12, 0.0), 'Matte': (0.32, 0.0), 'Textured': (0.48, 0.08)}


def slug(decor):
    return re.sub(r'[^a-z0-9]+', '-', decor.lower().replace('é', 'e')).strip('-')


def fetch(url, dest):
    # Plain curl: no browser identity.
    subprocess.run(['curl', '-sfL', '--retry', '2', '-o', str(dest), url], check=True)


def main(catalog, offline):
    raw = DECORS / 'zenith-originals'
    raw.mkdir(parents=True, exist_ok=True)
    boards, images, missing = [], {}, []
    for tier, decors in RANGE.items():
        for decor in decors:
            page, upload = PAGES[decor]
            original = raw / Path(upload).name
            if not original.exists() and not offline:
                fetch(UPLOADS + upload, original)
            finish = FINISH.get(decor, 'Polished')
            s = slug(decor)
            colour = None
            if original.exists():
                data = original.read_bytes()
                im = Image.open(io.BytesIO(data)).convert('RGB')
                # Slabs are photographed portrait; the catalogue's sheet is landscape.
                if im.height > im.width:
                    im = im.transpose(Image.Transpose.ROTATE_90)
                # The 2026 release photos carry a white "NEW" sticker by the lower left: cover it
                # with the stone beside it, keeping the slab's scale.
                if decor in NEW and im.size == (2560, 1685):
                    x0, y0, x1, y1 = 105, 1185, 305, 1580
                    im.paste(im.crop((x1, y0, x1 + (x1 - x0), y1)), (x0, y0))
                im.save(DECORS / f'zenith-{s}.jpg', quality=95, subsampling=0)
                sample = im.copy()
                sample.thumbnail((100, 100))
                colour = '#' + ''.join(f'{round(c):02x}' for c in ImageStat.Stat(sample).mean)
                roughness, micro = LOOK[finish]
                images[decor] = {
                    'name': f'zenith-{s}', 'pixels': list(im.size), 'covers': [3200, 1600],
                    'url': None, 'normal': None, 'stone': True,
                    'look': {'roughness': roughness, 'bump': 0, 'coat': 0, 'micro_bump': micro},
                    'original_file': upload, 'original_sha256': hashlib.sha256(data).hexdigest(),
                    'discontinued': False,
                }
            else:
                missing.append(decor)
            notes = [n for n, on in [('pattern match', decor in PATTERN_MATCH),
                                     ('WA only', decor in WA_ONLY)] if on]
            boards.append({
                'id': f'zenith.vcs.{s}.20', 'brand': 'Zenith Surfaces',
                'range': f'{tier}' + (f' ({", ".join(notes)})' if notes else ''),
                'decor': decor, 'finish': finish, 'kind': 'sintered_stone', 'substrate': 'other',
                'thickness': 20, 'sheets': [{'length': 3200, 'width': 1600}],
                'grained': True, 'double_sided': False, 'preview_colour': colour or '#d8d4cc',
                'source': {'url': f'https://stoneambassador.com.au/colours/{page}/',
                           'document_date': 'Zenith Surfaces brochure, June 2026 release (22 June 2026)',
                           'retrieved': '2026-11-27', 'verified': False},
            })
    (catalog / 'boards' / 'zenith.json').write_text(json.dumps({
        'brand': 'Zenith Surfaces', 'website': 'https://stoneambassador.com.au/zenith-surfaces-vcs/',
        'notes': ('Zenith Surfaces by Stone Ambassador: vitrified compact surface (VCS), 0% crystalline '
                  'silica, up to 89% recycled. The June 2026 brochure\'s current range: 20 mm, 3200 x 1600, '
                  'polished unless stated. Clearance and WA-only designs not in the brochure are left out. '
                  f'Brochure: {BROCHURE}. Kind sintered_stone is our slab kind for benchtop pickers. Render '
                  'roughness is our visual estimate. No prices or structural allowances inferred.'),
        'boards': sorted(boards, key=lambda b: b['id']), 'edge_tapes': []}, indent=2, ensure_ascii=False) + '\n')
    (catalog / 'textures' / 'zenith-surfaces.json').write_text(json.dumps({
        'source': f'Stone Ambassador product listing {LISTING}, read and downloaded 2026-11-27 (owner approved).',
        'notes': ('One photo per design from its listing card, rotated to landscape and mapped to the '
                  'whole 3200 x 1600 slab. Sizes vary from 500 x 760 to 2560 px: the smaller ones are '
                  'soft close up. Local only: the brand\'s images live in '
                  '~/Library/Application Support/CabinetStudio/decors as <name>.jpg '
                  '(scripts/import-zenith.py), originals in decors/zenith-originals; never committed.'),
        'verified': False, 'images': dict(sorted(images.items()))}, indent=2, ensure_ascii=False) + '\n')
    print(f'{len(boards)} designs, {len(images)} with photos' + (f'; missing {missing}' if missing else ''))


if __name__ == '__main__':
    p = argparse.ArgumentParser()
    p.add_argument('--catalog', type=Path, default=Path(__file__).resolve().parents[1] / 'catalog')
    p.add_argument('--offline', action='store_true')
    a = p.parse_args()
    main(a.catalog, a.offline)
