#!/usr/bin/env python3
"""Parse the Laminex National Availability Guide into catalog/boards/laminex.json (BACKLOG DATA-2).

    tools/parse_laminex.py [--pdf guide.pdf] [--out catalog/boards/laminex.json]

Without --pdf it downloads the current guide into target/laminex/ (never committed). The trade
pages print two tables side by side, and in them a colour, its finish, substrate and thickness are
merged cells: one label, set level with the middle of the product rows it covers. So the tables
are read from word positions (`pdftotext -bbox`): each product row is a line with a sheet size and
a product code, and each label goes to the run of rows it is centred on.

Scope: the Decorated MDF and Decorated Particleboard trade tables, with their product codes per
sheet size and the matching edging codes the tables give (thickness and roll length; the tables do
not print an edging's width or material, so those stay out). Whether a decor is a woodgrain comes
from the Specifier pages' WOODGRAINS group. Every record keeps `verified: false`. Standard library
only.
"""

import argparse
import html
import json
import re
import subprocess
import sys
import urllib.request
from pathlib import Path

from parse_polytec import ROOT, Word, lines, merge_existing, slug

URL = "https://www.laminex.com.au/documents/Laminex-National-Availability-Guide-Master"

TITLES = {"Decorated MDF": "Decorated MDF", "Decorated Particleboard": "Decorated Particleboard",
          "Compact Laminate": "Compact Laminate"}
# Tables with no SUBSTRATE column: what every row of them is (DATA-5b).
NO_SUBSTRATE = {"Compact Laminate": "Compact"}
# Trade Essentials pages: raw boards, and white melamine boards for carcasses (DATA-5).
ESSENTIALS = {"Raw Particleboard": "raw", "Raw MDF": "raw", "White Particleboard": "melamine",
              "White MDF": "melamine"}
PROD = re.compile(r"^\d{6}$")
NUMBER = re.compile(r"^\d{2,4}$")
THICKNESS = re.compile(r"^\d{1,2}(\.\d{1,2})?$")
# Colour codes: Laminex's are numbers, Formica's letters and digits (AWI, BC5, 1028).
COLOUR_CODE = re.compile(r"^[A-Z0-9]{2,4}$")
FINISH = re.compile(r"^[A-Z]{1,3}$")
FACE = re.compile(r"^(DS|SS[A-Z]*)$")
# Lines of one label start closer than this (pt) apart; separate labels are a row (about 11 pt).
LABEL_GAP = 8.5
# Matching edging column groups, by the word naming each: (thickness mm, roll m). Invisiedge
# prints no thickness, so its codes are not recorded.
EDGING = {"STANDARD": (1.0, 25.0), "LARGE": (2.0, 100.0), "INVISIEDGE": None}
CATEGORY = re.compile(r"^(WHITES|NEUTRALS|WOOD.?GRAINS|MINERALS|ACCENTS)$")


def read_pages(pdf):
    """Every page as a list of lines of words (whole pages: the trade pages' title is on the
    right)."""
    out = subprocess.run(
        ["pdftotext", "-bbox", str(pdf), "-"], check=True, capture_output=True, text=True
    ).stdout
    return [
        lines([
            Word(float(a), float(b), float(c), float(d), html.unescape(t))
            for a, b, c, d, t in re.findall(
                r'<word xMin="([\d.]+)" yMin="([\d.]+)" xMax="([\d.]+)" yMax="([\d.]+)">(.*?)</word>',
                page,
            )
        ])
        for page in re.findall(r"<page [^>]*>(.*?)</page>", out, re.S)
    ]


def substrate(text):
    """`MDF MR HD E0` → (catalog substrate, emission)."""
    words = text.split()
    mr = "MR" in words
    emission = next((w for w in words if re.match(r"^E\d$|^E-Zero$", w)), "")
    if words and words[0] == "MDF":
        return ("moisture_resistant_mdf" if mr else "mdf"), emission
    if words and words[0] in ("PB", "Particleboard"):
        return ("moisture_resistant_particleboard" if mr else "particleboard"), emission
    if words == ["Compact"]:
        return "compact", emission
    return None, emission


