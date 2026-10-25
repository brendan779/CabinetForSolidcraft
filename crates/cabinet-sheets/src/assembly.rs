//! Assembly sheets: each cabinet drawn exploded in isometric from the front right, every panel
//! moved out from the cabinet's centre (fronts further, towards the viewer) and labelled, beside a
//! list of its parts. One page per cabinet.

use std::fmt::Write;

use cabinet_core::geom::{Aabb, Vec3};
use cabinet_core::part::{Part, PartList};

use crate::pdf::{svg_to_ops, write_pdf};
use crate::{PAGE_H, PAGE_W, esc};

const MARGIN: f64 = 15.0;
/// The drawing's box on the page; the parts list sits to its right.
const DRAW_W: f64 = 190.0;
const DRAW_H: f64 = 160.0;
const TOP: f64 = 30.0;
/// How far each panel moves out from the cabinet's centre, as a share of its distance from it.
const EXPLODE: f64 = 0.45;
/// Fronts also come this far forward (mm), clear of the carcass.
const FRONT_PULL: f64 = 250.0;
/// Parts listed per page beside the drawing.
const LIST_ROWS: usize = 34;

/// A point in the isometric view from the front right and above: x to the right and back up,
/// y (depth) to the left and back up, z straight up. Screen v grows downward.
fn iso(p: Vec3) -> (f64, f64) {
    let (c, s) = (30f64.to_radians().cos(), 30f64.to_radians().sin());
    (p.x * c + p.y * c, -(p.z + p.y * s - p.x * s))
}

/// Each part's bounds, moved out for the exploded view.
pub fn exploded(list: &PartList) -> Vec<(&Part, Aabb)> {
    let Some(all) = list
        .parts
        .iter()
        .map(|p| p.bounds)
        .reduce(|a, b| a.union(&b))
    else {
        return Vec::new();
    };
    let c = all.center();
    list.parts
        .iter()
        .map(|p| {
            let pc = p.bounds.center();
            let mut d = Vec3::new(
                (pc.x - c.x) * EXPLODE,
                (pc.y - c.y) * EXPLODE,
                (pc.z - c.z) * EXPLODE,
            );
            if p.role.is_front() {
                d.y -= FRONT_PULL;
            }
            let mut b = p.bounds;
            b.min = b.min + d;
            (p, b)
        })
        .collect()
}

/// The three faces seen from the viewer (front, right side, top) as screen polygons.
fn faces(b: &Aabb) -> [[(f64, f64); 4]; 3] {
    let (lo, hi) = (b.min, b.max());
    let p = |x, y, z| iso(Vec3::new(x, y, z));
    [
        // Front (Y = min).
        [
            p(lo.x, lo.y, lo.z),
            p(hi.x, lo.y, lo.z),
            p(hi.x, lo.y, hi.z),
            p(lo.x, lo.y, hi.z),
        ],
        // Right (X = max).
        [
            p(hi.x, lo.y, lo.z),
            p(hi.x, hi.y, lo.z),
            p(hi.x, hi.y, hi.z),
            p(hi.x, lo.y, hi.z),
        ],
        // Top (Z = max).
        [
            p(lo.x, lo.y, hi.z),
            p(hi.x, lo.y, hi.z),
            p(hi.x, hi.y, hi.z),
            p(lo.x, hi.y, hi.z),
        ],
    ]
}

/// The short label of a part: its name without the cabinet id (`Door1` for `B01.Door1`).
fn short(name: &str) -> &str {
    name.split_once('.').map_or(name, |(_, n)| n)
}

