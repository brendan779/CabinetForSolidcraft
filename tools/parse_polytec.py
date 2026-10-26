#!/usr/bin/env python3
"""Parse the polytec Availability Guide into catalog/boards/polytec.json (BACKLOG DATA-1).

    tools/parse_polytec.py [--pdf guide.pdf] [--out catalog/boards/polytec.json]

Without --pdf it downloads the current guide into target/polytec/ (never committed). The PDF goes
through `pdftotext -bbox` (poppler) and the board tables are read from word positions: the guide
prints two A4 pages per A3 sheet, its columns drift from table to table and availability marks
(▲ ● ‡ ♦) sit between the columns, so fixed text columns do not work.

Scope: the DECORATIVE board tables (9, 16, 18, 25 and 32 mm, guide pages 3-13), with their item
codes per sheet size and the matching edge tapes, and the COMMERCIAL range (16-33 mm boards on
guide pages 14-15, their edge tapes on pages 16-17, matched by decor and finish), and the routed
wall lining panels on page 28 (`kind: lining`, read from `pdftotext -raw`: those tables are
plain). Every record keeps `verified: false`; an existing
record's `preview_colour` and `verified` are carried over. Standard library only.
"""

import argparse
import datetime
import html
import json
import re
import subprocess
import sys
import urllib.request
from pathlib import Path

URL = "https://www.polytec.com.au/docs/technical/polytec-technical-availability-guide.pdf"
ROOT = Path(__file__).resolve().parent.parent

# U+F0DE is the guide's "sold in pack quantities only" glyph.
MARKS = set("▲●‡♦◆■†*\uf0de")
CODE = re.compile(r"^\d{3,6}$")
# Words on a line closer than this (pt) vertically are one line.
LINE_TOLERANCE = 3.0
# Taller words in a table row are the page title set up the margin.
ROW_HEIGHT = 13.0
# The margin title's words are far taller than any column heading.
TITLE_HEIGHT = 40.0
# A code further than this (pt) from every column centre belongs to no column.
COLUMN_REACH = 40.0

SUBSTRATES = [
    ("MR Particleboard", "moisture_resistant_particleboard"),
    ("Particleboard", "particleboard"),
    ("MR MDF", "moisture_resistant_mdf"),
    ("MDF", "mdf"),
]
# Section headers start with one of these; the rest of the header is the finish.
COLLECTIONS = {"SOLID", "TIMBERPRINT", "ABSTRACT", "METALLIC", "SPECIALTY"}
GRAINED = {"TIMBERPRINT", "SPECIALTY"}
HEADER_NOISE = {"ITEM", "CODE", "EDGE", "TAPE", "NOTES", "DECORATIVE"}
# COMMERCIAL column headings name the substrate in short.
COMMERCIAL_SUBSTRATES = {
    "MR Particle": "moisture_resistant_particleboard",
    "MR MDF": "moisture_resistant_mdf",
    "STD LDF": "ldf",
    "MR LDF": "moisture_resistant_ldf",
}
# Our reading, not the guide's: COMMERCIAL lists oaks and walnuts under MATT too, so a decor named
# after a timber is grained whatever its section.
TIMBER = re.compile(r"\b(Oak|Walnut|Ash|Beech|Ply|Wood|Wenge|Native)\b")
# "ex NSW/VIC/QLD" after a section name: where it is stocked.
STOCKED = re.compile(r"^(ex|[A-Z]{2,3}(/[A-Z]{2,3})+)$")


class Word:
    def __init__(self, x0, y0, x1, y1, text):
        self.x0, self.y0, self.x1, self.y1, self.text = x0, y0, x1, y1, text

    @property
    def cx(self):
        return (self.x0 + self.x1) / 2

    @property
    def vertical(self):
        """The table headings are set rotated a quarter turn, reading upward."""
        # Level words of 3+ letters in the headings are wider than tall; turned, taller than wide.
        return len(self.text) >= 3 and self.y1 - self.y0 > self.x1 - self.x0

    def __repr__(self):
        return f"{self.text}@{self.cx:.0f}"


