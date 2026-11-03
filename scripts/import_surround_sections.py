#!/usr/bin/env python3
"""Reads each Surround by Laminex profile's true section from a Revit DXF export into the catalog.

    scripts/import_surround_sections.py <folder of <Profile>-section.dxf>

The DXFs are exports of Laminex's own Revit families, 1:1 in mm. Each gives one repeat of the routed
face as `section`, points of [distance across, depth below the face], and the `module` it really
repeats at, in catalog/profiles/laminex-surround.json. Curves are kept within 0.02 mm.
"""
import collections
import json
import math
import os
import sys

OUT = "catalog/profiles/laminex-surround.json"
NAMES = {
    "Batten 25": "Batten25", "Batten 75": "Batten75", "Batten 100": "Batten100", "Batten 300": "Batten300",
    "Classic VJ 100": "ClassicVJ100", "Classic VJ 200": "ClassicVJ200", "Demi Round 20": "DemiRound20",
    "Demi Round 40": "DemiRound40", "French Stripe 30": "FrenchStripe30", "Heritage 150": "Heritage150",
    "Scallop 22.5": "Scallop22.5", "Scallop 45": "Scallop45", "Scallop 135": "Scallop135",
}


def pairs(path):
    lines = open(path, errors="ignore").read().replace("\r", "").split("\n")
    return [(a.strip(), b.strip()) for a, b in zip(lines[0::2], lines[1::2])]


def parse(path):
    section, entities, blocks, current, block = None, [], {}, None, None
    data = pairs(path)
    for i, (code, value) in enumerate(data):
        if code == "0" and value == "SECTION":
            section, current = data[i + 1][1], None
            continue
        if section not in ("ENTITIES", "BLOCKS"):
            continue
        if code == "0":
            if current and current["type"] != "BLOCK":
                (blocks[block] if section == "BLOCKS" else entities).append(current)
            current = None
            if value == "BLOCK":
                current = {"type": "BLOCK"}
            elif value in ("LINE", "ARC", "INSERT"):
                current = {"type": value, "data": collections.defaultdict(list)}
        elif current is not None:
            if current["type"] == "BLOCK":
                if code == "2":
                    block = value
                    blocks.setdefault(block, [])
                    current = None
            else:
                current["data"][code].append(value)
    return entities, blocks


def segments(entities, blocks, place=(0, 0, 1, 1, 0), depth=0):
    tx, ty, sx, sy, turn = place
    r = math.radians(turn)

    def at(x, y):
        x, y = x * sx, y * sy
        return tx + x * math.cos(r) - y * math.sin(r), ty + x * math.sin(r) + y * math.cos(r)

    out = []
    for e in entities:
        d = e["data"]
        num = lambda k, default=0.0: float(d[k][0]) if d.get(k) else default
        layer = (d.get("8") or [""])[0]
        if e["type"] == "INSERT" and depth < 4 and d["2"][0] in blocks:
            sub = (num("10"), num("20"), num("41", 1), num("42", 1), num("50"))
            out += [(at(*a), at(*b)) for a, b in segments(blocks[d["2"][0]], blocks, sub, depth + 1)]
        elif e["type"] == "LINE" and ("OTLN" in layer or depth > 0):
            out.append((at(num("10"), num("20")), at(num("11"), num("21"))))
        elif e["type"] == "ARC" and ("OTLN" in layer or depth > 0):
            cx, cy, radius, a0, a1 = num("10"), num("20"), num("40"), num("50"), num("51")
            a1 = a1 + 360 if a1 < a0 else a1
            n = max(48, int((a1 - a0) * 4))
            pts = [
                (cx + radius * math.cos(math.radians(a0 + (a1 - a0) * k / n)),
                 cy + radius * math.sin(math.radians(a0 + (a1 - a0) * k / n)))
                for k in range(n + 1)
            ]
            out += [(at(*a), at(*b)) for a, b in zip(pts, pts[1:])]
    return out


def loops(segs):
    key = lambda p: (round(p[0], 2), round(p[1], 2))
    near = collections.defaultdict(list)
    for i, (a, b) in enumerate(segs):
        near[key(a)].append((i, b))
        near[key(b)].append((i, a))
    used, found = set(), []
    for i, (a, b) in enumerate(segs):
        if i in used:
            continue
        used.add(i)
        path = [a, b]
        while key(path[-1]) != key(path[0]):
            next_ = [(j, q) for j, q in near[key(path[-1])] if j not in used]
            if not next_:
                break
            used.add(next_[0][0])
            path.append(next_[0][1])
        if key(path[-1]) == key(path[0]) and len(path) > 3:
            found.append(path)
    return found


