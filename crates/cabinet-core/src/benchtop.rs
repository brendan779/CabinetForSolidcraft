//! Mitred stone blanks, cabinet-board packers and the visible banded shadow rail.
use crate::geom::{Aabb, Axis, Vec3};
use crate::part::{EdgeBanding, Operation, Part, PartList, PartRole};
use crate::run::{Benchtop, EdgeProfile};

pub(crate) fn validate(b: &Benchtop) -> Result<(), crate::Error> {
    let bad = |message: &str| crate::Error::Invalid(format!("{}: {message}", b.id));
    if !b.thickness.is_finite() || b.thickness <= 0.0 {
        return Err(bad("stone thickness must be positive"));
    }
    if let EdgeProfile::MitredApron { depth } = b.edge {
        if !depth.is_finite() || depth < b.thickness {
            return Err(bad(
                "finished edge depth must be at least the stone thickness",
            ));
        }
        let u = &b.build_up;
        if [u.shadow_height, u.setback, u.board_thickness]
            .iter()
            .any(|v| !v.is_finite() || *v < 0.0)
            || !u.strip_width.is_finite()
            || u.strip_width <= 0.0
        {
            return Err(bad(
                "shadow height and recess must be non-negative and support width positive",
            ));
        }
        if u.edge_tape.as_ref().is_some_and(|t| {
            !t.thickness.is_finite() || t.thickness <= 0.0 || t.thickness >= u.strip_width
        }) {
            return Err(bad(
                "edgebanding thickness must be positive and less than the support width",
            ));
        }
    }
    Ok(())
}

fn part(list: &mut PartList, name: String, b: Aabb, axis: Axis, board: &str, role: PartRole) {
    list.parts.push(Part {
        name,
        role,
        bounds: b,
        thickness_axis: axis,
        grain_axis: Some(if axis == Axis::X { Axis::Y } else { Axis::X }),
        board: board.into(),
        edges: EdgeBanding::default(),
    });
}

/// The triangular volume removed, with `at` on the corner being cut away.
fn mitre(list: &mut PartList, name: &str, bounds: Aabb, at: Vec3) {
    list.operations.push(Operation::Chamfer {
        part: name.into(),
        bounds,
        at,
    });
}

fn top_edge(list: &mut PartList, name: &str, b: Aabb, side: usize) {
    let (lo, hi, c, t) = (b.min, b.max(), b.center(), b.size.z);
    let (bounds, at) = match side {
        0 => (
            Aabb::new(lo, Vec3::new(b.size.x, t, t)),
            Vec3::new(c.x, lo.y, lo.z),
        ),
        1 => (
            Aabb::new(lo, Vec3::new(t, b.size.y, t)),
            Vec3::new(lo.x, c.y, lo.z),
        ),
        2 => (
            Aabb::new(Vec3::new(hi.x - t, lo.y, lo.z), Vec3::new(t, b.size.y, t)),
            Vec3::new(hi.x, c.y, lo.z),
        ),
        _ => (
            Aabb::new(Vec3::new(lo.x, hi.y - t, lo.z), Vec3::new(b.size.x, t, t)),
            Vec3::new(c.x, hi.y, lo.z),
        ),
    };
    mitre(list, name, bounds, at);
}

