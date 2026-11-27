#!/usr/bin/env python3
"""Imports ABI Interiors kitchen sinks from the BIM library (vendor-cad/abi-bim), as the handles are
(scripts/import_abi_handles.py): a catalog row each and a model from the product's IFC.

SINKS holds what each spec drawing prints: the sink's size, the bowls' opening at the rim (an
undermount cut-out: a double's divider sits under the top, so one opening spans both bowls, and a
drainer is part of it) and its corner radius, and for the granite sinks the body under the flange
(a top-mount cut-out). ABI prints neither as a cut-out; both are read from the drawing, so the
fabricator's template has the last word. The stainless sinks print no body under the flange: they
are listed for undermounting only. Farmhouse and butler sinks stand proud of the cabinets and are
left out.

Each model is re-oriented so the rim's top is at Z = 0 with the sink below it, its length on X and
its width on Y, centred on the opening, and written as a binary STL in vendor-cad/abi/sinks/.

    python3 scripts/import-abi-sinks.py [--dry-run]
"""
import glob
import json
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import import_abi_handles as h  # noqa: E402

BIM = 'vendor-cad/abi-bim'
OUT_DIR = 'vendor-cad/abi/sinks'
CATALOG = 'catalog/hardware/abi-sinks.json'

# name, folder (the one whose product notes and IFC are used), spec drawing, size (length, width,
# depth), opening (length, width, radius; round: diameter twice and radius half), body under the
# flange (None: not printed), bowls.
SINKS = [
    ('Vienna Double Sink 760mm', 'Vienna Double Sink 760mm (Overflow)', 'VIENNA_Double_Sink_Spec_UKNZ-01.jpg',
     (760, 450, 210), (710, 400, 5), (738, 428), 2),
    ('Vienna Single Sink 630mm', 'Vienna Single Sink 630mm (Overflow)', 'Vienna_Single_Sink_Spec_UKNZ.jpg',
     (630, 460, 235), (580, 410, 0), (590, 420), 1),
    ('Vienna Round Sink 450mm', 'Vienna Round Sink 450mm (Overflow)', 'Vienna_Round_Sink_Spec_UKNZ-01.jpg',
     (450, 450, 190), (375, 375, 187.5), (400, 400), 1),
    ('Vera Double Sink 790mm', 'Vera Double Sink 790mm (Overflow)', 'Vera_Double_Sink_No_Overflow_SPEC-01.jpg',
     (790, 450, 240), (730, 390, 0), (766, 426), 2),
    ('Vera Single Sink 560mm', 'Vera Single Sink 560mm (Overflow)', 'Vera_Single_Overflow__SPEC-01.jpg',
     (560, 430, 220), (500, 370, 25), (520, 390), 1),
    ('Vera Round Sink 420mm', 'Vera Round Sink 420mm', 'VeraRoundSink_SPEC-01.jpg',
     (420, 420, 220), (360, 360, 180), None, 1),
    ('Vita Double Kitchen Sink 760mm', 'Vita Double Kitchen Sink 760mm', 'Vita Kitchen Sink_SPEC-01.jpg',
     (760, 450, 200), (710, 400, 10), None, 2),
    ('Zalo Double Kitchen Sink 855mm', 'Zalo Double Kitchen Sink 855mm', 'Zalo_Kitchen_Sink_SPEC-01.jpg',
     (855, 450, 200), (805, 400, 10), None, 2),
    ('Ontario Double Kitchen Sink 1180mm', 'Ontario Double Kitchen Sink', 'Ontario_Sink_SPEC-01.jpg',
     (1180, 450, 200), (1130, 400, 10), None, 2),
    ('Seba Single Sink 550mm', 'Seba Single Sink 550mm', 'Seba_Kitchen_Sink_SPEC-01.jpg',
     (550, 450, 200), (500, 400, 10), None, 1),
    ('Seba Single Sink 550mm (Deep)', 'Seba Single Sink 550mm (Deep)', 'SEBA_Single_Kitchen_Sink_Deep_Spec-01.jpg',
     (550, 450, 250), (500, 400, 10), None, 1),
    ('Sola Outdoor Single Sink 550mm', 'Sola Outdor Single Sink 550mm', 'SOLA_Kitchen_Sink_SPEC-01.jpg',
     (550, 450, 200), (500, 400, 10), None, 1),
    ('Sola Small Outdoor Sink', 'Sola Small Outdoor Sink', 'Sola_Small_Outdoor_Sink_SPEC.jpg',
     (220, 440, 205), (180, 400, 10), None, 1),
    ('Ohelu Single Kitchen Sink 380mm', 'Ohelu Single Kitchen Sink 380mm', 'Ohelu_Sink_Spec-01.jpg',
     (380, 440, 200), (340, 400, 10), None, 1),
    ('Vari Single Sink 750mm', 'Vari Single Sink 750mm', 'Vari_Sink_Spec-01.jpg',
     (750, 450, 200), (700, 400, 10), None, 1),
    ('Jai Single Kitchen Sink 880mm', 'Jai Single Kitchen Sink 880mm',
     'Jai_Single_Kitchen_Sink_880mm_-_Brushed_Gunmetal_-_Specifications_1.gif',
     (880, 450, 200), (830, 400, 10), None, 1),
]