def legend(lines):
    """`FINISHES LEGEND: AG AbsoluteGloss AGR AbsoluteGrain ...` → {code: name}."""
    out = {}
    for line in lines:
        words = [w.text for w in line]
        # The finishes legend; some pages print it as plain LEGEND:. (Not the substrate legend.)
        if "LEGEND:" not in words or "SUBSTRATE" in words:
            continue
        rest = words[words.index("LEGEND:") + 1:]
        # A code, then its name up to the next code: `MR Moisture Resistant UF Unfinished`.
        code = None
        for w in rest:
            if FINISH.match(w) or w == "LEGEND:":
                code = w if w != "LEGEND:" else None
                if code:
                    out[code] = ""
            elif code:
                out[code] = (out[code] + " " + w).strip()
        out = {k: v for k, v in out.items() if v}
    return out


def woodgrain_codes(pages):
    """Colour codes listed under WOODGRAINS on the Specifier pages."""
    codes = set()
    for lines in pages:
        words = [w for line in lines for w in line]
        labels = [w for w in words if w.x0 < 45 and CATEGORY.match(w.text.replace("\xad", ""))]
        if not labels:
            continue
        rows = []
        for line in lines:
            code = next((w for w in line if 90 < w.x0 < 160 and NUMBER.match(w.text)), None)
            if code and any(w.x0 < code.x0 and w.x0 > 45 and w.text.isalpha() for w in line):
                rows.append(((code.y0 + code.y1) / 2, code.text))
        groups = []
        for w in sorted(labels, key=lambda w: w.y0):
            name = w.text.replace("\xad", "")
            # WHITES & NEUTRALS is set on two lines: one label.
            if groups and name == "NEUTRALS" and groups[-1][1] == "WHITES":
                groups[-1] = ((groups[-1][0] + (w.y0 + w.y1) / 2) / 2, "WHITES")
            else:
                groups.append(((w.y0 + w.y1) / 2, name))
        for (y, label), members in spans(groups, rows):
            if label.startswith("WOOD"):
                codes.update(code for _, code in members)
    return codes


def spans(labels, rows, cuts=None):
    """Gives each label (y, value) the run of rows (y, ...) it sits in the middle of, in order:
    [((y, value), [row, ...]), ...]. A run's middle is halfway between its first row's top and
    its last row's bottom, each row reaching halfway to its neighbours. With `cuts` (row indices a
    run may end before), runs end only there: a colour's rows are whole finishes, a finish's whole
    substrates."""
    labels = sorted(labels)
    rows = sorted(rows, key=lambda r: r[0])
    if not rows:
        return [(label, []) for label in labels]
    ys = [r[0] for r in rows]
    pitch = (ys[-1] - ys[0]) / (len(ys) - 1) if len(ys) > 1 else 11.0
    edges = [ys[0] - pitch / 2] + [(a + b) / 2 for a, b in zip(ys, ys[1:])] + [ys[-1] + pitch / 2]
    ends = sorted(set(cuts or range(1, len(rows) + 1)) | {len(rows)})
    out = []
    i = 0
    for n, label in enumerate(labels):
        if i >= len(rows):
            out.append((label, []))
            continue
        left = len(labels) - n - 1
        options = [e for e in ends if e > i]
        # Leave a run for each label still to come.
        options = options[:max(1, len(options) - left)]
        if n == len(labels) - 1:
            options = [len(rows)]
        end = min(options, key=lambda e: abs((edges[i] + edges[e]) / 2 - label[0]))
        out.append((label, rows[i:end]))
        i = end
    return out


def run_ends(assigned):
    """The row indices where the runs of `spans` end."""
    ends, n = [], 0
    for _, members in assigned:
        n += len(members)
        ends.append(n)
    return ends


def labels_in(words, x0, x1, accept):
    """Words between x0 and x1 that `accept` takes, joined into labels where lines are close:
    [(centre y, text), ...]."""
    picked = sorted((w for w in words if x0 <= w.x0 < x1 and accept(w)), key=lambda w: (w.y0, w.x0))
    # (top, last line's top, bottom, words): a word joins the label when its line starts within
    # LABEL_GAP of the label's last line.
    out = []
    for w in picked:
        if out and w.y0 - out[-1][1] < LABEL_GAP:
            top, _, bottom, parts = out[-1]
            parts.append(w)
            out[-1] = (top, w.y0, max(bottom, w.y1), parts)
        else:
            out.append((w.y0, w.y0, w.y1, [w]))
    labels = []
    for y0, _, y1, parts in out:
        lines = {}
        for p in parts:
            lines.setdefault(round(p.y0), []).append(p)
        text = " ".join(" ".join(p.text for p in sorted(l, key=lambda p: p.x0)) for _, l in sorted(lines.items()))
        labels.append(((y0 + y1) / 2, text))
    return labels


