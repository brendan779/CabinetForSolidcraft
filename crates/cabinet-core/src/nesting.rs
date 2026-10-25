//! Panel-saw nesting: cut list parts laid out on stock sheets so that every cut runs edge to
//! edge (guillotine cuts), with the saw kerf between parts, the sheet edges trimmed, grained parts
//! kept along the sheet's length and the usable leftovers listed as offcuts.
//!
//! The placement is the guillotine free-rectangle method (Jylänki, "A Thousand Ways to Pack the
//! Bin", 2010): parts, largest first, go into the free rectangle they fit most tightly (best short
//! side), and that rectangle is cut in two along the shorter leftover, so every cut is a through
//! cut of the piece it is made in.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::cutlist::CutRow;

/// A stock sheet: `length` runs along the decor's grain when it has one.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Stock {
    pub length: f64,
    pub width: f64,
    /// The decor has a direction: parts may not turn.
    pub grained: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    /// Saw kerf (mm).
    pub kerf: f64,
    /// Taken off each sheet edge before the first cut (factory edges are not square or clean).
    pub trim: f64,
    /// Leftovers at least this long and wide are kept as offcuts.
    pub offcut_length: f64,
    pub offcut_width: f64,
}

impl Default for Settings {
    /// Our defaults for a sliding panel saw: 4.4 mm kerf, 10 mm trim, offcuts from 300 × 100.
    fn default() -> Self {
        Self {
            kerf: 4.4,
            trim: 10.0,
            offcut_length: 300.0,
            offcut_width: 100.0,
        }
    }
}

/// A rectangle on a sheet: `x` along the sheet's length, `y` across it.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub length: f64,
    pub width: f64,
}

impl Rect {
    fn area(&self) -> f64 {
        self.length * self.width
    }
}

/// A part placed on a sheet: its cut size along the sheet's length and width.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Placement {
    pub part: String,
    pub cabinet: String,
    pub rect: Rect,
    /// Turned a quarter: the part's length runs across the sheet.
    pub turned: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Sheet {
    pub board: String,
    pub thickness: f64,
    pub stock: Stock,
    pub placements: Vec<Placement>,
    pub offcuts: Vec<Rect>,
}

