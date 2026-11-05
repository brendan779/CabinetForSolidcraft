//! Benchtop schedule for the benchtop maker: a summary of every piece (size, area, finished edge,
//! cut-outs), then each piece's dimensioned sheet with its cut-outs and tap holes.

use std::fmt::Write;

use cabinet_core::part::{Operation, Part, PartList, PartRole};

use crate::pdf::{svg_to_ops, write_pdf};
use crate::{PAGE_H, PAGE_W, esc, part_sheet};

const MARGIN: f64 = 15.0;
const ROWS: usize = 30;

/// The benchtop and splashback pieces of the job, with the list each belongs to.
fn pieces(lists: &[PartList]) -> Vec<(&PartList, &Part)> {
    lists
        .iter()
        .flat_map(|l| l.parts.iter().map(move |p| (l, p)))
        .filter(|(_, p)| matches!(p.role, PartRole::Benchtop | PartRole::Splashback))
        .collect()
}

/// One summary line per piece.
fn line(list: &PartList, p: &Part) -> String {
    let (l, w) = p.finished_size();
    let e = p.edges;
    // The finished (polished or taped) edges: those the part marks, at their lengths.
    let [a, b] = p.thickness_axis.others();
    let along = |axis| p.bounds.size.get(axis) / 1000.0;
    let mut metres = 0.0;
    for (ends, along_axis) in [(a, b), (b, a)] {
        let (low, high) = e.on_axis(ends);
        metres += (u8::from(low) + u8::from(high)) as f64 * along(along_axis);
    }
    let cutouts: Vec<String> = list
        .operations
        .iter()
        .filter_map(|o| match o {
            Operation::Chamfer { part, .. } if *part == p.name => {
                Some("45 degree mitre; dimensions to long points".into())
            }
            Operation::EdgeFinish {
                part, size, bevel, ..
            } if *part == p.name => Some(if *bevel {
                format!("upper edge bevel {size} mm at 45 degrees")
            } else {
                format!("upper and lower edges R{size}")
            }),
            Operation::Cutout {
                part,
                bounds,
                radius,
                note,
            } if *part == p.name => Some(format!(
                "{} {:.0} × {:.0}{}",
                if note.is_empty() { "cut-out" } else { note },
                bounds.size.get(a),
                bounds.size.get(b),
                if *radius > 0.0 {
                    format!(" R{radius:.0}")
                } else {
                    String::new()
                }
            )),
            _ => None,
        })
        .collect();
    let rounds: Vec<f64> = list
        .operations
        .iter()
        .filter_map(|o| match o {
            Operation::Round { part, radius, .. } if *part == p.name => Some(*radius),
            _ => None,
        })
        .collect();
    format!(
        "{}  {}  {:.0} × {:.0} × {:.0} mm  {:.2} m²  {:.2} m finished edge  {}{}{}",
        p.name,
        p.role.label(),
        l,
        w,
        p.thickness(),
        l * w / 1e6,
        metres,
        if cutouts.is_empty() {
            String::new()
        } else {
            format!("{}  ", cutouts.join("; "))
        },
        match rounds.first() {
            None => String::new(),
            Some(r) => format!("{} corners R{r:.0}  ", rounds.len()),
        },
        if p.board.is_empty() {
            "generic".to_string()
        } else {
            p.board.clone()
        }
    )
}

