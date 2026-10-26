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
    /// A kept offcut from an earlier job, not a new sheet: no trim, and it costs nothing.
    #[serde(default)]
    pub from_offcut: bool,
}

/// A piece kept from an earlier job, to nest on before new sheets.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Offcut {
    pub board: String,
    pub thickness: f64,
    /// Along the grain of the sheet it came from.
    pub length: f64,
    pub width: f64,
    #[serde(default)]
    pub grained: bool,
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

/// Nests every cut list row, grouped by board and thickness, on the sheet sizes `stock` offers
/// for that board and thickness: each size is tried and the one that places most parts on the
/// least sheet area wins (none offered: the group is reported as unplaced).
pub fn nest(
    rows: &[CutRow],
    stock: impl Fn(&str, f64) -> Vec<Stock>,
    settings: &Settings,
) -> Nesting {
    nest_with_offcuts(rows, stock, &[], settings)
}

/// As [`nest`], placing parts on the kept `offcuts` of their board and thickness first.
pub fn nest_with_offcuts(
    rows: &[CutRow],
    stock: impl Fn(&str, f64) -> Vec<Stock>,
    offcuts: &[Offcut],
    settings: &Settings,
) -> Nesting {
    let mut groups: BTreeMap<(String, i64), Vec<&CutRow>> = BTreeMap::new();
    // Benchtops and splashbacks are made to a template, not cut from the shop's sheets.
    for r in rows.iter().filter(|r| !r.templated) {
        let key = (r.board.clone(), (r.thickness * 100.0).round() as i64);
        groups.entry(key).or_default().push(r);
    }
    let mut out = Nesting::default();
    for ((board, _), group) in groups {
        let thickness = group[0].thickness;
        let mine: Vec<&Offcut> = offcuts
            .iter()
            .filter(|o| o.board == board && (o.thickness - thickness).abs() < 1e-6)
            .collect();
        let best = stock(&board, thickness)
            .iter()
            .map(|s| nest_group(&board, thickness, s, &mine, &group, settings))
            .min_by(|(a, ua), (b, ub)| {
                let area = |v: &Vec<Sheet>| -> f64 {
                    v.iter()
                        .filter(|s| !s.from_offcut)
                        .map(|s| s.stock.length * s.stock.width)
                        .sum()
                };
                ua.len().cmp(&ub.len()).then(area(a).total_cmp(&area(b)))
            });
        let Some((sheets, unplaced)) = best else {
            out.unplaced.extend(
                group
                    .iter()
                    .map(|r| format!("{}: no {thickness} mm sheet of {board}", r.part)),
            );
            continue;
        };
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
    offcuts: &[&Offcut],
    rows: &[&CutRow],
    settings: &Settings,
) -> (Vec<Sheet>, Vec<String>) {
    // Every part on its own, and each grain-matched group as one strip (its parts end to end
    // along the grain, a kerf apart); largest first (by area, then by its longer side).
    let mut pieces: Vec<Piece> = Vec::new();
    for r in rows {
        if r.grain_match.is_empty() {
            pieces.extend(std::iter::repeat_n(Piece::single(r), r.quantity as usize));
        } else if let Some(p) = pieces
            .iter_mut()
            .find(|p| p.rows[0].grain_match == r.grain_match)
        {
            p.cut_length += settings.kerf + r.cut_length;
            p.cut_width = p.cut_width.max(r.cut_width);
            p.rows.push(r);
        } else {
            let mut p = Piece::single(r);
            p.grain = true;
            pieces.push(p);
        }
    }
    pieces.sort_by(|a, b| {
        let key = |r: &Piece| (r.cut_length * r.cut_width, r.cut_length.max(r.cut_width));
        let (ka, kb) = (key(a), key(b));
        kb.0.total_cmp(&ka.0).then(kb.1.total_cmp(&ka.1))
    });
    let usable = Rect {
        x: settings.trim,
        y: settings.trim,
        length: stock.length - 2.0 * settings.trim,
        width: stock.width - 2.0 * settings.trim,
    };
    // Kept offcuts first: cut edges, no trim.
    let mut open: Vec<Open> = offcuts
        .iter()
        .map(|o| Open {
            sheet: Sheet {
                board: board.to_string(),
                thickness,
                stock: Stock {
                    length: o.length,
                    width: o.width,
                    grained: o.grained,
                },
                placements: Vec::new(),
                offcuts: Vec::new(),
                from_offcut: true,
            },
            free: vec![Rect {
                x: 0.0,
                y: 0.0,
                length: o.length,
                width: o.width,
            }],
        })
        .collect();
    let mut unplaced = Vec::new();
    for r in &pieces {
        let fits_blank = |turned: bool| {
            let (l, w) = size(r, turned);
            l <= usable.length + 1e-9 && w <= usable.width + 1e-9
        };
        let may_turn = !(stock.grained && r.grain);
        if !fits_blank(false) && !(may_turn && fits_blank(true)) {
            let what = if r.rows.len() > 1 {
                format!("{} (grain-matched)", r.rows[0].grain_match)
            } else {
                r.rows[0].part.clone()
            };
            unplaced.push(format!(
                "{what}: {:.0} × {:.0} is larger than a {:.0} × {:.0} sheet",
                r.cut_length, r.cut_width, stock.length, stock.width
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
                        from_offcut: false,
                    },
                    free: vec![usable],
                });
                (open.len() - 1, 0, !fits_blank(false))
            }
        };
        let o = &mut open[s];
        let rect = o.free.swap_remove(f);
        let (l, w) = size(r, turned);
        // A strip's parts one after the other from its start, along the grain.
        let mut along = 0.0;
        for m in &r.rows {
            let (ml, mw) = if turned {
                (m.cut_width, m.cut_length)
            } else {
                (m.cut_length, m.cut_width)
            };
            let (x, y) = if turned {
                (rect.x, rect.y + along)
            } else {
                (rect.x + along, rect.y)
            };
            o.sheet.placements.push(Placement {
                part: m.part.clone(),
                cabinet: m.cabinet.clone(),
                rect: Rect {
                    x,
                    y,
                    length: ml,
                    width: mw,
                },
                turned,
            });
            along += if turned { mw } else { ml } + settings.kerf;
        }
        o.free.extend(split(rect, l, w, settings.kerf));
    }
    let sheets = open
        .into_iter()
        .filter(|o| !(o.sheet.from_offcut && o.sheet.placements.is_empty()))
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

