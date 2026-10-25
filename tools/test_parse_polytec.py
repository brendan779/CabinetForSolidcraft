"""Tests for parse_polytec.py on a made-up page laid out like the guide (no supplier text)."""

import unittest

import parse_polytec as p


def level(x, y, *texts):
    """Words on one line, left to right, 5 pt per letter."""
    out = []
    for t in texts:
        out.append(p.Word(x, y, x + 5 * len(t), y + 10.5, t))
        x += 5 * len(t) + 3
    return out


def turned(x, y_bottom, *texts):
    """A heading set a quarter turn, reading upward from y_bottom."""
    out = []
    for t in texts:
        h = 4 if t == "x" else 5 * len(t)
        out.append(p.Word(x, y_bottom - h, x + 10.5, y_bottom, t))
        y_bottom -= h + 2
    return out


def page(title, headers, rows):
    words = level(100, 20, *title.split()) + [w for h in headers for w in h]
    y = 130
    for row in rows:
        for x, t in row:
            words += level(x, y, *t.split()) if t else []
        y += 12.5
    return p.lines(words)


BOARD_A, BOARD_B, NAME, TAPE_A, TAPE_B = 60, 105, 140, 250, 300

HEADERS = [
    turned(BOARD_A - 10, 111, "16mm"),
    turned(BOARD_A, 111, "2400", "x", "1200"),
    turned(BOARD_B - 10, 111, "16mm"),
    turned(BOARD_B, 111, "3115", "x", "1200"),
    turned(TAPE_A - 12, 111, "1mm", "x", "21mm"),
    turned(TAPE_A, 111, "Rigid", "100m"),
    turned(TAPE_A + 12, 111, "Un-glued"),
    turned(TAPE_B - 12, 111, "1mm", "x", "29mm"),
    turned(TAPE_B, 111, "Rigid", "100m"),
    turned(TAPE_B + 12, 111, "Pre-glued"),
    # The page title, up the margin.
    turned(500, 190, "DECORATIVE", "board"),
]


def parse(title, rows, headers=HEADERS):
    pg = page(title, headers, rows)
    return p.parse([pg], "2026-10-23", "https://example.invalid/guide.pdf")