def read_pages(pdf):
    """Every page as a list of guide pages (an A3 sheet is two), each a list of lines of words."""
    out = subprocess.run(
        ["pdftotext", "-bbox", str(pdf), "-"], check=True, capture_output=True, text=True
    ).stdout
    pages = []
    for page in re.finditer(r'<page width="([\d.]+)"[^>]*>(.*?)</page>', out, re.S):
        width = float(page.group(1))
        words = [
            Word(float(a), float(b), float(c), float(d), html.unescape(t))
            for a, b, c, d, t in re.findall(
                r'<word xMin="([\d.]+)" yMin="([\d.]+)" xMax="([\d.]+)" yMax="([\d.]+)">(.*?)</word>',
                page.group(2),
            )
        ]
        halves = [words] if width < 700 else [
            [w for w in words if w.cx < width / 2],
            [w for w in words if w.cx >= width / 2],
        ]
        pages.extend(lines(h) for h in halves)
    return pages


def lines(words):
    rows = []
    for w in sorted(words, key=lambda w: (w.y0, w.x0)):
        if rows and abs(rows[-1][0].y0 - w.y0) <= LINE_TOLERANCE:
            rows[-1].append(w)
        else:
            rows.append([w])
    return [sorted(r, key=lambda w: w.x0) for r in rows]


def phrases(header):
    """Header text as (text, x) phrases: rotated words stacked in one column read bottom to top,
    level words split where the gap between them is wide."""
    out = []
    columns = []
    for w in (w for line in header for w in line):
        if w.vertical or w.text == "x":
            col = next((c for c in columns if abs(c[0].cx - w.cx) < 3), None)
            if col is None:
                columns.append([w])
            else:
                col.append(w)
    for col in columns:
        if not any(w.vertical for w in col):
            continue
        col.sort(key=lambda w: -w.y1)
        out.append((" ".join(w.text for w in col), col[0].cx))
    stacked = {id(w) for col in columns if any(w.vertical for w in col) for w in col}
    for line in header:
        run = []
        for w in line:
            if id(w) in stacked:
                continue
            if run and w.x0 - run[-1].x1 > 8:
                out.append((text(run), (run[0].x0 + run[-1].x1) / 2))
                run = []
            run.append(w)
        if run:
            out.append((text(run), (run[0].x0 + run[-1].x1) / 2))
    return out


def text(line):
    return " ".join(w.text for w in line)


def slug(s):
    s = s.lower().replace("é", "e").replace("&", "and")
    return re.sub(r"[^a-z0-9]+", "-", s).strip("-")


def title_case(words):
    return " ".join(w.capitalize() for w in words)


def nearest(x, columns):
    best = min(columns, key=lambda c: abs(c["x"] - x), default=None)
    return best if best is not None and abs(best["x"] - x) <= COLUMN_REACH else None


def block_title(t):
    """`16mm MR MDF E-Zero, 2400x1200 & 3115x1200` → (thicknesses, substrate, emission, range)."""
    m = re.match(r"^(\d+)mm (?:or (\d+)mm )?(Internal (?:Coloured )?Carcass )?(.*)$", t)
    if not m:
        return None
    for name, substrate in SUBSTRATES:
        if m.group(4).startswith(name):
            emission = re.search(r"\b(E-Zero|E0|E1)\b", m.group(4))
            thicknesses = [float(g) for g in m.group(1, 2) if g]
            rng = "Internal Carcass" if m.group(3) else "Decorative"
            return thicknesses, substrate, emission.group(1) if emission else "", rng
    return None


