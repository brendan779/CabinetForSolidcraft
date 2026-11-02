#!/usr/bin/env python3
"""Imports ABI Interiors cabinetry handles from the BIM library (vendor-cad/abi-bim).

Each product folder there (a BIMobject download) holds an IFC exported from ABI's
Revit family: a faceted mesh in an arbitrary frame, plus product.txt with the SKUs,
finishes and prices, and ABI's dimensioned spec drawing.

ABI names a pull by its overall length, not its fixing distance (Tezra 220 is 220 long
with screws 192 apart), and many pulls are not two-screw (Pedra has three, Rappana 800
four, the T pulls one). SPECS below holds what each drawing prints; nothing about where
a screw goes is inferred from the name or the mesh.

For every cabinetry pull and knob this script:

  1. parses the IFC faces and triangulates them properly (they are n-gons, often
     non-convex, sometimes with holes),
  2. uses SPECS to find the axes: the length, and the projection the drawing prints,
  3. re-orients the mesh into the app's handle frame: the screw line on X and centred on
     0, the door face at Y = 0, the handle towards -Y, and writes a binary STL under
     vendor-cad/abi/<article>.stl,
  4. records the catalog row (length, projection, width, the screw pattern as `drilling`,
     and the finishes from product.txt) into catalog/hardware/abi-handles.json.

The STLs are committed under vendor-cad/abi; the catalog holds facts
with their source. Run from the repository root:

    python3 scripts/import_abi_handles.py [--dry-run]

Three Hegman pulls have no mesh in their IFC; those read the product's FBX through assimp
(brew install assimp).
"""
import glob
import json
import os
import re
import struct
import subprocess
import sys
import tempfile

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


def pair(cc):
    return [-cc / 2, cc / 2]