pub(crate) fn build_up(
    b: &Benchtop,
    list: &mut PartList,
    n: &str,
    top: Aabb,
    frame: Aabb,
    source: &[&Part],
    walls: (bool, bool),
) {
    let EdgeProfile::MitredApron { depth } = b.edge else {
        return;
    };
    let (lo, hi, t) = (top.min, top.max(), b.thickness);
    support_frame(b, list, n, top, frame, source, walls);
    let top_name = format!("{}.Benchtop{n}", b.id);
    top_edge(list, &top_name, top, 0);
    let apron = format!("{}.Apron{n}", b.id);
    let face = Aabb::new(
        Vec3::new(lo.x, lo.y, hi.z - depth),
        Vec3::new(top.size.x, t, depth),
    );
    part(
        list,
        apron.clone(),
        face,
        Axis::Y,
        &b.board,
        PartRole::Benchtop,
    );
    mitre(
        list,
        &apron,
        Aabb::new(Vec3::new(lo.x, lo.y, hi.z - t), Vec3::new(top.size.x, t, t)),
        Vec3::new(top.center().x, lo.y + t, hi.z),
    );
    for (side, waterfall, wall, x) in [
        (1, b.waterfall_left, walls.0, lo.x),
        (2, b.waterfall_right, walls.1, hi.x - t),
    ] {
        if wall {
            continue;
        }
        top_edge(list, &top_name, top, side);
        if !waterfall {
            let name = format!("{}.Apron{n}{}", b.id, if side == 1 { "L" } else { "R" });
            let end = Aabb::new(
                Vec3::new(x, lo.y, hi.z - depth),
                Vec3::new(t, top.size.y, depth),
            );
            part(
                list,
                name.clone(),
                end,
                Axis::X,
                &b.board,
                PartRole::Benchtop,
            );
            end_mitres(list, &name, end, side == 1, t, depth);
        }
        // The front drop meets each return on a vertical 45-degree mitre.
        let cx = if side == 1 { lo.x } else { hi.x };
        mitre(
            list,
            &apron,
            Aabb::new(Vec3::new(x, lo.y, hi.z - depth), Vec3::new(t, t, depth)),
            Vec3::new(cx, lo.y + t, hi.z - depth / 2.0),
        );
    }
    if b.back_overhang > 0.0 || source.iter().any(|p| p.name.ends_with(".SeenBack")) {
        rear_return(b, list, n, top, walls);
    }
    cantilever_lining(b, list, n, top, frame, walls);
}

fn end_mitres(list: &mut PartList, name: &str, end: Aabb, left: bool, t: f64, front_drop: f64) {
    let (lo, hi, c) = (end.min, end.max(), end.center());
    let inner = if left { hi.x } else { lo.x };
    mitre(
        list,
        name,
        Aabb::new(Vec3::new(lo.x, lo.y, hi.z - t), Vec3::new(t, end.size.y, t)),
        Vec3::new(inner, c.y, hi.z),
    );
    if front_drop > 0.0 {
        mitre(
            list,
            name,
            Aabb::new(
                Vec3::new(lo.x, lo.y, hi.z - front_drop),
                Vec3::new(t, t, front_drop),
            ),
            Vec3::new(inner, lo.y, hi.z - front_drop / 2.0),
        );
    }
}

pub(crate) fn mitre_waterfalls(b: &Benchtop, list: &mut PartList, n: usize) {
    let Some(top) = list
        .part(&format!("{}.Benchtop{n}", b.id))
        .map(|p| p.bounds)
    else {
        return;
    };
    for (left, suffix) in [(true, "L"), (false, "R")] {
        let name = format!("{}.Waterfall{n}{suffix}", b.id);
        if let Some(end) = list.part(&name).map(|p| p.bounds) {
            if !matches!(b.edge, EdgeProfile::MitredApron { .. }) {
                top_edge(
                    list,
                    &format!("{}.Benchtop{n}", b.id),
                    top,
                    if left { 1 } else { 2 },
                );
            }
            let drop = match b.edge {
                EdgeProfile::MitredApron { depth } => depth,
                _ => 0.0,
            };
            end_mitres(list, &name, end, left, b.thickness, drop);
            if list.part(&format!("{}.Apron{n}Back", b.id)).is_some() && drop > 0.0 {
                let t = b.thickness;
                mitre(
                    list,
                    &name,
                    Aabb::new(
                        Vec3::new(end.min.x, end.max().y - t, end.max().z - drop),
                        Vec3::new(t, t, drop),
                    ),
                    Vec3::new(
                        if left { end.max().x } else { end.min.x },
                        end.max().y,
                        end.max().z - drop / 2.0,
                    ),
                );
            }
        }
    }
}