class ParsePolytec(unittest.TestCase):
    def test_turned_headings_read_bottom_to_top(self):
        texts = [t for t, _ in p.phrases([HEADERS[1] + HEADERS[4]])]
        self.assertIn("2400 x 1200", texts)
        self.assertIn("1mm x 21mm", texts)

    def test_a_row_gives_a_board_per_thickness_with_item_codes_per_sheet(self):
        boards, tapes, rows, problems = parse(
            "18mm MR MDF E-Zero, 2400x1200 & 3115x1200",
            [
                [(NAME, "SOLID MATT")],
                [(BOARD_A - 3, "6604"), (BOARD_B - 3, "531 ▲"), (NAME, "Classic White"),
                 (TAPE_A - 3, "1879"), (TAPE_B - 3, "51836 ▲")],
            ],
        )
        self.assertEqual(problems, [])
        self.assertEqual(rows, 1)
        (b,) = boards
        # The title says 18 mm; the guide's own column headings say 16mm there.
        self.assertEqual(b["id"], "polytec.decorative.classic-white-matt.18.mr-mdf")
        self.assertEqual(b["thickness"], 18.0)
        self.assertEqual(b["item_codes"], {"2400x1200": "6604", "3115x1200": "531"})
        self.assertEqual(b["emission"], "E-Zero")
        self.assertFalse(b["grained"])
        self.assertFalse(b["source"]["verified"])
        self.assertEqual(
            b["edge_tapes"],
            ["polytec.abs.classic-white-matt.1x21.100m", "polytec.abs.classic-white-matt.1x29.100m"],
        )
        by_id = {t["id"]: t for t in tapes}
        self.assertEqual(by_id["polytec.abs.classic-white-matt.1x29.100m"]["code"], "51836")
        self.assertTrue(by_id["polytec.abs.classic-white-matt.1x29.100m"]["pre_glued"])
        self.assertFalse(by_id["polytec.abs.classic-white-matt.1x21.100m"]["pre_glued"])

    def test_na_cells_add_no_sheet_and_no_tape(self):
        boards, tapes, _, _ = parse(
            "16mm MR MDF E-Zero, 2400x1200 & 3115x1200",
            [
                [(NAME, "TIMBERPRINT WOODMATT")],
                [(BOARD_A, "na"), (BOARD_B - 3, "55572"), (NAME, "Prime Oak"),
                 (TAPE_A - 3, "53306"), (TAPE_B, "na")],
            ],
        )
        (b,) = boards
        self.assertEqual(b["sheets"], [{"length": 3115.0, "width": 1200.0}])
        self.assertTrue(b["grained"])
        self.assertEqual(len(tapes), 1)

    def test_two_thickness_tables_take_each_columns_thickness(self):
        headers = [
            turned(BOARD_A - 10, 111, "25mm"),
            turned(BOARD_A, 111, "2400", "x", "1200"),
            turned(BOARD_B - 10, 111, "32mm"),
            turned(BOARD_B, 111, "2400", "x", "1200"),
            HEADERS[-1],
        ]
        boards, _, _, _ = parse(
            "25mm or 32mm MR MDF E-Zero, 2400x1200",
            [[(NAME, "SOLID VENETTE")], [(BOARD_A - 3, "47265"), (BOARD_B - 3, "47279"), (NAME, "Black")]],
            headers,
        )
        self.assertEqual(sorted(b["thickness"] for b in boards), [25.0, 32.0])

    def test_a_tape_whose_code_differs_elsewhere_keeps_both(self):
        boards, tapes, _, _ = parse(
            "16mm MR MDF E-Zero, 2400x1200 & 3115x1200",
            [
                [(NAME, "SOLID SHEEN")],
                [(BOARD_A - 3, "625"), (NAME, "Classic White"), (TAPE_A - 3, "51962")],
                [(NAME, "SOLID SHEEN")],
                [(BOARD_B - 3, "1076"), (NAME, "Classic White"), (TAPE_A, "228")],
            ],
        )
        codes = sorted(t["code"] for t in tapes)
        self.assertEqual(codes, ["228", "51962"])
        self.assertEqual(len({t["id"] for t in tapes}), 2)

    def test_merge_keeps_preview_colours(self):
        boards, tapes, _, _ = parse(
            "16mm MR MDF E-Zero, 2400x1200 & 3115x1200",
            [[(NAME, "SOLID MATT")], [(BOARD_A - 3, "8417"), (NAME, "Alabaster")]],
        )
        old = {"boards": [{**boards[0], "preview_colour": "#eeeae0"}]}
        p.merge_existing(boards, tapes, old)
        self.assertEqual(boards[0]["preview_colour"], "#eeeae0")

    def test_a_section_name_beside_the_margin_title_still_starts_a_section(self):
        title = p.Word(500, 120, 510.5, 200, "Carcass")
        boards, _, _, _ = parse(
            "16mm MR MDF E-Zero, 2400x1200 & 3115x1200",
            [
                [(NAME, "SOLID TEXTURE")],
                [(BOARD_A - 3, "100"), (NAME, "Black")],
                [(NAME, "SOLID MATT")],
                [(BOARD_A - 3, "200"), (NAME, "Black")],
            ],
            HEADERS + [[title]],
        )
        self.assertEqual(sorted(b["finish"] for b in boards), ["Matt", "Texture"])


def commercial_page(headers, rows):
    """A COMMERCIAL page: `Colour` heading at x 100, sections and names under it."""
    return page("NOTES", [turned(100, 111, "Colour")] + headers, rows)


