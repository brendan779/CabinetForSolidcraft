//! Cut sheets: one A4-landscape SVG per part, showing the part's main face with every operation
//! dimensioned from the reference corner (bottom-left as drawn), edge tape marked and a title
//! block. PDF output and multi-part sheets are BACKLOG items.

use std::fmt::Write;

use cabinet_core::geom::{Axis, Vec3};
use cabinet_core::part::{BorePurpose, Operation, Part, PartList};

/// A4 landscape in mm; the SVG uses mm units.
const PAGE_W: f64 = 297.0;
const PAGE_H: f64 = 210.0;
const MARGIN: f64 = 12.0;
const TITLE_H: f64 = 30.0;

/// Something machined on the part, in the drawing's 2D frame (u across, v up, from the
/// reference corner), with the face it is on.
#[derive(Debug, Clone, PartialEq)]
pub enum Feature {
    /// A bore into the drawn face.
    FaceBore {
        u: f64,
        v: f64,
        diameter: f64,
        depth: f64,
        purpose: BorePurpose,
    },
    /// A bore into an edge: shown as a tick at the edge, with its depth.
    EdgeBore {
        u: f64,
        v: f64,
        diameter: f64,
        depth: f64,
        edge: Edge,
    },
    /// A slot or groove footprint on the drawn face (or through an edge).
    Rect {
        u: f64,
        v: f64,
        w: f64,
        h: f64,
        label: &'static str,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    Left,
    Right,
    Bottom,
    Top,
}

/// The drawing frame for a part: which cabinet axes map to u and v.
fn frame(p: &Part) -> (Axis, Axis) {
    let [a, b] = p.thickness_axis.others();
    let (len, _) = p.finished_size();
    if (p.bounds.size.get(a) - len).abs() < 1e-9 && p.grain_axis != Some(b) {
        (a, b)
    } else {
        (b, a)
    }
}

fn to_uv(p: &Part, at: Vec3) -> (f64, f64) {
    let (ua, va) = frame(p);
    (
        at.get(ua) - p.bounds.min.get(ua),
        at.get(va) - p.bounds.min.get(va),
    )
}

/// The machining on `part`, flattened into its drawing frame.
pub fn features(list: &PartList, part: &Part) -> Vec<Feature> {
    let (ua, va) = frame(part);
    let (lu, lv) = (part.bounds.size.get(ua), part.bounds.size.get(va));
    let mut out = Vec::new();
    for op in &list.operations {
        match op {
            Operation::Bore {
                part: name,
                at,
                direction,
                diameter,
                depth,
                purpose,
            } if *name == part.name => {
                let (u, v) = to_uv(part, *at);
                if direction.axis == part.thickness_axis {
                    out.push(Feature::FaceBore {
                        u,
                        v,
                        diameter: *diameter,
                        depth: *depth,
                        purpose: *purpose,
                    });
                } else {
                    let edge = if direction.axis == ua {
                        if u < lu / 2.0 {
                            Edge::Left
                        } else {
                            Edge::Right
                        }
                    } else if v < lv / 2.0 {
                        Edge::Bottom
                    } else {
                        Edge::Top
                    };
                    out.push(Feature::EdgeBore {
                        u,
                        v,
                        diameter: *diameter,
                        depth: *depth,
                        edge,
                    });
                }
            }
            Operation::BiscuitSlot { parts, bounds, .. } if parts.contains(&part.name) => {
                out.push(rect(part, bounds.min, bounds.size, "biscuit"));
            }
            Operation::Groove { part: name, bounds } if *name == part.name => {
                out.push(rect(part, bounds.min, bounds.size, "groove"));
            }
            _ => {}
        }
    }
    out
}

fn rect(part: &Part, min: Vec3, size: Vec3, label: &'static str) -> Feature {
    let (ua, va) = frame(part);
    let (u0, v0) = to_uv(part, min);
    // Clip to the part so slots shared by two parts show only this part's half.
    let u1 = (u0 + size.get(ua)).min(part.bounds.size.get(ua));
    let v1 = (v0 + size.get(va)).min(part.bounds.size.get(va));
    let (u0, v0) = (u0.max(0.0), v0.max(0.0));
    Feature::Rect {
        u: u0,
        v: v0,
        w: u1 - u0,
        h: v1 - v0,
        label,
    }
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn fmt(v: f64) -> String {
    let r = (v * 10.0).round() / 10.0;
    if r.fract() == 0.0 {
        format!("{r:.0}")
    } else {
        format!("{r:.1}")
    }
}

/// The cut sheet for one part as an SVG document.
pub fn part_sheet(list: &PartList, part: &Part, board: &str, job: &str) -> String {
    let (ua, va) = frame(part);
    let (lu, lv) = (part.bounds.size.get(ua), part.bounds.size.get(va));
    let draw_w = PAGE_W - 2.0 * MARGIN - 30.0;
    let draw_h = PAGE_H - 2.0 * MARGIN - TITLE_H - 24.0;
    let s = (draw_w / lu).min(draw_h / lv);
    let (ox, oy) = (MARGIN + 22.0, MARGIN + 8.0 + draw_h); // reference corner on the page
    let x = |u: f64| ox + u * s;
    let y = |v: f64| oy - v * s;

    let mut svg = String::new();
    let _ = write!(
        svg,
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{PAGE_W}mm" height="{PAGE_H}mm" viewBox="0 0 {PAGE_W} {PAGE_H}" font-family="Helvetica, Arial, sans-serif">
<rect width="{PAGE_W}" height="{PAGE_H}" fill="#fff"/>
<style>.o{{fill:#f6f3ec;stroke:#222;stroke-width:0.35}}.t{{font-size:3px;fill:#222}}.s{{font-size:2.4px;fill:#555}}.d{{stroke:#4a6fa5;stroke-width:0.2;fill:none}}.e{{stroke:#c0392b;stroke-width:1.1}}</style>
"##
    );
    let _ = writeln!(
        svg,
        r#"<rect class="o" x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}"/>"#,
        x(0.0),
        y(lv),
        lu * s,
        lv * s
    );

    // Edge tape: a heavy line along each taped edge.
    let (l0, l1) = part.edges.on_axis(ua);
    let (v0e, v1e) = part.edges.on_axis(va);
    let taped = [
        (l0, x(0.0), y(0.0), x(0.0), y(lv)),
        (l1, x(lu), y(0.0), x(lu), y(lv)),
        (v0e, x(0.0), y(0.0), x(lu), y(0.0)),
        (v1e, x(0.0), y(lv), x(lu), y(lv)),
    ];
    for (on, x1, y1, x2, y2) in taped {
        if on {
            let _ = writeln!(
                svg,
                r#"<line class="e" x1="{x1:.2}" y1="{y1:.2}" x2="{x2:.2}" y2="{y2:.2}"/>"#
            );
        }
    }

    // Overall dimensions.
    let _ = writeln!(
        svg,
        r#"<line class="d" x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}"/>"#,
        x(0.0),
        y(0.0) + 7.0,
        x(lu),
        y(0.0) + 7.0
    );
    let _ = writeln!(
        svg,
        r#"<text class="t" x="{:.2}" y="{:.2}" text-anchor="middle">{}</text>"#,
        x(lu / 2.0),
        y(0.0) + 11.0,
        fmt(lu)
    );
    let _ = writeln!(
        svg,
        r#"<line class="d" x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}"/>"#,
        x(0.0) - 7.0,
        y(0.0),
        x(0.0) - 7.0,
        y(lv)
    );
    let _ = writeln!(
        svg,
        r#"<text class="t" x="{:.2}" y="{:.2}" text-anchor="middle" transform="rotate(-90 {:.2} {:.2})">{}</text>"#,
        x(0.0) - 9.0,
        y(lv / 2.0),
        x(0.0) - 9.0,
        y(lv / 2.0),
        fmt(lv)
    );
    let _ = writeln!(
        svg,
        r##"<circle cx="{:.2}" cy="{:.2}" r="1" fill="#c0392b"/>"##,
        x(0.0),
        y(0.0)
    );

    // Machining.
    let feats = features(list, part);
    let mut table = Vec::new();
    for f in &feats {
        match f {
            Feature::FaceBore {
                u,
                v,
                diameter,
                depth,
                purpose,
            } => {
                let r = (diameter / 2.0 * s).max(0.6);
                let _ = writeln!(
                    svg,
                    r##"<circle cx="{:.2}" cy="{:.2}" r="{r:.2}" fill="none" stroke="#222" stroke-width="0.25"/>"##,
                    x(*u),
                    y(*v)
                );
                table.push(format!(
                    "Ø{} × {} {:?} @ {}, {}",
                    fmt(*diameter),
                    fmt(*depth),
                    purpose,
                    fmt(*u),
                    fmt(*v)
                ));
            }
            Feature::EdgeBore {
                u,
                v,
                diameter,
                depth,
                edge,
            } => {
                let (cx, cy) = (x(*u), y(*v));
                let len = (depth * s).max(1.5);
                let (x2, y2) = match edge {
                    Edge::Left => (cx + len, cy),
                    Edge::Right => (cx - len, cy),
                    Edge::Bottom => (cx, cy - len),
                    Edge::Top => (cx, cy + len),
                };
                let _ = writeln!(
                    svg,
                    r##"<line x1="{cx:.2}" y1="{cy:.2}" x2="{x2:.2}" y2="{y2:.2}" stroke="#7a3fa0" stroke-width="{:.2}"/>"##,
                    (diameter * s).max(0.4)
                );
                table.push(format!(
                    "Edge Ø{} × {} ({edge:?}) @ {}, {}",
                    fmt(*diameter),
                    fmt(*depth),
                    fmt(*u),
                    fmt(*v)
                ));
            }
            Feature::Rect { u, v, w, h, label } => {
                let fill = if *label == "biscuit" {
                    "#e8c27a"
                } else {
                    "#9bb7d4"
                };
                let _ = writeln!(
                    svg,
                    r##"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="{fill}" stroke="#222" stroke-width="0.15"/>"##,
                    x(*u),
                    y(v + h),
                    (w * s).max(0.3),
                    (h * s).max(0.3)
                );
                table.push(format!(
                    "{label} {} × {} @ {}, {}",
                    fmt(*w),
                    fmt(*h),
                    fmt(*u),
                    fmt(*v)
                ));
            }
        }
    }

    // Title block.
    let ty = PAGE_H - MARGIN - TITLE_H;
    let (fl, fw) = part.finished_size();
    let _ = writeln!(
        svg,
        r##"<rect x="{MARGIN}" y="{ty:.2}" width="{:.2}" height="{TITLE_H}" fill="none" stroke="#222" stroke-width="0.3"/>"##,
        PAGE_W - 2.0 * MARGIN
    );
    let lines = [
        format!("{}  —  {}", esc(&part.name), esc(part.role.label())),
        format!(
            "Finished {} × {} × {} mm   Board: {}",
            fmt(fl),
            fmt(fw),
            fmt(part.thickness()),
            esc(board)
        ),
        format!(
            "Job: {}   Cabinet: {}   Operations: {}   Reference corner ●   Edge tape ▬",
            esc(job),
            esc(&list.cabinet),
            feats.len()
        ),
    ];
    for (i, l) in lines.iter().enumerate() {
        let _ = writeln!(
            svg,
            r#"<text class="t" x="{:.2}" y="{:.2}">{l}</text>"#,
            MARGIN + 3.0,
            ty + 7.0 + i as f64 * 7.0
        );
    }
    // Operation table, right of the title text.
    for (i, l) in table.iter().take(9).enumerate() {
        let _ = writeln!(
            svg,
            r#"<text class="s" x="{:.2}" y="{:.2}">{}</text>"#,
            PAGE_W / 2.0 + 10.0,
            ty + 4.5 + i as f64 * 2.9,
            esc(l)
        );
    }
    if table.len() > 9 {
        let _ = writeln!(
            svg,
            r#"<text class="s" x="{:.2}" y="{:.2}">+{} more</text>"#,
            PAGE_W - MARGIN - 20.0,
            ty + TITLE_H - 2.0,
            table.len() - 9
        );
    }
    svg.push_str("</svg>\n");
    svg
}

#[cfg(test)]
mod tests {
    use super::*;
    use cabinet_core::{Cabinet, CabinetKind, build_cabinet};

    #[test]
    fn the_left_side_shows_its_shelf_pins_and_hinge_plate_holes_on_its_face() {
        let list = build_cabinet(&Cabinet::new("B01", CabinetKind::Base)).unwrap();
        let side = list.part("B01.LeftSide").unwrap();
        let f = features(&list, side);
        assert!(f.iter().any(|f| matches!(
            f,
            Feature::FaceBore {
                purpose: BorePurpose::ShelfPin,
                ..
            }
        )));
        assert!(f.iter().any(|f| matches!(
            f,
            Feature::FaceBore {
                purpose: BorePurpose::HingePlate,
                ..
            }
        )));
        assert!(f.iter().any(|f| matches!(
            f,
            Feature::Rect {
                label: "biscuit",
                ..
            }
        )));
    }

    #[test]
    fn the_bottom_shows_screw_pilots_in_its_edges() {
        let list = build_cabinet(&Cabinet::new("B01", CabinetKind::Base)).unwrap();
        let bottom = list.part("B01.Bottom").unwrap();
        let f = features(&list, bottom);
        assert!(f.iter().any(|f| matches!(f, Feature::EdgeBore { .. })));
    }

    #[test]
    fn features_lie_on_the_part() {
        let list = build_cabinet(&Cabinet::new("T01", CabinetKind::Tall)).unwrap();
        for p in &list.parts {
            let (ua, va) = frame(p);
            let (lu, lv) = (p.bounds.size.get(ua), p.bounds.size.get(va));
            for f in features(&list, p) {
                let (u, v) = match f {
                    Feature::FaceBore { u, v, .. }
                    | Feature::EdgeBore { u, v, .. }
                    | Feature::Rect { u, v, .. } => (u, v),
                };
                assert!(
                    (-1e-6..=lu + 1e-6).contains(&u) && (-1e-6..=lv + 1e-6).contains(&v),
                    "{} {f:?}",
                    p.name
                );
            }
        }
    }

    #[test]
    fn sheet_is_well_formed_svg() {
        let list = build_cabinet(&Cabinet::new("B01", CabinetKind::Base)).unwrap();
        let svg = part_sheet(
            &list,
            list.part("B01.Door1").unwrap(),
            "Polytec Prime Oak Matt 18",
            "Smith <kitchen>",
        );
        assert!(svg.starts_with("<svg") && svg.trim_end().ends_with("</svg>"));
        assert!(svg.contains("Smith &lt;kitchen&gt;"));
    }
}