/// Carry appliance and tap openings through the second stone skin where they overlap it.
pub(crate) fn lining_openings(list: &mut PartList) {
    use crate::part::{BorePurpose, Dir};
    let openings = list.operations.clone();
    for cap in list
        .parts
        .iter()
        .filter(|p| p.name.contains(".Underlining") && p.role == PartRole::Benchtop)
    {
        for op in &openings {
            match op {
                Operation::Cutout {
                    part,
                    bounds,
                    radius,
                    note,
                } if list.part(part).is_some_and(|p| {
                    p.role == PartRole::Benchtop && p.thickness_axis == Axis::Z
                }) && bounds.min.x < cap.bounds.max().x
                    && bounds.max().x > cap.bounds.min.x
                    && bounds.min.y < cap.bounds.max().y
                    && bounds.max().y > cap.bounds.min.y =>
                {
                    let mut cut = *bounds;
                    cut.min.z = cap.bounds.min.z;
                    cut.size.z = cap.bounds.size.z;
                    list.operations.push(Operation::Cutout {
                        part: cap.name.clone(),
                        bounds: cut,
                        radius: *radius,
                        note: note.clone(),
                    });
                }
                Operation::Bore {
                    at,
                    diameter,
                    purpose: BorePurpose::TapHole,
                    ..
                } if at.x + diameter / 2.0 > cap.bounds.min.x
                    && at.x - diameter / 2.0 < cap.bounds.max().x
                    && at.y + diameter / 2.0 > cap.bounds.min.y
                    && at.y - diameter / 2.0 < cap.bounds.max().y =>
                {
                    list.operations.push(Operation::Bore {
                        part: cap.name.clone(),
                        at: Vec3::new(at.x, at.y, cap.bounds.max().z),
                        direction: Dir::new(Axis::Z, false),
                        diameter: *diameter,
                        depth: cap.bounds.size.z,
                        purpose: BorePurpose::TapHole,
                    });
                }
                _ => {}
            }
        }
    }
}

pub(crate) fn clear_support_openings(list: &mut PartList) {
    let holes: Vec<_> = list
        .operations
        .iter()
        .filter_map(|o| match o {
            Operation::Cutout { part, bounds, .. }
                if list.part(part).is_some_and(|p| {
                    p.role == PartRole::Benchtop && p.thickness_axis == Axis::Z
                }) =>
            {
                Some(*bounds)
            }
            _ => None,
        })
        .collect();
    let mut removed = Vec::new();
    for p in &list.parts {
        if !matches!(p.role, PartRole::BenchtopSupport | PartRole::EdgeBand) {
            continue;
        }
        for h in &holes {
            let x = p.bounds.min.x.max(h.min.x);
            let y = p.bounds.min.y.max(h.min.y);
            let w = p.bounds.max().x.min(h.max().x) - x;
            let d = p.bounds.max().y.min(h.max().y) - y;
            if w <= 0.0 || d <= 0.0 {
                continue;
            }
            if w >= p.bounds.size.x - 1e-6 && d >= p.bounds.size.y - 1e-6 {
                removed.push(p.name.clone());
            } else {
                list.operations.push(Operation::Cutout {
                    part: p.name.clone(),
                    bounds: Aabb::new(
                        Vec3::new(x, y, p.bounds.min.z),
                        Vec3::new(w, d, p.bounds.size.z),
                    ),
                    radius: 0.0,
                    note: "Clear appliance opening through support".into(),
                });
            }
        }
    }
    for name in removed {
        list.parts.retain(|p| p.name != name);
        list.edge_tapes.remove(&name);
        list.warnings.push(format!(
            "{name}: appliance opening removes this support; provide support around the opening"
        ));
    }
}

