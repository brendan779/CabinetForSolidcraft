//! Cut sheets: one A4-landscape SVG per part, showing the part's main face with every operation
//! dimensioned from the reference corner (bottom-left as drawn), edge tape marked and a title
//! block. [`pdf::job_pdf`] puts every sheet of a job in one PDF.

use std::fmt::Write;

pub mod assembly;
pub mod dxf;
pub mod labels;
pub mod nesting;
pub mod pdf;

use cabinet_core::geom::{Axis, Vec3};
use cabinet_core::part::{BorePurpose, Operation, Part, PartList};

/// A4 landscape in mm; the SVG uses mm units.
pub(crate) const PAGE_W: f64 = 297.0;
pub(crate) const PAGE_H: f64 = 210.0;
const MARGIN: f64 = 12.0;
const TITLE_H: f64 = 30.0;

/// Something machined on the part, in the drawing's 2D frame (u across, v up, from the
/// reference corner), with the face it is on.
#[derive(Debug, Clone, PartialEq)]
pub enum Feature {
    /// A bore into one of the part's faces.
    FaceBore {
        u: f64,
        v: f64,
        diameter: f64,
        depth: f64,
        purpose: BorePurpose,
        face: Face,
    },
    /// A bore into an edge: shown as a tick at the edge, with its depth.
    EdgeBore {
        u: f64,
        v: f64,
        diameter: f64,
        depth: f64,
        edge: Edge,
    },
    /// A slot or groove footprint, on one face, or `None` when it goes through or is cut from an
    /// edge (both faces show it).
    Rect {
        u: f64,
        v: f64,
        w: f64,
        h: f64,
        /// How deep it goes into the part, across its thickness.
        depth: f64,
        label: &'static str,
        face: Option<Face>,
    },
}

/// A part's two big faces: at the low and the high end of its thickness axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Face {
    Low,
    High,
}

impl Feature {
    /// The face it is machined into, if it is on one face only.
    pub fn face(&self) -> Option<Face> {
        match self {
            Feature::FaceBore { face, .. } => Some(*face),
            Feature::Rect { face, .. } => *face,
            Feature::EdgeBore { .. } => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    Left,
    Right,
    Bottom,
    Top,
}

/// The drawing frame for a part: which cabinet axes map to u and v.
pub(crate) fn frame(p: &Part) -> (Axis, Axis) {
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
                    // Drilled towards +t, the hole opens on the low face.
                    let face = if direction.positive {
                        Face::Low
                    } else {
                        Face::High
                    };
                    out.push(Feature::FaceBore {
                        u,
                        v,
                        diameter: *diameter,
                        depth: *depth,
                        purpose: *purpose,
                        face,
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
                // A groove through the whole thickness takes material away: a notch at an edge, a
                // cut-out (a sink's, a cooktop's) inside the part.
                let through = bounds.size.get(part.thickness_axis) >= part.thickness() - 1e-6;
                let inside = Axis::ALL
                    .iter()
                    .filter(|&&a| a != part.thickness_axis)
                    .all(|&a| {
                        bounds.min.get(a) > part.bounds.min.get(a) + 1e-6
                            && bounds.max().get(a) < part.bounds.max().get(a) - 1e-6
                    });
                let label = match (through, inside) {
                    (true, true) => "cutout",
                    (true, false) => "notch",
                    _ => "groove",
                };
                out.push(rect(part, bounds.min, bounds.size, label));
            }
            Operation::Mortise {
                part: name, bounds, ..
            } if *name == part.name => {
                out.push(rect(part, bounds.min, bounds.size, "mortise"));
            }
            _ => {}
        }
    }
    out
}

fn rect(part: &Part, min: Vec3, size: Vec3, label: &'static str) -> Feature {
    let (ua, va) = frame(part);
    let t = part.thickness_axis;
    let (lo, hi) = (part.bounds.min.get(t), part.bounds.max().get(t));
    let (r0, r1) = (min.get(t), min.get(t) + size.get(t));
    let face = match (r0 <= lo + 1e-6, r1 >= hi - 1e-6) {
        (true, false) => Some(Face::Low),
        (false, true) => Some(Face::High),
        _ => None,
    };
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
        depth: r1.min(hi) - r0.max(lo),
        label,
        face,
    }
}

pub(crate) fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub(crate) fn fmt(v: f64) -> String {
    let r = (v * 10.0).round() / 10.0;
    if r.fract() == 0.0 {
        format!("{r:.0}")
    } else {
        format!("{r:.1}")
    }
}