def header_columns(header, thicknesses):
    """Board and tape columns from the header lines between a block title and its first section."""
    boards, tapes, widths, marks, sided, glue = [], [], [], [], [], []
    for t, x in phrases(header):
        if m := re.match(r"^(\d{4}) ?x ?(\d{4})$", t):
            boards.append({"x": x, "size": (float(m.group(1)), float(m.group(2)))})
        elif m := re.match(r"^1mm x (\d+)mm$", t):
            widths.append((x, float(m.group(1))))
        elif m := re.match(r"^(\d+)mm\b", t):
            marks.append((x, float(m.group(1))))
        elif m := re.search(r"\b(\d+)m\b", t):
            tapes.append({"x": x, "roll": float(m.group(1)), "pre_glued": "Pre-glued" in t})
        if "glued" in t:
            glue.append((x, "Pre-glued" in t))
        if t.startswith(("Single Side", "Double Side")):
            sided.append((x, t.startswith("Double")))
    for b in boards:
        # The block title is authoritative: the 18 mm tables' column headings say 16mm.
        near = min(marks, key=lambda m: abs(m[0] - b["x"]), default=None)
        if len(thicknesses) > 1 and near is not None and near[1] in thicknesses:
            b["thickness"] = near[1]
        else:
            b["thickness"] = thicknesses[0]
        b["double_sided"] = True
    if any(not double for _, double in sided):
        x = next(x for x, double in sided if not double)
        min(boards, key=lambda b: abs(b["x"] - x))["double_sided"] = False
    for t in tapes:
        near = min(widths, key=lambda w: abs(w[0] - t["x"]), default=None)
        t["width"] = near[1] if near is not None and abs(near[0] - t["x"]) < 30 else None
        # "Un-glued" / "Pre-glued" may be its own heading just right of the roll length.
        g = min(glue, key=lambda g: abs(g[0] - t["x"]), default=None)
        if g is not None and abs(g[0] - t["x"]) < 30:
            t["pre_glued"] = g[1]
    return boards, tapes


def section(line):
    words = [w.text for w in line if w.text not in HEADER_NOISE]
    if not words or not all(re.match(r"^[A-Z][A-Z-]+$", w) for w in words):
        return None
    if words[0] in COLLECTIONS:
        return words[0], words[1:] or [words[0]]
    return "", words


def parse(pages, today, source_url):
    boards, tapes, problems = {}, {}, []
    rows_seen = 0
    commercial = {"boards": [], "tapes": []}
    for number, page in enumerate(pages, start=1):
        words = {w.text for line in page for w in line}
        if {"COMMERCIAL", "Colour"} <= words:
            rows_seen += commercial_page(page, number, commercial, problems)
            continue
        # The DECORATIVE board pages; door and laminate pages have other tables.
        if not {"DECORATIVE", "board"} <= words or words & {"COMMERCIAL", "doors"}:
            continue
        block = None
        for line in page:
            t = text([w for w in line if w.text != "NOTES"])
            title = block_title(t)
            if title:
                block = {"title": title, "header": [], "cols": None, "section": None}
                continue
            if block is None:
                continue
            # A section name can share its line with the margin title.
            sec = section([w for w in line if w.y1 - w.y0 < TITLE_HEIGHT])
            if sec:
                if block["cols"] is None:
                    block["cols"] = header_columns(block["header"], block["title"][0])
                block["section"] = sec
                continue
            if block["section"] is None:
                block["header"].append(line)
                continue
            # The page title runs up the margin and shares lines with table rows. Row text is one
            # ~10.5 pt line tall; a turned title word is taller (and "Ply" is taller than wide).
            line = [w for w in line if w.y1 - w.y0 < ROW_HEIGHT]
            codes = [w for w in line if CODE.match(w.text) or w.text == "na"]
            name = [w.text for w in line if not (CODE.match(w.text) or w.text == "na")
                    and not set(w.text) <= MARKS and w.text != "NEW"]
            if not codes or not name or not re.match(r"^[A-Z]", name[0]):
                continue
            rows_seen += 1
            board_cols, tape_cols = block["cols"]
            row = {}
            clash = False
            for w in codes:
                col = nearest(w.cx, board_cols + tape_cols)
                if col is None or id(col) in row:
                    clash = True
                    break
                row[id(col)] = (col, w.text)
            if clash:
                problems.append(f"page {number}: cannot place the codes of: {t}")
                continue
            add_row(boards, tapes, problems, block, " ".join(name), row.values(), today, source_url)
    add_commercial(boards, tapes, problems, commercial, today, source_url)
    same_code_widths(boards, tapes)
    return list(boards.values()), list(tapes.values()), rows_seen, problems


