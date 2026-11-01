#!/usr/bin/env python3
"""Imports ABI Interiors cabinetry handles from the BIM library (vendor-cad/abi-bim).

Each product folder there (a BIMobject download) holds an IFC exported from ABI's
Revit family: a faceted mesh in an arbitrary frame, with the finish colour, plus
product.txt with the SKUs, finishes and prices. ABI names a pull by its screw
centres, and its models put the mounting feet at the ends of that same span.

For every cabinetry pull and knob this script:

  1. parses the IFC mesh (IFCPOLYLOOP faces) and its IFCCOLOURRGB,
  2. finds the mounting face (the extreme plane most vertices share), the length
     axis across the screw span and the width axis,
  3. re-orients the mesh into the app's handle frame — screw holes on X at
     ±centres/2, the door face at Y = 0, the handle standing out towards -Y —
     and writes it as a binary STL under vendor-cad/abi/<article>.stl,
  4. records the catalog row (measured centres/projection/size, the finish list
     from product.txt with SKUs and prices, and the model's colour as that
     finish's preview) into catalog/hardware/abi-handles.json.

The STLs stay local (vendor-cad is never committed); the catalog holds facts
with their source. Run from the repository root:

    python3 scripts/import_abi_handles.py [--dry-run]
"""
import glob
import json
import os
import re
import struct
import sys

BIM = 'vendor-cad/abi-bim'
OUT_DIR = 'vendor-cad/abi'
CATALOG = 'catalog/hardware/abi-handles.json'

# Cabinetry hardware only: pulls, knobs. Flush pulls sit in a routed pocket the
# front model cannot machine yet (a rebate op, WF-4's finger pull is closest),
# and entry-door pulls and tapware are out of scope; their folders stay put.
KIND_WORDS = {
    'CABINETRY', 'PULL', 'PULLS', 'KNOB', 'T', 'FARMHOUSE', 'EXTENDED', 'PAIR',
}
# Cabinetry hardware only: pulls and knobs on cabinet fronts. Flush pulls need a
# routed pocket (no rebate op for them yet); entry-door pulls and tapware are
# other products entirely.
CABINETRY = re.compile(
    r'(Cabinetry (Pull|Knob|T Pull)|Classic Pull|Round Pull|Half Moon Pull|'
    r'Kurve Pull|Farmhouse Cabinetry|Large Knob|Small Knob)', re.I)
SKIP = re.compile(r'Stainless Steel|Entry Door|Flush Pull', re.I)


def family_of(folder_name):
    """('Tezra Textured', 'Pull', 220.0) for 'BIMobject-Tezra Textured Cabinetry Pull 220mm'.

    The family is the product's own name with the category words and the size
    stripped, so every geometry gets its own article (Hegman Round and Hegman
    Half Moon both exist at 200 mm).
    """
    name = folder_name.removeprefix('BIMobject-')
    if not CABINETRY.search(name) or SKIP.search(name):
        return None
    mm = re.search(r'(\d+(?:\.\d+)?)\s*mm', name)
    named = float(mm.group(1)) if mm else None
    tokens = name.split()
    while tokens and (tokens[-1].upper().strip('-–') in KIND_WORDS | {''}
                      or re.fullmatch(r'[-–]?\d+(\.\d+)?(mm)?|mm|\(\w+\)', tokens[-1], re.I)):
        tokens.pop()
    if not tokens:
        return None
    kind = 'Knob' if 'Knob' in name else ('TPull' if 'T Pull' in name else 'Pull')
    return ' '.join(tokens), kind, named