# From each product's spec drawing (the "*Spec*" image in its BIM folder) and product.txt.
#   holes: screw positions along the length from the pattern's centre; (x, z) off the line
#   proj: projection off the door, printed
#   axis: the mesh axis off the door, where the width equals the projection and size can't tell
#   edge: a shape with one straight edge: its distance from the screw line, printed
#   flip: the mesh's door face is the other extreme; set after looking at the render
#   adjustable: ABI's recommended range for a pull whose collars slide
#   edge_mount: sits over the front's top edge, not screwed through its face: 'back_leg' screws to
#   the back of the front, 'top_plate' down into its top edge.
SPECS = {
    'Beta Cabinetry Pull 115mm': {'holes': pair(64), 'proj': 18},
    'Beta Cabinetry Pull 400mm': {'holes': pair(355), 'proj': 18},
    'Camille Farmhouse Cabinetry Pull': {'holes': pair(64), 'proj': 23, 'edge': 3},
    'Imes Cabinetry Pull': {'holes': pair(64), 'proj': 14, 'edge': 15},
    'Kingsley Adjustable Cabinetry Pull 150mm': {'holes': pair(98), 'proj': 39, 'adjustable': (70, 120)},
    'Kingsley Adjustable Cabinetry Pull 250mm': {'holes': pair(198), 'proj': 39, 'adjustable': (100, 220)},
    'Kingsley Adjustable Cabinetry Pull 350mm': {'holes': pair(298), 'proj': 39, 'adjustable': (140, 320)},
    'Modi Adjustable Cabinetry Pull 150mm': {'holes': pair(96), 'proj': 36, 'adjustable': (70, 120)},
    'Modi Adjustable Cabinetry Pull 250mm': {'holes': pair(192), 'proj': 36, 'adjustable': (100, 220)},
    'Modi Adjustable Cabinetry Pull 350mm': {'holes': pair(288), 'proj': 36, 'adjustable': (140, 320)},
    'Ziggy Adjustable Cabinetry Pull 150mm': {'holes': pair(100), 'proj': 36},
    'Ziggy Adjustable Cabinetry Pull 250mm': {'holes': pair(192), 'proj': 36},
    'Ziggy Adjustable Cabinetry Pull 350mm': {'holes': pair(280), 'proj': 36},
    'Pedra Cabinetry Pull 220mm': {'holes': [-87, 0, 87], 'proj': 21},
    'Pedra Cabinetry Pull 400mm': {'holes': [-180, 0, 180], 'proj': 21},
    'Pedra Curve Cabinetry Pull 320mm (Pair)': {'holes': [-80, 0, 80], 'proj': 20, 'edge': 15},
    'Pika Cabinetry Pull 138mm': {'holes': pair(128), 'proj': 24},
    'Rappana Cabinetry Pull 50mm': {'holes': pair(25), 'proj': 36, 'edge_mount': 'back_leg'},
    'Rappana Cabinetry Pull Extended 100mm': {'holes': pair(75), 'proj': 36, 'edge_mount': 'back_leg'},
    'Rappana Cabinetry Pull Extended 500mm': {'holes': [-237, 0, 237], 'proj': 36, 'edge_mount': 'back_leg'},
    'Rappana Cabinetry Pull Extended 800mm': {'holes': [-387, -129, 129, 387], 'proj': 36, 'edge_mount': 'back_leg'},
    'Scalo Cabinetry Pull': {'holes': pair(32), 'proj': 35, 'edge_mount': 'top_plate'},
    'Tezra Cabinetry Pull 220mm': {'holes': pair(192), 'proj': 32},
    'Tezra Cabinetry Pull 500mm': {'holes': pair(450), 'proj': 42},
    'Tezra Textured Cabinetry Pull 220mm': {'holes': pair(192), 'proj': 31},
    'Tezra Textured Cabinetry Pull 500mm': {'holes': pair(450), 'proj': 41},
    'Tezra Cabinetry T Pull': {'holes': [0], 'proj': 35},
    'Tezra Textured Cabinetry T Pull': {'holes': [0], 'proj': 35},
    'Hegman Classic Pull 130mm': {'holes': pair(50), 'proj': 20, 'axis': 2},
    'Hegman Classic Pull 250mm': {'holes': pair(154), 'proj': 20, 'axis': 2},
    'Hegman Half Moon Pull 90mm Pair': {'holes': pair(22), 'proj': 25, 'edge': 10},
    'Hegman Half Moon Pull 200mm Pair': {'holes': pair(50), 'proj': 52, 'edge': 15},
    'Hegman Kurve Pull 250mm Pair': {'holes': pair(110), 'proj': 26, 'edge': 7.5},
    # Four M4 around a centre kept for an M10 through-bolt on glass, which we do not use.
    'Hegman Round Pull 200mm': {'holes': [(-25, 0), (0, -25), (0, 25), (25, 0)], 'proj': 52},
    'Elsa Cabinetry Knob': {'holes': [0], 'proj': 20},
    'Myka Cabinetry Knob 60mm': {'holes': [0], 'proj': 23},
    'Pika Cabinetry Knob': {'holes': [0], 'proj': 24},
    'Tezra Textured Cabinetry Knob': {'holes': [0], 'proj': 35, 'flip': True},
    'Pedra Large Knob 35mm': {'holes': [0], 'proj': 21.5},
    'Pedra Small Knob 20mm': {'holes': [0], 'proj': 25, 'flip': True},
    'Hegman Round Pull - 30mm': {'holes': [0], 'proj': 19},
    'Hegman Round Pull 60mm': {'holes': [0], 'proj': 23},
    'Hegman Round Pull 90mm': {'holes': [0], 'proj': 25},
}


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
    """(triangles, first colour): every IFCFACE, its outer bound and any holes triangulated."""
    text = open(path, encoding='latin-1').read()
    if 'DATA;' not in text:
        return None
    data = text[text.index('DATA;'):text.index('ENDSEC;', text.index('DATA;'))]
    pts, loops, bounds, faces, colours = {}, {}, {}, [], []
    for m in re.finditer(r"#(\d+)\s*=\s*(.*?);\s*(?=#|\Z)", data, re.S):
        s, n = m.group(2).strip(), int(m.group(1))
        if s.startswith('IFCCARTESIANPOINT'):
            mm = re.findall(r"\(([^()]*)\)", s)
            try:
                v = [float(x) for x in mm[-1].split(',')]
                if len(v) == 3:
                    pts[n] = v
            except ValueError:
                pass
        elif s.startswith('IFCPOLYLOOP'):
            loops[n] = [pts[int(r)] for r in re.findall(r"#(\d+)", s) if int(r) in pts]
        elif s.startswith('IFCFACEOUTERBOUND') or s.startswith('IFCFACEBOUND'):
            r = re.search(r"#(\d+)", s)
            bounds[n] = (int(r.group(1)), s.startswith('IFCFACEOUTERBOUND'))
        elif s.startswith('IFCFACE('):
            faces.append([int(r) for r in re.findall(r"#(\d+)", s)])
        elif 'IFCCOLOURRGB' in s and '(' in s:
            try:
                inner = s[s.index('(') + 1:s.rindex(')')]
                colours.append(tuple(float(x) for x in inner.split(',')[-3:]))
            except ValueError:
                pass
    tris = []
    for f in faces:
        fb = [bounds[b] for b in f if b in bounds and len(loops.get(bounds[b][0], [])) >= 3]
        outer = [loops[b] for b, is_outer in fb if is_outer] or [loops[b] for b, _ in fb[:1]]
        if not outer:
            continue
        holes = [loops[b] for b, is_outer in fb if not is_outer and loops[b] is not outer[0]]
        tris.extend(triangulate(outer[0], holes))
    return (tris, colours[0] if colours else None) if tris else None


