//! Whole-part-list moves for corner cabinets built from straight legs: a quarter turn, a mirror,
//! renaming, and opening a leg's end where it meets the other leg. Every part stays an
//! axis-aligned box: a quarter turn about Z swaps X and Y.

use crate::geom::{Aabb, Axis, Vec3};
use crate::part::{Dir, EdgeBanding, Operation, PartList, PartRole};

/// Applies `point`, `axis` and `dir` to every part, operation and hardware placement, and
/// `edges` to every part's banding.
fn transform(
    list: &mut PartList,
    point: impl Fn(Vec3) -> Vec3,
    axis: impl Fn(Axis) -> Axis,
    dir: impl Fn(Dir) -> Dir,
    edges: impl Fn(EdgeBanding) -> EdgeBanding,
) {
    let boxed = |b: Aabb| {
        let (p, q) = (point(b.min), point(b.max()));
        let min = Vec3::new(p.x.min(q.x), p.y.min(q.y), p.z.min(q.z));
        Aabb::new(
            min,
            Vec3::new(
                p.x.max(q.x) - min.x,
                p.y.max(q.y) - min.y,
                p.z.max(q.z) - min.z,
            ),
        )
    };
    for p in &mut list.parts {
        p.bounds = boxed(p.bounds);
        p.thickness_axis = axis(p.thickness_axis);
        p.grain_axis = p.grain_axis.map(&axis);
        p.edges = edges(p.edges);
    }
    for op in &mut list.operations {
        match op {
            Operation::Bore { at, direction, .. } => {
                *at = point(*at);
                *direction = dir(*direction);
            }
            Operation::Round { at, .. } => *at = point(*at),
            Operation::BiscuitSlot { bounds, along, .. } => {
                *bounds = boxed(*bounds);
                *along = axis(*along);
            }
            Operation::Groove { bounds, .. } | Operation::Mortise { bounds, .. } => {
                *bounds = boxed(*bounds)
            }
        }
    }
    for h in &mut list.hardware {
        h.at = point(h.at);
    }
}

fn swap_xy(a: Axis) -> Axis {
    match a {
        Axis::X => Axis::Y,
        Axis::Y => Axis::X,
        Axis::Z => Axis::Z,
    }
}

/// Turns a leg built facing −Y a quarter turn so it faces +X: its front (Y = 0) lands on
/// X = `depth`, its back on X = 0, and its width runs along +Y from `y0`.
pub fn turn_to_face_x(list: &mut PartList, depth: f64, y0: f64) {
    transform(
        list,
        |p| Vec3::new(depth - p.y, y0 + p.x, p.z),
        swap_xy,
        |d| match d.axis {
            Axis::X => Dir::new(Axis::Y, d.positive),
            Axis::Y => Dir::new(Axis::X, !d.positive),
            Axis::Z => d,
        },
        |e| EdgeBanding {
            front: e.left,
            back: e.right,
            left: e.back,
            right: e.front,
            ..e
        },
    );
}

/// Mirrors a cabinet left to right across its `width`; left and right sides swap names, and so
/// do handed hardware items (`….left` / `….right`).
pub fn mirror_x(list: &mut PartList, width: f64) {
    transform(
        list,
        |p| Vec3::new(width - p.x, p.y, p.z),
        |a| a,
        |d| match d.axis {
            Axis::X => Dir::new(Axis::X, !d.positive),
            _ => d,
        },
        |e| EdgeBanding {
            left: e.right,
            right: e.left,
            ..e
        },
    );
    let (l, r) = (PartRole::LeftSide.code(), PartRole::RightSide.code());
    rename(list, |name| {
        if name.contains(l) {
            name.replace(l, r)
        } else {
            name.replace(r, l)
        }
    });
    for h in &mut list.hardware {
        if let Some(base) = h.item.strip_suffix(".left") {
            h.item = format!("{base}.right");
            h.description = h.description.replace("left", "right");
        } else if let Some(base) = h.item.strip_suffix(".right") {
            h.item = format!("{base}.left");
            h.description = h.description.replace("right", "left");
        }
    }
    for p in &mut list.parts {
        p.role = match p.role {
            PartRole::LeftSide => PartRole::RightSide,
            PartRole::RightSide => PartRole::LeftSide,
            other => other,
        };
    }
}