def parse_ifc(path):
    text = open(path, encoding='latin-1').read()
    if 'DATA;' not in text:
        return None
    data = text[text.index('DATA;'):text.index('ENDSEC;', text.index('DATA;'))]
    pts, loops, colours = {}, [], []
    for m in re.finditer(r"#(\d+)\s*=\s*(.*?);\s*(?=#|\Z)", data, re.S):
        s = m.group(2).strip()
        if s.startswith('IFCCARTESIANPOINT'):
            mm = re.findall(r"\(([^()]*)\)", s)
            try:
                v = [float(x) for x in mm[-1].split(',')]
                if len(v) == 3:
                    pts[int(m.group(1))] = v
            except ValueError:
                pass
        elif s.startswith('IFCPOLYLOOP'):
            lp = [pts[int(r)] for r in re.findall(r"#(\d+)", s) if int(r) in pts]
            if len(lp) >= 3:
                loops.append(lp)
        elif 'IFCCOLOURRGB' in s and '(' in s:
            try:
                inner = s[s.index('(') + 1:s.rindex(')')]
                c = [float(x) for x in inner.split(',')[-3:]]
                colours.append(tuple(c))
            except ValueError:
                pass
    return (loops, colours[0] if colours else None) if loops else None


def unit(v):
    n = sum(c * c for c in v) ** 0.5
    return [c / n for c in v]


def cross(a, b):
    return [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]


def sub(a, b):
    return [a[i] - b[i] for i in range(3)]


def dot(a, b):
    return sum(a[i] * b[i] for i in range(3))


def components(loops):
    """Handle multi-solid IFC exports: rejoin everything except a duplicated Pair.

    Revit exports a family's solids un-stitched (an adjustable pull is its bar
    plus end caps as separate shells), so a split means one product, not two —
    unless every piece is the same size, when the file models a sold-as-pair
    product twice and one copy is kept.
    """
    key = lambda p: (round(p[0], 2), round(p[1], 2), round(p[2], 2))
    verts = {}
    edges = []
    for lp in loops:
        idx = []
        for p in lp:
            k = key(p)
            if k not in verts:
                verts[k] = len(verts)
            idx.append(verts[k])
        for i in range(len(idx)):
            edges.append((idx[i], idx[(i + 1) % len(idx)], lp))
    parent = list(range(len(verts)))

    def find(a):
        while parent[a] != a:
            parent[a] = parent[parent[a]]
            a = parent[a]
        return a

    for a, b, _ in edges:
        ra, rb = find(a), find(b)
        if ra != rb:
            parent[ra] = rb
    groups = {}
    for a, b, lp in edges:
        groups.setdefault(find(a), []).append(lp)
    if len(groups) <= 1:
        return loops
    sizes = []
    for g in groups.values():
        pts = [p for lp in g for p in lp]
        sizes.append([round(max(p[a] for p in pts) - min(p[a] for p in pts), 1) for a in range(3)])
    same = all(
        all(abs(sizes[0][a] - s[a]) < 1.0 for a in range(3))
        for s in sizes[1:]
    )
    if same and len(groups) == 2:
        return max(groups.values(), key=len)  # a Pair: one copy
    return [lp for g in groups.values() for lp in g]  # one product, many solids