def header_x(words, name, after=0.0):
    hits = [w.x0 for w in words if w.text == name and w.x0 >= after]
    return min(hits) if hits else None


HEADINGS = {"TYPE", "SUBSTRATE", "THICKNESS", "FINISH", "PROD", "PACK", "FACE", "25MT", "100MT"}


def table(words, title, brand, legend_y, finishes, grained, source, problems):
    """One trade table (half a page): its boards, each with its edging codes."""
    if not any(PROD.match(w.text) for w in words):
        return []
    header = [w for w in words if w.y0 < 150 and w.text in HEADINGS]
    cols = {n: header_x(header, n) for n in ("TYPE", "SUBSTRATE", "THICKNESS", "FINISH", "PROD", "PACK")}
    fixed_substrate = NO_SUBSTRATE.get(title)
    if fixed_substrate:
        cols.pop("SUBSTRATE")
    edging_x = sorted(w.x0 for w in header if w.text in ("25MT", "100MT"))
    if None in cols.values():
        problems.append(f"{title}: table header not found ({cols})")
        return []
    # Each roll column belongs to the group named nearest above it. The suggested (not matching)
    # edgings of some tables print no thickness: none is recorded for them.
    groups = [(w.x0, w.text) for w in words if w.y0 < 150 and w.text in EDGING]
    edging = [EDGING[min(groups, key=lambda g: abs(g[0] - x))[1]] if groups else None for x in edging_x]
    top = max(w.y1 for w in header) + 0.5
    body = [w for w in words if top <= w.y0 < legend_y - 2 and w.text.strip()]
    # Product rows: a six-digit product code in the PROD column.
    rows = []
    for w in body:
        if PROD.match(w.text) and abs(w.x0 - cols["PROD"]) < 12:
            y = (w.y0 + w.y1) / 2
            same = [v for v in body if abs((v.y0 + v.y1) / 2 - y) < 3]
            size = [v.text for v in same if cols["THICKNESS"] + 8 < v.x0 < cols["PROD"] - 12]
            sheet = re.findall(r"(\d{3,4}) × (\d{3,4})", " ".join(size))
            face = next((v.text for v in same if FACE.match(v.text)), "")
            codes = [None] * len(edging_x)
            for v in same:
                if PROD.match(v.text) and v.x0 > cols["PACK"] + 10:
                    i = min(range(len(edging_x)), key=lambda k: abs(edging_x[k] - v.x0))
                    codes[i] = v.text
            if not sheet:
                problems.append(f"{title}: product {w.text} has no sheet size")
                continue
            rows.append((y, {"prod": w.text, "sheet": sheet[0], "face": face,
                             "edging": list(zip(edging, codes))}))
    if not rows:
        return []
    # Labels sit left of their headings: the finish codes start a little before FINISH and the
    # substrate right after them.
    fin_x = cols["FINISH"] - 6
    sub_x = cols["FINISH"] + 14
    # The TYPE column holds the table's name set on end; colours start past it.
    # The colour codes of tables without a substrate column start a little further left.
    code_x = fin_x - (26 if fixed_substrate else 22)
    colour = labels_in(body, cols["TYPE"] + 12, code_x, lambda w: not NUMBER.match(w.text) and w.text[0].isalpha())
    code = labels_in(body, code_x, fin_x, lambda w: COLOUR_CODE.match(w.text))
    finish = labels_in(body, fin_x, sub_x, lambda w: FINISH.match(w.text))
    subst = [] if fixed_substrate else labels_in(body, sub_x, cols["THICKNESS"] - 3,
                                                 lambda w: w.text.isalnum())
    # Without a substrate column the thickness sits centred under its wider heading.
    thick_end = cols["THICKNESS"] + (16 if fixed_substrate else 8)
    thick = labels_in(body, cols["THICKNESS"] - 3, thick_end, lambda w: THICKNESS.match(w.text))
    # Finest first: each coarser column's runs end where a finer one's do.
    rows.sort(key=lambda r: r[0])
    cuts = None
    for name, labels in (("thickness", thick), ("substrate", subst), ("finish", finish),
                         ("colour", colour), ("code", code)):
        assigned = spans(labels, rows, cuts)
        for label, members in assigned:
            for _, r in members:
                r[name] = label[1]
        if name != "colour":
            cuts = run_ends(assigned)
    boards = {}
    for _, r in rows:
        if fixed_substrate:
            r["substrate"] = fixed_substrate
        if not all(k in r for k in ("colour", "finish", "substrate", "thickness")):
            problems.append(f"{title}: product {r['prod']} is missing a label: {r}")
            continue
        sub, emission = substrate(r["substrate"])
        if sub is None:
            problems.append(f"{title}: product {r['prod']}: unknown substrate {r['substrate']!r}")
            continue
        fin = finishes.get(r["finish"], r["finish"])
        single = r["face"] not in ("DS", "")
        key = (r["colour"], fin, sub, r["thickness"], r["substrate"], single)
        rid = f"{slug(brand)}.{slug(title)}.{slug(r['colour'])}-{slug(fin)}.{r['thickness']}.{slug(r['substrate'])}"
        if single:
            rid += ".single-sided"
        b = boards.setdefault(key, {
            "id": rid, "brand": brand, "range": title, "decor": r["colour"], "finish": fin,
            "kind": "laminated_board" if fixed_substrate else "melamine", "substrate": sub, "thickness": float(r["thickness"]), "sheets": [],
            "grained": r.get("code") in grained, "double_sided": not single, "emission": emission,
            "edge_tapes": [], "preview_colour": "", "item_codes": {}, "source": source,
            # The brand's colour code: kept for checks, not written (the schema has no place).
            "_code": r.get("code"),
        })
        length, width = r["sheet"]
        if {"length": float(length), "width": float(width)} not in b["sheets"]:
            b["sheets"].append({"length": float(length), "width": float(width)})
        size = f"{length}x{width}"
        if b["item_codes"].get(size, r["prod"]) != r["prod"]:
            problems.append(f"{rid}: two products for {size}: {b['item_codes'][size]}, {r['prod']}")
        b["item_codes"][size] = r["prod"]
        for column, c in r["edging"]:
            tape = (c, *column, r["colour"], fin) if column and c else None
            if tape and tape not in b["edge_tapes"]:
                b["edge_tapes"].append(tape)
    return list(boards.values())


