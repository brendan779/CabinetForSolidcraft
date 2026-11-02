//! The front elevation: the whole run drawn flat from the front, to scale, with its widths and
//! heights and any return walls. One page as SVG or PDF, and the same lines as DXF.

use std::fmt::Write;

use cabinet_core::part::{Part, PartList};
use cabinet_core::run::Walls;

use crate::dxf::Dxf;
use crate::pdf::{svg_to_ops, write_pdf};
use crate::{PAGE_H, PAGE_W, esc};

const MARGIN: f64 = 15.0;
const TOP: f64 = 34.0;
/// Room under the drawing for the cabinet and overall widths.
const BELOW: f64 = 30.0;
/// Room to the right for the heights.
const RIGHT: f64 = 24.0;
/// How wide a return wall is drawn (mm in the room).
const WALL: f64 = 100.0;
/// How far a wall is drawn above the cabinets: its whole height would shrink a base run.
const WALL_ABOVE: f64 = 200.0;

/// One item of the run, for its width dimension.
struct Item<'a> {
    id: &'a str,
    x0: f64,
    x1: f64,
}

/// What the page draws, in room millimetres: x along the run, z up.
struct Drawing<'a> {
    /// Back to front, so a front covers what is behind it.
    parts: Vec<&'a Part>,
    items: Vec<Item<'a>>,
    x0: f64,
    x1: f64,
    wall_top: f64,
}

/// An L-corner's forward leg stands out of the run's plane, and a tilted shelf is inside.
fn drawn(list: &PartList, part: &Part) -> bool {
    part.bounds.min.y >= -100.0 && !list.tilts.contains_key(&part.name)
}

fn drawing<'a>(lists: &'a [PartList], walls: Option<&Walls>) -> Option<Drawing<'a>> {
    let mut parts: Vec<&Part> = lists
        .iter()
        .flat_map(|l| l.parts.iter().filter(move |p| drawn(l, p)))
        .collect();
    if parts.is_empty() {
        return None;
    }
    parts.sort_by(|a, b| b.bounds.min.y.total_cmp(&a.bounds.min.y));
    let x0 = parts
        .iter()
        .map(|p| p.bounds.min.x)
        .fold(f64::MAX, f64::min);
    let x1 = parts
        .iter()
        .map(|p| p.bounds.max().x)
        .fold(f64::MIN, f64::max);
    let top = parts.iter().map(|p| p.bounds.max().z).fold(0.0, f64::max);
    // Items with a place of their own in the run: not a benchtop or moulding, which span it.
    let items = lists
        .iter()
        .filter(|l| {
            l.parts.iter().any(|p| drawn(l, p))
                && !l.parts.iter().all(|p| {
                    use cabinet_core::part::PartRole::*;
                    matches!(p.role, Benchtop | Cornice | LightRail | Splashback)
                })
        })
        .map(|l| {
            let ps = || l.parts.iter().filter(|p| drawn(l, p));
            Item {
                id: &l.cabinet,
                x0: ps().map(|p| p.bounds.min.x).fold(f64::MAX, f64::min),
                x1: ps().map(|p| p.bounds.max().x).fold(f64::MIN, f64::max),
            }
        })
        .collect();
    Some(Drawing {
        parts,
        items,
        x0,
        x1,
        wall_top: walls.map_or(top, |w| (top + WALL_ABOVE).min(w.height.max(top))),
    })
}