/// The offcuts to keep after cutting a job: the kept ones it did not use, and every usable
/// leftover of the sheets and offcuts it did.
pub fn kept_offcuts(n: &Nesting, stored: &[Offcut]) -> Vec<Offcut> {
    let mut left: Vec<Offcut> = stored.to_vec();
    for s in n.sheets.iter().filter(|s| s.from_offcut) {
        if let Some(i) = left.iter().position(|o| {
            o.board == s.board
                && (o.thickness - s.thickness).abs() < 1e-6
                && o.length == s.stock.length
                && o.width == s.stock.width
        }) {
            left.remove(i);
        }
    }
    for s in &n.sheets {
        left.extend(s.offcuts.iter().map(|r| Offcut {
            board: s.board.clone(),
            thickness: s.thickness,
            length: r.length,
            width: r.width,
            grained: s.stock.grained,
        }));
    }
    left
}

/// What the saw cuts out in one go: a part, or a grain-matched strip of them.
#[derive(Clone)]
struct Piece<'a> {
    rows: Vec<&'a CutRow>,
    cut_length: f64,
    cut_width: f64,
    grain: bool,
}

impl<'a> Piece<'a> {
    fn single(r: &'a CutRow) -> Self {
        Self {
            rows: vec![r],
            cut_length: r.cut_length,
            cut_width: r.cut_width,
            grain: r.grain,
        }
    }
}