def parse_fbx(path):
    """(triangles, first colour) from a binary FBX, by way of assimp's OBJ export."""
    with tempfile.TemporaryDirectory() as tmp:
        obj = os.path.join(tmp, 'mesh.obj')
        subprocess.run(['assimp', 'export', path, obj], check=True, capture_output=True)
        verts, tris = [], []
        for line in open(obj):
            if line.startswith('v '):
                verts.append([float(v) for v in line.split()[1:4]])
            elif line.startswith('f '):
                f = [verts[int(c.split('/')[0]) - 1] for c in line.split()[1:]]
                tris.extend([f[0], f[i], f[i + 1]] for i in range(1, len(f) - 1))
        colour = None
        for line in open(obj.replace('.obj', '.mtl')):
            if line.startswith('Kd '):
                colour = tuple(float(v) for v in line.split()[1:4])
                break
    return (tris, colour) if tris else None


def newell(poly):
    n = [0.0, 0.0, 0.0]
    for i, p in enumerate(poly):
        q = poly[(i + 1) % len(poly)]
        n[0] += (p[1] - q[1]) * (p[2] + q[2])
        n[1] += (p[2] - q[2]) * (p[0] + q[0])
        n[2] += (p[0] - q[0]) * (p[1] + q[1])
    return n


def triangulate(outer, holes):
    """Ear-clips a planar n-gon with holes, keeping the outer loop's winding."""
    n = newell(outer)
    if sum(c * c for c in n) < 1e-18:
        return []
    drop = max(range(3), key=lambda i: abs(n[i]))
    # Project so the outer loop runs counter-clockwise in 2D.
    u, v = [i for i in range(3) if i != drop]
    if n[drop] < 0:
        u, v = v, u
    if (drop == 1):
        u, v = v, u
    flat = lambda p: (p[u], p[v])

    def signed(poly):
        return sum(flat(poly[i])[0] * flat(poly[(i + 1) % len(poly)])[1]
                   - flat(poly[(i + 1) % len(poly)])[0] * flat(poly[i])[1]
                   for i in range(len(poly))) / 2

    ring = list(outer)
    flipped = signed(ring) < 0
    if flipped:
        ring.reverse()
    # Bridge each hole, rightmost first, to the nearest outer vertex that can see it.
    for hole in sorted(holes, key=lambda h: -max(flat(p)[0] for p in h)):
        h = list(hole)
        if signed(h) > 0:
            h.reverse()
        start = max(range(len(h)), key=lambda i: flat(h[i])[0])
        hp = flat(h[start])

        def crosses(a, b):
            for poly in [ring, h]:
                for i in range(len(poly)):
                    c, d = flat(poly[i]), flat(poly[(i + 1) % len(poly)])
                    if a in (c, d) or b in (c, d):
                        continue
                    if segments_cross(a, b, c, d):
                        return True
            return False

        order = sorted(range(len(ring)), key=lambda i: dist2(flat(ring[i]), hp))
        pick = next((i for i in order if not crosses(flat(ring[i]), hp)), order[0])
        h = h[start:] + h[:start]
        ring = ring[:pick + 1] + h + [h[0]] + ring[pick:]
    out = []
    idx = list(range(len(ring)))
    guard = 0
    while len(idx) > 3 and guard < 20000:
        guard += 1
        clipped = False
        for k in range(len(idx)):
            i0, i1, i2 = idx[k - 1], idx[k], idx[(k + 1) % len(idx)]
            a, b, c = flat(ring[i0]), flat(ring[i1]), flat(ring[i2])
            area = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
            if abs(area) < 1e-12:  # a straight run: drop the middle point
                idx.pop(k)
                clipped = True
                break
            if area < 0:
                continue
            if any(in_triangle(flat(ring[j]), a, b, c) for j in idx
                   if j not in (i0, i1, i2) and flat(ring[j]) not in (a, b, c)):
                continue
            out.append([ring[i0], ring[i1], ring[i2]])
            idx.pop(k)
            clipped = True
            break
        if not clipped:  # not simple (bad source data): finish as a fan rather than lose it
            break
    for i in range(1, len(idx) - 1):
        out.append([ring[idx[0]], ring[idx[i]], ring[idx[i + 1]]])
    return [t[::-1] for t in out] if flipped else out