/// The elevation as one page of SVG; `None` with nothing in the run.
pub fn elevation_svg(lists: &[PartList], walls: Option<&Walls>, job: &str) -> Option<String> {
    let d = drawing(lists, walls)?;
    let (left, right) = (
        walls.is_some_and(|w| w.left),
        walls.is_some_and(|w| w.right),
    );
    let room_x0 = d.x0 - if left { WALL } else { 0.0 };
    let room_x1 = d.x1 + if right { WALL } else { 0.0 };
    let scale = ((PAGE_W - 2.0 * MARGIN - RIGHT) / (room_x1 - room_x0))
        .min((PAGE_H - TOP - BELOW - MARGIN) / d.wall_top.max(1.0));
    let floor = TOP + d.wall_top * scale;
    // Centred across the page, which a low wide run and a tall narrow one both leave room in.
    let spare = PAGE_W - 2.0 * MARGIN - RIGHT - (room_x1 - room_x0) * scale;
    let px = |x: f64| MARGIN + spare / 2.0 + (x - room_x0) * scale;
    let py = |z: f64| floor - z * scale;

    let mut svg = String::new();
    let _ = writeln!(
        svg,
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{PAGE_W}mm" height="{PAGE_H}mm" viewBox="0 0 {PAGE_W} {PAGE_H}" font-family="Helvetica, Arial, sans-serif">"##
    );
    let _ = writeln!(
        svg,
        r##"<text x="{MARGIN}" y="16" font-size="6" fill="#000">Front elevation</text>"##
    );
    let _ = writeln!(
        svg,
        r##"<text x="{:.2}" y="16" font-size="3" fill="#333" text-anchor="end">{}</text>"##,
        PAGE_W - MARGIN,
        esc(job)
    );
    let _ = writeln!(
        svg,
        r##"<text x="{MARGIN}" y="22" font-size="3" fill="#444">Scale 1:{:.0}. Millimetres. Seen from the front.</text>"##,
        (1.0 / scale).round()
    );
    for (on, wx0, wx1) in [(left, d.x0 - WALL, d.x0), (right, d.x1, d.x1 + WALL)] {
        if on {
            let _ = writeln!(
                svg,
                r##"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="#d9d6cf" stroke="#000" stroke-width="0.35"/>"##,
                px(wx0),
                py(d.wall_top),
                (wx1 - wx0) * scale,
                d.wall_top * scale
            );
        }
    }
    let _ = writeln!(
        svg,
        r##"<line x1="{:.2}" y1="{floor:.2}" x2="{:.2}" y2="{floor:.2}" stroke="#000" stroke-width="0.5"/>"##,
        px(room_x0),
        px(room_x1)
    );
    for p in &d.parts {
        let b = &p.bounds;
        let fill = if p.role.is_front() {
            "#f3ead9"
        } else {
            "#fafaf7"
        };
        let _ = writeln!(
            svg,
            r##"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="{fill}" stroke="#222" stroke-width="0.2"/>"##,
            px(b.min.x),
            py(b.max().z),
            b.size.x * scale,
            b.size.z * scale
        );
    }
    let dim = |svg: &mut String, x0: f64, x1: f64, y: f64, label: String| {
        let _ = writeln!(
            svg,
            r##"<line x1="{x0:.2}" y1="{y:.2}" x2="{x1:.2}" y2="{y:.2}" stroke="#555" stroke-width="0.2"/><line x1="{x0:.2}" y1="{:.2}" x2="{x0:.2}" y2="{:.2}" stroke="#555" stroke-width="0.2"/><line x1="{x1:.2}" y1="{:.2}" x2="{x1:.2}" y2="{:.2}" stroke="#555" stroke-width="0.2"/><text x="{:.2}" y="{:.2}" font-size="2.6" fill="#111" text-anchor="middle">{}</text>"##,
            y - 1.2,
            y + 1.2,
            y - 1.2,
            y + 1.2,
            (x0 + x1) / 2.0,
            y - 0.8,
            esc(&label)
        );
    };
    let y_items = floor + 8.0;
    for i in &d.items {
        let width = (i.x1 - i.x0).round();
        dim(&mut svg, px(i.x0), px(i.x1), y_items, format!("{width:.0}"));
        let _ = writeln!(
            svg,
            r##"<text x="{:.2}" y="{:.2}" font-size="2.6" fill="#333" text-anchor="middle">{}</text>"##,
            px((i.x0 + i.x1) / 2.0),
            y_items + 4.0,
            esc(i.id)
        );
    }
    let y_total = floor + 18.0;
    dim(
        &mut svg,
        px(d.x0),
        px(d.x1),
        y_total,
        format!("{:.0}", (d.x1 - d.x0).round()),
    );
    if let Some(w) = walls.filter(|w| w.left && w.right && w.length > 0.0) {
        let label = format!("{:.0} between the walls", w.length);
        dim(
            &mut svg,
            px(d.x0),
            px(d.x0 + w.length),
            y_total + 7.0,
            label,
        );
    }
    // The heights, up the right: the floor to each distinct top of the run's items.
    let mut tops: Vec<f64> = lists
        .iter()
        .flat_map(|l| l.parts.iter().filter(move |p| drawn(l, p)))
        .map(|p| p.bounds.max().z.round())
        .collect();
    tops.sort_by(f64::total_cmp);
    tops.dedup();
    let x_h = px(room_x1) + 7.0;
    let tallest = tops.last().copied().unwrap_or(0.0);
    let _ = writeln!(
        svg,
        r##"<line x1="{x_h:.2}" y1="{floor:.2}" x2="{x_h:.2}" y2="{:.2}" stroke="#555" stroke-width="0.2"/><text x="{:.2}" y="{:.2}" font-size="2.6" fill="#111">{tallest:.0}</text>"##,
        py(tallest),
        x_h + 1.5,
        py(tallest) + 1.0,
    );
    svg.push_str("</svg>\n");
    Some(svg)
}