def frame(loops, name):
    """The app-frame axes from the mesh: (x, y, z, origin, centres, knob) with y into the door.

    Candidate mounting planes are extremes with ≥ 4 vertices spanning at least
    60 % of another axis. ABI names a pull by its screw centres, so with a named
    size the winning candidate is the one whose feet span matches it (± 15 mm);
    without one, the face spanning the longest axis wins, and among equals the
    one whose vertices hug the centre of the third axis (feet, not a bar side).
    A face spanning both other axes with no gap between the feet is a knob's
    base: one screw at the centre.
    """
    allp = [p for lp in loops for p in lp]
    distinct = {tuple(round(c, 3) for c in p) for p in allp}
    dv = [list(p) for p in distinct]
    lo = [min(p[i] for p in dv) for i in range(3)]
    hi = [max(p[i] for p in dv) for i in range(3)]
    size = [hi[i] - lo[i] for i in range(3)]
    named = re.search(r'(\d+(?:\.\d+)?)\s*mm', name)
    named = float(named.group(1)) if named else None
    cands = []
    for ax in range(3):
        for val in (lo[ax], hi[ax]):
            on = [p for p in dv if abs(p[ax] - val) < 0.3]
            if len(on) < 4:
                continue
            spans = {}
            for a in range(3):
                if a == ax:
                    continue
                span = max(p[a] for p in on) - min(p[a] for p in on)
                if span >= 0.6 * size[a]:
                    spans[a] = span
            if spans:
                cands.append((ax, val, on, spans))
    if not cands:
        return None
    # A knob: the face with the most vertices, small (its two spans under
    # 100 mm and within 25 % of each other), its stand-off under 60 mm, and one
    # filled disc — not two feet, which make a pull despite a square base.
    for ax, val, on, spans in sorted(cands, key=lambda c: -len(c[2])):
        rest = [a for a in range(3) if a != ax]
        if (len(spans) != 2 or len(on) < 8 or max(size[a] for a in rest) > 100.0
                or size[ax] > 60.0):
            continue
        if abs(size[rest[0]] - size[rest[1]]) > 0.25 * max(size[rest[0]], size[rest[1]]):
            continue
        feet = False
        for a in rest:
            xs = sorted(p[a] for p in on)
            span = xs[-1] - xs[0]
            mid = len(xs) // 2
            if span > 30 and xs[mid] - xs[mid - 1] > 0.35 * span and (
                xs[mid - 1] - xs[0] < 0.2 * span and xs[-1] - xs[mid] < 0.2 * span
            ):
                feet = True
        if not feet:
            into = [0.0, 0.0, 0.0]
            into[ax] = 1.0 if val == hi[ax] else -1.0
            length = max(rest, key=lambda a: size[a])
            width = next(a for a in rest if a != length)
            x = [0.0, 0.0, 0.0]
            x[length] = 1.0
            z = unit(cross(x, into))
            origin = [(lo[a] + hi[a]) / 2.0 for a in range(3)]
            origin[ax] = val
            return x, into, z, origin, size, 0.0, True, length, ax, width
    # A pull: the face whose feet span matches the named centres closest; the
    # name is ABI's specification, the measured span is recorded beside it.
    pick = None
    if named is not None:
        best_err = 1e9
        for ax, val, on, spans in cands:
            for a, span in spans.items():
                if size[a] < 0.8 * named:
                    continue
                if abs(span - named) < best_err:
                    best_err, pick = abs(span - named), (ax, val, a)
    if pick is None:
        def third_spread(c):
            ax, on, spans = c[0], c[2], c[3]
            thirds = [a for a in range(3) if a != ax and a not in spans]
            if not thirds:
                return 0.0
            return max(p[thirds[0]] for p in on) - min(p[thirds[0]] for p in on)

        ranked = sorted(cands, key=lambda c: (-max(c[3].values()), third_spread(c), -len(c[2])))
        ax, val, on, spans = ranked[0]
        a = max(spans, key=spans.get)
        pick = (ax, val, a)
    proj, face_val, length = pick
    on = [p for p in dv if abs(p[proj] - face_val) < 0.3]
    feet_span = max(p[length] for p in on) - min(p[length] for p in on)
    centres = named if named is not None else feet_span
    into = [0.0, 0.0, 0.0]
    into[proj] = 1.0 if face_val == hi[proj] else -1.0
    width = next(a for a in range(3) if a != proj and a != length)
    x = [0.0, 0.0, 0.0]
    x[length] = 1.0
    z = unit(cross(x, into))
    origin = [(lo[a] + hi[a]) / 2.0 for a in range(3)]
    origin[proj] = face_val
    return x, into, z, origin, size, centres, False, length, proj, width


def write_stl(path, loops, x, y, z, origin):
    tris = 0
    with open(path, 'wb') as fh:
        fh.write(b'ABI Interiors handle, re-oriented by scripts/import_abi_handles.py'.ljust(80, b'\0')[:80])
        fh.write(struct.pack('<I', 0))  # count patched below
        for lp in loops:
            mapped = []
            for p in lp:
                d = sub(p, origin)
                mapped.append((dot(d, x), dot(d, y), dot(d, z)))
            for i in range(1, len(mapped) - 1):
                a, b, c = mapped[0], mapped[i], mapped[i + 1]
                # fan-triangulate the polygon
                u, v = sub(b, a), sub(c, a)
                n = cross(u, v)
                nn = sum(t * t for t in n) ** 0.5
                n = [t / nn if nn > 1e-12 else 0.0 for t in n]
                fh.write(struct.pack('<3f', *n))
                for t in (a, b, c):
                    fh.write(struct.pack('<3f', *t))
                fh.write(b'\0\0')
                tris += 1
        fh.seek(80)
        fh.write(struct.pack('<I', tris))
    return tris