def dist2(a, b):
    return (a[0] - b[0]) ** 2 + (a[1] - b[1]) ** 2


def segments_cross(a, b, c, d):
    def side(p, q, r):
        return (q[0] - p[0]) * (r[1] - p[1]) - (q[1] - p[1]) * (r[0] - p[0])
    return side(a, b, c) * side(a, b, d) < 0 and side(c, d, a) * side(c, d, b) < 0


def in_triangle(p, a, b, c):
    def side(q, r):
        return (r[0] - q[0]) * (p[1] - q[1]) - (r[1] - q[1]) * (p[0] - q[0])
    return side(a, b) >= -1e-9 and side(b, c) >= -1e-9 and side(c, a) >= -1e-9


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
    # Each loop once, not once per edge.
    groups = {}
    seen = set()
    for a, b, lp in edges:
        if id(lp) not in seen:
            seen.add(id(lp))
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


def frame(tris, spec):
    """The handle-frame axes from the drawing: (x, y into the door, z, origin).

    The length is the longest axis (for a one-screw pull or a knob, the longer axis off the
    door); the projection is the axis nearest what the drawing prints, and the door face is
    the extreme further from the surface's centroid, unless `flip` says the drawing shows
    otherwise.
    """
    pts = [p for t in tris for p in t]
    lo = [min(p[i] for p in pts) for i in range(3)]
    hi = [max(p[i] for p in pts) for i in range(3)]
    size = [hi[i] - lo[i] for i in range(3)]
    proj = spec.get('axis', min(range(3), key=lambda a: abs(size[a] - spec['proj'])))
    rest = [a for a in range(3) if a != proj]
    length = max(rest, key=lambda a: size[a])
    width = next(a for a in rest if a != length)

    # Most of a handle's surface is what you grip, away from the door. (Flat area at each
    # extreme does not work: a round bar touches its plane along a line, so it reads as a foot.)
    area = moment = 0.0
    for t in tris:
        n = cross(sub(t[1], t[0]), sub(t[2], t[0]))
        a = dot(n, n) ** 0.5 / 2
        area += a
        moment += a * sum(p[proj] for p in t) / 3
    centroid = moment / area
    door = lo[proj] if centroid - lo[proj] > hi[proj] - centroid else hi[proj]
    if spec.get('flip'):
        door = hi[proj] if door == lo[proj] else lo[proj]
    into = [0.0, 0.0, 0.0]
    into[proj] = 1.0 if door == hi[proj] else -1.0
    x = [0.0, 0.0, 0.0]
    x[length] = 1.0
    z = unit(cross(x, into))
    origin = [(lo[a] + hi[a]) / 2.0 for a in range(3)]
    origin[proj] = door
    if 'edge' in spec:
        # The straight edge runs the full length: the width extreme whose points span most.
        def span(val):
            on = [p[length] for p in pts if abs(p[width] - val) < 0.5]
            return max(on) - min(on) if on else 0.0
        straight = lo[width] if span(lo[width]) >= span(hi[width]) else hi[width]
        # Keep it up the handle frame's +Z, and the screw line at Z = 0.
        toward = -1.0 if straight == lo[width] else 1.0
        if toward * z[width] < 0:
            x = [-c for c in x]
            z = [-c for c in z]
        origin[width] = straight - toward * spec['edge']
    return x, into, z, origin


