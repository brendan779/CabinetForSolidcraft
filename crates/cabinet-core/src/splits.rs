//! Moving the splits between a cabinet's sections (SEC-6): a line between two columns, or between
//! two rows of a column. Only the sections either side of a line change; the others keep their
//! sizes however they were set, so a move fixes them and leaves one section to take up the rest.

use crate::build::share;
use crate::build_cabinet;
use crate::model::{Cabinet, FrontLayout, Section, SectionFront};

/// The smallest a section can be dragged to: room for a front and its fixings.
pub const MIN_SECTION: f64 = 60.0;

/// The sections' sizes as the builder resolves them, in the fronts' own zone.
#[derive(Clone, Debug, PartialEq)]
pub struct SplitLayout {
    pub width_total: f64,
    pub height_total: f64,
    pub gap: f64,
    /// The gap between rows, wider than `gap` on sharknose fronts.
    pub row_gap: f64,
    pub widths: Vec<f64>,
    /// Per column, bottom to top.
    pub heights: Vec<Vec<f64>>,
}

fn row_asks(rows: &[Section], t: f64, g: f64) -> Vec<f64> {
    rows.iter()
        .map(|r| match r.front {
            // A niche's height is its clear opening, as the builder reads it.
            SectionFront::Niche if r.height > 0.0 => r.height + t - g,
            _ => r.height,
        })
        .collect()
}

pub fn layout(cab: &Cabinet) -> Option<SplitLayout> {
    let FrontLayout::Sections { columns } = &cab.fronts else {
        return None;
    };
    let (width_total, height_total) = build_cabinet(cab).ok()?.section_zone?;
    let (g, rg, t) = (
        cab.standard.front_gap,
        cab.standard.row_gap(),
        cab.standard.carcass_thickness,
    );
    let asks: Vec<f64> = columns.iter().map(|c| c.width).collect();
    Some(SplitLayout {
        width_total,
        height_total,
        gap: g,
        row_gap: rg,
        widths: share(width_total, &asks, g)?,
        heights: columns
            .iter()
            .map(|c| share(height_total, &row_asks(&c.rows, t, rg), rg))
            .collect::<Option<_>>()?,
    })
}

/// Where the line above or right of section `line` is: its distance from the zone's start.
pub fn line_position(sizes: &[f64], line: usize, gap: f64) -> f64 {
    sizes[..=line].iter().sum::<f64>() + gap * line as f64
}

/// Where a line snaps, ascending: where it is now, halfway between its neighbours (equal spaces
/// either side), making a section match the one beyond it, the whole axis shared evenly, and the
/// matching lines of other columns (`others`).
pub fn snaps(sizes: &[f64], line: usize, gap: f64, others: &[f64]) -> Vec<f64> {
    if line + 1 >= sizes.len() {
        return Vec::new();
    }
    let (a, b) = (sizes[line], sizes[line + 1]);
    let start = line_position(sizes, line, gap) - a;
    let mut out = vec![start + a, start + (a + b) / 2.0];
    if line > 0 {
        out.push(start + sizes[line - 1]);
    }
    if let Some(next) = sizes.get(line + 2) {
        out.push(start + a + b - next);
    }
    let n = sizes.len() as f64;
    let total: f64 = sizes.iter().sum();
    out.push((line + 1) as f64 * total / n + gap * line as f64);
    out.extend_from_slice(others);
    let (lo, hi) = (start + MIN_SECTION, start + a + b - MIN_SECTION);
    out.retain(|p| (lo..=hi).contains(p));
    out.sort_by(f64::total_cmp);
    out.dedup_by(|p, q| (*p - *q).abs() < 0.5);
    out
}

/// Sizes with `line` moved to `position`, kept off its neighbours.
fn moved(sizes: &[f64], line: usize, position: f64, gap: f64) -> Vec<f64> {
    let mut out = sizes.to_vec();
    let (a, b) = (sizes[line], sizes[line + 1]);
    let start = line_position(sizes, line, gap) - a;
    let wanted = (position - start).clamp(MIN_SECTION, (a + b - MIN_SECTION).max(MIN_SECTION));
    out[line] = wanted;
    out[line + 1] = a + b - wanted;
    out
}