/// Build the return in its own cabinet frame, then bring its parts and cuts into the run.
/// The two fascia blanks meet on a vertical mitre at the concave corner.
pub(crate) fn extend_leg(b: &Benchtop, list: &mut PartList, n: usize, base: f64, source: &[&Part]) {
    let EdgeProfile::MitredApron { depth } = b.edge else {
        return;
    };
    let name = format!("{}.Benchtop{n}Leg", b.id);
    let Some(leg) = list.part(&name).map(|p| p.bounds) else {
        return;
    };
    let main_name = format!("{}.Benchtop{n}", b.id);
    let main = list.part(&main_name).unwrap().bounds;
    let plus = leg.center().x < main.center().x;
    let t = b.thickness;
    // A quarter turn, rather than a reflection, preserves all cut orientations.
    let local = |p: Vec3| {
        if plus {
            Vec3::new(p.y, -p.x, p.z)
        } else {
            Vec3::new(-p.y, p.x, p.z)
        }
    };
    let world = |p: Vec3| {
        if plus {
            Vec3::new(-p.y, p.x, p.z)
        } else {
            Vec3::new(p.y, -p.x, p.z)
        }
    };
    let map_box = |a: Aabb, f: &dyn Fn(Vec3) -> Vec3| {
        let (p, q) = (f(a.min), f(a.max()));
        let lo = Vec3::new(p.x.min(q.x), p.y.min(q.y), p.z.min(q.z));
        Aabb::new(
            lo,
            Vec3::new((q.x - p.x).abs(), (q.y - p.y).abs(), (q.z - p.z).abs()),
        )
    };
    let mut virtual_top = map_box(leg, &local);
    // The long point of an inside mitre extends into the main piece by one slab thickness.
    if plus {
        virtual_top.size.x += t;
    } else {
        virtual_top.min.x -= t;
        virtual_top.size.x += t;
    }
    let mut virtual_b = b.clone();
    virtual_b.waterfall_left = false;
    virtual_b.waterfall_right = false;
    let mut extra = PartList::default();
    build_up(
        &virtual_b,
        &mut extra,
        &format!("{n}Leg"),
        virtual_top,
        Aabb::new(
            Vec3::new(
                virtual_top.min.x + b.side_overhang,
                virtual_top.min.y + b.front_overhang,
                base,
            ),
            Vec3::new(
                virtual_top.size.x - b.side_overhang,
                virtual_top.size.y - b.front_overhang,
                0.0,
            ),
        ),
        source,
        (!plus, plus),
    );
    for p in &mut extra.parts {
        p.bounds = map_box(p.bounds, &world);
        p.thickness_axis = match p.thickness_axis {
            Axis::X => Axis::Y,
            Axis::Y => Axis::X,
            a => a,
        };
        p.grain_axis = p.grain_axis.map(|a| match a {
            Axis::X => Axis::Y,
            Axis::Y => Axis::X,
            a => a,
        });
        let e = p.edges;
        p.edges = if plus {
            EdgeBanding {
                left: e.back,
                right: e.front,
                front: e.left,
                back: e.right,
                top: e.top,
                bottom: e.bottom,
            }
        } else {
            EdgeBanding {
                left: e.front,
                right: e.back,
                front: e.right,
                back: e.left,
                top: e.top,
                bottom: e.bottom,
            }
        };
    }
    for op in &mut extra.operations {
        if let Operation::Chamfer { bounds, at, .. } = op {
            *bounds = map_box(*bounds, &world);
            *at = world(*at);
        }
    }
    let apron = format!("{}.Apron{n}", b.id);
    let joint = if plus { leg.max().x - t } else { leg.min.x + t };
    // Trim the main front at the inside corner; its side returns stay at the outside ends.
    let trim = |bounds: &mut Aabb| {
        if plus {
            let end = bounds.max().x;
            bounds.min.x = joint;
            bounds.size.x = end - joint;
        } else {
            bounds.size.x = joint - bounds.min.x;
        }
    };
    for p in &mut list.parts {
        if p.name == apron
            || p.name == format!("{}.ShadowRail{n}", b.id)
            || p.name == format!("{}.ShadowBand{n}", b.id)
            || p.name == format!("{}.Packer{n}Front", b.id)
        {
            trim(&mut p.bounds);
        }
    }
    let old_side = if plus { main.min.x } else { main.max().x - t };
    for op in &mut list.operations {
        if let Operation::Chamfer { part, bounds, at } = op {
            if (part == &main_name && (at.y - main.min.y).abs() < 1e-6)
                || (part == &apron && (at.z - main.max().z).abs() < 1e-6)
            {
                trim(bounds);
                at.x = bounds.center().x;
            } else if part == &apron && (bounds.min.x - old_side).abs() < 1e-6 {
                bounds.min.x = if plus { joint } else { joint - t };
                at.x = joint;
            }
        }
    }
    // An exposed outside end no longer meets the front fascia at this old corner.
    let outside = format!("{}.Apron{n}{}", b.id, if plus { "L" } else { "R" });
    list.operations.retain(|op| !matches!(op,Operation::Chamfer{part,bounds,..} if part==&outside && (bounds.size.z-depth).abs()<1e-6));
    // The return fascia's concave end and the main fascia's start share one diagonal.
    let leg_apron = format!("{}.Apron{n}Leg", b.id);
    let join_bounds = Aabb::new(
        Vec3::new(
            if plus { joint } else { joint - t },
            main.min.y,
            main.max().z - depth,
        ),
        Vec3::new(t, t, depth),
    );
    mitre(
        &mut extra,
        &leg_apron,
        join_bounds,
        Vec3::new(
            if plus { joint + t } else { joint - t },
            main.min.y,
            main.max().z - depth / 2.0,
        ),
    );
    // At the concave corner the return's underside mitre also passes through the main slab.
    let corner_cuts: Vec<_> = extra
        .operations
        .iter()
        .filter_map(|op| match op {
            Operation::Chamfer { part, bounds, at }
                if part == &name && bounds.max().y > main.min.y + 1e-6 =>
            {
                Some((*bounds, *at))
            }
            _ => None,
        })
        .collect();
    for (bounds, at) in corner_cuts {
        mitre(list, &main_name, bounds, at);
    }
    list.parts.extend(extra.parts);
    list.operations.extend(extra.operations);
    list.edge_tapes.extend(extra.edge_tapes);
    list.warnings.extend(extra.warnings);
}

