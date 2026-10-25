#!/usr/bin/env python3
"""Read the EGGER Decorative Collection range for Australia into catalog/boards/egger.json
(BACKLOG DATA-3).

    tools/parse_egger.py [--pdf brochure.pdf] [--out catalog/boards/egger.json]

The source is ForestOne's (EGGER's Australian distributor) "EGGER Decorative Collection for
trades" brochure. Its "Sorted by A-Z" page lists every decor (code, texture, name, section) with a
dot per product it comes in: laminate, 18 mm and 25 mm Eurodekor panels, worktops, Eurolight and
compact laminate. The rows come from `pdftotext -bbox`; the dots are drawn, not text, so they come
from the page as SVG (`pdftocairo -svg`): small round paths in the product columns.

Recorded: the 18 and 25 mm panels (2800 x 2070, HMR particleboard Super E0, PerfectSense on MR MDF
E0), 50 mm Eurolight (the décor option; 38 mm is raw only) and 13 mm compact laminate
(2790 x 2060). Laminate sheets and worktops are not cabinet boards and are left out. The brochure
prints no item codes and no edging codes ("matching ABS edging available for all colours").
Every record keeps `verified: false`. Standard library only.
"""

import argparse
import datetime
import html
import json
import re
import subprocess
import tempfile
import urllib.request
from pathlib import Path

URL = ("https://forest.one/media/sparsh/product_attachment/"
       "25072_AW_ForestOne_EGGER_3_x_digital_pdfs_-_for_trades_v6.pdf")
ROOT = Path(__file__).resolve().parent.parent
DOCUMENT_DATE = "2024-11"

SECTIONS = {
    "SOLID COLOURS": ("Solid Colours", False),
    "PERFECTSENSE": ("PerfectSense", False),
    "MATERIALS": ("Materials", False),
    "WOODGRAINS": ("Woodgrains", True),
    "FEELWOOD": ("Feelwood", True),
}
# The product columns, left to right, as headed on the page.
COLUMNS = ["laminate", "panel18", "panel25", "worktop", "eurolight", "compact"]
DECOR = re.compile(r"^[A-Z]\d{3,4}$")
TEXTURE = re.compile(r"^(ST\d+|PM|TM\d+)$")
# The decor names end before the first product column.
NAME_MAX_X = 265.0
# A dot belongs to the row whose centre is this close (pt).
ROW_REACH = 3.0


def words_of(pdf, page):
    """(x0, y0, x1, y1, text) for every word on `page`."""
    out = subprocess.run(["pdftotext", "-f", str(page), "-l", str(page), "-bbox", str(pdf), "-"],
                         check=True, capture_output=True, text=True).stdout
    return [(float(a), float(b), float(c), float(d), html.unescape(t)) for a, b, c, d, t in
            re.findall(r'<word xMin="([\d.]+)" yMin="([\d.]+)" xMax="([\d.]+)" yMax="([\d.]+)">'
                       r'(.*?)</word>', out)]


def dots_of(svg):
    """Centres of the small round marks drawn on the page: paths 1-5 pt across, about square."""
    body = svg[svg.index("</defs>"):] if "</defs>" in svg else svg
    dots = []
    for m in re.finditer(r"<path ([^>]*)>", body):
        attrs = m.group(1)
        d = re.search(r'd="([^"]*)"', attrs)
        if not d:
            continue
        nums = [float(v) for v in re.findall(r"-?\d+\.?\d*", d.group(1))]
        xs, ys = nums[0::2], nums[1::2]
        if not xs or len(xs) != len(ys):
            continue
        w, h = max(xs) - min(xs), max(ys) - min(ys)
        if not (1 < w < 5 and 1 < h < 5 and abs(w - h) < 0.5):
            continue
        cx, cy = (max(xs) + min(xs)) / 2, (max(ys) + min(ys)) / 2
        if t := re.search(r'transform="matrix\(([^)]*)\)"', attrs):
            a, b, c, dd, e, f = (float(v) for v in t.group(1).split(","))
            cx, cy = a * cx + c * cy + e, b * cx + dd * cy + f
        dots.append((cx, cy))
    return dots


def rows_of(words):
    """Decor rows of the A-Z table: (y centre, code, texture, name, section, grained)."""
    lines = {}
    for w in words:
        lines.setdefault(round((w[1] + w[3]) / 2), []).append(w)
    rows, section = [], None
    for y in sorted(lines):
        line = sorted(lines[y], key=lambda w: w[0])
        text = " ".join(w[4] for w in line)
        for head, sec in SECTIONS.items():
            if text.startswith(head):
                section = sec
        toks = [w[4] for w in line]
        if section and len(toks) >= 3 and DECOR.match(toks[0]) and TEXTURE.match(toks[1]):
            # The product columns and the specifications beside them can share the line.
            name = [w[4] for w in line[2:] if w[4] != "(NEW)" and w[0] < NAME_MAX_X]
            ys = [(w[1] + w[3]) / 2 for w in line[:2]]
            rows.append((sum(ys) / len(ys), toks[0], toks[1], " ".join(name), *section))
    return rows