# Our choice for the 16 mm screws in the box: a pilot, not through the front.
EDGE_PILOT = 2.5
EDGE_PILOT_DEPTH = 12.0
# ABI's drawing of the Rappana 100: the sheet is 2 thick. Its back leg's screws are 10 below the top;
# Scalo's are 10 in from the plate's back.
EDGE_SHEET = 2.0
EDGE_HOLE_OFFSET = 10.0


def edge_frame(tris, mount):
    """The frame of a pull that sits over a front's top edge: x along it, y towards the cabinet,
    z up, the origin on the front's back and top faces at the middle of its length.

    The plate is the end of the shortest axis with the most flat area. A Rappana's back is the
    taller end of the depth axis, its leg against its lip. A Scalo's half disc hangs at the front,
    proud of the face, so its back is the plain end of the plate.
    """
    pts = [p for t in tris for p in t]
    lo = [min(p[i] for p in pts) for i in range(3)]
    hi = [max(p[i] for p in pts) for i in range(3)]
    size = [hi[i] - lo[i] for i in range(3)]
    up = min(range(3), key=lambda a: size[a])
    rest = sorted((a for a in range(3) if a != up), key=lambda a: size[a])
    # The depth is the plate's short side, except that a Rappana 50 is barely longer than deep.
    depth, length = rest
    flat = {lo[up]: 0.0, hi[up]: 0.0}
    for t in tris:
        n = cross(sub(t[1], t[0]), sub(t[2], t[0]))
        area = dot(n, n) ** 0.5 / 2
        for end in flat:
            if all(abs(p[up] - end) < 0.3 for p in t):
                flat[end] += area
    top = max(flat, key=flat.get)
    z = [0.0, 0.0, 0.0]
    z[up] = 1.0 if top == hi[up] else -1.0

    def score(end):
        near = [p for p in pts if abs(p[depth] - end) < 0.5]
        tall = max(p[up] for p in near) - min(p[up] for p in near)
        long = max(p[length] for p in near) - min(p[length] for p in near)
        return tall + long
    taller = lo[depth] if score(lo[depth]) > score(hi[depth]) else hi[depth]
    plain = hi[depth] if taller == lo[depth] else lo[depth]
    back = taller if mount == 'back_leg' else plain
    y = [0.0, 0.0, 0.0]
    y[depth] = 1.0 if back == hi[depth] else -1.0
    x = cross(y, z)
    origin = [0.0, 0.0, 0.0]
    origin[length] = (lo[length] + hi[length]) / 2.0
    origin[depth] = back
    origin[up] = top - z[up] * EDGE_SHEET
    return x, y, z, origin