def same_code_widths(boards, tapes):
    """A tape printed without its width is the tape another table prints with the same item code
    and a width (Internal Carcass Black 1mm Rigid 9053 is COMMERCIAL's 1mm x 21mm)."""
    sized = {}
    for t in tapes.values():
        if "width" in t:
            sized.setdefault(t["code"], t["id"])
    swap = {tid: sized[t["code"]] for tid, t in tapes.items()
            if "width" not in t and t["code"] in sized}
    for tid in swap:
        del tapes[tid]
    for b in boards.values():
        b["edge_tapes"] = list(dict.fromkeys(swap.get(t, t) for t in b["edge_tapes"]))


def stacked(words):
    """A turned heading's words read bottom to top."""
    return " ".join(w.text for w in sorted(words, key=lambda w: -(w.y0 + w.y1) / 2))


def commercial_columns(header):
    """COMMERCIAL board columns (thickness, then substrate, then sheet size, each turned) or edge
    tape columns (`1mm x 21mm Rigid` beside `100m Un-glued`)."""
    # Headings are turned; only short words ("x", "MR") are set level among them.
    words = [w for line in header for w in line if w.vertical or len(w.text) <= 2]
    boards, tapes = [], []
    for r in (w for w in words if w.vertical and re.match(r"^\d+m$", w.text)):
        near = [w for w in words if r.cx - 20 < w.cx < r.cx - 5]
        m = re.match(r"^(\d)mm x (\d+)mm\b", stacked(near))
        if not m or not near:
            continue
        glue = [w.text for w in words if abs(w.cx - r.cx) < 3 and "glued" in w.text]
        tapes.append({
            "x": sum(w.cx for w in near) / len(near), "thickness": float(m.group(1)),
            "width": float(m.group(2)), "roll": float(r.text[:-1]),
            "pre_glued": glue == ["Pre-glued"],
        })
    if tapes:
        return boards, tapes
    for t in (w for w in words if w.vertical and re.match(r"^\d+mm$", w.text)):
        # Pack quantities (2-digit numbers) sit under the substrate.
        near = [w for w in words if t.cx + 4 < w.cx < t.cx + 18 and not re.match(r"^\d{1,3}$", w.text)]
        far = [w for w in words if t.cx + 18 <= w.cx < t.cx + 32]
        size = re.match(r"^(\d{4}) x (\d{4})$", stacked(far))
        name = stacked(near)
        substrate = next((v for k, v in COMMERCIAL_SUBSTRATES.items() if name.startswith(k)), None)
        if not (size and substrate and near):
            continue
        emission = re.search(r"\b(E-Zero|E0|E1)\b", name)
        boards.append({
            "x": sum(w.cx for w in near) / len(near), "thickness": float(t.text[:-2]),
            "substrate": substrate, "emission": emission.group(1) if emission else "",
            "size": (float(size.group(1)), float(size.group(2))),
        })
    return boards, tapes