def essentials_table(words, title, legend_y, finishes, source, problems):
    """One Trade Essentials table: TYPE, then substrate and thickness as merged cells, then sheet
    size, face, colour, finish code and product code on each product's own line."""
    hx = {}
    for w in words:
        if w.y0 < 150 and w.text in ("TYPE", "SUBSTRATE", "THICKNESS", "SHEET", "FACE", "COLOUR",
                                     "FINISH", "FINSIH", "PROD", "PACK"):
            hx.setdefault("FINISH" if w.text == "FINSIH" else w.text, w.x0)
    if not all(k in hx for k in ("TYPE", "THICKNESS", "SHEET", "FINISH", "PROD")):
        problems.append(f"{title}: table header not found ({hx})")
        return []
    top = max(w.y1 for w in words if w.y0 < 150 and w.text in ("PROD", "PACK", "SHEET")) + 0.5
    body = [w for w in words if top <= w.y0 < legend_y - 2 and w.text.strip()]
    right = sorted(x for k, x in hx.items() if x > hx["SHEET"] + 5)
    rows = []
    for w in body:
        if not (PROD.match(w.text) and abs(w.x0 - hx["PROD"]) < 14):
            continue
        y = (w.y0 + w.y1) / 2
        same = [v for v in body if abs((v.y0 + v.y1) / 2 - y) < 3]
        sheet = re.findall(r"(\d{3,4}) × (\d{3,4})", " ".join(
            v.text for v in sorted(same, key=lambda v: v.x0) if hx["SHEET"] - 10 <= v.x0 < right[0] - 3))
        if not sheet:
            problems.append(f"{title}: product {w.text} has no sheet size")
            continue
        r = {"prod": w.text, "sheet": sheet[0]}
        if "FACE" in hx:
            r["face"] = next((v.text for v in same if FACE.match(v.text) and abs(v.x0 - hx["FACE"]) < 8), "")
        r["finish"] = next((v.text for v in same if FINISH.match(v.text)
                            and -8 <= v.x0 - hx["FINISH"] <= 12), "")
        rows.append((y, r))
    if not rows:
        return []
    thick_x0 = hx["THICKNESS"] - 4
    thick = labels_in(body, thick_x0, hx["SHEET"] - 8, lambda w: THICKNESS.match(w.text))
    # The TYPE column holds the table's title set on end, except where the heading reads MDF TYPE:
    # there it holds the MDF type, and the grade follows under SUBSTRATE.
    mdf_type = any(w.text == "MDF" and abs(w.x0 - hx["TYPE"]) < 4 for w in words if w.y0 < 150)
    sub_x0 = hx["TYPE"] - 3 if mdf_type else hx["TYPE"] + 12
    subst = labels_in(body, sub_x0, thick_x0, lambda w: w.text.isalnum())
    rows.sort(key=lambda r: r[0])
    assigned = spans(thick, rows)
    for label, members in assigned:
        for _, r in members:
            r["thickness"] = label[1]
    for label, members in spans(subst, rows, run_ends(assigned)):
        for _, r in members:
            r["substrate"] = label[1]
    # A colour may be set on two lines around its row ("White / Aquaban"), or span rows.
    if "COLOUR" in hx:
        colour = labels_in(body, hx["COLOUR"] - 3, hx["FINISH"] - 8, lambda w: w.text[0].isalpha())
        for label, members in spans(colour, rows):
            for _, r in members:
                r["colour"] = label[1]
    kind = ESSENTIALS[title]
    boards = {}
    for _, r in rows:
        if "thickness" not in r or "substrate" not in r:
            problems.append(f"{title}: product {r['prod']} is missing a label: {r}")
            continue
        sub, emission = substrate(r["substrate"])
        if sub is None:
            problems.append(f"{title}: product {r['prod']}: unknown substrate {r['substrate']!r}")
            continue
        decor = r.get("colour") or "Raw"
        fin = finishes.get(r["finish"], r["finish"])
        single = r.get("face", "DS") not in ("DS", "")
        rid = f"trade-essentials.{slug(title)}.{slug(decor)}-{slug(fin)}.{r['thickness']}.{slug(r['substrate'])}"
        if single:
            rid += ".single-sided"
        b = boards.setdefault(rid, {
            "id": rid, "brand": "Trade Essentials", "range": title, "decor": decor, "finish": fin,
            "kind": kind, "substrate": sub, "thickness": float(r["thickness"]), "sheets": [],
            "grained": False, "double_sided": not single, "emission": emission, "edge_tapes": [],
            "preview_colour": "", "item_codes": {}, "source": source,
        })
        length, width = r["sheet"]
        size = f"{length}x{width}"
        if {"length": float(length), "width": float(width)} not in b["sheets"]:
            b["sheets"].append({"length": float(length), "width": float(width)})
        if b["item_codes"].get(size, r["prod"]) != r["prod"]:
            problems.append(f"{rid}: two products for {size}: {b['item_codes'][size]}, {r['prod']}")
        b["item_codes"][size] = r["prod"]
    return list(boards.values())