def write_stl(path, tris_in, x, y, z, origin):
    tris = 0
    with open(path, 'wb') as fh:
        fh.write(b'ABI Interiors handle, re-oriented by scripts/import_abi_handles.py'.ljust(80, b'\0')[:80])
        fh.write(struct.pack('<I', 0))  # count patched below
        for t in tris_in:
            a, b, c = [(dot(sub(p, origin), x), dot(sub(p, origin), y), dot(sub(p, origin), z))
                       for p in t]
            n = cross(sub(b, a), sub(c, a))
            nn = dot(n, n) ** 0.5
            if nn < 1e-9:
                continue
            fh.write(struct.pack('<3f', *[t / nn for t in n]))
            for v in (a, b, c):
                fh.write(struct.pack('<3f', *v))
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
    # From ABI's round finish swatch on each product page (centre of the image, see
    # scripts/fetch_abi_photos.py), for finishes no BIM model carries.
    'brushed copper': ((198, 142, 106), "ABI's finish swatch"),
    'chrome': ((171, 170, 171), "ABI's finish swatch"),
    'tumbled aged brass': ((140, 115, 80), "ABI's finish swatch"),
    'carrara marble': ((203, 203, 203), "ABI's finish swatch"),
    'red travertine': ((148, 96, 79), "ABI's finish swatch"),
    'verde alpi marble': ((39, 53, 50), "ABI's finish swatch"),
    'antique aurum': ((159, 125, 84), "ABI's finish swatch"),
    'antique bronze': ((109, 76, 60), "ABI's finish swatch"),
    'antique slate': ((69, 69, 69), "ABI's finish swatch"),
    'antique steel': ((149, 149, 149), "ABI's finish swatch"),
    'white': ((231, 230, 231), "ABI's finish swatch"),
}


# Finishes that are not bare metal, so the views do not draw them shining.
MATTE = re.compile(r'marble|travertine|ash|timber|white|matte', re.I)

