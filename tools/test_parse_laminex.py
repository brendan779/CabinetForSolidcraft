"""Tests for parse_laminex: a trade table built from word positions as pdftotext -bbox gives them."""

import unittest

from parse_laminex import essentials_table, labels_in, spans, table
from parse_polytec import Word

SOURCE = {"url": "u", "document_date": "2025-09", "retrieved": "2026-10-23", "verified": False}
FINISHES = {"NT": "Natural", "F": "Flint"}
# Column x positions, as on the guide's left-hand tables.
X = {"TYPE": 33, "colour": 48, "code": 105, "FINISH": 122, "sub": 143, "THICKNESS": 183,
     "sheet": 199, "FACE": 238, "PROD": 255, "PACK": 283, "std": 317, "large": 345}


def word(x, y, text, width=None):
    w = width or 5.0 * len(text)
    return Word(x, y, x + w, y + 5, text)


def header():
    words = [word(X[k], 120, k) for k in ("TYPE", "FINISH", "THICKNESS", "FACE", "PROD", "PACK")]
    words += [word(154, 107, "SUBSTRATE"), word(X["std"], 130, "25MT"), word(346, 130, "100MT"),
              word(319, 98, "STANDARD"), word(348, 110, "LARGE")]
    return words


def row(y, prod, size="2400 × 1200", face="DS", std=None, large=None, thickness=None):
    a, _, b = size.split()
    words = [word(X["sheet"], y, a), word(213, y, "×", 3), word(217, y, b),
             word(X["FACE"], y, face), word(X["PROD"], y, prod), word(X["PACK"], y, "20")]
    if thickness:
        words.append(word(X["THICKNESS"], y, thickness))
    if std:
        words.append(word(X["std"], y, std))
    if large:
        words.append(word(X["large"], y, large))
    return words


class ParseLaminex(unittest.TestCase):
    def test_a_label_takes_the_rows_it_is_centred_on(self):
        rows = [(y, i) for i, y in enumerate([10, 21, 32, 43, 54])]
        out = spans([(21, "A"), (48.5, "B")], rows)
        self.assertEqual([[r[1] for r in m] for _, m in out], [[0, 1, 2], [3, 4]])

    def test_a_coarser_label_ends_only_where_a_finer_one_does(self):
        rows = [(y, i) for i, y in enumerate([10, 21, 32, 43])]
        # Centred on rows 0-1 alone it would end after row 1, but the finer column allows
        # ends only after rows 0 and 2.
        out = spans([(16, "A"), (43, "B")], rows, cuts=[1, 3])
        self.assertEqual([[r[1] for r in m] for _, m in out], [[0, 1, 2], [3]])

    def test_lines_of_one_label_join_and_rows_apart_stay_apart(self):
        words = [word(143, 100, "MDF"), word(160, 100, "MR"), word(147, 107, "HD"),
                 word(160, 107, "E0"), word(143, 118, "MDF")]
        labels = labels_in(words, 140, 180, lambda w: True)
        self.assertEqual([t for _, t in labels], ["MDF MR HD E0", "MDF"])

    def test_a_table_gives_boards_per_colour_finish_substrate_and_thickness(self):
        words = header()
        words += row(150, "100001", thickness="16", std="200001", large="300001")
        words += row(161, "100002", size="3600 × 1200", thickness="16", std="200001")
        words += row(172, "100003", thickness="18", face="SSBB")
        words += [word(X["colour"], 161, "Coastal"), word(80, 161, "Oak"), word(X["code"], 161, "123"),
                  word(X["FINISH"] + 2, 161, "NT"), word(X["sub"], 155, "MDF"), word(156, 155, "MR"),
                  word(165, 155, "E0"), word(X["sub"], 172, "MDF"), word(156, 172, "STD"),
                  word(170, 172, "E0")]
        problems = []
        boards = table(words, "Decorated MDF", "Laminex", 10_000, FINISHES, {"123"}, SOURCE, problems)
        self.assertEqual(problems, [])
        by_id = {b["id"]: b for b in boards}
        self.assertEqual(sorted(by_id), [
            "laminex.decorated-mdf.coastal-oak-natural.16.mdf-mr-e0",
            "laminex.decorated-mdf.coastal-oak-natural.18.mdf-std-e0.single-sided",
        ])
        b = by_id["laminex.decorated-mdf.coastal-oak-natural.16.mdf-mr-e0"]
        self.assertEqual(b["item_codes"], {"2400x1200": "100001", "3600x1200": "100002"})
        self.assertEqual(b["substrate"], "moisture_resistant_mdf")
        self.assertTrue(b["grained"] and b["double_sided"])
        # Standard edging is 1 mm on 25 m rolls, large format 2 mm on 100 m rolls.
        self.assertEqual(b["edge_tapes"], [("200001", 1.0, 25.0, "Coastal Oak", "Natural"),
                                           ("300001", 2.0, 100.0, "Coastal Oak", "Natural")])
        single = by_id["laminex.decorated-mdf.coastal-oak-natural.18.mdf-std-e0.single-sided"]
        self.assertEqual(single["substrate"], "mdf")
        self.assertFalse(single["double_sided"])


    def test_an_essentials_table_reads_merged_thickness_and_a_title_set_on_end(self):
        hx = {"TYPE": 33, "SUBSTRATE": 62, "THICKNESS": 91, "SHEET": 128, "FINISH": 168,
              "PROD": 198, "PACK": 228}
        words = [word(x, 120, k) for k, x in hx.items()]
        # The title column: upright words, not a substrate.
        words += [Word(33, 300, 40, 330, "RAW"), Word(33, 330, 40, 345, "MDF")]
        for y, prod, size in ((144, "994289", "3050 × 1200"), (155, "994340", "3600 × 1200")):
            a, _, b = size.split()
            words += [word(120, y, a), word(134, y, "×", 3), word(138, y, b), word(168, y, "UF"),
                      word(192, y, prod), word(228, y, "60")]
        words += [word(48, 150, "MDF"), word(61, 150, "STD"), word(72, 150, "E0"), word(95, 150, "4.75")]
        problems = []
        boards = essentials_table(words, "Raw MDF", 10_000, {"UF": "No Finish"}, SOURCE, problems)
        self.assertEqual(problems, [])
        self.assertEqual(len(boards), 1)
        b = boards[0]
        self.assertEqual(b["id"], "trade-essentials.raw-mdf.raw-no-finish.4.75.mdf-std-e0")
        self.assertEqual((b["kind"], b["substrate"], b["thickness"]), ("raw", "mdf", 4.75))
        self.assertEqual(b["item_codes"], {"3050x1200": "994289", "3600x1200": "994340"})


if __name__ == "__main__":
    unittest.main()