impl Sheet {
    /// Share of the sheet's area in parts.
    pub fn yield_ratio(&self) -> f64 {
        let used: f64 = self.placements.iter().map(|p| p.rect.area()).sum();
        used / (self.stock.length * self.stock.width)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Nesting {
    pub sheets: Vec<Sheet>,
    /// Parts larger than their stock sheet (less trim), as `part: why`.
    pub unplaced: Vec<String>,
}

/// Nests every cut list row, grouped by board and thickness, on the sheet `stock` gives for that
/// board and thickness (`None`: the group is reported as unplaced).
pub fn nest(
    rows: &[CutRow],
    stock: impl Fn(&str, f64) -> Option<Stock>,
    settings: &Settings,
) -> Nesting {
    let mut groups: BTreeMap<(String, i64), Vec<&CutRow>> = BTreeMap::new();
    for r in rows {
        let key = (r.board.clone(), (r.thickness * 100.0).round() as i64);
        groups.entry(key).or_default().push(r);
    }
    let mut out = Nesting::default();
    for ((board, _), group) in groups {
        let thickness = group[0].thickness;
        let Some(s) = stock(&board, thickness) else {
            out.unplaced.extend(
                group
                    .iter()
                    .map(|r| format!("{}: no {thickness} mm sheet of {board}", r.part)),
            );
            continue;
        };
        let (sheets, unplaced) = nest_group(&board, thickness, &s, &group, settings);
        out.sheets.extend(sheets);
        out.unplaced.extend(unplaced);
    }
    out
}

struct Open {
    sheet: Sheet,
    free: Vec<Rect>,
}

fn nest_group(
    board: &str,
    thickness: f64,
    stock: &Stock,
    rows: &[&CutRow],
    settings: &Settings,
) -> (Vec<Sheet>, Vec<String>) {
    // Every part on its own, largest first (by area, then by its longer side).
    let mut parts: Vec<&CutRow> = rows
        .iter()
        .flat_map(|r| std::iter::repeat_n(*r, r.quantity as usize))
        .collect();
    parts.sort_by(|a, b| {
        let key = |r: &CutRow| (r.cut_length * r.cut_width, r.cut_length.max(r.cut_width));
        let (ka, kb) = (key(a), key(b));
        kb.0.total_cmp(&ka.0).then(kb.1.total_cmp(&ka.1))
    });
    let usable = Rect {
        x: settings.trim,
        y: settings.trim,
        length: stock.length - 2.0 * settings.trim,
        width: stock.width - 2.0 * settings.trim,
    };
    let mut open: Vec<Open> = Vec::new();
    let mut unplaced = Vec::new();
    for r in parts {
        let fits_blank = |turned: bool| {
            let (l, w) = size(r, turned);
            l <= usable.length + 1e-9 && w <= usable.width + 1e-9
        };
        let may_turn = !(stock.grained && r.grain);
        if !fits_blank(false) && !(may_turn && fits_blank(true)) {
            unplaced.push(format!(
                "{}: {:.0} × {:.0} is larger than a {:.0} × {:.0} sheet",
                r.part, r.cut_length, r.cut_width, stock.length, stock.width
            ));
            continue;
        }
        let best = open
            .iter()
            .enumerate()
            .flat_map(|(s, o)| o.free.iter().enumerate().map(move |(f, rect)| (s, f, rect)))
            .flat_map(|(s, f, rect)| {
                [false, true]
                    .into_iter()
                    .filter(move |&t| !t || may_turn)
                    .filter_map(move |t| {
                        let (l, w) = size(r, t);
                        (l <= rect.length + 1e-9 && w <= rect.width + 1e-9)
                            .then(|| ((rect.length - l).min(rect.width - w), s, f, t))
                    })
            })
            .min_by(|a, b| a.0.total_cmp(&b.0));
        let (s, f, turned) = match best {
            Some((_, s, f, t)) => (s, f, t),
            None => {
                open.push(Open {
                    sheet: Sheet {
                        board: board.to_string(),
                        thickness,
                        stock: stock.clone(),
                        placements: Vec::new(),
                        offcuts: Vec::new(),
                    },
                    free: vec![usable],
                });
                (open.len() - 1, 0, !fits_blank(false))
            }
        };
        let o = &mut open[s];
        let rect = o.free.swap_remove(f);
        let (l, w) = size(r, turned);
        o.sheet.placements.push(Placement {
            part: r.part.clone(),
            cabinet: r.cabinet.clone(),
            rect: Rect {
                x: rect.x,
                y: rect.y,
                length: l,
                width: w,
            },
            turned,
        });
        o.free.extend(split(rect, l, w, settings.kerf));
    }
    let sheets = open
        .into_iter()
        .map(|mut o| {
            o.sheet.offcuts = o
                .free
                .into_iter()
                .filter(|f| {
                    let (a, b) = (f.length.max(f.width), f.length.min(f.width));
                    a >= settings.offcut_length && b >= settings.offcut_width
                })
                .collect();
            o.sheet
        })
        .collect();
    (sheets, unplaced)
}

/// A part's cut size on the sheet, turned or not.
fn size(r: &CutRow, turned: bool) -> (f64, f64) {
    if turned {
        (r.cut_width, r.cut_length)
    } else {
        (r.cut_length, r.cut_width)
    }
}

/// What is left of `rect` after an `l` × `w` part in its corner: two rectangles, cut along the
/// shorter leftover first (so the larger piece stays whole), a kerf from the part.
fn split(rect: Rect, l: f64, w: f64, kerf: f64) -> Vec<Rect> {
    let right = rect.length - l - kerf;
    let above = rect.width - w - kerf;
    // The first cut runs right through `rect`: across it (at x = l) or along it (at y = w).
    let cut_across_first = right < above;
    let (a, b) = if cut_across_first {
        (
            // Beside the part, full width.
            Rect {
                x: rect.x + l + kerf,
                y: rect.y,
                length: right,
                width: rect.width,
            },
            // Above the part, its length.
            Rect {
                x: rect.x,
                y: rect.y + w + kerf,
                length: l,
                width: above,
            },
        )
    } else {
        (
            Rect {
                x: rect.x + l + kerf,
                y: rect.y,
                length: right,
                width: w,
            },
            Rect {
                x: rect.x,
                y: rect.y + w + kerf,
                length: rect.length,
                width: above,
            },
        )
    };
    [a, b]
        .into_iter()
        .filter(|r| r.length > 1e-6 && r.width > 1e-6)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(part: &str, l: f64, w: f64, quantity: u32, grain: bool) -> CutRow {
        CutRow {
            cabinet: "B01".into(),
            part: part.into(),
            description: String::new(),
            board: "white".into(),
            thickness: 16.0,
            length: l,
            width: w,
            cut_length: l,
            cut_width: w,
            edges: [false; 4],
            grain,
            quantity,
        }
    }

    fn stock(grained: bool) -> impl Fn(&str, f64) -> Option<Stock> {
        move |_, _| {
            Some(Stock {
                length: 2400.0,
                width: 1200.0,
                grained,
            })
        }
    }

    fn overlap(a: &Rect, b: &Rect) -> bool {
        a.x < b.x + b.length - 1e-9
            && b.x < a.x + a.length - 1e-9
            && a.y < b.y + b.width - 1e-9
            && b.y < a.y + a.width - 1e-9
    }

    /// Every placement inside the trimmed sheet, a kerf from every other, none overlapping.
    fn assert_valid(n: &Nesting, s: &Settings) {
        for sheet in &n.sheets {
            let ps = &sheet.placements;
            for (i, p) in ps.iter().enumerate() {
                let r = p.rect;
                assert!(r.x >= s.trim - 1e-9 && r.y >= s.trim - 1e-9, "{p:?}");
                assert!(
                    r.x + r.length <= sheet.stock.length - s.trim + 1e-9,
                    "{p:?}"
                );
                assert!(r.y + r.width <= sheet.stock.width - s.trim + 1e-9, "{p:?}");
                let grown = Rect {
                    x: r.x - s.kerf + 1e-6,
                    y: r.y - s.kerf + 1e-6,
                    length: r.length + 2.0 * s.kerf - 2e-6,
                    width: r.width + 2.0 * s.kerf - 2e-6,
                };
                for q in &ps[i + 1..] {
                    assert!(!overlap(&grown, &q.rect), "{p:?} and {q:?}");
                }
                for o in &sheet.offcuts {
                    assert!(!overlap(&r, o), "{p:?} in offcut {o:?}");
                }
            }
        }
    }

    #[test]
    fn parts_are_placed_inside_the_trim_a_kerf_apart() {
        let s = Settings::default();
        let rows = [
            row("side", 820.0, 560.0, 6, true),
            row("bottom", 568.0, 560.0, 3, true),
            row("rail", 568.0, 100.0, 6, true),
        ];
        let n = nest(&rows, stock(true), &s);
        assert!(n.unplaced.is_empty());
        let placed: usize = n.sheets.iter().map(|s| s.placements.len()).sum();
        assert_eq!(placed, 15);
        assert_valid(&n, &s);
    }

    #[test]
    fn grained_parts_never_turn_and_plain_ones_may() {
        let s = Settings::default();
        // 1100 long across a 1200 wide sheet fits only turned.
        let tall = [row("panel", 1100.0, 2300.0, 1, true)];
        let grained = nest(&tall, stock(true), &s);
        assert_eq!(grained.sheets.len(), 0);
        assert!(grained.unplaced[0].contains("larger than"));
        let plain = nest(&tall, stock(false), &s);
        assert_eq!(plain.sheets.len(), 1);
        assert!(plain.sheets[0].placements[0].turned);
        // A grained part on a grained sheet keeps its length along the sheet's.
        let n = nest(&[row("door", 700.0, 400.0, 8, true)], stock(true), &s);
        assert!(
            n.sheets
                .iter()
                .flat_map(|s| &s.placements)
                .all(|p| !p.turned)
        );
        assert_valid(&n, &s);
    }

    #[test]
    fn a_full_sheet_of_parts_fits_on_one_sheet_and_the_rest_spills() {
        let s = Settings {
            kerf: 4.0,
            trim: 10.0,
            ..Settings::default()
        };
        // (2400 − 20 + 4) / (592 + 4) = 4 across, (1200 − 20 + 4) / (292 + 4) = 4 down.
        let n = nest(&[row("p", 592.0, 292.0, 16, true)], stock(true), &s);
        assert_eq!(n.sheets.len(), 1);
        let n = nest(&[row("p", 592.0, 292.0, 17, true)], stock(true), &s);
        assert_eq!(n.sheets.len(), 2);
        assert_valid(&n, &s);
    }

    #[test]
    fn leftovers_large_enough_are_offcuts() {
        let s = Settings::default();
        let n = nest(&[row("shelf", 600.0, 500.0, 1, true)], stock(true), &s);
        let sheet = &n.sheets[0];
        assert!(!sheet.offcuts.is_empty());
        let area: f64 = sheet.offcuts.iter().map(|o| o.area()).sum();
        let usable = (2400.0 - 20.0) * (1200.0 - 20.0);
        // Everything but the part and the two kerfs beside it comes back as offcuts here.
        assert!(area >= usable - 600.0 * 500.0 - 4.4 * (2380.0 + 1180.0));
        assert!(sheet.yield_ratio() > 0.1 && sheet.yield_ratio() < 0.11);
    }

    #[test]
    fn each_board_and_thickness_nests_on_its_own_sheets() {
        let mut a = row("a", 500.0, 400.0, 1, false);
        a.board = "oak".into();
        let b = row("b", 500.0, 400.0, 1, false);
        let mut c = row("c", 500.0, 400.0, 1, false);
        c.thickness = 18.0;
        let n = nest(&[a, b, c], stock(false), &Settings::default());
        assert_eq!(n.sheets.len(), 3);
        let none = nest(
            &[row("x", 10.0, 10.0, 1, false)],
            |_, _| None,
            &Settings::default(),
        );
        assert!(none.unplaced[0].contains("no 16 mm sheet"));
    }

    /// A whole drawer bank and base cabinet nest without overlaps.
    #[test]
    fn real_cabinets_nest_validly() {
        use crate::{Cabinet, CabinetKind, build_cabinet};
        let lists: Vec<_> = [
            CabinetKind::Base,
            CabinetKind::DrawerBank,
            CabinetKind::Tall,
        ]
        .into_iter()
        .map(|k| build_cabinet(&Cabinet::new("C01", k)).unwrap())
        .collect();
        let cut = crate::cutlist::cut_list(&lists, "white", 1.0);
        let s = Settings::default();
        let n = nest(&cut.rows, stock(true), &s);
        assert!(n.unplaced.is_empty(), "{:?}", n.unplaced);
        assert_valid(&n, &s);
        let parts: u32 = cut.rows.iter().map(|r| r.quantity).sum();
        let placed: usize = n.sheets.iter().map(|s| s.placements.len()).sum();
        assert_eq!(placed as u32, parts);
    }
}