# Where the chosen folder's IFC holds no mesh: the overflow version, the same size.
ALT = {
    'Vita Double Kitchen Sink 760mm': 'Vita - Kitchen Sink Double 760mm (Overflow)',
    'Zalo Double Kitchen Sink 855mm': 'Zalo - Kitchen Sink Double 855mm (Overflow)',
    'Seba Single Sink 550mm (Deep)': 'Seba - Overflow Sink 550mm (250 Deep) (Overflow)',
    'Ohelu Single Kitchen Sink 380mm': 'Ohelu Single Sink 380mm (Overflow)',
    'Vari Single Sink 750mm': 'Vari - Single Sink 750mm (Overflow)',
}


def model(folder, size):
    """The IFC's triangles in sink axes: length on X, width on Y, the rim's top at Z = 0 and the
    sink below it, centred on its outline. None without a usable mesh."""
    tris = None
    for f in glob.glob(os.path.join(folder, '*.ifc')):
        parsed = h.parse_ifc(f)
        if parsed and parsed[0]:
            tris = parsed[0]
            break
    if not tris:
        for f in glob.glob(os.path.join(folder, '*.fbx')):
            parsed = h.parse_fbx(f)
            if parsed and parsed[0]:
                tris = parsed[0]
                break
    if not tris:
        return None
    pts = [p for t in tris for p in t]
    lo = [min(p[i] for p in pts) for i in range(3)]
    hi = [max(p[i] for p in pts) for i in range(3)]
    ext = [hi[i] - lo[i] for i in range(3)]
    # Units: the length printed against the model's longest extent.
    scale = max(size[0], size[1]) / max(ext)
    # Up is the axis nearest the printed depth; the longer of the others runs along the sink.
    up = min(range(3), key=lambda i: abs(ext[i] * scale - size[2]))
    flat = [i for i in range(3) if i != up]
    along, across = (flat if ext[flat[0]] >= ext[flat[1]] else flat[::-1])
    if size[1] > size[0]:
        along, across = across, along
    axis = lambda i: tuple(scale if k == i else 0.0 for k in range(3))
    origin = tuple((lo[i] + hi[i]) / 2 if i != up else hi[i] for i in range(3))
    got = [round(ext[along] * scale), round(ext[across] * scale), round(ext[up] * scale)]
    return tris, axis(along), axis(across), axis(up), origin, got


def notes(folder):
    txt = open(os.path.join(folder, 'product.txt'), encoding='utf-8').read()
    url = re.search(r'Source URL: (\S+)', txt)
    sku = re.search(r'SKU: (\d+)', txt)
    finishes = re.findall(r'^- ([A-Z][\w ]+): https://www\.abiinteriors\.com\.au/product/', txt, re.M)
    return (url.group(1) if url else ''), (int(sku.group(1)) if sku else 0), finishes


def main(dry):
    os.makedirs(OUT_DIR, exist_ok=True)
    items, report = [], []
    for name, folder, spec, size, opening, body, bowls in SINKS:
        path = os.path.join(BIM, f'BIMobject-{folder}')
        url, sku, finishes = notes(path)
        slug = h.slug(name)
        article = f'ABI-SINK-{slug.upper()}'
        got = None
        m = model(path, size)
        if not m and name in ALT:
            m = model(os.path.join(BIM, f'BIMobject-{ALT[name]}'), size)
        if m:
            tris, x, y, z, origin, got = m
            if not dry:
                h.write_stl(os.path.join(OUT_DIR, f'{article}.stl'), tris, x, y, z, origin)
        report.append((name, size, got))
        params = {
            'length': size[0], 'width': size[1], 'depth': size[2], 'bowls': bowls,
            'under_length': opening[0], 'under_width': opening[1], 'under_radius': opening[2],
        }
        if body:
            params['top_length'], params['top_width'] = body
            params['top_radius'] = opening[2] if opening[0] == opening[1] else 0
        if sku:
            params['sku'] = sku
        items.append({
            'id': f'sink.abi.{slug}', 'brand': 'ABI Interiors', 'article': article,
            'family': f'ABI {name.split()[0]}', 'variant': name, 'kind': 'sink',
            'parameters': params,
            'source': {
                'url': (f'ABI Interiors {name} ({url}); finishes {", ".join(finishes) or "as listed"} '
                        f'(product.txt of the BIMobject download); sizes and the opening from ABI\'s spec '
                        f'drawing {spec} ("dimensions are nominal measurements only"); the cut-outs are read '
                        f'from it, not printed as such; model from the product\'s IFC by '
                        f'scripts/import-abi-sinks.py'),
                'document_date': '2026-10-27', 'retrieved': '2026-11-27', 'verified': False,
            },
        })
    doc = {
        'brand': 'ABI Interiors', 'website': 'https://www.abiinteriors.com.au',
        'notes': ('ABI kitchen sinks (BENCH-15a), from the BIM library downloads: size, the bowls\' opening '
                  'at the rim (undermount cut-out) and, for granite sinks, the body under the flange '
                  '(top-mount cut-out), all from the spec drawings; models re-oriented from the IFC, rim '
                  'top at Z = 0. A sink without top_length is undermount only here: ABI prints no body '
                  'size to cut for. Not verified by a person.'),
        'items': items,
    }
    for r in report:
        print(*r)
    if not dry:
        open(CATALOG, 'w').write(json.dumps(doc, indent=2, ensure_ascii=False) + '\n')


if __name__ == '__main__':
    main('--dry-run' in sys.argv)
