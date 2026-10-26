#!/usr/bin/env python3
"""Read Nikpol's Full Decor Availability into catalog/boards/nikpol.json (BACKLOG DATA-3b).

    tools/parse_nikpol.py [--pdf FullDecorAvailability.pdf] [--out catalog/boards/nikpol.json]

Without --pdf it downloads the current document (linked from nikpol.com.au › Downloads) into
target/nikpol/ (never committed). Each range's table has a colour, code and finish per row and a
dot in each column the decor is stocked in. Columns are read from word positions
(`pdftotext -bbox`): a board column is a thickness heading (16mm...) with a sheet size heading
(2800x2070mm) above it and a substrate phrase (E0 MR PB, E0 MR MDF...) over its group. Edging and
laminate columns are not recorded, nor are the benchtop and laminate pages. The document prints no
item codes: the decor keeps Nikpol's colour code. Every record keeps `verified: false`. Standard
library only.
"""

import argparse
import json
import re
import sys
import urllib.request
from pathlib import Path

from parse_polytec import ROOT, read_pages, slug, text

URL = "https://www.nikpol.com.au/Uploads/Pdf/ItemPdf_172926102937_FullDecorAvailability.pdf"
THICKNESS = re.compile(r"^(\d+)mm$")
SIZE = re.compile(r"^(\d{4})x(\d{4})mm$")
CODE = re.compile(r"^[A-Z]?\d{3,5}[A-Z]?$")
# A thickness heading this close (pt, centre to centre) to a size heading is under it.
SIZE_REACH = 80.0
# Words on one line further apart than this start a new phrase.
PHRASE_GAP = 6.0
# A dot this close to a board column's centre is in it.
DOT_REACH = 14.0
# Pages whose tables are not decorative boards.
SKIP_TITLES = {"benchtops", "laminate"}
# Our reading, as for polytec: decors named after a timber are grained.
TIMBER = re.compile(r"\b(Oak|Walnut|Ash|Elm|Pine|Birch|Wood|Chestnut|Teak|Maple|Cherry)\b")


def phrases(line):
    """A line's words split at wide gaps: [(text, x0, x1, y)]."""
    out, run = [], []
    for w in line:
        if run and w.x0 - run[-1].x1 > PHRASE_GAP:
            out.append(run)
            run = []
        run.append(w)
    if run:
        out.append(run)
    return [(text(r), r[0].x0, r[-1].x1, r[0].y0) for r in out]


def substrate(phrase):
    """`E0 MR PB` → (catalog substrate, emission); None when it names no board."""
    p = phrase.upper()
    emission = next((e for e in ("E0", "E1") if re.search(rf"\b{e}\b", p)), "")
    if "F****" in p:
        # Japan's top formaldehyde class (JIS), as printed.
        emission = "F****"
    mr = re.search(r"\bMR\b", p) and "NON MR" not in p
    if "MDF" in p:
        return ("moisture_resistant_mdf" if mr else "mdf"), emission
    if "PB" in p.split() or "PARTICLE" in p:
        return ("moisture_resistant_particleboard" if mr else "particleboard"), emission
    return None


def columns(header):
    """Board columns of one table: [{x, thickness, size, substrate, emission}]."""
    words = [w for line in header for w in line]
    bare = lambda w: w.text.rstrip(",")
    # Board columns sit under "Decorative Boards", left of the Edging heading (laminate and
    # film sizes follow it).
    edging = min((w.x0 for w in words if w.text == "Edging"), default=float("inf"))
    sizes = [w for w in words if SIZE.match(bare(w)) and w.cx < edging]
    # Substrate phrases, with a phrase directly under one joined to it ("E0 P2 STD" over
    # "Particle Board").
    subs = []
    for line in header:
        # Thickness and size headings are not part of a substrate's name.
        named = [w for w in line if not (THICKNESS.match(bare(w)) or SIZE.match(bare(w)))]
        for t, x0, x1, y in phrases(named):
            above = next((s for s in subs if abs((s[1] + s[2]) / 2 - (x0 + x1) / 2) < 20
                          and 0 < y - s[3] < 14), None)
            if above is not None:
                subs.remove(above)
                subs.append((above[0] + " " + t, min(above[1], x0), max(above[2], x1), y))
            else:
                subs.append((t, x0, x1, y))
    subs = [(t, x0, x1, y) for t, x0, x1, y in subs if substrate(t)]
    thicknesses = [w for w in words if THICKNESS.match(bare(w)) and w.cx < edging
                   and int(THICKNESS.match(bare(w)).group(1)) >= 9]
    out = []
    for size in sizes:
        if not subs:
            break
        # The substrate printed over this size: the phrase spanning its centre, else the nearest.
        over = [p for p in subs if p[1] - 2 <= size.cx <= p[2] + 2 and p[3] <= size.y0 + 1]
        sub = min(over or subs, key=lambda p: abs((p[1] + p[2]) / 2 - size.cx))
        kind, emission = substrate(sub[0])
        mine = [t for t in thicknesses
                if min(sizes, key=lambda z: abs(z.cx - t.cx)) is size
                and abs(size.cx - t.cx) <= SIZE_REACH]
        if not mine:
            # No heading of its own (Carcass): the nearest one's thickness.
            near = min(thicknesses, key=lambda t: abs(t.cx - size.cx), default=None)
            if near is None:
                continue
            mine = [near]
        l, wd = SIZE.match(bare(size)).groups()
        # Thicknesses printed beside the size ("2800x2070mm, 18mm, 25mm") share one dot under
        # the middle of the heading; thicknesses under it have a dot each.
        beside = [t for t in mine if abs(t.y0 - size.y0) < 2]
        middle = (size.x0 + max(t.x1 for t in beside)) / 2 if beside else size.cx
        for t in mine:
            out.append({
                "x": middle if len(mine) == 1 or t in beside else t.cx,
                "thickness": float(THICKNESS.match(bare(t)).group(1)),
                "size": (float(l), float(wd)), "substrate": kind, "emission": emission,
                "phrase": sub[0],
            })
    return out