def lining_table(words, title, legend_y, source, problems):
    """The MDF Primed Panels (Surround by Laminex) table: profile, thickness and substrate as
    merged cells, nested in that order, then sheet size, colour and product code on each line."""
    hx = {}
    for w in words:
        if w.y0 < 105 and w.text in ("FACE", "THICKNESS", "SUBSTRATE", "SHEET", "COLOUR", "PROD"):
            hx.setdefault(w.text, w.x0)
    if len(hx) < 6:
        problems.append(f"{title}: table header not found ({hx})")
        return []
    top = max(w.y1 for w in words if w.y0 < 105 and w.text in hx) + 0.5
    body = [w for w in words if top <= w.y0 < legend_y - 2 and w.text.strip()]
    rows = []
    for w in body:
        if not (PROD.match(w.text) and abs(w.x0 - hx["PROD"]) < 12):
            continue
        y = (w.y0 + w.y1) / 2
        same = sorted((v for v in body if abs((v.y0 + v.y1) / 2 - y) < 3), key=lambda v: v.x0)
        sheet = re.findall(r"(\d{3,4}) × (\d{3,4})", " ".join(
            v.text for v in same if hx["SHEET"] - 5 <= v.x0 < hx["COLOUR"] - 3))
        colour = " ".join(v.text for v in same if hx["COLOUR"] - 3 <= v.x0 < hx["PROD"] - 5)
        if not sheet:
            problems.append(f"{title}: product {w.text} has no sheet size")
            continue
        rows.append((y, {"prod": w.text, "sheet": sheet[0], "colour": colour}))
    rows.sort(key=lambda r: r[0])
    columns = [
        ("substrate", hx["SUBSTRATE"] - 3, hx["SHEET"] - 5, lambda w: w.text.isalnum()),
        ("thickness", hx["THICKNESS"] - 3, hx["SUBSTRATE"] - 3, lambda w: THICKNESS.match(w.text)),
        ("profile", hx["FACE"] - 3, hx["THICKNESS"] - 3, lambda w: re.match(r"^[A-Za-z0-9.]+$", w.text)),
    ]
    cuts = None
    for name, x0, x1, accept in columns:
        assigned = spans(labels_in(body, x0, x1, accept), rows, cuts)
        for label, members in assigned:
            for _, r in members:
                r[name] = label[1]
        cuts = run_ends(assigned)
    boards = {}
    for _, r in rows:
        if not all(k in r for k in ("substrate", "thickness", "profile")):
            problems.append(f"{title}: product {r['prod']} is missing a label: {r}")
            continue
        sub, emission = substrate(r["substrate"])
        if sub is None:
            problems.append(f"{title}: product {r['prod']}: unknown substrate {r['substrate']!r}")
            continue
        profile = " ".join(p if p == "VJ" else p.capitalize() for p in r["profile"].split())
        rid = f"laminex.primed-panels.{slug(profile)}.{r['thickness']}.{slug(r['substrate'])}"
        b = boards.setdefault(rid, {
            "id": rid, "brand": "Laminex", "range": "MDF Primed Panels (Surround by Laminex)",
            "decor": r["colour"] or "Primed", "finish": profile, "kind": "lining", "substrate": sub,
            "thickness": float(r["thickness"]), "sheets": [], "grained": False,
            # The profile is machined into one face.
            "double_sided": False, "emission": emission, "edge_tapes": [], "preview_colour": "",
            "item_codes": {}, "source": source,
        })
        length, width = r["sheet"]
        size = f"{length}x{width}"
        if {"length": float(length), "width": float(width)} not in b["sheets"]:
            b["sheets"].append({"length": float(length), "width": float(width)})
        if b["item_codes"].get(size, r["prod"]) != r["prod"]:
            problems.append(f"{rid}: two products for {size}: {b['item_codes'][size]}, {r['prod']}")
        b["item_codes"][size] = r["prod"]
    return list(boards.values())