def commercial_page(page, number, out, problems):
    """Reads one COMMERCIAL board or edge tape page into `out`; returns the rows read."""
    header, cols, sec, rows = [], None, None, 0
    colour = next((w for line in page for w in line if w.text == "Colour"), None)
    for line in page:
        line = [w for w in line if not STOCKED.match(w.text)]
        # Section names are level, under the Colour heading; the column headings are not.
        s = None
        if line and line[0].cx < colour.cx + 60 and not any(w.vertical for w in line):
            s = section(line)
        if s:
            if cols is None:
                cols = commercial_columns(header)
            sec = s
            continue
        if sec is None:
            header.append(line)
            continue
        line = [w for w in line if w.y1 - w.y0 < ROW_HEIGHT]
        cells = [(w, w.text.rstrip("".join(MARKS))) for w in line]
        codes = [(w, c) for w, c in cells if CODE.match(c) or c == "na"]
        name = [c for w, c in cells if c and not (CODE.match(c) or c == "na") and c != "NEW"]
        if not codes or not name or not re.match(r"^[A-Z]", name[0]):
            continue
        rows += 1
        board_cols, tape_cols = cols
        placed = {}
        for w, code in codes:
            col = nearest(w.cx, board_cols + tape_cols)
            if col is None or id(col) in placed:
                problems.append(f"page {number}: cannot place the codes of: {text(line)}")
                placed = {}
                break
            placed[id(col)] = (col, code)
        collection, finish_words = sec
        for col, code in placed.values():
            if code == "na":
                continue
            kind = "tapes" if "roll" in col else "boards"
            out[kind].append((" ".join(name), collection, title_case(finish_words), col, code))
    return rows


def add_commercial(boards, tapes, problems, commercial, today, source_url):
    """COMMERCIAL boards, each with the tapes its decor and finish has on the tape pages."""
    source = {"url": source_url, "document_date": today, "retrieved": today, "verified": False}
    by_decor = {}
    for decor, _, finish, col, code in commercial["tapes"]:
        base = f"{slug(decor)}-{slug(finish)}"
        tid = f"polytec.abs.{base}.{col['thickness']:g}x{col['width']:g}.{col['roll']:g}m"
        if tid in tapes and tapes[tid]["code"] != code:
            tid = f"{tid}.{code}"
        tapes.setdefault(tid, {
            "id": tid, "brand": "Polytec", "code": code, "decor": decor, "finish": finish,
            "material": "abs", "thickness": col["thickness"], "width": col["width"],
            "rolls": [col["roll"]], "pre_glued": col["pre_glued"], "laser": False,
            "source": source,
        })
        by_decor.setdefault(base, []).append(tid)
    for decor, collection, finish, col, code in commercial["boards"]:
        base = f"{slug(decor)}-{slug(finish)}"
        substrate = col["substrate"]
        short = substrate.replace("moisture_resistant_", "mr-").replace("_", "-")
        bid = f"polytec.commercial.{base}.{col['thickness']:g}.{short}"
        board = boards.setdefault(bid, {
            "id": bid, "brand": "Polytec", "range": "Commercial", "decor": decor,
            "finish": finish, "kind": "melamine", "substrate": substrate,
            "thickness": col["thickness"], "sheets": [],
            "grained": collection in GRAINED or finish == "Woodmatt" or bool(TIMBER.search(decor)),
            "double_sided": True, "emission": col["emission"],
            "edge_tapes": by_decor.get(base, []), "preview_colour": "", "item_codes": {},
            "source": source,
        })
        length, width = col["size"]
        key = f"{length:g}x{width:g}"
        if board["item_codes"].get(key, code) != code:
            problems.append(f"{bid}: two item codes for {key}")
            continue
        if key not in board["item_codes"]:
            board["sheets"].append({"length": length, "width": width})
            board["item_codes"][key] = code