/// Supports follow the finished cabinet footprint, never the stone's seating overhang.
fn support_frame(
    b: &Benchtop,
    list: &mut PartList,
    n: &str,
    top: Aabb,
    frame: Aabb,
    source: &[&Part],
    walls: (bool, bool),
) {
    let board_part = source.iter().find(|p| p.role == PartRole::LeftSide);
    let board = if b.build_up.board.is_empty() {
        board_part.map_or("", |p| p.board.as_str())
    } else {
        &b.build_up.board
    };
    let ct = if b.build_up.board_thickness > 0.0 {
        b.build_up.board_thickness
    } else {
        board_part.map_or(18.0, |p| p.thickness())
    };
    let base = frame.min.z;
    let rise = top.min.z - base;
    if rise <= 0.0 {
        return;
    }
    let rail_t = ct.min(rise);
    if rise < ct {
        list.warnings.push(format!(
            "{}: plane the {ct} mm support board to {rise} mm for this shallow build-up",
            b.id
        ));
    }
    let inset = b.build_up.setback;
    // A waterfall closes the end: carry the rails into it while retaining the front/rear recess.
    let (x0, x1) = (
        frame.min.x
            + if b.waterfall_left && !walls.0 {
                0.0
            } else {
                inset
            },
        frame.max().x
            - if b.waterfall_right && !walls.1 {
                0.0
            } else {
                inset
            },
    );
    let (front, back) = (frame.min.y + inset, frame.max().y - inset);
    let width = b
        .build_up
        .strip_width
        .max(ct)
        .min((back - front) / 2.0)
        .min((x1 - x0) / 2.0);
    if width < ct || x1 <= x0 || back <= front {
        list.warnings.push(format!(
            "{}: shadow recess leaves no room for support strips",
            b.id
        ));
        return;
    }
    // Front/back rails run through. End rails fit between them, with no overlapping blanks.
    let mut rails = vec![
        (
            "",
            Axis::Y,
            false,
            Aabb::new(
                Vec3::new(x0, front, base),
                Vec3::new(x1 - x0, width, rail_t),
            ),
        ),
        (
            "Back",
            Axis::Y,
            true,
            Aabb::new(
                Vec3::new(x0, back - width, base),
                Vec3::new(x1 - x0, width, rail_t),
            ),
        ),
    ];
    if back - front > 2.0 * width + 1e-6 {
        for (suffix, on, high, x) in [
            ("Left", !b.waterfall_left && !walls.0, false, x0),
            ("Right", !b.waterfall_right && !walls.1, true, x1 - width),
        ] {
            if on {
                rails.push((
                    suffix,
                    Axis::X,
                    high,
                    Aabb::new(
                        Vec3::new(x, front + width, base),
                        Vec3::new(width, back - front - 2.0 * width, rail_t),
                    ),
                ));
            }
        }
    }
    for (suffix, axis, high, bounds) in rails {
        let rail = format!("{}.ShadowRail{n}{suffix}", b.id);
        part(
            list,
            rail.clone(),
            bounds,
            Axis::Z,
            board,
            PartRole::BenchtopSupport,
        );
        let wood = list.parts.last_mut().unwrap();
        match (axis, high) {
            (Axis::Y, false) => wood.edges.front = true,
            (Axis::Y, true) => wood.edges.back = true,
            (Axis::X, false) => wood.edges.left = true,
            _ => wood.edges.right = true,
        };
        if axis == Axis::Y {
            wood.edges.left = !walls.0;
            wood.edges.right = !walls.1;
        } else {
            wood.grain_axis = Some(Axis::Y);
        }
        if let Some(tape) = &b.build_up.edge_tape {
            list.edge_tapes.insert(rail, tape.clone());
        }
    }
    if rise <= rail_t {
        return;
    }
    let z = base + rail_t;
    let h = rise - rail_t;
    let (px0, px1) = (x0 + ct, x1 - ct);
    if px1 <= px0 {
        return;
    }
    for (suffix, y) in [("Front", front + width - ct), ("Back", back - width)] {
        part(
            list,
            format!("{}.Packer{n}{suffix}", b.id),
            Aabb::new(Vec3::new(px0, y, z), Vec3::new(px1 - px0, ct, h)),
            Axis::Y,
            board,
            PartRole::BenchtopSupport,
        );
    }
    for (suffix, on, x) in [
        ("Left", !b.waterfall_left && !walls.0, x0 + width - ct),
        ("Right", !b.waterfall_right && !walls.1, x1 - width),
    ] {
        if on && back - front > 2.0 * width {
            part(
                list,
                format!("{}.Packer{n}{suffix}", b.id),
                Aabb::new(
                    Vec3::new(x, front + width, z),
                    Vec3::new(ct, back - front - 2.0 * width, h),
                ),
                Axis::X,
                board,
                PartRole::BenchtopSupport,
            );
        }
    }
}

