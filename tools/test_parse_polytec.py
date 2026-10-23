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


if __name__ == "__main__":
    unittest.main()
