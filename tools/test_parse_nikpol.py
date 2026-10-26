"""Tests for parse_nikpol.py on made-up tables (no supplier text)."""

import unittest

import parse_nikpol as n
from parse_polytec import Word

SOURCE = {"url": "u", "document_date": "2026-06", "retrieved": "2026-10-26", "verified": False}


def line(y, *placed):
    """Words at given left edges: line(10, (60, "Test"), (90, "Blue"))."""
    return [Word(x, y - 4, x + 5 * len(t), y + 4, t) for x, t in placed]


def dots(y, *xs):
    # A dot 4 pt wide centred on x.
    return [Word(x - 2, y - 4, x + 2, y + 4, "•") for x in xs]


def row(y, decor, code, finish, *xs):
    return sorted(line(y, (60, decor), (150, code), (180, finish)) + dots(y, *xs),
                  key=lambda w: w.x0)


# Two substrates, thicknesses under their size, then edging and a laminate size that is not a
# board. Column centres: 16mm 215, 18mm 245 (STD); 16mm 300 (MR).
TABLE = [
    line(10, (60, "Test Range"), (220, "Decorative"), (280, "Boards"), (400, "Edging")),
    line(20, (210, "E0 STD PB"), (290, "E0 MR PB")),
    line(26, (60, "Colour"), (150, "Code"), (180, "Finish")),
    line(32, (200, "2800x2070mm"), (275, "2800x2070mm"), (430, "3050x1310mm")),
    line(40, (205, "16mm"), (235, "18mm"), (290, "16mm")),
    row(50, "Test Oak", "9001", "MW", 215, 300, 440),
    row(58, "Test Blue", "9002", "SU", 245),
]

# Thicknesses beside the size share one dot under the middle of the heading (240 here).
BESIDE = [
    line(10, (60, "Beside"), (220, "Decorative"), (280, "Boards"), (400, "Edging")),
    line(20, (200, "E1 MR MDF")),
    line(26, (60, "Colour"), (150, "Code"), (180, "Finish")),
    line(32, (200, "2800x2070mm,"), (265, "18mm,"), (295, "25mm")),
    row(50, "Test Grey", "9003", "ST", 247),
]


class ParseNikpol(unittest.TestCase):
    def test_board_columns_stop_at_the_edging_heading(self):
        cols = n.columns(n.tables(TABLE)[0][1])
        self.assertEqual([(c["thickness"], c["substrate"]) for c in cols], [
            (16, "particleboard"), (18, "particleboard"), (16, "moisture_resistant_particleboard"),
        ])

    def test_each_dot_is_a_board_of_its_column(self):
        boards, dots_in, dots_out, _ = n.parse([TABLE], SOURCE)
        self.assertEqual(sorted(b["id"] for b in boards), [
            "nikpol.test-range.test-blue-su.18.particleboard",
            "nikpol.test-range.test-oak-mw.16.mr-particleboard",
            "nikpol.test-range.test-oak-mw.16.particleboard",
        ])
        # The laminate dot is outside every board column.
        self.assertEqual((dots_in, dots_out), (3, 1))
        oak = next(b for b in boards if b["id"].endswith("oak-mw.16.particleboard"))
        self.assertEqual(oak["decor"], "Test Oak 9001")
        self.assertEqual(oak["range"], "Test Range")
        self.assertEqual(oak["emission"], "E0")
        self.assertTrue(oak["grained"])
        self.assertEqual(oak["sheets"], [{"length": 2800, "width": 2070}])

    def test_thicknesses_beside_a_size_share_its_dot(self):
        boards, _, _, _ = n.parse([BESIDE], SOURCE)
        self.assertEqual(sorted((b["thickness"], b["substrate"], b["emission"]) for b in boards),
                         [(18, "moisture_resistant_mdf", "E1"), (25, "moisture_resistant_mdf", "E1")])

    def test_substrate_names(self):
        self.assertEqual(n.substrate("E0 NON MR PB"), ("particleboard", "E0"))
        self.assertEqual(n.substrate("F**** (0.3MG/L) MR Particle Board"),
                         ("moisture_resistant_particleboard", "F****"))
        self.assertEqual(n.substrate("ABS 100M"), None)


if __name__ == "__main__":
    unittest.main()