/// A flush stone return can cover the upper corner of a stock-thickness shadow rail.
/// Notch that corner instead of putting wood through stone or losing the rail entirely.
pub(crate) fn relieve_returns(list: &mut PartList) {
    let returns: Vec<_> = list
        .parts
        .iter()
        .filter(|p| {
            p.role == PartRole::Benchtop
                && (p.thickness_axis != Axis::Z || p.name.contains(".Underlining"))
        })
        .map(|p| p.bounds)
        .collect();
    let mut remove = Vec::new();
    for p in list
        .parts
        .iter()
        .filter(|p| matches!(p.role, PartRole::BenchtopSupport | PartRole::EdgeBand))
    {
        for r in &returns {
            let lo = Vec3::new(
                p.bounds.min.x.max(r.min.x),
                p.bounds.min.y.max(r.min.y),
                p.bounds.min.z.max(r.min.z),
            );
            let hi = Vec3::new(
                p.bounds.max().x.min(r.max().x),
                p.bounds.max().y.min(r.max().y),
                p.bounds.max().z.min(r.max().z),
            );
            if Axis::ALL.iter().any(|&a| hi.get(a) - lo.get(a) <= 1e-6) {
                continue;
            }
            let bounds = Aabb::new(lo, Vec3::new(hi.x - lo.x, hi.y - lo.y, hi.z - lo.z));
            if (bounds.volume() - p.bounds.volume()).abs() < 1e-6 {
                remove.push(p.name.clone());
            } else {
                list.operations.push(Operation::Groove {
                    part: p.name.clone(),
                    bounds,
                });
            }
        }
    }
    list.parts.retain(|p| !remove.contains(&p.name));
    list.operations.retain(|op| !matches!(op,Operation::Groove{part,..}|Operation::Cutout{part,..} if remove.contains(part)));
    for name in remove {
        list.edge_tapes.remove(&name);
    }
}