/// One cabinet's assembly page(s): the exploded drawing and its parts list (continued on more
/// pages when long).
pub fn assembly_pages(list: &PartList, job: &str) -> Vec<String> {
    let parts = exploded(list);
    let corners: Vec<(f64, f64)> = parts
        .iter()
        .flat_map(|(_, b)| faces(b).into_iter().flatten())
        .collect();
    let (u0, u1) = corners.iter().fold((f64::MAX, f64::MIN), |(a, b), (u, _)| {
        (a.min(*u), b.max(*u))
    });
    let (v0, v1) = corners.iter().fold((f64::MAX, f64::MIN), |(a, b), (_, v)| {
        (a.min(*v), b.max(*v))
    });
    let scale = (DRAW_W / (u1 - u0).max(1.0)).min(DRAW_H / (v1 - v0).max(1.0));
    let to_page = |(u, v): (f64, f64)| (MARGIN + (u - u0) * scale, TOP + (v - v0) * scale);
    // Farthest first: away from the viewer is +y, −x, −z.
    let mut order: Vec<&(&Part, Aabb)> = parts.iter().collect();
    let depth = |b: &Aabb| {
        let c = b.center();
        c.y - c.x - c.z
    };
    order.sort_by(|a, b| depth(&b.1).total_cmp(&depth(&a.1)));

    let mut drawing = String::new();
    for (p, b) in &order {
        let fill = if p.role.is_front() {
            ["#e9dcc6", "#d8c7aa", "#f3ead9"]
        } else {
            ["#efece4", "#dcd8cc", "#f8f6f1"]
        };
        for (face, colour) in faces(b).iter().zip(fill) {
            let pts: Vec<String> = face
                .iter()
                .map(|&q| {
                    let (x, y) = to_page(q);
                    format!("{x:.2},{y:.2}")
                })
                .collect();
            let _ = writeln!(
                drawing,
                r##"<polygon points="{}" fill="{colour}" stroke="#333" stroke-width="0.15"/>"##,
                pts.join(" ")
            );
        }
    }
    for (p, b) in &order {
        let (x, y) = to_page(iso(b.center()));
        let _ = writeln!(
            drawing,
            r##"<text x="{x:.2}" y="{:.2}" font-size="2.2" fill="#1d3f73" text-anchor="middle">{}</text>"##,
            y + 0.8,
            esc(short(&p.name))
        );
    }

    let rows: Vec<String> = list
        .parts
        .iter()
        .map(|p| {
            let (l, w) = p.finished_size();
            format!(
                "{}  {}  {:.0} × {:.0} × {:.0}",
                short(&p.name),
                p.role.label(),
                l,
                w,
                p.thickness()
            )
        })
        .collect();
    let chunks: Vec<&[String]> = if rows.is_empty() {
        vec![&[]]
    } else {
        rows.chunks(LIST_ROWS).collect()
    };
    let n = chunks.len();
    chunks
        .iter()
        .enumerate()
        .map(|(i, chunk)| {
            let mut svg = String::new();
            let _ = writeln!(
                svg,
                r##"<svg xmlns="http://www.w3.org/2000/svg" width="{PAGE_W}mm" height="{PAGE_H}mm" viewBox="0 0 {PAGE_W} {PAGE_H}" font-family="Helvetica, Arial, sans-serif">"##
            );
            let _ = writeln!(
                svg,
                r##"<text x="{MARGIN:.2}" y="18.00" font-size="6">Assembly: {}{}</text>"##,
                esc(&list.cabinet),
                if n > 1 {
                    format!(" ({} of {n})", i + 1)
                } else {
                    String::new()
                }
            );
            let _ = writeln!(
                svg,
                r##"<text x="{:.2}" y="18.00" font-size="3" fill="#555" text-anchor="end">{}</text>"##,
                PAGE_W - MARGIN,
                esc(job)
            );
            svg.push_str(&drawing);
            let x = MARGIN + DRAW_W + 8.0;
            let _ = writeln!(
                svg,
                r##"<text x="{x:.2}" y="{TOP:.2}" font-size="3.4">Parts (finished L × W × T, mm)</text>"##
            );
            for (j, r) in chunk.iter().enumerate() {
                let _ = writeln!(
                    svg,
                    r##"<text x="{x:.2}" y="{:.2}" font-size="2.6" fill="#333">{}</text>"##,
                    TOP + 6.0 + j as f64 * 4.2,
                    esc(r)
                );
            }
            let _ = writeln!(
                svg,
                r##"<text x="{MARGIN:.2}" y="{:.2}" font-size="2.6" fill="#555">Exploded from the cabinet's centre, seen from the front right; fronts pulled forward.</text>"##,
                PAGE_H - 8.0
            );
            svg.push_str("</svg>\n");
            svg
        })
        .collect()
}

/// Every cabinet's assembly page(s) in one PDF.
pub fn assembly_pdf(lists: &[PartList], job: &str) -> Vec<u8> {
    let pages: Vec<String> = lists
        .iter()
        .filter(|l| !l.parts.is_empty())
        .flat_map(|l| assembly_pages(l, job))
        .collect();
    write_pdf(&pages.iter().map(|svg| svg_to_ops(svg)).collect::<Vec<_>>())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cabinet_core::{Cabinet, CabinetKind, build_cabinet};

    #[test]
    fn exploding_moves_panels_apart_and_fronts_forward() {
        let list = build_cabinet(&Cabinet::new("B01", CabinetKind::Base)).unwrap();
        let ex = exploded(&list);
        let get = |n: &str| ex.iter().find(|(p, _)| p.name == n).unwrap().1;
        let (l, r) = (get("B01.LeftSide"), get("B01.RightSide"));
        let (l0, r0) = (
            list.part("B01.LeftSide").unwrap().bounds,
            list.part("B01.RightSide").unwrap().bounds,
        );
        assert!(r.min.x - l.max().x > r0.min.x - l0.max().x);
        let door = get("B01.Door1");
        assert!(door.min.y < list.part("B01.Door1").unwrap().bounds.min.y - FRONT_PULL + 1.0);
    }

    #[test]
    fn an_assembly_page_draws_three_faces_and_a_label_per_part() {
        let list = build_cabinet(&Cabinet::new("B01", CabinetKind::Base)).unwrap();
        let pages = assembly_pages(&list, "Job");
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].matches("<polygon").count(), 3 * list.parts.len());
        // Every point is on the page.
        for pts in pages[0].split("points=\"").skip(1) {
            for xy in pts.split('"').next().unwrap().split(' ') {
                let (x, y) = xy.split_once(',').unwrap();
                let (x, y): (f64, f64) = (x.parse().unwrap(), y.parse().unwrap());
                assert!((0.0..=PAGE_W).contains(&x) && (0.0..=PAGE_H).contains(&y));
            }
        }
    }

    #[test]
    fn the_pdf_has_a_page_per_cabinet() {
        let lists = vec![
            build_cabinet(&Cabinet::new("B01", CabinetKind::Base)).unwrap(),
            build_cabinet(&Cabinet::new("D01", CabinetKind::DrawerBank)).unwrap(),
        ];
        let pdf = assembly_pdf(&lists, "Job");
        assert_eq!(
            String::from_utf8_lossy(&pdf)
                .matches("/Type /Page ")
                .count(),
            2
        );
        // For looking at: ASSEMBLY_PDF_OUT=target/assembly.pdf cargo test -p cabinet-sheets.
        if let Ok(out) = std::env::var("ASSEMBLY_PDF_OUT") {
            std::fs::write(out, &pdf).unwrap();
        }
    }
}