# Through-hole for ABI's M4 screws, as the generic handles bore.
SCREW_HOLE = 5
# More than any front is thick: the builder bores the front through.
SCREW_REACH = 40


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
                'length': r['length'],
                'centres': r['centres'],
                'projection': r['projection'],
                'width': r['width'],
                'sku': int(f['sku']),
                'price_aud': f['price_aud'],
            }
            params['across_low'], params['across_high'] = r['across']
            if r['knob']:
                params['knob'] = 1
            if MATTE.search(f['name']):
                params['matte'] = 1
            if r['adjustable']:
                params['adjustable_min'], params['adjustable_max'] = r['adjustable']
            if r['edge_mount']:
                params['edge_mount'] = 1 if r['edge_mount'] == 'back_leg' else 2
                params['sheet_thickness'] = EDGE_SHEET
                params['hole_offset'] = EDGE_HOLE_OFFSET
                params['leg_height'] = r['leg_height']
            if colour:
                params['finish_colour_rgb'] = colour[0] * 65536 + colour[1] * 256 + colour[2]
            variant = f"{f['name']}"
            if not colour:
                variant += ' (colour to verify)'
            src = (
                f"ABI Interiors {r['family']} ({r['url']}); finish {f['name']}: {f['url']}, "
                f"SKU {f['sku']}, ${f['price_aud']:.2f} AUD (product.txt of the BIMobject "
                f"download, 2026-10-27); screw pattern from ABI's spec drawing {r['spec_file']}; "
                f"geometry re-oriented from the product's {r['geometry']} by "
                f"scripts/import_abi_handles.py"
            )
            if colour and origin:
                src += f"; finish colour from {origin}"
            items.append({
                'id': f"handle.abi.{slug(r['article'].removeprefix('ABI-'))}.{slug(f['name'])}",
                'brand': 'ABI Interiors',
                'article': r['article'],
                'family': f"ABI {r['family']}",
                'variant': variant,
                'kind': 'handle',
                'parameters': params,
                'drilling': [
                    {'x': hx, 'y': hz, 'diameter': SCREW_HOLE, 'depth': SCREW_REACH,
                     'note': 'screw through the front, whatever its thickness; x along the length, y across it'}
                    for hx, hz in r['holes']
                ] if not r['edge_mount'] else [
                    {'x': hx, 'y': 0.0, 'diameter': EDGE_PILOT, 'depth': EDGE_PILOT_DEPTH,
                     'note': 'pilot for the screw in the box: x along the length, hole_offset from the leg top or plate back'}
                    for hx, hz in r['holes']
                ],
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
            'SKU, finishes and price from each product\'s product.txt (BIMobject download of '
            '2026-10-27). length is ABI\'s named size; centres is the span of the screw pattern '
            'in drilling, both from ABI\'s spec drawings: a pull\'s name is not its fixing '
            'distance. Geometry from the product IFC (or its FBX, where the IFC has no mesh), '
            're-oriented by '
            'scripts/import_abi_handles.py. Never commit ABI\'s BIM files or images '
            '(vendor-cad/abi-bim and vendor-cad/abi-web stay local).'
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
        spec = SPECS.get(name)
        if spec is None:
            print(f"{name:48s} NOT IN SPECS: read its spec drawing first")
            continue
        parsed = parse_ifc(ifc[0])
        geometry = 'IFC'
        fbx = glob.glob(folder + '/*.fbx')
        if parsed is None and fbx:
            parsed = parse_fbx(fbx[0])
            geometry = 'FBX'
        if parsed is None:
            print(f"{name:48s} NO MESH: skipped")
            continue
        tris, colour = parsed
        tris = components(tris)
        if spec.get('edge_mount'):
            x, y, z, origin = edge_frame(tris, spec['edge_mount'])
        else:
            x, y, z, origin = frame(tris, spec)
        allp = [p for t in tris for p in t]
        along = [dot(sub(p, origin), x) for p in allp]
        off = [dot(sub(p, origin), y) for p in allp]
        across = [dot(sub(p, origin), z) for p in allp]
        length_mm = max(along) - min(along)
        proj_mm = max(abs(v) for v in off)
        width_mm = max(across) - min(across)
        holes = [(h, 0.0) if not isinstance(h, tuple) else (float(h[0]), float(h[1]))
                 for h in spec['holes']]
        holes = [(float(a), float(b)) for a, b in holes]
        centres = max(a for a, _ in holes) - min(a for a, _ in holes)
        size_mm = named if named else length_mm
        article = f"ABI-{family.upper().replace(' ', '')}-{size_mm:.0f}"
        tri_count = 0 if dry else write_stl(os.path.join(OUT_DIR, f"{article}.stl"), tris, x, y, z, origin)
        txt = open(glob.glob(folder + '/product.txt')[0], encoding='utf-8', errors='replace').read()
        fins = finishes_from(txt)
        rgb = tuple(round(c * 255) for c in colour) if colour else None
        drawing = [f for f in os.listdir(folder) if 'spec' in f.lower()]
        warn = ''
        if abs(proj_mm - spec['proj']) > 5:
            warn = f"  <-- PROJECTION {proj_mm:.1f} vs {spec['proj']} printed"
        rows.append({
            'folder': name,
            'article': article,
            'family': family,
            'kind': kind,
            'knob': kind == 'Knob' or (centres == 0 and kind != 'TPull'),
            'length': round(size_mm, 1),
            'centres': centres,
            'holes': holes,
            'adjustable': spec.get('adjustable'),
            'edge_mount': spec.get('edge_mount'),
            'leg_height': round(max(dot(sub(p, origin), z) for p in allp)
                                - min(dot(sub(p, origin), z) for p in allp), 1),
            'measured_length': round(length_mm, 1),
            'projection': round(proj_mm, 1),
            'width': round(width_mm, 1),
            'across': (round(min(across), 1), round(max(across), 1)),
            'colour_rgb': rgb,
            'triangles': tri_count,
            'geometry': geometry,
            'finishes': fins,
            'spec_file': drawing[0] if drawing else 'none',
            'url': (re.search(r'Source URL: (\S+)', txt) or [None, ''])[1],
        })
        print(f"{article:26s} L={length_mm:6.1f} cc={centres:5.0f} proj={proj_mm:5.1f} "
              f"w={width_mm:5.1f} holes={len(holes)} tris={tri_count} finishes={len(fins)}{warn}")
    if not dry:
        json.dump(rows, open(os.path.join(OUT_DIR, 'import-report.json'), 'w'), indent=1)
        n = write_catalog(rows, CATALOG)
        print(f"\n{len(rows)} models into {OUT_DIR}; {n} catalog rows into {CATALOG};"
              f" report at {OUT_DIR}/import-report.json")


if __name__ == '__main__':
    main()