/// The cut sheet for one part as an SVG document. A part machined on both faces gets a view of
/// each (the far one turned over, so mirrored); an edge with bores in it gets an edge view.
pub fn part_sheet(list: &PartList, part: &Part, board: &str, job: &str) -> String {
    let (ua, va) = frame(part);
    let (lu, lv) = (part.bounds.size.get(ua), part.bounds.size.get(va));
    let feats = features(list, part);
    let has = |f: Face| feats.iter().any(|x| x.face() == Some(f));
    let two_faces = has(Face::Low) && has(Face::High);
    let mut edges: Vec<Edge> = Vec::new();
    for f in &feats {
        if let Feature::EdgeBore { edge, .. } = f
            && !edges.contains(edge)
        {
            edges.push(*edge);
        }
    }
    let edge_views = two_faces || !edges.is_empty();
    if !edge_views {
        edges.clear();
    }
    let draw_w = PAGE_W - 2.0 * MARGIN - 30.0;
    let draw_h = PAGE_H - 2.0 * MARGIN - TITLE_H - 24.0 - EDGE_ROW * edges.len() as f64;
    let views = if two_faces { 2.0 } else { 1.0 };
    let view_w = (draw_w - VIEW_GAP * (views - 1.0)) / views;
    let s = (view_w / lu).min(draw_h / lv);
    let oy = MARGIN + 8.0 + draw_h; // reference corner on the page

    let mut svg = String::new();
    let _ = write!(
        svg,
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{PAGE_W}mm" height="{PAGE_H}mm" viewBox="0 0 {PAGE_W} {PAGE_H}" font-family="Helvetica, Arial, sans-serif">
<rect width="{PAGE_W}" height="{PAGE_H}" fill="#fff"/>
<style>.o{{fill:#f6f3ec;stroke:#222;stroke-width:0.35}}.t{{font-size:3px;fill:#222}}.s{{font-size:2.4px;fill:#555}}.d{{stroke:#4a6fa5;stroke-width:0.2;fill:none}}.e{{stroke:#c0392b;stroke-width:1.1}}.o2{{font-size:1.7px;fill:#4a6fa5}}</style>
"##
    );
    // Which face is inside: the one facing the middle of the cabinet.
    let inside = {
        let t = part.thickness_axis;
        let mid = list
            .parts
            .iter()
            .map(|p| p.bounds.center().get(t))
            .sum::<f64>()
            / list.parts.len().max(1) as f64;
        if mid < part.bounds.center().get(t) {
            Face::Low
        } else {
            Face::High
        }
    };
    let outside = if inside == Face::Low {
        Face::High
    } else {
        Face::Low
    };
    let mut table = Vec::new();
    if two_faces {
        let ox = MARGIN + 22.0;
        let view = View {
            ox,
            oy,
            s,
            lu,
            lv,
            mirror: false,
            face: Some(inside),
            tag: "in ",
            shared: true,
        };
        face_view(&mut svg, part, &feats, &view, &mut table);
        caption(&mut svg, &view, "Inside face");
        let view = View {
            ox: ox + view_w + VIEW_GAP,
            mirror: true,
            face: Some(outside),
            tag: "out ",
            shared: false,
            ..view
        };
        face_view(&mut svg, part, &feats, &view, &mut table);
        caption(
            &mut svg,
            &view,
            "Outside face, turned over (left and right swap)",
        );
    } else {
        let view = View {
            ox: MARGIN + 22.0,
            oy,
            s,
            lu,
            lv,
            mirror: false,
            face: None,
            tag: "",
            shared: true,
        };
        face_view(&mut svg, part, &feats, &view, &mut table);
        if edge_views {
            caption(&mut svg, &view, "Face");
        }
    }
    for (i, edge) in edges.iter().enumerate() {
        edge_view(
            &mut svg,
            part,
            &feats,
            *edge,
            MARGIN + 22.0,
            oy + 16.0 + EDGE_ROW * i as f64,
            s,
        );
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

/// Height of one edge view row, and the gap between two face views.
const EDGE_ROW: f64 = 9.0;
const VIEW_GAP: f64 = 16.0;

/// A face view: its reference corner on the page, the scale, the part's size, whether it is seen
/// turned over (u runs right to left), the face it shows (`None`: every feature), the tag its
/// features get in the table, and whether it lists the edge bores and through cuts every view
/// shows.
#[derive(Clone, Copy)]
struct View {
    ox: f64,
    oy: f64,
    s: f64,
    lu: f64,
    lv: f64,
    mirror: bool,
    face: Option<Face>,
    tag: &'static str,
    shared: bool,
}

impl View {
    fn x(&self, u: f64) -> f64 {
        let u = if self.mirror { self.lu - u } else { u };
        self.ox + u * self.s
    }

    fn y(&self, v: f64) -> f64 {
        self.oy - v * self.s
    }
}

fn caption(svg: &mut String, v: &View, text: &str) {
    let _ = writeln!(
        svg,
        r#"<text class="s" x="{:.2}" y="{:.2}">{}</text>"#,
        v.ox,
        v.y(v.lv) - 2.0,
        esc(text)
    );
}

/// One face of the part: outline, edge tape, overall dimensions, reference corner and the
/// view's features, each added to `table`.
fn face_view(
    svg: &mut String,
    part: &Part,
    feats: &[Feature],
    view: &View,
    table: &mut Vec<String>,
) {
    let (face, tag, shared) = (view.face, view.tag, view.shared);
    let (ua, va) = frame(part);
    let (lu, lv, s) = (view.lu, view.lv, view.s);
    let x = |u: f64| view.x(u);
    let y = |v: f64| view.y(v);
    let left = x(0.0).min(x(lu));
    let _ = writeln!(
        svg,
        r#"<rect class="o" x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}"/>"#,
        left,
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
        left - 7.0,
        y(0.0),
        left - 7.0,
        y(lv)
    );
    let _ = writeln!(
        svg,
        r#"<text class="t" x="{:.2}" y="{:.2}" text-anchor="middle" transform="rotate(-90 {:.2} {:.2})">{}</text>"#,
        left - 9.0,
        y(lv / 2.0),
        left - 9.0,
        y(lv / 2.0),
        fmt(lv)
    );
    let _ = writeln!(
        svg,
        r##"<circle cx="{:.2}" cy="{:.2}" r="1" fill="#c0392b"/>"##,
        x(0.0),
        y(0.0)
    );

    // Machining. Face bores, slots and grooves are listed by kind (size, depth, purpose) with a
    // count; bores are also dimensioned from the reference corner.
    let mut holes: Vec<(String, f64, f64)> = Vec::new();
    for f in feats {
        if face.is_some() && f.face().is_some_and(|ff| Some(ff) != face) {
            continue;
        }
        match f {
            Feature::FaceBore {
                u,
                v,
                diameter,
                depth,
                purpose,
                ..
            } => {
                let r = (diameter / 2.0 * s).max(0.6);
                let _ = writeln!(
                    svg,
                    r##"<circle cx="{:.2}" cy="{:.2}" r="{r:.2}" fill="none" stroke="#222" stroke-width="0.25"/>"##,
                    x(*u),
                    y(*v)
                );
                holes.push((
                    format!("{tag}Ø{} × {} {:?}", fmt(*diameter), fmt(*depth), purpose),
                    *u,
                    *v,
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
                // Turned over, left and right edges swap sides on the page.
                let (x2, y2) = match (edge, view.mirror) {
                    (Edge::Left, false) | (Edge::Right, true) => (cx + len, cy),
                    (Edge::Right, false) | (Edge::Left, true) => (cx - len, cy),
                    (Edge::Bottom, _) => (cx, cy - len),
                    (Edge::Top, _) => (cx, cy + len),
                };
                let _ = writeln!(
                    svg,
                    r##"<line x1="{cx:.2}" y1="{cy:.2}" x2="{x2:.2}" y2="{y2:.2}" stroke="#7a3fa0" stroke-width="{:.2}"/>"##,
                    (diameter * s).max(0.4)
                );
                if shared {
                    table.push(format!(
                        "Edge Ø{} × {} ({edge:?}) @ {}, {}",
                        fmt(*diameter),
                        fmt(*depth),
                        fmt(*u),
                        fmt(*v)
                    ));
                }
            }
            Feature::Rect {
                u,
                v,
                w,
                h,
                label,
                face: on,
                ..
            } => {
                // A notch is drawn as the paper showing through: no board there.
                let fill = match *label {
                    "biscuit" => "#e8c27a",
                    "notch" | "cutout" => "#ffffff",
                    _ => "#9bb7d4",
                };
                let (xa, xb) = (x(*u), x(u + w));
                let _ = writeln!(
                    svg,
                    r##"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="{fill}" stroke="#222" stroke-width="0.15"/>"##,
                    xa.min(xb),
                    y(v + h),
                    (w * s).max(0.3),
                    (h * s).max(0.3)
                );
                if on.is_some()
                    || face != Some(Face::High) && face != Some(Face::Low)
                    || tag == "in "
                {
                    holes.push((format!("{tag}{label} {} × {}", fmt(*w), fmt(*h)), *u, *v));
                }
            }
        }
    }
    let mut kinds: Vec<(String, Vec<(f64, f64)>)> = Vec::new();
    for (kind, u, v) in &holes {
        match kinds.iter_mut().find(|(k, _)| k == kind) {
            Some((_, at)) => at.push((*u, *v)),
            None => kinds.push((kind.clone(), vec![(*u, *v)])),
        }
    }
    for (kind, at) in &kinds {
        let first = at
            .iter()
            .map(|(u, v)| format!("{}, {}", fmt(*u), fmt(*v)))
            .take(2)
            .collect::<Vec<_>>()
            .join("; ");
        let more = if at.len() > 2 { "; …" } else { "" };
        table.push(format!("{kind} ×{}  @ {first}{more}", at.len()));
    }
    // Only bores are dimensioned; slots and grooves are in the table.
    let bores: Vec<(String, f64, f64)> = holes
        .iter()
        .filter(|h| h.0.contains('Ø'))
        .cloned()
        .collect();
    ordinates(svg, view, &bores);
}

/// Ordinate dimensions from the reference corner: every distinct hole position across (u) is
/// ticked and labelled under the part, every distinct position up (v) to its left, so each hole
/// row and column reads straight off the sheet.
fn ordinates(svg: &mut String, view: &View, holes: &[(String, f64, f64)]) {
    let distinct = |mut vals: Vec<f64>| {
        vals.sort_by(f64::total_cmp);
        vals.dedup_by(|a, b| (*a - *b).abs() < 0.05);
        vals
    };
    let us = distinct(holes.iter().map(|h| h.1).collect());
    let vs = distinct(holes.iter().map(|h| h.2).collect());
    let left = view.x(0.0).min(view.x(view.lu));
    let base = view.y(0.0);
    // Labels closer than a text height to the last one drop to a second line.
    const ROOM: f64 = 2.2;
    let mut last = f64::NEG_INFINITY;
    let mut staggered = false;
    let mut xs: Vec<f64> = us.iter().map(|u| view.x(*u)).collect();
    xs.sort_by(f64::total_cmp);
    for x in xs {
        let u = if view.mirror {
            view.lu - (x - view.ox) / view.s
        } else {
            (x - view.ox) / view.s
        };
        staggered = (x - last).abs() < ROOM && !staggered;
        if !staggered {
            last = x;
        }
        let y0 = base + if staggered { 5.5 } else { 2.0 };
        let _ = writeln!(
            svg,
            r#"<line class="d" x1="{x:.2}" y1="{:.2}" x2="{x:.2}" y2="{:.2}"/>"#,
            base,
            base + 1.5
        );
        let _ = writeln!(
            svg,
            r#"<text class="o2" x="{x:.2}" y="{y0:.2}" transform="rotate(90 {x:.2} {y0:.2})">{}</text>"#,
            fmt(u)
        );
    }
    let mut last = f64::NEG_INFINITY;
    let mut staggered = false;
    for v in vs {
        let y = view.y(v);
        staggered = (y - last).abs() < ROOM && !staggered;
        if !staggered {
            last = y;
        }
        let x0 = left - if staggered { 9.0 } else { 2.0 };
        let _ = writeln!(
            svg,
            r#"<line class="d" x1="{:.2}" y1="{y:.2}" x2="{left:.2}" y2="{y:.2}"/>"#,
            left - 1.5
        );
        let _ = writeln!(
            svg,
            r#"<text class="o2" x="{x0:.2}" y="{:.2}" text-anchor="end">{}</text>"#,
            y + 0.6,
            fmt(v)
        );
    }
}

/// One edge seen straight on, as long as the face view and as thick as the board, with its bores.
fn edge_view(
    svg: &mut String,
    part: &Part,
    feats: &[Feature],
    edge: Edge,
    ox: f64,
    top: f64,
    s: f64,
) {
    let (ua, va) = frame(part);
    let length = match edge {
        Edge::Bottom | Edge::Top => part.bounds.size.get(ua),
        Edge::Left | Edge::Right => part.bounds.size.get(va),
    };
    // Thin boards are drawn at least 4 mm thick so the holes show.
    let th = (part.thickness() * s).max(4.0);
    let _ = writeln!(
        svg,
        r#"<rect class="o" x="{ox:.2}" y="{top:.2}" width="{:.2}" height="{th:.2}"/>"#,
        length * s
    );
    let name = match edge {
        Edge::Bottom => "Bottom edge",
        Edge::Top => "Top edge",
        Edge::Left => "Left edge",
        Edge::Right => "Right edge",
    };
    let _ = writeln!(
        svg,
        r#"<text class="s" x="{:.2}" y="{:.2}">{name}</text>"#,
        ox + length * s + 2.0,
        top + th / 2.0 + 0.8
    );
    for f in feats {
        if let Feature::EdgeBore {
            u,
            v,
            diameter,
            edge: e,
            ..
        } = f
            && *e == edge
        {
            let along = match edge {
                Edge::Bottom | Edge::Top => *u,
                Edge::Left | Edge::Right => *v,
            };
            let r = (diameter / 2.0 * s).max(0.6).min(th / 2.0);
            let _ = writeln!(
                svg,
                r##"<circle cx="{:.2}" cy="{:.2}" r="{r:.2}" fill="none" stroke="#7a3fa0" stroke-width="0.25"/>"##,
                ox + along * s,
                top + th / 2.0
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cabinet_core::{Cabinet, CabinetKind, build_cabinet};

    #[test]
    fn a_side_machined_on_both_faces_shows_the_inside_and_the_outside_turned_over() {
        let list = build_cabinet(&Cabinet::new("B01", CabinetKind::Base)).unwrap();
        let side = list.part("B01.LeftSide").unwrap();
        let svg = part_sheet(&list, side, "Board", "Job");
        assert!(svg.contains(">Inside face<"));
        assert!(svg.contains(">Outside face, turned over"));
        // Shelf pins and hinge plates are inside; the screws into the bottom come from outside.
        let feats = features(&list, side);
        let face_of = |p: BorePurpose| {
            feats.iter().find_map(|f| match f {
                Feature::FaceBore { purpose, face, .. } if *purpose == p => Some(*face),
                _ => None,
            })
        };
        assert_ne!(face_of(BorePurpose::ShelfPin), face_of(BorePurpose::Screw));
        assert_eq!(
            face_of(BorePurpose::ShelfPin),
            face_of(BorePurpose::HingePlate)
        );
    }

    #[test]
    fn domino_mortises_show_on_the_side_and_the_bottom() {
        let mut cab = Cabinet::new("B01", CabinetKind::Base);
        cab.standard.fixings = cabinet_core::model::FixingRecipe::Domino;
        let list = build_cabinet(&cab).unwrap();
        for part in ["B01.LeftSide", "B01.Bottom"] {
            let p = list.part(part).unwrap();
            assert!(
                features(&list, p).iter().any(|f| matches!(
                    f,
                    Feature::Rect {
                        label: "mortise",
                        ..
                    }
                )),
                "{part}"
            );
            assert!(
                part_sheet(&list, p, "Board", "Job").contains("mortise"),
                "{part}"
            );
        }
    }

    #[test]
    fn edges_with_bores_get_an_edge_view_and_a_one_face_part_keeps_one_view() {
        let list = build_cabinet(&Cabinet::new("B01", CabinetKind::Base)).unwrap();
        let bottom = part_sheet(&list, list.part("B01.Bottom").unwrap(), "Board", "Job");
        assert!(bottom.contains(">Left edge<") && bottom.contains(">Right edge<"));
        let door = part_sheet(&list, list.part("B01.Door1").unwrap(), "Board", "Job");
        assert!(!door.contains("face<") && !door.contains(" edge<"));
    }

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
    fn an_integrated_toe_kick_shows_as_a_notch_at_the_front_bottom_of_each_side() {
        let mut cab = Cabinet::new("B01", CabinetKind::Base);
        cab.standard.toe_kick = cabinet_core::model::ToeKick::IntegratedNotch;
        let list = build_cabinet(&cab).unwrap();
        for side in ["B01.LeftSide", "B01.RightSide"] {
            let part = list.part(side).unwrap();
            let notches: Vec<_> = features(&list, part)
                .into_iter()
                .filter(|f| matches!(f, Feature::Rect { label: "notch", .. }))
                .collect();
            let s = &cab.standard;
            assert_eq!(
                notches,
                [Feature::Rect {
                    u: 0.0,
                    v: 0.0,
                    // The side is drawn with its height along u and its depth along v.
                    w: s.toe_kick_height,
                    h: s.toe_kick_setback,
                    depth: s.carcass_thickness,
                    label: "notch",
                    face: None,
                }],
                "{side}"
            );
            let svg = part_sheet(&list, part, "Board", "Job");
            assert!(svg.contains("notch 150 × 50 ×1  @ 0, 0"), "{side}");
        }
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