/// A piece's cut size on the sheet, turned or not.
fn size(r: &Piece, turned: bool) -> (f64, f64) {
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
            templated: false,
            grain_match: String::new(),
        }
    }

    fn stock(grained: bool) -> impl Fn(&str, f64) -> Vec<Stock> {
        move |_, _| {
            vec![Stock {
                length: 2400.0,
                width: 1200.0,
                grained,
            }]
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

    /// STYLE-1: a grain-matched group is cut as one strip: its parts end to end along the
    /// sheet's length, a kerf apart, in order.
    #[test]
    fn grain_matched_parts_are_cut_end_to_end_in_order() {
        let s = Settings::default();
        let mut rows = vec![row("side", 2000.0, 560.0, 2, true)];
        for (part, l) in [("door", 900.0), ("drawer1", 300.0), ("drawer2", 200.0)] {
            let mut r = row(part, l, 596.0, 1, true);
            r.grain_match = "T01.grain1".into();
            rows.push(r);
        }
        let n = nest(&rows, stock(true), &s);
        assert!(n.unplaced.is_empty());
        let sheet = n
            .sheets
            .iter()
            .find(|sh| sh.placements.iter().any(|p| p.part == "door"))
            .unwrap();
        let at = |part: &str| {
            sheet
                .placements
                .iter()
                .find(|p| p.part == part)
                .unwrap()
                .rect
        };
        let (door, d1, d2) = (at("door"), at("drawer1"), at("drawer2"));
        assert_eq!((door.y, d1.y, d2.y), (door.y, door.y, door.y));
        assert!((d1.x - (door.x + 900.0 + s.kerf)).abs() < 1e-9);
        assert!((d2.x - (d1.x + 300.0 + s.kerf)).abs() < 1e-9);
        assert_valid(&n, &s);
        // A strip too long for the sheet is reported by its group.
        rows[1].cut_length = 2000.0;
        let n = nest(&rows, stock(true), &s);
        assert!(n.unplaced.iter().any(|u| u.contains("T01.grain1")));
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
            |_, _| Vec::new(),
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

    /// OUT-5d: of the sizes offered, the one using least sheet area.
    #[test]
    fn the_sheet_size_using_least_area_is_chosen() {
        let sizes = |_: &str, _: f64| {
            vec![
                Stock {
                    length: 2400.0,
                    width: 1200.0,
                    grained: false,
                },
                Stock {
                    length: 3600.0,
                    width: 1800.0,
                    grained: false,
                },
            ]
        };
        // Two 1700 × 1100 parts: a big sheet takes both, two small ones are smaller in all.
        let n = nest(
            &[row("p", 1700.0, 1100.0, 2, false)],
            sizes,
            &Settings::default(),
        );
        assert_eq!(n.sheets.len(), 2);
        assert_eq!(n.sheets[0].stock.length, 2400.0);
        // A part only the big sheet holds.
        let n = nest(
            &[row("p", 3000.0, 1500.0, 1, false)],
            sizes,
            &Settings::default(),
        );
        assert!(n.unplaced.is_empty());
        assert_eq!(n.sheets[0].stock.length, 3600.0);
    }

    /// OUT-5d: a kept offcut takes the parts it can before a new sheet is opened; afterwards it
    /// is gone from the store and the job's leftovers are in it.
    #[test]
    fn kept_offcuts_are_used_first_and_the_store_follows() {
        let offcut = Offcut {
            board: "white".into(),
            thickness: 16.0,
            length: 900.0,
            width: 600.0,
            grained: true,
        };
        let other = Offcut {
            thickness: 18.0,
            ..offcut.clone()
        };
        let stored = [offcut.clone(), other.clone()];
        let s = Settings::default();
        let n = nest_with_offcuts(
            &[row("shelf", 800.0, 500.0, 1, true)],
            stock(true),
            &stored,
            &s,
        );
        assert_eq!(n.sheets.len(), 1);
        assert!(n.sheets[0].from_offcut);
        assert_eq!(
            n.sheets[0].placements[0].rect.x, 0.0,
            "no trim on a cut edge"
        );
        let kept = kept_offcuts(&n, &stored);
        assert!(kept.contains(&other));
        assert!(!kept.contains(&offcut));
        // Too big for the offcut: a new sheet, the offcut stays.
        let n = nest_with_offcuts(
            &[row("side", 1000.0, 560.0, 1, true)],
            stock(true),
            &stored,
            &s,
        );
        assert!(n.sheets.iter().all(|s| !s.from_offcut));
        assert!(kept_offcuts(&n, &stored).contains(&offcut));
    }
}