def plywood_table(words, legend_y, source, problems):
    """The Trade Essentials plywood table's decorative and non-structural half (DATA-5b): product
    type, bond, grade and species as merged cells over thickness and one product per line.
    Coated plywood (grade Coated: its species cell names the coating) is a laminated board,
    the rest raw plywood."""
    hx = {n: header_x([w for w in words if w.y0 < 150], n)
          for n in ("PRODUCT", "BOND", "GRADE", "SPECIES", "THICKNESS", "PROD", "PACK")}
    if None in hx.values():
        problems.append(f"Plywood: table header not found ({hx})")
        return []
    top = max(w.y1 for w in words if w.y0 < 150 and w.text in ("PRODUCT", "SPECIES")) + 0.5
    body = [w for w in words if top <= w.y0 < legend_y - 2]
    rows = []
    for w in body:
        if PROD.match(w.text) and abs(w.x0 - hx["PROD"]) < 12:
            y = (w.y0 + w.y1) / 2
            same = [v for v in body if abs((v.y0 + v.y1) / 2 - y) < 3]
            sheet = re.findall(r"(\d{3,4}) × (\d{3,4})",
                               " ".join(v.text for v in same if hx["THICKNESS"] + 15 < v.x0 < hx["PROD"] - 2))
            if not sheet:
                problems.append(f"Plywood: product {w.text} has no sheet size")
                continue
            rows.append((y, {"prod": w.text, "sheet": sheet[0]}))
    rows.sort(key=lambda r: r[0])
    columns = [
        ("thickness", hx["THICKNESS"], hx["THICKNESS"] + 15, lambda w: THICKNESS.match(w.text)),
        ("species", hx["SPECIES"], hx["THICKNESS"], lambda w: w.text[0].isalpha()),
        ("grade", hx["GRADE"], hx["SPECIES"], lambda w: True),
        ("bond", hx["BOND"], hx["GRADE"], lambda w: True),
        ("type", hx["PRODUCT"], hx["BOND"], lambda w: w.y1 - w.y0 < 10),
    ]
    cuts = None
    for name, x0, x1, accept in columns:
        assigned = spans(labels_in(body, x0 - 2, x1 - 2, accept), rows, cuts)
        for label, members in assigned:
            for _, r in members:
                r[name] = label[1]
        cuts = run_ends(assigned)
    boards = {}
    for _, r in rows:
        if not all(k in r for k in ("thickness", "species", "grade", "bond", "type")):
            problems.append(f"Plywood: product {r['prod']} is missing a label: {r}")
            continue
        kind = re.sub(r"\s*Manufactured to \S+", "", r["type"]).replace("Species", "").strip()
        kind = re.sub(r"\s+", " ", kind)
        coated = r["grade"] == "Coated"
        rid = (f"trade-essentials.plywood.{slug(kind)}.{slug(r['species'])}-{slug(r['grade'])}."
               f"{r['thickness']}")
        b = boards.setdefault(rid, {
            "id": rid, "brand": "Trade Essentials", "range": f"Plywood: {kind}",
            "decor": r["species"], "finish": r["grade"],
            "kind": "laminated_board" if coated else "raw", "substrate": "plywood",
            "thickness": float(r["thickness"]), "sheets": [], "grained": not coated,
            "double_sided": True, "emission": "", "edge_tapes": [], "preview_colour": "",
            "item_codes": {}, "source": source,
        })
        # As printed, grain along the first number: bending plywood comes both ways round.
        length, width = r["sheet"]
        size = f"{length}x{width}"
        if b["item_codes"].get(size, r["prod"]) != r["prod"]:
            problems.append(f"{rid}: {size} is both {b['item_codes'][size]} and {r['prod']}")
            continue
        if size not in b["item_codes"]:
            b["sheets"].append({"length": float(length), "width": float(width)})
            b["item_codes"][size] = r["prod"]
    return list(boards.values())