fn rear_return(b: &Benchtop, list: &mut PartList, n: &str, top: Aabb, walls: (bool, bool)) {
    let EdgeProfile::MitredApron { depth } = b.edge else {
        return;
    };
    let (lo, hi, t) = (top.min, top.max(), b.thickness);
    let name = format!("{}.Apron{n}Back", b.id);
    let bounds = Aabb::new(
        Vec3::new(lo.x, hi.y - t, hi.z - depth),
        Vec3::new(top.size.x, t, depth),
    );
    part(
        list,
        name.clone(),
        bounds,
        Axis::Y,
        &b.board,
        PartRole::Benchtop,
    );
    top_edge(list, &format!("{}.Benchtop{n}", b.id), top, 3);
    mitre(
        list,
        &name,
        Aabb::new(
            Vec3::new(lo.x, hi.y - t, hi.z - t),
            Vec3::new(top.size.x, t, t),
        ),
        Vec3::new(top.center().x, hi.y - t, hi.z),
    );
    for (left, wall, fall) in [
        (true, walls.0, b.waterfall_left),
        (false, walls.1, b.waterfall_right),
    ] {
        if wall {
            continue;
        }
        let x = if left { lo.x } else { hi.x - t };
        let corner = Aabb::new(Vec3::new(x, hi.y - t, hi.z - depth), Vec3::new(t, t, depth));
        mitre(
            list,
            &name,
            corner,
            Vec3::new(if left { lo.x } else { hi.x }, hi.y - t, hi.z - depth / 2.0),
        );
        if !fall {
            mitre(
                list,
                &format!("{}.Apron{n}{}", b.id, if left { "L" } else { "R" }),
                corner,
                Vec3::new(
                    if left { lo.x + t } else { hi.x - t },
                    hi.y,
                    hi.z - depth / 2.0,
                ),
            );
        }
    }
}

/// Band the finished rail layout after the L-return and any shortened rails are positioned.
/// The front/rear faces run through the corner; their short ends meet the side strips.
pub(crate) fn band_shadow_rails(list: &mut PartList) {
    let rails: Vec<_> = list
        .parts
        .iter()
        .filter_map(|p| list.edge_tapes.get(&p.name).map(|t| (p.clone(), t.clone())))
        .collect();
    for (rail, tape) in rails {
        let t = tape.thickness;
        let primary = if rail.grain_axis == Some(Axis::Y) {
            Axis::X
        } else {
            Axis::Y
        };
        let Some(wood) = list.parts.iter_mut().find(|p| p.name == rail.name) else {
            continue;
        };
        for axis in rail.thickness_axis.others() {
            let (low, high) = rail.edges.on_axis(axis);
            wood.bounds.min = wood
                .bounds
                .min
                .with(axis, wood.bounds.min.get(axis) + if low { t } else { 0.0 });
            wood.bounds.size = wood.bounds.size.with(
                axis,
                wood.bounds.size.get(axis) - t * (u8::from(low) + u8::from(high)) as f64,
            );
        }
        for axis in [primary, if primary == Axis::X { Axis::Y } else { Axis::X }] {
            let (low, high) = rail.edges.on_axis(axis);
            for (on, at_high) in [(low, false), (high, true)] {
                if !on {
                    continue;
                }
                let mut bounds = rail.bounds;
                bounds.size = bounds.size.with(axis, t);
                if at_high {
                    bounds.min = bounds.min.with(axis, rail.bounds.max().get(axis) - t);
                }
                let base = rail.name.replacen(".ShadowRail", ".ShadowBand", 1);
                let name = if axis == primary {
                    base
                } else {
                    let side = match (axis, at_high) {
                        (Axis::X, false) => "Left",
                        (Axis::X, true) => "Right",
                        (Axis::Y, false) => "Front",
                        _ => "Back",
                    };
                    // The primary face owns the tiny square at the outside arris.
                    let (pl, ph) = rail.edges.on_axis(primary);
                    bounds.min = bounds
                        .min
                        .with(primary, bounds.min.get(primary) + if pl { t } else { 0.0 });
                    bounds.size = bounds.size.with(
                        primary,
                        bounds.size.get(primary) - t * (u8::from(pl) + u8::from(ph)) as f64,
                    );
                    format!("{base}End{side}")
                };
                part(
                    list,
                    name,
                    bounds,
                    axis,
                    &format!("edgeband:{}:{}", tape.colour, tape.id),
                    PartRole::EdgeBand,
                );
            }
        }
    }
}