def add_row(boards, tapes, problems, block, decor, cells, today, source_url):
    thicknesses, substrate, emission, rng = block["title"]
    collection, finish_words = block["section"]
    finish = title_case(finish_words)
    source = {"url": source_url, "document_date": today, "retrieved": today, "verified": False}
    base = f"{slug(decor)}-{slug(finish)}"
    tape_ids = []
    for col, code in cells:
        if "roll" not in col or code == "na":
            continue
        # The Internal Carcass tapes print "1mm Rigid" and no width.
        if col["width"] is None and rng != "Internal Carcass":
            continue
        roll = col["roll"]
        size = f"1x{col['width']:g}" if col["width"] is not None else "1mm"
        tid = f"polytec.abs.{base}.{size}.{roll:g}m"
        if tid in tapes and tapes[tid]["code"] != code:
            # The same decor's tape has another code in another table (16 vs 18 mm Classic White
            # Sheen): keep both, the second one named by its code.
            tid = f"{tid}.{code}"
        tape = tapes.setdefault(tid, {
            "id": tid, "brand": "Polytec", "code": code, "decor": decor, "finish": finish,
            "material": "abs", "thickness": 1.0, "width": col["width"], "rolls": [roll],
            "pre_glued": col["pre_glued"], "laser": False, "source": source,
        })
        if tape["width"] is None:
            del tape["width"]
        tape_ids.append(tid)
    for col, code in cells:
        if "size" not in col or code == "na":
            continue
        thickness = col["thickness"]
        bid = f"polytec.{slug(rng)}.{base}.{thickness:g}.{substrate.replace('moisture_resistant_', 'mr-').replace('_', '-')}"
        if not col["double_sided"]:
            bid += ".single-sided"
        board = boards.setdefault(bid, {
            "id": bid, "brand": "Polytec", "range": rng, "decor": decor, "finish": finish,
            "kind": "melamine", "substrate": substrate, "thickness": thickness, "sheets": [],
            "grained": collection in GRAINED, "double_sided": col["double_sided"],
            "emission": emission, "edge_tapes": [], "preview_colour": "", "item_codes": {},
            "source": source,
        })
        length, width = col["size"]
        key = f"{length:g}x{width:g}"
        if key in board["item_codes"] and board["item_codes"][key] != code:
            problems.append(f"{bid}: two item codes for {key}")
            continue
        if key not in board["item_codes"]:
            board["sheets"].append({"length": length, "width": width})
            board["item_codes"][key] = code
        for tid in tape_ids:
            if tid not in board["edge_tapes"]:
                board["edge_tapes"].append(tid)


def lining_tables(raw, source):
    """Routed lining panels: a `Profile` heading over one (thickness, `2400mm x 1200mm`,
    substrate) group per column, the table title, then a row per profile with an item code per
    column."""
    boards, sizes = [], []
    lines = [line.strip() for line in raw.splitlines()]
    for i, line in enumerate(lines):
        if line == "Profile":
            sizes = []
        elif m := re.match(r"^(\d{4})mm x (\d{4})mm$", line):
            sizes.append((float(lines[i - 1][:-2]), float(m.group(1)), float(m.group(2))))
        elif "ROUTED PANELS" in line and sizes:
            rng = "Easycraft Panels" + (" (wet area)" if line.startswith("WET AREA") else "")
            for row in lines[i + 1:]:
                m = re.match(r"^(\S.*?) ((?:\d{5,6}|na)(?: (?:\d{5,6}|na))*)$", row)
                if not m:
                    break
                codes = m.group(2).split()
                if len(codes) != len(sizes):
                    raise ValueError(f"{row!r}: {len(codes)} codes for {len(sizes)} sizes")
                profile = m.group(1)
                thickness = sizes[0][0]
                board = {
                    "id": f"polytec.{slug(rng)}.{slug(profile)}.{thickness:g}.mr-mdf",
                    "brand": "Polytec", "range": rng, "decor": "Pre-primed", "finish": profile,
                    "kind": "lining", "substrate": "moisture_resistant_mdf",
                    "thickness": thickness, "sheets": [], "grained": False,
                    "double_sided": False, "emission": "E-Zero", "edge_tapes": [],
                    "preview_colour": "", "item_codes": {}, "source": source,
                }
                for (_, length, width), code in zip(sizes, codes):
                    if code != "na":
                        board["sheets"].append({"length": length, "width": width})
                        board["item_codes"][f"{length:g}x{width:g}"] = code
                boards.append(board)
            sizes = []
    return boards


SIZED = re.compile(r"^(\d{3,4})x(\d{2,4})mm$")


def nearest_heading(headings, w):
    """The heading (x centre, value) nearest a cell's centre."""
    return min(headings, key=lambda h: abs(h[0] - w.cx))[1]