def columns_of(dots):
    """The product columns' x centres: the dots' x positions, clustered."""
    xs = sorted(x for x, _ in dots)
    clusters = []
    for x in xs:
        if clusters and x - clusters[-1][-1] < 4:
            clusters[-1].append(x)
        else:
            clusters.append([x])
    centres = [sum(c) / len(c) for c in clusters]
    if len(centres) != len(COLUMNS):
        raise ValueError(f"expected {len(COLUMNS)} columns of dots, found {len(centres)}")
    return centres


def availability(rows, dots):
    """For each row, the products it has a dot for. A dot near no row is an error."""
    centres = columns_of(dots)
    out = {i: set() for i in range(len(rows))}
    for x, y in dots:
        i = min(range(len(rows)), key=lambda i: abs(rows[i][0] - y))
        if abs(rows[i][0] - y) > ROW_REACH:
            raise ValueError(f"a dot at ({x:.0f}, {y:.0f}) is near no decor row")
        col = min(range(len(centres)), key=lambda c: abs(centres[c] - x))
        out[i].add(COLUMNS[col])
    return out


def slug(s):
    return re.sub(r"[^a-z0-9]+", "-", s.lower()).strip("-")


def boards(rows, avail, source):
    out = []
    for i, (_, code, texture, name, section, grained) in enumerate(rows):
        has = avail[i]
        perfect = section == "PerfectSense"
        offers = []
        for col, thickness in (("panel18", 18.0), ("panel25", 25.0)):
            if col in has:
                offers.append(("Eurodekor MF", "melamine",
                               "moisture_resistant_mdf" if perfect else "hmr",
                               "E0" if perfect else "Super E0", thickness, 2800.0, 2070.0))
        if "eurolight" in has:
            offers.append(("Eurolight", "melamine", "other", "", 50.0, 2800.0, 2070.0))
        if "compact" in has:
            offers.append(("Compact Laminate", "laminated_board", "compact", "", 13.0,
                           2790.0, 2060.0))
        for rng, kind, substrate, emission, thickness, length, width in offers:
            out.append({
                "id": f"egger.{slug(rng)}.{slug(code + ' ' + texture)}.{thickness:g}",
                "brand": "EGGER", "range": rng, "decor": f"{name} {code}",
                "finish": texture, "kind": kind, "substrate": substrate, "thickness": thickness,
                "sheets": [{"length": length, "width": width}], "grained": grained,
                "double_sided": True, "emission": emission, "edge_tapes": [],
                "preview_colour": "", "item_codes": {}, "source": source,
            })
    return sorted(out, key=lambda b: b["id"])


def find_page(pdf):
    """The page sorted A-Z (the next one repeats the table by colour code)."""
    n = int(re.search(r"Pages:\s+(\d+)", subprocess.run(
        ["pdfinfo", str(pdf)], check=True, capture_output=True, text=True).stdout).group(1))
    for page in range(1, n + 1):
        if any(w[4] == "A-Z" for w in words_of(pdf, page)):
            return page
    raise ValueError("no 'Sorted by A-Z' page")


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--pdf", type=Path)
    ap.add_argument("--out", type=Path, default=ROOT / "catalog/boards/egger.json")
    args = ap.parse_args()
    pdf = args.pdf
    if pdf is None:
        pdf = ROOT / "target/egger/forestone-egger-trades.pdf"
        pdf.parent.mkdir(parents=True, exist_ok=True)
        urllib.request.urlretrieve(URL, pdf)
    page = find_page(pdf)
    with tempfile.TemporaryDirectory() as tmp:
        svg = Path(tmp) / "page.svg"
        subprocess.run(["pdftocairo", "-f", str(page), "-l", str(page), "-svg", str(pdf), str(svg)],
                       check=True)
        dots = dots_of(svg.read_text())
    rows = rows_of(words_of(pdf, page))
    avail = availability(rows, dots)
    source = {"url": URL, "document_date": DOCUMENT_DATE,
              "retrieved": datetime.date.today().isoformat(), "verified": False}
    out = {
        "brand": "EGGER",
        "website": "https://forest.one/our-brands/egger",
        "notes": "Generated by tools/parse_egger.py from ForestOne's EGGER Decorative Collection "
                 f"for trades ({DOCUMENT_DATE}), the decor-by-product table sorted A-Z: 18 and "
                 "25 mm Eurodekor MF panels (HMR particleboard Super E0; PerfectSense on MR MDF "
                 "E0), 50 mm Eurolight décor panels (lightweight core, recorded as substrate "
                 "other) and 13 mm compact laminate. The decor keeps EGGER's code; the finish is "
                 "the texture code as printed (ST9, PM...). No item or edging codes are printed. "
                 "Do not edit by hand: rerun the tool.",
        "boards": boards(rows, avail, source),
        "edge_tapes": [],
    }
    args.out.write_text(json.dumps(out, indent=2, ensure_ascii=False) + "\n")
    counts = {c: sum(c in a for a in avail.values()) for c in COLUMNS}
    print(f"page {page}: {len(rows)} decors, {len(dots)} dots {counts}, "
          f"{len(out['boards'])} boards → {args.out}")


if __name__ == "__main__":
    main()