/// Close only the seating pocket. Its lower face is flush with the bottom of the built-up
/// stone, and its inboard edge bears on the cabinet's rear shadow rail.
fn cantilever_lining(
    b: &Benchtop,
    list: &mut PartList,
    n: &str,
    top: Aabb,
    frame: Aabb,
    walls: (bool, bool),
) {
    let EdgeProfile::MitredApron { depth } = b.edge else {
        return;
    };
    if b.back_overhang <= 0.0 || top.max().y <= frame.max().y + 1e-6 {
        return;
    }
    let thickness = b.thickness.min(depth - b.thickness);
    if thickness <= 1e-6 {
        return;
    } // A one-slab profile already has a closed underside.
    let Some(rail) = list
        .part(&format!("{}.ShadowRail{n}Back", b.id))
        .map(|p| p.bounds)
    else {
        return;
    };
    let lo = Vec3::new(top.min.x, rail.min.y, top.max().z - depth);
    let bounds = Aabb::new(lo, Vec3::new(top.size.x, top.max().y - lo.y, thickness));
    let name = format!("{}.Underlining{n}", b.id);
    part(
        list,
        name.clone(),
        bounds,
        Axis::Z,
        &b.board,
        PartRole::Benchtop,
    );
    if thickness < b.thickness - 1e-6 {
        list.notes.push(format!("{name}: underside thickness {thickness} mm to fit the {depth} mm edge; reduce from {} mm stone",b.thickness));
    }
    // Upper outside corner off the lower slab, and the complementary lower inside corner
    // off the rear drop: both visible faces meet at the lower long point.
    let hi = bounds.max();
    let cut = Aabb::new(
        Vec3::new(lo.x, hi.y - b.thickness, lo.z),
        Vec3::new(bounds.size.x, b.thickness, thickness),
    );
    mitre(list, &name, cut, Vec3::new(bounds.center().x, hi.y, hi.z));
    mitre(
        list,
        &format!("{}.Apron{n}Back", b.id),
        cut,
        Vec3::new(bounds.center().x, hi.y - b.thickness, lo.z),
    );
    for (left, wall, fall) in [
        (true, walls.0, b.waterfall_left),
        (false, walls.1, b.waterfall_right),
    ] {
        if wall {
            continue;
        }
        let x = if left { lo.x } else { hi.x - b.thickness };
        let cut = Aabb::new(
            Vec3::new(x, lo.y, lo.z),
            Vec3::new(b.thickness, bounds.size.y, thickness),
        );
        mitre(
            list,
            &name,
            cut,
            Vec3::new(if left { lo.x } else { hi.x }, bounds.center().y, hi.z),
        );
        mitre(
            list,
            &format!(
                "{}.{}{n}{}",
                b.id,
                if fall { "Waterfall" } else { "Apron" },
                if left { "L" } else { "R" }
            ),
            cut,
            Vec3::new(
                if left {
                    lo.x + b.thickness
                } else {
                    hi.x - b.thickness
                },
                bounds.center().y,
                lo.z,
            ),
        );
    }
    // The rear riser now rests on the lining. A separate lower piece fills any packing gap
    // between stock-thickness horizontal rail and the underside's bearing plane.
    let packer_name = format!("{}.Packer{n}Back", b.id);
    if let Some(i) = list.parts.iter().position(|p| p.name == packer_name) {
        let old = list.parts.remove(i);
        if old.bounds.min.z < lo.z - 1e-6 {
            let mut bearer = old.clone();
            bearer.name = format!("{}.UnderliningBearer{n}", b.id);
            bearer.bounds.size.z = lo.z - old.bounds.min.z;
            list.parts.push(bearer);
        }
        if old.bounds.max().z > hi.z + 1e-6 {
            let mut upper = old.clone();
            upper.bounds.min.z = hi.z;
            upper.bounds.size.z = old.bounds.max().z - hi.z;
            list.parts.push(upper);
        }
    }
}