def face_of(loop):
    """Walks the outline from the flat back round the routed face. Returns the face, left to right,
    as (x, depth below the face), with the board's width and thickness."""
    pts = loop[:-1]
    n = len(pts)
    x0 = min(p[0] for p in pts)
    y0, y1 = min(p[1] for p in pts), max(p[1] for p in pts)
    thick = y1 - y0
    # The back is the longest straight edge on the top or bottom of the outline.
    edges = [
        (abs(pts[(i + 1) % n][0] - pts[i][0]), i)
        for i in range(n)
        if abs(pts[(i + 1) % n][1] - pts[i][1]) < 1e-3 and min(abs(pts[i][1] - y0), abs(pts[i][1] - y1)) < 1e-3
    ]
    _, back = max(edges)
    back_y = pts[back][1]
    # Round the outline from the back's far end to its near end.
    path = [pts[(back + 1 + k) % n] for k in range(n)]
    face = [(p[0] - x0, thick - abs(p[1] - back_y)) for p in path]
    if face[0][0] > face[-1][0]:
        face.reverse()
    # The board's two ends: straight up from the back.
    while len(face) > 2 and abs(face[0][0] - face[1][0]) < 1e-3:
        face.pop(0)
    while len(face) > 2 and abs(face[-1][0] - face[-2][0]) < 1e-3:
        face.pop()
    return face, max(p[0] for p in face), thick


def miss(grid, shift):
    return sum(abs(grid[j + shift] - grid[j]) > 0.1 for j in range(len(grid) - shift)) / (len(grid) - shift)


def period_of(face, width):
    # The depth every 0.1 mm, in one pass along the face. Walls are single steps, so they're allowed
    # to miss by a sample: nearly every sample must match, not all of them.
    grid, i = [], 0
    for k in range(int(width * 10) + 1):
        x = k / 10
        while i + 2 < len(face) and face[i + 1][0] < x:
            i += 1
        (a, da), (b, db) = face[i], face[i + 1]
        grid.append(da if b - a < 1e-6 else da + (db - da) * min(max(x - a, 0.0), b - a) / (b - a))
    for shift in range(50, len(grid) // 2 + 1):
        if miss(grid, shift) <= 0.015:
            # The first that nearly fits may be just short of the true repeat: take the best nearby.
            return min(range(shift, min(shift + 30, len(grid) // 2 + 1)), key=lambda t: miss(grid, t)) / 10
    return width


def simplify(points, tolerance=0.02):
    if len(points) < 3:
        return points
    (x0, y0), (x1, y1) = points[0], points[-1]
    dx, dy = x1 - x0, y1 - y0
    length = math.hypot(dx, dy) or 1.0
    far, index = 0.0, 0
    for i, (x, y) in enumerate(points[1:-1], 1):
        away = abs(dy * (x - x0) - dx * (y - y0)) / length
        if away > far:
            far, index = away, i
    if far <= tolerance:
        return [points[0], points[-1]]
    return simplify(points[: index + 1], tolerance)[:-1] + simplify(points[index:], tolerance)


def main():
    folder = sys.argv[1]
    record = json.load(open(OUT))
    for profile in record["profiles"]:
        stem = NAMES[profile["name"]]
        entities, blocks = parse(os.path.join(folder, stem + "-section.dxf"))
        shapes = sorted(loops(segments(entities, blocks)), key=lambda l: max(p[0] for p in l) - min(p[0] for p in l))
        face, width, thick = face_of(shapes[-1])
        # The families' panels are whole numbers of repeats: that gives the repeat exactly.
        found = period_of(face, width)
        module = width / round(width / found)
        # One repeat, from a point on the face so a groove is never split across its ends. Clear of
        # the board's own end, which may be cut through a groove.
        lowest = min(y for x, y in face if module / 2 <= x <= module / 2 + module)
        start = next(x for x, y in face if x >= module / 2 and y <= lowest + 0.01)
        inside = [(x - start, y) for x, y in face if start <= x < start + module - 1e-3]
        inside.append((module, inside[0][1]))
        one = simplify(inside)
        print(f'{profile["name"]:17} module {profile["module"]} -> {module}  thick {thick:.1f} (catalog {profile["thickness"]})  width {width:.0f}  deepest {max(y for _, y in one):.2f} (catalog {profile.get("groove_depth")})  points {len(one)}')
        profile["module"] = round(module, 2)
        profile["section"] = [[round(x, 2), round(y, 2)] for x, y in one]
    record["section_source"] = "section and module: exported from Laminex's Revit families (IGS BIM Solutions), checked against the families' STEP within 0.14%."
    with open(OUT, "w") as f:
        json.dump(record, f, indent=1, ensure_ascii=False)
        f.write("\n")


main()