def parse(pages, sources):
    """Boards and edgings per brand: `sources` maps a brand to its source record."""
    finishes = {}
    for lines in pages:
        finishes.update(legend(lines))
    grained = woodgrain_codes(pages[:45])
    # The guide covers Laminex, then Formica from the page that introduces it (after the Laminex
    # trade pages; the contents mention Formica too).
    formica = next((i for i, lines in enumerate(pages)
                    if i > 62 and "formica.com.au" in " ".join(w.text for l in lines for w in l)),
                   len(pages))
    boards, problems = [], []
    for n, lines in enumerate(pages):
        words = [w for line in lines for w in line]
        top = " ".join(w.text for w in sorted((w for w in words if w.y0 < 40), key=lambda w: w.x0))
        essentials = next((t for t in ESSENTIALS if top.endswith(t)), None)
        page_finishes = {**finishes, **legend(lines)}
        legend_y = min((w.y0 for w in words if w.text == "LEGEND:"), default=10_000)
        if top.endswith("MDF Primed Panels") and n < formica:
            mid = max(w.x1 for w in words) / 2
            for half in ([w for w in words if w.cx < mid], [w for w in words if w.cx >= mid]):
                if any(PROD.match(w.text) for w in half):
                    boards += lining_table(half, "MDF Primed Panels", legend_y, sources["Laminex"], problems)
            continue
        if essentials and n >= formica:
            # Up to four tables a page, each starting at its TYPE column.
            starts = sorted(w.x0 for w in words if w.text == "TYPE" and w.y0 < 150)
            for a, b in zip(starts, starts[1:] + [10_000]):
                part = [w for w in words if a - 3 <= w.x0 < b - 3]
                boards += essentials_table(part, essentials, legend_y, page_finishes,
                                           sources["Trade Essentials"], problems)
            continue
        if top == "Plywood" and n >= formica:
            mid = max(w.x1 for w in words) / 2
            # The left half: decorative and non-structural plywood; structural is not cabinet board.
            boards += plywood_table([w for w in words if w.cx < mid], legend_y,
                                    sources["Trade Essentials"], problems)
            continue
        title = next((t for t in TITLES if top.endswith(t)), None)
        if title is None or n < 45:
            continue
        brand = "Formica" if n >= formica else "Laminex"
        finishes_here = page_finishes
        # Only Laminex's Specifier pages group colours into WOODGRAINS and the rest.
        woodgrains = grained if brand == "Laminex" else set()
        mid = max(w.x1 for w in words) / 2 if words else 420
        for half in ([w for w in words if w.cx < mid], [w for w in words if w.cx >= mid]):
            boards += table(half, title, brand, legend_y, finishes_here, woodgrains, sources[brand], problems)
    tapes = {}
    for b in boards:
        ids = []
        for code, thickness, roll, decor, fin in b["edge_tapes"]:
            tid = f"{slug(b['brand'])}.edging.{code}"
            tapes.setdefault(tid, {
                "id": tid, "brand": b["brand"], "code": code, "decor": decor, "finish": fin,
                "thickness": thickness, "rolls": [roll], "source": b["source"],
            })
            if tid not in ids:
                ids.append(tid)
        b["edge_tapes"] = ids
    # Merge rows of the same board printed in two places (a colour split across pages).
    merged = {}
    for b in boards:
        m = merged.setdefault(b["id"], b)
        if m is not b:
            m["sheets"] += [s for s in b["sheets"] if s not in m["sheets"]]
            m["item_codes"].update(b["item_codes"])
            m["edge_tapes"] += [t for t in b["edge_tapes"] if t not in m["edge_tapes"]]
    return list(merged.values()), list(tapes.values()), problems