def finishes_from(txt):
    out = []
    in_block = False
    for line in txt.splitlines():
        if line.startswith('Finishes and variants:'):
            in_block = True
            continue
        if in_block:
            m = re.match(r'- ([^:]+): (\S+) — SKU (\d+), \$([\d.]+) AUD', line)
            if m:
                out.append({'name': m.group(1).strip(), 'url': m.group(2),
                            'sku': m.group(3), 'price_aud': float(m.group(4))})
            elif not line.startswith('-'):
                if out:
                    break
    if not out:
        # A one-finish product (the Hegman timber pulls): its SKU sits in the
        # warranty line, its price may not be printed.
        m = re.search(r'^(?P<name>.+?) \((?P<sku>SKU \d+)\);', txt, re.M) or \
            re.search(r'\((?P<sku>SKU \d+)\)', txt)
        if m:
            name = re.sub(r'\s*\(SKU \d+\).*$', '', m.group('name') if 'name' in m.groupdict() else '')
            name = name.split(' - ')[-1].strip() if ' - ' in name else (name or 'Natural')
            price = re.search(r'\$([\d.]+)', txt)
            out.append({'name': name or 'Natural', 'url': '',
                        'sku': re.search(r'\d+', m.group('sku')).group(),
                        'price_aud': float(price.group(1)) if price else 0.0})
    return out


# Finish colours measured from the library's own models (RGB from each model's
# IFCCOLOURRGB). Finishes without a measured model carry no colour: the picker
# shows them and their colour waits for a check against a sample.
FINISH_COLOURS = {
    'brushed brass': ((202, 154, 58), 'Beta/Tezra BIM models'),
    'matte black': ((15, 15, 15), 'Barre Progressive Wall-Mounted Set model'),
    'black': ((15, 15, 15), 'Barre Progressive Wall-Mounted Set model'),
    'brushed gunmetal': ((63, 63, 63), 'Kingsley Shower Head Round model'),
    'gunmetal': ((63, 63, 63), 'Kingsley Shower Head Round model'),
    'stainless steel': ((189, 187, 185), 'Rappana Extended 500/800 models'),
    'steel': ((189, 187, 185), 'Rappana Extended 500/800 models'),
    'brushed nickel': ((205, 210, 215), 'Pedra Large Knob 35mm model'),
    'ash': ((244, 206, 157), 'Hegman BIM models'),
    'ash timber': ((244, 206, 157), 'Hegman BIM models'),
}


def slug(s):
    return re.sub(r'[^a-z0-9]+', '-', s.lower()).strip('-')


def write_catalog(rows, path):
    items = []
    for r in rows:
        modelled = r['colour_rgb']
        for f in r['finishes']:
            name_l = f['name'].lower()
            colour, origin = FINISH_COLOURS.get(
                name_l,
                (modelled if modelled and f is r['finishes'][0] else None, None))
            params = {
                'centres': r['centres'],
                'projection': r['projection'],
                'width': r['width'],
                'sku': int(f['sku']),
                'price_aud': f['price_aud'],
            }
            if r['knob']:
                params['knob'] = 1
            if colour:
                params['finish_colour_rgb'] = colour[0] * 65536 + colour[1] * 256 + colour[2]
            variant = f"{f['name']}"
            if not colour:
                variant += ' (colour to verify)'
            src = (
                f"ABI Interiors {r['family']} ({r['url']}); finish {f['name']}: {f['url']}, "
                f"SKU {f['sku']}, ${f['price_aud']:.2f} AUD (product.txt of the BIMobject "
                f"download, 2026-10-27); geometry re-oriented from the product's IFC by "
                f"scripts/import_abi_handles.py (screw centres {r['centres']:.0f} mm from the "
                f"model's feet, projection {r['projection']:.1f} mm measured)"
            )
            if colour and origin:
                src += f"; finish colour measured on {origin}"
            items.append({
                'id': f"handle.abi.{slug(r['article'].removeprefix('ABI-'))}.{slug(f['name'])}",
                'brand': 'ABI Interiors',
                'article': r['article'],
                'family': f"ABI {r['family']}",
                'variant': variant,
                'kind': 'handle',
                'parameters': params,
                'source': {
                    'url': src,
                    'document_date': '2026-10-27',
                    'retrieved': '2026-11-01',
                    'verified': False,
                },
            })
    doc = {
        'brand': 'ABI Interiors',
        'website': 'https://www.abiinteriors.com.au',
        'notes': (
            'Facts from each product\'s product.txt (BIMobject download of 2026-10-27): SKU, '
            'finishes, price. Geometry measured from the product IFC and re-oriented by '
            'scripts/import_abi_handles.py. Finish colours were measured from the models where '
            'a finish had one; rows whose variant says "colour to verify" have none. Never '
            'commit ABI\'s BIM files or images (vendor-cad/abi-bim stays local).'
        ),
        'items': items,
    }
    json.dump(doc, open(path, 'w'), indent=1, ensure_ascii=False)
    return len(items)