def contours_boards(page, source):
    """Easycraft Contours timber lining boards (guide p.30): a profile name in capitals over a
    row per timber species, with an item code (or `na`) under each board length's `900x91mm` /
    `2700x91mm` heading. The curved wall beads beside them are mouldings, not boards."""
    sizes = []
    for line in page:
        for w in line:
            m = SIZED.match(w.text)
            if m and m.group(2) == "91":
                sizes.append((w.cx, (float(m.group(1)), float(m.group(2)))))
    if not sizes:
        return []
    reach = max(x for x, _ in sizes) + 30
    boards, profile = [], None
    for line in page:
        words = [w for w in line if w.x0 < reach]
        cells = [w for w in words if re.match(r"^(\d{6}|na)$", w.text)]
        name = " ".join(w.text for w in words if w not in cells and w.x0 < min(x for x, _ in sizes) - 40)
        if not cells:
            if re.match(r"^[A-Z]+$", name) and name not in ("NOTES",):
                profile = name.title()
            if name.startswith("TIMBER LINING"):
                break
            continue
        if profile is None or not name:
            continue
        board = {
            "id": f"polytec.easycraft-contours.{slug(profile)}-{slug(name)}.21.timber",
            "brand": "Polytec", "range": "Easycraft Contours", "decor": name, "finish": profile,
            "kind": "lining", "substrate": "timber", "thickness": 21.0, "sheets": [],
            "grained": True, "double_sided": False, "emission": "", "edge_tapes": [],
            "preview_colour": "", "item_codes": {}, "source": source,
        }
        for c in cells:
            length, width = nearest_heading(sizes, c)
            if c.text != "na":
                board["sheets"].append({"length": length, "width": width})
                board["item_codes"][f"{length:g}x{width:g}"] = c.text
        if board["sheets"]:
            board["sheets"].sort(key=lambda s: -s["length"])
            boards.append(board)
    return boards


def ikara_boards(page, source):
    """Easycraft Ikara MR MDF E-Zero lining boards (guide p.31): a row per colour (finish
    Woodmatt) with an item code under each profile (`Convex`, `Channel`), whose width the
    `118x2700` / `105x2700` heading over it gives. Only the table's lines: from its title down to
    the profile drawings' caption (the DAR table beside it has its own thickness headings)."""
    start = next((i for i, line in enumerate(page)
                  if {"Ikara", "Lining", "Boards"} <= {w.text for w in line}), None)
    if start is None:
        return []
    end = next((i for i in range(start, len(page)) if "PROFILES" in {w.text for w in page[i]}),
               len(page))
    page = page[start:end]
    widths, profiles = [], []
    for line in page:
        for w in line:
            if m := re.match(r"^(\d{3})x(\d{4})$", w.text):
                widths.append((w.cx, (float(m.group(2)), float(m.group(1)))))
            if w.text in ("Convex", "Channel"):
                profiles.append((w.cx, w.text))
    thickness = next((float(w.text[:-2]) for line in page for w in line
                      if re.match(r"^\d+mm$", w.text) and widths
                      and abs(w.cx - widths[0][0]) < 20), None)
    if not widths or not profiles or thickness is None:
        return []
    boards = []
    for line in page:
        cells = [w for w in line if re.match(r"^\d{5}$", w.text)]
        if not cells or not any(w.text == "Woodmatt" for w in line):
            continue
        colour = " ".join(w.text for w in line if w.x1 < min(x for x, _ in profiles) - 60
                          and w.text != "Woodmatt")
        for c in cells:
            profile = nearest_heading(profiles, c)
            length, width = nearest_heading(widths, c)
            boards.append({
                "id": f"polytec.easycraft-ikara.{slug(colour)}-{slug(profile)}.{thickness:g}.mr-mdf",
                "brand": "Polytec", "range": "Easycraft Ikara", "decor": colour,
                "finish": f"{profile} Woodmatt", "kind": "lining",
                "substrate": "moisture_resistant_mdf", "thickness": thickness,
                "sheets": [{"length": length, "width": width}], "grained": True,
                "double_sided": False, "emission": "E-Zero", "edge_tapes": [],
                "preview_colour": "", "item_codes": {f"{length:g}x{width:g}": c.text},
                "source": source,
            })
    return boards