/// The elevation as a one-page PDF.
pub fn elevation_pdf(lists: &[PartList], walls: Option<&Walls>, job: &str) -> Option<Vec<u8>> {
    let page = elevation_svg(lists, walls, job)?;
    Some(write_pdf(&[svg_to_ops(&page)]))
}

/// The elevation's lines as DXF at full size, x along the run and y up from the floor: layers
/// `FRONTS`, `CARCASS` and `WALLS`. Everything is drawn; a CAD program hides nothing here.
pub fn elevation_dxf(lists: &[PartList], walls: Option<&Walls>) -> Option<String> {
    let d = drawing(lists, walls)?;
    let mut dxf = Dxf::new();
    for p in &d.parts {
        let b = &p.bounds;
        let layer = if p.role.is_front() {
            "FRONTS"
        } else {
            "CARCASS"
        };
        dxf.rect(layer, b.min.x, b.min.z, b.size.x, b.size.z);
    }
    if walls.is_some_and(|w| w.left) {
        dxf.rect("WALLS", d.x0 - WALL, 0.0, WALL, d.wall_top);
    }
    if walls.is_some_and(|w| w.right) {
        dxf.rect("WALLS", d.x1, 0.0, WALL, d.wall_top);
    }
    Some(dxf.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cabinet_core::run::{Run, RunItem, build_run};
    use cabinet_core::{Cabinet, CabinetKind};

    fn lists(with_walls: Option<Walls>) -> (Vec<PartList>, Option<Walls>) {
        let mut items = vec![
            RunItem::Cabinet(Box::new(Cabinet::new("B1", CabinetKind::Base))),
            RunItem::Cabinet(Box::new(Cabinet::new("D1", CabinetKind::DrawerBank))),
        ];
        if let Some(w) = with_walls.clone() {
            items.push(RunItem::Walls(w));
        }
        let run = Run {
            origin: Default::default(),
            items,
        };
        (build_run(&run).unwrap(), with_walls)
    }

    fn walls() -> Walls {
        Walls {
            id: "W1".into(),
            left: true,
            right: true,
            length: 1300.0,
            height: 2400.0,
        }
    }

    #[test]
    fn the_elevation_gives_each_cabinets_width_and_the_runs() {
        let (l, _) = lists(None);
        let svg = elevation_svg(&l, None, "Job").unwrap();
        assert!(svg.contains(">B1<") && svg.contains(">D1<"));
        let width = Cabinet::new("B1", CabinetKind::Base).width;
        assert!(svg.contains(&format!(">{width:.0}<")));
        assert!(svg.contains(&format!(">{:.0}<", 2.0 * width)));
    }

    #[test]
    fn return_walls_and_the_gap_between_them_are_drawn() {
        let (l, w) = lists(Some(walls()));
        let svg = elevation_svg(&l, w.as_ref(), "Job").unwrap();
        assert!(svg.contains("1300 between the walls"));
        assert_eq!(svg.matches(r##"fill="#d9d6cf""##).count(), 2);
    }

    #[test]
    fn fronts_are_drawn_after_what_is_behind_them() {
        let (l, _) = lists(None);
        let d = drawing(&l, None).unwrap();
        let first_front = d.parts.iter().position(|p| p.role.is_front()).unwrap();
        assert!(
            d.parts[first_front..]
                .iter()
                .all(|p| p.role.is_front() || p.bounds.min.y <= d.parts[first_front].bounds.min.y)
        );
    }

    #[test]
    fn the_elevation_dxf_puts_fronts_and_walls_on_their_layers() {
        let (l, w) = lists(Some(walls()));
        let dxf = elevation_dxf(&l, w.as_ref()).unwrap();
        for layer in ["FRONTS", "CARCASS", "WALLS"] {
            assert!(dxf.contains(layer), "{layer}");
        }
        assert!(dxf.ends_with("EOF\n"));
    }

    #[test]
    fn nothing_in_the_run_draws_nothing() {
        assert!(elevation_svg(&[], None, "Job").is_none());
        assert!(elevation_dxf(&[], None).is_none());
    }
}