class ParseCommercial(unittest.TestCase):
    BOARDS = [
        turned(200, 111, "16mm"), turned(212, 111, "Particle"), level(208, 101, "MR"),
        turned(224, 111, "3600", "x", "1800"),
        turned(240, 111, "25mm"), turned(252, 111, "STD", "LDF", "E-Zero"),
        turned(264, 111, "3600", "x", "1800"),
        level(208, 113, "20"), level(248, 113, "15"),
        turned(500, 190, "COMMERCIAL"),
    ]
    TAPES = [
        turned(200, 111, "1mm", "x", "21mm", "Rigid"), turned(212, 111, "100m", "Un-glued"),
        turned(245, 111, "2mm", "x", "38mm", "Rigid"), turned(257, 111, "100m", "Un-glued"),
        turned(500, 190, "COMMERCIAL"),
    ]

    def test_boards_take_thickness_substrate_and_size_from_their_column(self):
        pg = commercial_page(self.BOARDS, [
            [(110, "MATT ex NSW/VIC/QLD")],
            [(110, "Natural Oak"), (206, "47516"), (246, "na")],
            [(110, "Black"), (206, "25232"), (246, "26474 ‡")],
        ])
        tapes_pg = commercial_page(self.TAPES, [
            [(110, "MATT")],
            [(110, "Black"), (203, "43744"), (248, "na")],
        ])
        boards, tapes, rows, problems = p.parse([pg, tapes_pg], "2026-10-23", "https://x.invalid")
        self.assertEqual(problems, [])
        self.assertEqual(rows, 3)
        by_id = {b["id"]: b for b in boards}
        self.assertEqual(sorted(by_id), [
            "polytec.commercial.black-matt.16.mr-particleboard",
            "polytec.commercial.black-matt.25.ldf",
            "polytec.commercial.natural-oak-matt.16.mr-particleboard",
        ])
        ldf = by_id["polytec.commercial.black-matt.25.ldf"]
        self.assertEqual(ldf["item_codes"], {"3600x1800": "26474"})
        self.assertEqual(ldf["emission"], "E-Zero")
        self.assertEqual(ldf["edge_tapes"], ["polytec.abs.black-matt.1x21.100m"])
        # Oaks under MATT are still woodgrains.
        self.assertTrue(by_id["polytec.commercial.natural-oak-matt.16.mr-particleboard"]["grained"])
        self.assertFalse(ldf["grained"])
        (tape,) = tapes
        self.assertEqual((tape["code"], tape["thickness"], tape["width"]), ("43744", 1.0, 21.0))

    def test_a_tape_without_a_width_becomes_the_sized_tape_with_its_code(self):
        tapes = {
            "a.1mm.100m": {"id": "a.1mm.100m", "code": "9053"},
            "b.1x21.100m": {"id": "b.1x21.100m", "code": "9053", "width": 21.0},
            "c.1mm.50m": {"id": "c.1mm.50m", "code": "10010"},
        }
        boards = {"x": {"edge_tapes": ["a.1mm.100m", "c.1mm.50m"]}}
        p.same_code_widths(boards, tapes)
        self.assertEqual(sorted(tapes), ["b.1x21.100m", "c.1mm.50m"])
        self.assertEqual(boards["x"]["edge_tapes"], ["b.1x21.100m", "c.1mm.50m"])


class ParseLinings(unittest.TestCase):
    RAW = """Profile
9mm
2400mm x 1200mm
MR MDF E-Zero
9mm
3000mm x 1200mm
MR MDF E-Zero
WET AREA PRE-PRIMED 9MM MADE-UP PANELS ROUTED PANELS
Groove 10 11111 22222
Bead 20 33333 na
ITEM CODE COLOUR WIDTH
"""

    def test_each_profile_row_is_a_lining_with_a_code_per_sheet_length(self):
        src = {"url": "https://x.invalid", "verified": False}
        boards = p.lining_tables(self.RAW, src)
        self.assertEqual([b["id"] for b in boards], [
            "polytec.easycraft-panels-wet-area.groove-10.9.mr-mdf",
            "polytec.easycraft-panels-wet-area.bead-20.9.mr-mdf",
        ])
        self.assertEqual(boards[0]["item_codes"], {"2400x1200": "11111", "3000x1200": "22222"})
        self.assertEqual(boards[1]["sheets"], [{"length": 2400.0, "width": 1200.0}])
        self.assertEqual({b["kind"] for b in boards}, {"lining"})

    def test_a_row_with_the_wrong_number_of_codes_is_refused(self):
        with self.assertRaises(ValueError):
            p.lining_tables(self.RAW.replace("Bead 20 33333 na", "Bead 20 33333"), {})


if __name__ == "__main__":
    unittest.main()