def tables(page):
    """(title, header lines, row lines) per table on a page: a table starts at the line above
    its `Colour Code Finish` headings that names the range."""
    out = []
    i = 0
    while i < len(page):
        words = {w.text for w in page[i]}
        if {"Colour", "Code", "Finish"} <= words:
            # The header runs from the range title (the nearest line above with "Decorative")
            # down to the last line before the first row with a code.
            start = max((j for j in range(i) if "Decorative" in {w.text for w in page[j]}),
                        default=i)
            title = phrases(page[start])[0][0]
            code_x = next(w.x0 for w in page[i] if w.text == "Code")
            # Rows start at the first line with a colour code under the Code heading (an edging
            # roll length, 100M, looks like a code elsewhere).
            j = i + 1
            while j < len(page) and not any(CODE.match(w.text) and abs(w.x0 - code_x) < 15
                                            for w in page[j]):
                j += 1
            k = j
            while k < len(page) and not {"Colour", "Code", "Finish"} <= {w.text for w in page[k]} \
                    and not "Decorative" in {w.text for w in page[k]}:
                k += 1
            out.append((title, page[start:j], page[j:k]))
            i = k
        else:
            i += 1
    return out


def parse(pages, source):
    boards, problems = {}, []
    dots_in, dots_out = 0, 0
    for n, page in enumerate(pages, start=1):
        first = text(page[0]).lower() if page else ""
        if any(first.startswith(s) for s in SKIP_TITLES):
            continue
        for title, header, rows in tables(page):
            cols = columns(header)
            if not cols:
                continue
            code_x = next(w.x0 for line in header for w in line if w.text == "Code")
            first_col = min(c["x"] for c in cols) - DOT_REACH
            for line in rows:
                codes = [w for w in line if CODE.match(w.text) and abs(w.x0 - code_x) < 15]
                if not codes:
                    continue
                code = codes[0]
                decor = " ".join(w.text for w in line if w.x1 <= code.x0 - 2)
                finish = " ".join(w.text for w in line if code.x1 < w.x0 < first_col
                                  and not w.text.startswith("•"))
                if not decor:
                    continue
                for w in (w for w in line if w.text.startswith("•")):
                    near = min(abs(c["x"] - w.cx) for c in cols)
                    if near > DOT_REACH:
                        dots_out += 1
                        continue
                    dots_in += 1
                    for col in (c for c in cols if abs(c["x"] - w.cx) == near):
                        add(boards, title, decor, finish, code, col, source)
    return list(boards.values()), dots_in, dots_out, problems


def add(boards, title, decor, finish, code, col, source):
    """The board in `col` for one row, or one more sheet size of it."""
    short = col["substrate"].replace("moisture_resistant_", "mr-").replace("_", "-")
    bid = f"nikpol.{slug(title)}.{slug(decor)}-{slug(finish)}.{col['thickness']:g}.{short}"
    b = boards.setdefault(bid, {
        "id": bid, "brand": "Nikpol", "range": title,
        "decor": f"{decor} {code.text}", "finish": finish, "kind": "melamine",
        "substrate": col["substrate"], "thickness": col["thickness"],
        "sheets": [], "grained": bool(TIMBER.search(decor)),
        "double_sided": True, "emission": col["emission"], "edge_tapes": [],
        "preview_colour": "", "item_codes": {}, "source": source,
    })
    sheet = {"length": col["size"][0], "width": col["size"][1]}
    if sheet not in b["sheets"]:
        b["sheets"].append(sheet)


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--pdf", type=Path)
    ap.add_argument("--out", type=Path, default=ROOT / "catalog/boards/nikpol.json")
    ap.add_argument("--date", default="2026-06", help="the document's date (its PDF creation date)")
    ap.add_argument("--retrieved", required=True)
    args = ap.parse_args()
    pdf = args.pdf
    if pdf is None:
        pdf = ROOT / "target/nikpol/full-decor-availability.pdf"
        pdf.parent.mkdir(parents=True, exist_ok=True)
        urllib.request.urlretrieve(URL, pdf)
    source = {"url": URL, "document_date": args.date, "retrieved": args.retrieved,
              "verified": False}
    boards, dots_in, dots_out, problems = parse(read_pages(pdf), source)
    out = {
        "brand": "Nikpol",
        "website": "https://www.nikpol.com.au",
        "notes": f"Generated by tools/parse_nikpol.py from Nikpol's Full Decor Availability "
                 f"({args.date}): decorative board columns of each range's table (thickness, sheet "
                 "size, substrate). The decor keeps Nikpol's colour code (no item codes are "
                 "printed). Edging, laminate and benchtop columns and pages are not recorded. "
                 "Decors named after a timber are marked grained. Do not edit by hand: rerun the tool.",
        "boards": sorted(boards, key=lambda b: b["id"]),
        "edge_tapes": [],
    }
    args.out.write_text(json.dumps(out, indent=2, ensure_ascii=False) + "\n")
    print(f"{len(boards)} boards from {dots_in} dots ({dots_out} in other columns) → {args.out}")
    for p in problems:
        print("warning:", p, file=sys.stderr)


if __name__ == "__main__":
    main()