/// Moves a line: between columns with `column` None, else between rows `line` and `line + 1` of
/// that column. False when there is no such line.
pub fn move_split(cab: &mut Cabinet, column: Option<usize>, line: usize, position: f64) -> bool {
    let Some(l) = layout(cab) else {
        return false;
    };
    let (g, rg, t) = (
        cab.standard.front_gap,
        cab.standard.row_gap(),
        cab.standard.carcass_thickness,
    );
    let FrontLayout::Sections { columns } = &mut cab.fronts else {
        return false;
    };
    match column {
        None => {
            if line + 1 >= columns.len() {
                return false;
            }
            let sizes = moved(&l.widths, line, position, g);
            for (i, (c, size)) in columns.iter_mut().zip(sizes).enumerate() {
                c.width = if i == line + 1 { 0.0 } else { size };
            }
        }
        Some(ci) => {
            let Some(col) = columns.get_mut(ci) else {
                return false;
            };
            if line + 1 >= col.rows.len() {
                return false;
            }
            let sizes = moved(&l.heights[ci], line, position, rg);
            for (i, (r, size)) in col.rows.iter_mut().zip(sizes).enumerate() {
                r.height = match r.front {
                    _ if i == line + 1 => 0.0,
                    SectionFront::Niche => size - t + rg,
                    _ => size,
                };
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{CabinetKind, SectionColumn, SectionInside};

    fn row() -> Section {
        Section {
            height: 0.0,
            front: SectionFront::drawers(1),
            inside: SectionInside::Empty,
            shelf_above: false,
            opening: None,
        }
    }

    fn cabinet(rows: usize, columns: usize) -> Cabinet {
        let mut cab = Cabinet::new("B01", CabinetKind::Base);
        cab.fronts = FrontLayout::Sections {
            columns: (0..columns)
                .map(|_| SectionColumn {
                    width: 0.0,
                    rows: vec![row(); rows],
                })
                .collect(),
        };
        cab
    }

    #[test]
    fn moving_a_row_split_leaves_the_other_rows_where_they_were() {
        let mut cab = cabinet(4, 1);
        let before = layout(&cab).unwrap().heights[0].clone();
        let at = line_position(&before, 1, cab.standard.row_gap()) + 50.0;
        assert!(move_split(&mut cab, Some(0), 1, at));
        let after = layout(&cab).unwrap().heights[0].clone();
        assert!((after[0] - before[0]).abs() < 1e-6);
        assert!((after[1] - (before[1] + 50.0)).abs() < 1e-6);
        assert!((after[2] - (before[2] - 50.0)).abs() < 1e-6);
        assert!((after[3] - before[3]).abs() < 1e-6);
    }

    #[test]
    fn moving_a_column_split_keeps_the_fronts_zone_filled() {
        let mut cab = cabinet(1, 3);
        cab.width = 1200.0;
        let before = layout(&cab).unwrap();
        let g = before.gap;
        assert!(move_split(&mut cab, None, 0, before.widths[0] + 100.0));
        let after = layout(&cab).unwrap();
        assert!((after.widths[0] - (before.widths[0] + 100.0)).abs() < 1e-6);
        assert!((after.widths[2] - before.widths[2]).abs() < 1e-6);
        let filled = after.widths.iter().sum::<f64>() + 2.0 * g;
        assert!((filled - after.width_total).abs() < 1e-6);
    }

    #[test]
    fn a_split_stops_short_of_its_neighbours() {
        let mut cab = cabinet(2, 1);
        assert!(move_split(&mut cab, Some(0), 0, 1.0));
        assert_eq!(layout(&cab).unwrap().heights[0][0], MIN_SECTION);
        assert!(
            !move_split(&mut cab, Some(0), 1, 100.0),
            "no line above the top row"
        );
    }

    #[test]
    fn a_niche_keeps_its_clear_height_meaning_when_a_split_moves() {
        let mut cab = cabinet(2, 1);
        let FrontLayout::Sections { columns } = &mut cab.fronts else {
            unreachable!()
        };
        columns[0].rows[0].front = SectionFront::Niche;
        assert!(move_split(&mut cab, Some(0), 0, 300.0));
        assert!((layout(&cab).unwrap().heights[0][0] - 300.0).abs() < 1e-6);
    }

    #[test]
    fn a_split_snaps_to_where_it_was_halfway_and_matching_sizes() {
        let sizes = [100.0, 300.0, 200.0, 150.0];
        let got = snaps(&sizes, 1, 0.0, &[250.0]);
        for wanted in [
            400.0, // where it is
            350.0, // halfway between its neighbours
            200.0, // the section under it matches the one below that
            450.0, // the section over it matches the one above that
            250.0, // another column's line
        ] {
            assert!(got.contains(&wanted), "{wanted} in {got:?}");
        }
        // Shared evenly: two of four sections of 187.5.
        assert!(got.contains(&375.0), "{got:?}");
    }

    #[test]
    fn snaps_stay_inside_the_room_a_line_has() {
        let got = snaps(&[100.0, 100.0], 0, 3.0, &[10.0, 190.0]);
        assert_eq!(got, vec![100.0]);
    }
}