/// Every benchtop piece: summary pages, then a sheet per piece. Empty when the job has none.
pub fn benchtops_pdf(lists: &[PartList], job: &str) -> Vec<u8> {
    let pieces = pieces(lists);
    let mut lines: Vec<String> = pieces.iter().map(|(l, p)| line(l, p)).collect();
    // The maker's notes (the edge profile), once per benchtop.
    for l in lists
        .iter()
        .filter(|l| pieces.iter().any(|(pl, _)| pl.cabinet == l.cabinet))
    {
        lines.extend(l.notes.iter().map(|n| format!("{}: {n}", l.cabinet)));
        lines.extend(
            l.warnings
                .iter()
                .map(|n| format!("{} warning: {n}", l.cabinet)),
        );
    }
    let total: f64 = pieces
        .iter()
        .map(|(_, p)| {
            let (l, w) = p.finished_size();
            l * w / 1e6
        })
        .sum();
    let mut pages = Vec::new();
    let chunks: Vec<&[String]> = if lines.is_empty() {
        vec![&[]]
    } else {
        lines.chunks(ROWS).collect()
    };
    for (i, chunk) in chunks.iter().enumerate() {
        let mut svg = String::new();
        let _ = writeln!(
            svg,
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="{PAGE_W}mm" height="{PAGE_H}mm" viewBox="0 0 {PAGE_W} {PAGE_H}" font-family="Helvetica, Arial, sans-serif">"##
        );
        let _ = writeln!(
            svg,
            r##"<text x="{MARGIN:.2}" y="20.00" font-size="6">Benchtops{}</text>"##,
            if chunks.len() > 1 {
                format!(" {} of {}", i + 1, chunks.len())
            } else {
                String::new()
            }
        );
        let _ = writeln!(
            svg,
            r##"<text x="{:.2}" y="20.00" font-size="3" fill="#555" text-anchor="end">{}</text>"##,
            PAGE_W - MARGIN,
            esc(job)
        );
        for (j, l) in chunk.iter().enumerate() {
            let _ = writeln!(
                svg,
                r##"<text x="{MARGIN:.2}" y="{:.2}" font-size="3" fill="#333">{}</text>"##,
                32.0 + j as f64 * 5.0,
                esc(l)
            );
        }
        if i + 1 == chunks.len() {
            let _ = writeln!(
                svg,
                r##"<text x="{MARGIN:.2}" y="{:.2}" font-size="3.4">{} pieces, {total:.2} m² in all. Each piece's sheet follows: cut-outs and tap holes from its reference corner.</text>"##,
                PAGE_H - 12.0,
                pieces.len()
            );
        }
        svg.push_str("</svg>\n");
        pages.push(svg);
    }
    for (l, p) in &pieces {
        let board = if p.board.is_empty() {
            "Generic".to_string()
        } else {
            p.board.clone()
        };
        pages.push(part_sheet(l, p, &board, job));
    }
    write_pdf(&pages.iter().map(|svg| svg_to_ops(svg)).collect::<Vec<_>>())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cabinet_core::run::{Benchtop, Cutout, CutoutKind, Run, RunItem, build_run};
    use cabinet_core::{Cabinet, CabinetKind};

    #[test]
    fn a_thickness_profile_is_scheduled_without_rounding_the_dxf_outline() {
        use cabinet_core::geom::{Aabb, Axis, Vec3};
        let p = Part {
            name: "BT1.Benchtop1".into(),
            role: PartRole::Benchtop,
            bounds: Aabb::new(Vec3::default(), Vec3::new(900.0, 600.0, 33.0)),
            thickness_axis: Axis::Z,
            grain_axis: Some(Axis::X),
            board: String::new(),
            edges: Default::default(),
        };
        let list = PartList {
            cabinet: "BT1".into(),
            parts: vec![p.clone()],
            operations: vec![Operation::EdgeFinish {
                part: p.name.clone(),
                edges: vec![Vec3::new(450.0, 0.0, 33.0)],
                size: 2.0,
                bevel: false,
            }],
            ..PartList::default()
        };
        assert!(line(&list, &p).contains("upper and lower edges R2"));
        assert!(
            !crate::dxf::part_dxfs(&list, &p)[0]
                .contents
                .contains("0.414213562")
        );
    }

    #[test]
    fn the_schedule_counts_each_finished_edge_in_the_parts_own_plane() {
        use cabinet_core::geom::{Aabb, Axis, Vec3};
        use cabinet_core::part::EdgeBanding;
        for (axis, size) in [
            (Axis::Z, Vec3::new(1200.0, 600.0, 20.0)),
            (Axis::Y, Vec3::new(1200.0, 20.0, 600.0)),
            (Axis::X, Vec3::new(20.0, 600.0, 1200.0)),
        ] {
            let part = Part {
                name: "BT1.Piece".into(),
                role: PartRole::Benchtop,
                bounds: Aabb::new(Vec3::default(), size),
                thickness_axis: axis,
                grain_axis: None,
                board: String::new(),
                edges: EdgeBanding::all(),
            };
            let summary = line(&PartList::default(), &part);
            assert!(
                summary.contains("3.60 m finished edge"),
                "{axis:?}: {summary}"
            );
        }
    }

    #[test]
    fn splashback_openings_are_scheduled_by_width_and_height_not_slab_thickness() {
        use cabinet_core::geom::{Aabb, Axis, Vec3};
        let part = Part {
            name: "BT1.Splashback1".into(),
            role: PartRole::Splashback,
            bounds: Aabb::new(Vec3::default(), Vec3::new(1200.0, 20.0, 600.0)),
            thickness_axis: Axis::Y,
            grain_axis: None,
            board: String::new(),
            edges: Default::default(),
        };
        let list = PartList {
            operations: vec![Operation::Cutout {
                part: part.name.clone(),
                bounds: Aabb::new(Vec3::default(), Vec3::new(500.0, 20.0, 300.0)),
                radius: 0.0,
                note: "niche".into(),
            }],
            ..PartList::default()
        };
        let summary = line(&list, &part);
        assert!(summary.contains("niche 500 × 300"), "{summary}");
    }

    #[test]
    fn the_schedule_has_a_summary_and_a_sheet_per_piece() {
        let run = Run {
            origin: Default::default(),
            items: vec![
                RunItem::Cabinet(Box::new(Cabinet::new("B01", CabinetKind::Base))),
                RunItem::Cabinet(Box::new(Cabinet::new("S01", CabinetKind::Sink))),
                RunItem::Benchtop(Benchtop {
                    build_up: Default::default(),
                    id: "BT1".into(),
                    thickness: 33.0,
                    front_overhang: 20.0,
                    side_overhang: 0.0,
                    back_clearance: 0.0,
                    board: String::new(),
                    edge: Default::default(),
                    cutouts: vec![Cutout::new("S01", CutoutKind::Sink, 760.0, 440.0)],
                    back_overhang: 0.0,
                    waterfall_left: false,
                    waterfall_right: false,
                    splashback: 600.0,
                    splashback_thickness: 20.0,
                    niches: Vec::new(),
                    power_points: Vec::new(),
                    end_radius: 100.0,
                    material: "laminate".into(),
                    material_rules: Default::default(),
                }),
            ],
        };
        let lists = build_run(&run).unwrap();
        let p = pieces(&lists);
        assert_eq!(p.len(), 2);
        let summary = line(p[0].0, p[0].1);
        assert!(
            summary.contains("sink, undermount: edge finished 760 × 440"),
            "{summary}"
        );
        // BENCH-3e: both front corners rounded, on the summary, the sheet and the DXF outline.
        assert!(summary.contains("2 corners R100"), "{summary}");
        let sheet = part_sheet(p[0].0, p[0].1, "Generic", "Job");
        assert!(sheet.contains("corner R100"));
        let dxf = &crate::dxf::part_dxfs(p[0].0, p[0].1)[0].contents;
        assert_eq!(dxf.matches("\n42\n0.414213562\n").count(), 2, "{dxf}");
        let pdf = benchtops_pdf(&lists, "Job");
        assert_eq!(
            String::from_utf8_lossy(&pdf)
                .matches("/Type /Page ")
                .count(),
            3
        );
    }
}