/// Renames every part and every reference to it.
pub fn rename(list: &mut PartList, f: impl Fn(&str) -> String) {
    for p in &mut list.parts {
        p.name = f(&p.name);
    }
    for op in &mut list.operations {
        match op {
            Operation::Bore { part, .. }
            | Operation::Groove { part, .. }
            | Operation::Mortise { part, .. }
            | Operation::Round { part, .. } => *part = f(part),
            Operation::BiscuitSlot { parts, .. } => {
                for p in parts {
                    *p = f(p);
                }
            }
        }
    }
    for h in &mut list.hardware {
        h.part = f(&h.part);
    }
}

/// Opens a straight leg's end where it meets the other leg: drops the side `side` and every
/// operation in it or across its joints, and runs the parts that stopped at it (bottom, rails,
/// back, their grooves) on to `end` (the leg's outer face there, along X).
pub fn open_end(list: &mut PartList, side: &str, end: f64) {
    let Some(gone) = list.parts.iter().find(|p| p.name == side).map(|p| p.bounds) else {
        return;
    };
    let inner = gone.min.x;
    let touches = |b: &Aabb| {
        const EPS: f64 = 1e-6;
        Axis::ALL.iter().all(|&a| {
            b.min.get(a) <= gone.max().get(a) + EPS && gone.min.get(a) <= b.max().get(a) + EPS
        })
    };
    let at_point = |p: &Vec3| touches(&Aabb::new(*p, Vec3::default()));
    list.parts.retain(|p| p.name != side);
    list.operations.retain(|op| match op {
        Operation::Bore { part, at, .. } => part != side && !at_point(at),
        Operation::BiscuitSlot { parts, .. } => !parts.iter().any(|p| p == side),
        Operation::Groove { part, .. } | Operation::Round { part, .. } => part != side,
        Operation::Mortise { part, bounds, .. } => part != side && !touches(bounds),
    });
    list.hardware.retain(|h| h.part != side);
    let run_on = |b: &mut Aabb| {
        let max = b.max().x;
        if max > inner - 1e-6 && max < end - 1e-6 && b.min.x < inner {
            b.size.x = end - b.min.x;
        }
    };
    for p in &mut list.parts {
        run_on(&mut p.bounds);
    }
    for op in &mut list.operations {
        if let Operation::Groove { bounds, .. } = op {
            run_on(bounds);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::FrontLayout;
    use crate::{Cabinet, CabinetKind, build_cabinet};

    fn leg() -> PartList {
        let mut cab = Cabinet::new("C01", CabinetKind::Base);
        cab.fronts = FrontLayout::Open;
        build_cabinet(&cab).unwrap()
    }

    #[test]
    fn a_quarter_turn_keeps_every_size_and_faces_the_front_along_x() {
        let before = leg();
        let mut after = before.clone();
        turn_to_face_x(&mut after, 580.0, -600.0);
        for (a, b) in before.parts.iter().zip(&after.parts) {
            assert!((a.thickness() - b.thickness()).abs() < 1e-9, "{}", a.name);
            assert_eq!(a.finished_size(), b.finished_size(), "{}", a.name);
        }
        // The right side (end of the leg) now closes the leg at Y = 0.
        let side = after.part("C01.RightSide").unwrap();
        assert!((side.bounds.max().y - 0.0).abs() < 1e-9);
        assert_eq!(side.thickness_axis, Axis::Y);
        // The bottom's front edge faces +X at the leg's front.
        let bottom = after.part("C01.Bottom").unwrap();
        assert!((bottom.bounds.max().x - 580.0).abs() < 1e-9);
    }

    #[test]
    fn opening_an_end_drops_the_side_and_its_joints_and_runs_the_bottom_on() {
        let mut l = leg();
        let w = 600.0;
        open_end(&mut l, "C01.RightSide", w);
        assert!(l.part("C01.RightSide").is_none());
        assert!((l.part("C01.Bottom").unwrap().bounds.max().x - w).abs() < 1e-9);
        // Nothing is left in or across the missing side's joints.
        for op in &l.operations {
            if let Operation::BiscuitSlot { parts, .. } = op {
                assert!(!parts.iter().any(|p| p == "C01.RightSide"));
            }
        }
        assert!(crate::collisions::check(&l).is_empty());
    }

    #[test]
    fn mirroring_swaps_the_sides() {
        let mut l = leg();
        let left_x = l.part("C01.LeftSide").unwrap().bounds.min.x;
        mirror_x(&mut l, 600.0);
        let right = l.part("C01.RightSide").unwrap();
        assert_eq!(right.role, PartRole::RightSide);
        assert!((right.bounds.max().x - (600.0 - left_x)).abs() < 1e-9);
    }
}
