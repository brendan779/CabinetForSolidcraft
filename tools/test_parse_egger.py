"""Tests for parse_egger.py on a made-up table (no supplier text)."""

import unittest

import parse_egger as e

SOURCE = {"url": "u", "document_date": "2024-11", "retrieved": "2026-10-26", "verified": False}


def line(y, *texts, x=60.0):
    out = []
    for t in texts:
        out.append((x, y - 4, x + 5 * len(t), y + 4, t))
        x += 5 * len(t) + 3
    return out


WORDS = (line(50, "SOLID", "COLOURS") + line(60, "U100", "ST9", "(NEW)", "Test", "Blue")
         + line(68, "PERFECTSENSE") + line(76, "W200", "PM", "Test", "White")
         + line(84, "WOODGRAINS") + line(92, "H300", "ST12", "Test", "Oak")
         # The specifications column shares a line with a row.
         + line(92, "2800", "x", "2070", x=400))
# Column centres as on the page: laminate, 18, 25, worktop, Eurolight, compact.
X = [276, 295, 315, 334, 354, 373]


def circle(x, y):
    r = 1.7
    return (f'<path d="M {x - r} {y} C {x - r} {y - r} {x + r} {y - r} {x + r} {y} '
            f'C {x + r} {y + r} {x - r} {y + r} {x - r} {y} Z"/>')


def svg(marks):
    return "<svg><defs></defs>" + "".join(circle(X[c], y) for c, y in marks) + "</svg>"


class ParseEgger(unittest.TestCase):
    def test_rows_take_code_texture_name_and_section(self):
        rows = e.rows_of(WORDS)
        self.assertEqual([r[1:] for r in rows], [
            ("U100", "ST9", "Test Blue", "Solid Colours", False),
            ("W200", "PM", "Test White", "PerfectSense", False),
            ("H300", "ST12", "Test Oak", "Woodgrains", True),
        ])

    def test_dots_give_each_row_its_products_and_boards(self):
        # Every column has a dot somewhere, so all six are found.
        marks = [(0, 60), (1, 60), (5, 60), (1, 76), (2, 76), (1, 92), (3, 92), (4, 92)]
        rows = e.rows_of(WORDS)
        avail = e.availability(rows, e.dots_of(svg(marks)))
        self.assertEqual(avail[0], {"laminate", "panel18", "compact"})
        ids = [b["id"] for b in e.boards(rows, avail, SOURCE)]
        self.assertEqual(ids, [
            "egger.compact-laminate.u100-st9.13",
            "egger.eurodekor-mf.h300-st12.18",
            "egger.eurodekor-mf.u100-st9.18",
            "egger.eurodekor-mf.w200-pm.18",
            "egger.eurodekor-mf.w200-pm.25",
            "egger.eurolight.h300-st12.50",
        ])
        by_id = {b["id"]: b for b in e.boards(rows, avail, SOURCE)}
        self.assertEqual(by_id["egger.eurodekor-mf.w200-pm.25"]["substrate"], "moisture_resistant_mdf")
        self.assertEqual(by_id["egger.eurodekor-mf.u100-st9.18"]["substrate"], "hmr")
        self.assertTrue(by_id["egger.eurodekor-mf.h300-st12.18"]["grained"])

    def test_a_dot_off_every_row_is_an_error(self):
        marks = [(c, 60) for c in range(6)] + [(1, 72)]
        with self.assertRaises(ValueError):
            e.availability(e.rows_of(WORDS), e.dots_of(svg(marks)))


if __name__ == "__main__":
    unittest.main()