BRANDS = {
    "Laminex": ("https://www.laminex.com.au", "catalog/boards/laminex.json"),
    "Formica": ("https://www.formica.com.au", "catalog/boards/formica.json"),
    "Trade Essentials": ("https://www.tradeessentials.com.au", "catalog/boards/trade-essentials.json"),
}


def notes(brand, date):
    what = {
        "Trade Essentials": "Trade Essentials Raw and White Particleboard and MDF tables (DATA-5); "
                            "`kind` raw for raw boards, melamine for white ones. ",
    }.get(brand, f"{brand} Decorated MDF, Decorated Particleboard and Compact Laminate trade "
                 "tables (compact: `kind` laminated_board, substrate compact). Edging records "
                 "carry only what the tables print (code, thickness, roll): width and material are "
                 "not printed there, and the suggested (not exact match) edgings of some tables "
                 "print no thickness, so they are left out. ")
    grain = {
        "Laminex": "Woodgrains are the decors the guide's Specifier pages list under WOODGRAINS. ",
        "Formica": "The guide does not say which Formica decors are woodgrains: `grained` is false "
                   "throughout, so check the grain direction of woodgrain decors yourself. ",
    }.get(brand, "")
    return (f"Generated by tools/parse_laminex.py from the Laminex National Availability Guide "
            f"({date}): {what}Item codes are per sheet size. Stocking columns are not recorded. "
            f"{grain}Do not edit by hand: rerun the tool.")


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--pdf", type=Path)
    ap.add_argument("--out", type=Path, default=ROOT, help="the repository (catalog/boards/ inside it)")
    ap.add_argument("--date", default="2025-09", help="the guide's date as printed on its cover")
    ap.add_argument("--retrieved", required=True, help="ISO date the guide was downloaded")
    args = ap.parse_args()
    pdf = args.pdf
    if pdf is None:
        pdf = ROOT / "target/laminex/national-availability-guide.pdf"
        pdf.parent.mkdir(parents=True, exist_ok=True)
        urllib.request.urlretrieve(URL, pdf)
    pages = read_pages(pdf)
    sources = {b: {"url": URL, "document_date": args.date, "retrieved": args.retrieved, "verified": False}
               for b in BRANDS}
    boards, tapes, problems = parse(pages, sources)
    for brand, (website, path) in BRANDS.items():
        out_path = args.out / path
        mine = [{k: v for k, v in b.items() if not k.startswith("_")} for b in boards if b["brand"] == brand]
        my_tapes = [t for t in tapes if t["brand"] == brand]
        existing = json.loads(out_path.read_text()) if out_path.exists() else {}
        merge_existing(mine, my_tapes, existing)
        out = {
            "brand": brand,
            "website": website,
            "notes": notes(brand, args.date),
            "boards": sorted(mine, key=lambda b: b["id"]),
            "edge_tapes": sorted(my_tapes, key=lambda t: t["id"]),
        }
        out_path.parent.mkdir(parents=True, exist_ok=True)
        out_path.write_text(json.dumps(out, indent=2, ensure_ascii=False) + "\n")
        print(f"{brand}: {len(mine)} boards, {len(my_tapes)} edgings → {out_path}")
    for p in problems:
        print("warning:", p, file=sys.stderr)


if __name__ == "__main__":
    main()