def raw_pages(pdf):
    out = subprocess.run(["pdftotext", "-raw", str(pdf), "-"], check=True, capture_output=True,
                         text=True).stdout
    return out.split("\f")


def merge_existing(boards, tapes, existing):
    """Keep what people added to earlier records: preview colours and verification."""
    old = {r["id"]: r for r in existing.get("boards", []) + existing.get("edge_tapes", [])}
    for r in boards + tapes:
        prev = old.get(r["id"])
        if prev is None:
            continue
        if prev.get("preview_colour"):
            r["preview_colour"] = prev["preview_colour"]
        # A person's check holds only while the facts they checked are unchanged.
        same = all(r.get(k) == v for k, v in prev.items() if k not in ("source", "preview_colour"))
        if same and prev.get("source", {}).get("verified"):
            r["source"] = {**r["source"], "verified": True}


def document_date(pages):
    """`23rd October 2026` on the cover → `2026-10-23`."""
    for line in pages[0]:
        m = re.search(r"(\d{1,2})(?:st|nd|rd|th) (\w+) (\d{4})", text(line))
        if m:
            d = datetime.datetime.strptime(f"{m.group(1)} {m.group(2)} {m.group(3)}", "%d %B %Y")
            return d.date().isoformat()
    return datetime.date.today().isoformat()


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--pdf", type=Path)
    ap.add_argument("--out", type=Path, default=ROOT / "catalog/boards/polytec.json")
    args = ap.parse_args()
    pdf = args.pdf
    if pdf is None:
        pdf = ROOT / "target/polytec/availability-guide.pdf"
        pdf.parent.mkdir(parents=True, exist_ok=True)
        urllib.request.urlretrieve(URL, pdf)
    pages = read_pages(pdf)
    date = document_date(pages)
    boards, tapes, rows, problems = parse(pages, date, URL)
    source = {"url": URL, "document_date": date, "retrieved": date, "verified": False}
    boards += [b for raw in raw_pages(pdf) if "ROUTED PANELS" in raw
               for b in lining_tables(raw, source)]
    for page in pages:
        words = {w.text for line in page for w in line}
        if "CIRQUE" in words:
            boards += contours_boards(page, source)
        if "Ikara" in words and "Woodmatt" in words:
            boards += ikara_boards(page, source)
    existing = json.loads(args.out.read_text()) if args.out.exists() else {}
    merge_existing(boards, tapes, existing)
    used = {t for b in boards for t in b["edge_tapes"]}
    tapes = [t for t in tapes if t["id"] in used]
    out = {
        "brand": "Polytec",
        "website": "https://www.polytec.com.au",
        "notes": f"Generated by tools/parse_polytec.py from the polytec Availability Guide of {date} "
                 "(DECORATIVE board tables, guide pages 3-13; COMMERCIAL boards and edge tapes, "
                 "pages 14-17; routed wall lining panels, page 28; Easycraft Contours timber and "
                 "Ikara MR MDF lining boards, pages 30-31, as boards one lining board wide). "
                 "Item codes are per sheet size. Curved wall beads and DAR mouldings are not "
                 "recorded. "
                 "Availability marks (limited stock, made to order, ex NSW) and COMMERCIAL pack "
                 "quantities are not recorded. "
                 "Do not edit by hand: rerun the tool.",
        "boards": sorted(boards, key=lambda b: b["id"]),
        "edge_tapes": sorted(tapes, key=lambda t: t["id"]),
    }
    args.out.write_text(json.dumps(out, indent=2, ensure_ascii=False) + "\n")
    print(f"{rows} board rows read, {len(boards)} boards, {len(tapes)} edge tapes → {args.out}")
    for p in problems:
        print("warning:", p, file=sys.stderr)


if __name__ == "__main__":
    main()