def main():
    dry = '--dry-run' in sys.argv
    rows = []
    if not dry:
        os.makedirs(OUT_DIR, exist_ok=True)
    for folder in sorted(glob.glob(BIM + '/BIMobject-*')):
        name = os.path.basename(folder).replace('BIMobject-', '')
        fam = family_of(os.path.basename(folder))
        ifc = glob.glob(folder + '/*.ifc')
        if not fam or not ifc:
            continue
        family, kind, named = fam
        parsed = parse_ifc(ifc[0])
        if parsed is None:
            print(f"{name:48s} NO IFC MESH — skipped (needs the FBX reader)")
            continue
        loops, colour = parsed
        loops = components(loops)
        fr = frame(loops, name)
        if fr is None:
            print(f"{name:48s} NO MOUNTING FACE FOUND — skipped")
            continue
        x, y, z, origin, size, centres, knob, length, proj, width = fr
        allp = [p for lp in loops for p in lp]
        proj_mm = max(abs(dot(sub(p, origin), y)) for p in allp)
        width_mm = max(abs(dot(sub(p, origin), z)) for p in allp) * 2
        length_mm = max(abs(dot(sub(p, origin), x)) for p in allp) * 2
        size_mm = named if (knob and named) else (centres if centres else max(length_mm, width_mm))
        article = f"ABI-{family.upper().replace(' ', '')}-{size_mm:.0f}"
        model_file = f"{article}.stl"
        tris = 0 if dry else write_stl(os.path.join(OUT_DIR, model_file), loops, x, y, z, origin)
        txt = open(glob.glob(folder + '/product.txt')[0], encoding='utf-8', errors='replace').read()
        fins = finishes_from(txt)
        rgb = tuple(round(c * 255) for c in colour) if colour else None
        warn = ''
        if not knob and abs(length_mm - centres) > 6:
            warn = f"  <-- LENGTH {length_mm:.0f} != CENTRES {centres:.0f}"
        rows.append({
            'folder': name,
            'article': article,
            'family': family,
            'kind': kind,
            'knob': knob,
            'centres': centres,
            'measured_length': round(length_mm, 1),
            'projection': round(proj_mm, 1),
            'width': round(width_mm, 1),
            'colour_rgb': rgb,
            'triangles': tris,
            'finishes': fins,
            'url': (re.search(r'Source URL: (\S+)', txt) or [None, ''])[1],
        })
        print(f"{article:24s} {name[:40]:40s} cc={centres:5.0f} proj={proj_mm:5.1f} "
              f"w={width_mm:5.1f} knob={int(knob)} colour={rgb} finishes={len(fins)}{warn}")
    if not dry:
        json.dump(rows, open(os.path.join(OUT_DIR, 'import-report.json'), 'w'), indent=1)
        n = write_catalog(rows, CATALOG)
        print(f"\n{len(rows)} models into {OUT_DIR}; {n} catalog rows into {CATALOG};"
              f" report at {OUT_DIR}/import-report.json")


if __name__ == '__main__':
    main()
