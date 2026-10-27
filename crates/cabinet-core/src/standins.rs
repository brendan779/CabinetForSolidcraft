//! Stand-in hardware models: simple solids that show where a hinge or drawer runner sits and how
//! much room it takes, until the supplier's own models are imported (BLUM-9).
//!
//! The shapes follow the machining they sit in (cup bores, plate holes) and the panels around
//! them. Their own proportions are drawn, not ordered: nothing here is a supplier dimension.

use crate::geom::{Aabb, Axis, Vec3};
use crate::part::{BorePurpose, Dir, HardwarePlacement, Operation, Part, PartList, PartRole};

/// How far a hinge cup stands proud of the door's back face.
const CUP_PROUD: f64 = 2.0;
/// The hinge arm and mounting plate as boxes: arm width (Z), plate thickness off the side (X),
/// plate length past its outer holes (Z) and width either side of the hole row (Y).
const ARM_WIDTH: f64 = 14.0;
const PLATE_THICKNESS: f64 = 10.0;
const PLATE_END: f64 = 6.0;
const PLATE_HALF_WIDTH: f64 = 8.0;
/// A drawer runner as a bar along the inside of each side: thickness (X), height (Z), how far it
/// sits above the bottom of its front, and the gap it leaves at the back.
const RUNNER_THICKNESS: f64 = 13.0;
const RUNNER_HEIGHT: f64 = 20.0;
const RUNNER_LIFT: f64 = 10.0;
const RUNNER_BACK_GAP: f64 = 10.0;
/// The drawer's box side above the runner: thin, and short of the front's top edge.
const BOX_SIDE_THICKNESS: f64 = 10.0;
/// A concealed runner under a wooden box, drawn inboard of the box side.
const WOOD_RUNNER_WIDTH: f64 = 20.0;
const BOX_SIDE_TOP_GAP: f64 = 40.0;
const BOX_SIDE_MIN_HEIGHT: f64 = 40.0;
/// An adjustable plinth leg (Häfele AXILO 78 class): foot, column and mounting plate. Drawn, not
/// ordered: the owner's Häfele documents give the real sizes (HAF-1).
const LEG_FOOT_DIAMETER: f64 = 50.0;
const LEG_FOOT_HEIGHT: f64 = 8.0;
const LEG_COLUMN_DIAMETER: f64 = 30.0;
const LEG_PLATE: f64 = 60.0;
const LEG_PLATE_THICKNESS: f64 = 4.0;

/// One solid of a stand-in.
#[derive(Clone, Debug, PartialEq)]
pub enum Shape {
    Box(Aabb),
    /// `base` is the centre of the end it starts from; it runs `height` along `axis`.
    Cylinder {
        base: Vec3,
        axis: Dir,
        diameter: f64,
        height: f64,
    },
}

/// One stand-in body, built from its shapes joined together.
#[derive(Clone, Debug, PartialEq)]
pub struct StandIn {
    /// `B01.Hinge2`, `B01.Runner1L`: numbered per cabinet, stable across rebuilds.
    pub name: String,
    /// The hardware item it stands in for.
    pub item: String,
    pub shapes: Vec<Shape>,
}

impl Shape {
    pub fn bounds(&self) -> Aabb {
        match self {
            Shape::Box(b) => *b,
            Shape::Cylinder {
                base,
                axis,
                diameter,
                height,
            } => {
                let end = base.with(axis.axis, base.get(axis.axis) + axis.sign() * height);
                let r = diameter / 2.0;
                let mut b = boxed(*base, end);
                for a in axis.axis.others() {
                    b.min = b.min.with(a, b.min.get(a) - r);
                    b.size = b.size.with(a, 2.0 * r);
                }
                b
            }
        }
    }
}

impl StandIn {
    /// The box around all its shapes.
    pub fn bounds(&self) -> Aabb {
        let mut it = self.shapes.iter().map(Shape::bounds);
        let first = it
            .next()
            .unwrap_or(Aabb::new(Vec3::default(), Vec3::default()));
        it.fold(first, |acc, b| {
            let (a0, a1, b1) = (acc.min, acc.max(), b.max());
            boxed(
                Vec3::new(a0.x.min(b.min.x), a0.y.min(b.min.y), a0.z.min(b.min.z)),
                Vec3::new(a1.x.max(b1.x), a1.y.max(b1.y), a1.z.max(b1.z)),
            )
        })
    }
}

/// Stand-ins for the hinges and drawer runners in `list`, in the order of its hardware.
pub fn stand_ins(list: &PartList) -> Vec<StandIn> {
    let mut out = Vec::new();
    let (mut hinges, mut drawers, mut legs) = (0, 0, 0);
    for h in &list.hardware {
        if h.item.starts_with("blum.clip-top") {
            if let Some(shapes) = hinge(list, h) {
                hinges += 1;
                out.push(StandIn {
                    name: format!("{}.Hinge{hinges}", list.cabinet),
                    item: h.item.clone(),
                    shapes,
                });
            }
        } else if crate::cad::is_leg(&h.item) {
            if let Some(shapes) = leg(list, h) {
                legs += 1;
                out.push(StandIn {
                    name: format!("{}.Leg{legs}", list.cabinet),
                    item: h.item.clone(),
                    shapes,
                });
            }
        } else if crate::hardware::item(&h.item)
            .is_ok_and(|i| i.kind == cabinet_catalog::HardwareKind::Runner)
            && let Some(sides) = runners(list, h)
        {
            drawers += 1;
            for (suffix, shapes) in ["L", "R"].into_iter().zip(sides) {
                out.push(StandIn {
                    name: format!("{}.Runner{drawers}{suffix}", list.cabinet),
                    item: h.item.clone(),
                    shapes,
                });
            }
        }
    }
    out
}

/// An adjustable leg from the floor to the carcass bottom: a foot, a column and a mounting plate
/// under the bottom panel.
fn leg(list: &PartList, h: &HardwarePlacement) -> Option<Vec<Shape>> {
    let bottom = list.part(&h.part)?.bounds.min.z;
    if bottom <= LEG_FOOT_HEIGHT + LEG_PLATE_THICKNESS {
        return None;
    }
    let (x, y) = (h.at.x, h.at.y);
    let up = Dir::new(Axis::Z, true);
    Some(vec![
        Shape::Cylinder {
            base: Vec3::new(x, y, 0.0),
            axis: up,
            diameter: LEG_FOOT_DIAMETER,
            height: LEG_FOOT_HEIGHT,
        },
        Shape::Cylinder {
            base: Vec3::new(x, y, LEG_FOOT_HEIGHT),
            axis: up,
            diameter: LEG_COLUMN_DIAMETER,
            height: bottom - LEG_FOOT_HEIGHT - LEG_PLATE_THICKNESS,
        },
        Shape::Box(boxed(
            Vec3::new(
                x - LEG_PLATE / 2.0,
                y - LEG_PLATE / 2.0,
                bottom - LEG_PLATE_THICKNESS,
            ),
            Vec3::new(x + LEG_PLATE / 2.0, y + LEG_PLATE / 2.0, bottom),
        )),
    ])
}

fn side(list: &PartList, role: PartRole) -> Option<&Part> {
    list.by_role(role).next()
}

/// Cup in its bore, an arm across to the side, and the mounting plate on its two holes.
fn hinge(list: &PartList, h: &HardwarePlacement) -> Option<Vec<Shape>> {
    let door = list.part(&h.part)?;
    let near = |a: &Vec3, b: &Vec3| {
        (a.x - b.x).abs() < 1e-6 && (a.y - b.y).abs() < 1e-6 && (a.z - b.z).abs() < 1e-6
    };
    let (diameter, depth) = list.operations.iter().find_map(|op| match op {
        Operation::Bore {
            part,
            at,
            diameter,
            depth,
            purpose: BorePurpose::HingeCup,
            ..
        } if *part == h.part && near(at, &h.at) => Some((*diameter, *depth)),
        _ => None,
    })?;
    let left = h.at.x < door.bounds.center().x;
    let side = side(
        list,
        if left {
            PartRole::LeftSide
        } else {
            PartRole::RightSide
        },
    )?;
    let inner = if left {
        side.bounds.max().x
    } else {
        side.bounds.min.x
    };
    // The plate's two holes nearest the cup, on the inside of the hinge side.
    let mut holes: Vec<Vec3> = list
        .operations
        .iter()
        .filter_map(|op| match op {
            Operation::Bore {
                part,
                at,
                purpose: BorePurpose::HingePlate,
                ..
            } if *part == side.name => Some(*at),
            _ => None,
        })
        .collect();
    holes.sort_by(|a, b| (a.z - h.at.z).abs().total_cmp(&(b.z - h.at.z).abs()));
    holes.truncate(2);
    let plate_y = holes.first()?.y;
    let (z0, z1) = holes
        .iter()
        .fold((h.at.z, h.at.z), |(lo, hi), p| (lo.min(p.z), hi.max(p.z)));
    let face = h.at.y;
    let (plate_x0, plate_x1) = if left {
        (inner, inner + PLATE_THICKNESS)
    } else {
        (inner - PLATE_THICKNESS, inner)
    };
    let (arm_x0, arm_x1) = if left {
        (plate_x0, h.at.x)
    } else {
        (h.at.x, plate_x1)
    };
    Some(vec![
        Shape::Cylinder {
            base: h.at.with(Axis::Y, face - depth),
            axis: Dir::new(Axis::Y, true),
            diameter,
            height: depth + CUP_PROUD,
        },
        Shape::Box(boxed(
            Vec3::new(arm_x0, face, h.at.z - ARM_WIDTH / 2.0),
            Vec3::new(arm_x1, plate_y, h.at.z + ARM_WIDTH / 2.0),
        )),
        Shape::Box(boxed(
            Vec3::new(plate_x0, plate_y - PLATE_HALF_WIDTH, z0 - PLATE_END),
            Vec3::new(plate_x1, plate_y + PLATE_HALF_WIDTH, z1 + PLATE_END),
        )),
    ])
}

/// A runner along the inside of each side under the drawer's front, with the drawer's box side
/// standing on it; left then right.
fn runners(list: &PartList, h: &HardwarePlacement) -> Option<[Vec<Shape>; 2]> {
    let front = list.part(&h.part)?;
    if let Some(shapes) = wood_runners(list, &h.part) {
        return Some(shapes);
    }
    let (l, r) = (
        side(list, PartRole::LeftSide)?,
        side(list, PartRole::RightSide)?,
    );
    let back = list
        .by_role(PartRole::Back)
        .map(|b| b.bounds.min.y)
        .fold(l.bounds.max().y, f64::min);
    let (y0, y1) = (front.bounds.max().y.max(0.0), back - RUNNER_BACK_GAP);
    // Inside the carcass: an overlay front reaches over the bottom and the top or rails.
    let floor = list
        .by_role(PartRole::Bottom)
        .map(|b| b.bounds.max().z)
        .fold(f64::MIN, f64::max);
    let ceiling = list
        .parts
        .iter()
        .filter(|p| {
            matches!(
                p.role,
                PartRole::Top | PartRole::FrontRail | PartRole::BackRail
            )
        })
        .map(|p| p.bounds.min.z)
        .fold(f64::MAX, f64::min);
    let z0 = front.bounds.min.z.max(floor) + RUNNER_LIFT;
    let z1 = z0 + RUNNER_HEIGHT;
    let top = (front.bounds.max().z.min(ceiling) - BOX_SIDE_TOP_GAP).max(z1 + BOX_SIDE_MIN_HEIGHT);
    let one = |inner: f64, sign: f64| {
        let runner = boxed(
            Vec3::new(inner, y0, z0),
            Vec3::new(inner + sign * RUNNER_THICKNESS, y1, z1),
        );
        let box_side = boxed(
            Vec3::new(
                inner + sign * (RUNNER_THICKNESS - BOX_SIDE_THICKNESS),
                y0,
                z1,
            ),
            Vec3::new(inner + sign * RUNNER_THICKNESS, y1, top),
        );
        vec![Shape::Box(runner), Shape::Box(box_side)]
    };
    Some([one(l.bounds.max().x, 1.0), one(r.bounds.min.x, -1.0)])
}

/// Under a wooden box the runners sit in the recess below its base, along each side's inner face.
fn wood_runners(list: &PartList, front: &str) -> Option<[Vec<Shape>; 2]> {
    let n = front.rsplit_once(PartRole::DrawerFront.code())?.1;
    let cab = &list.cabinet;
    let left = list.part(&format!("{cab}.{}{n}", PartRole::DrawerLeftSide.code()))?;
    let right = list.part(&format!("{cab}.{}{n}", PartRole::DrawerRightSide.code()))?;
    let base = list.part(&format!("{cab}.{}{n}", PartRole::DrawerBottom.code()))?;
    let (z0, z1) = (left.bounds.min.z, base.bounds.min.z);
    let (y0, y1) = (left.bounds.min.y, left.bounds.max().y);
    let one = |inner: f64, sign: f64| {
        vec![Shape::Box(boxed(
            Vec3::new(inner, y0, z0),
            Vec3::new(inner + sign * WOOD_RUNNER_WIDTH, y1, z1),
        ))]
    };
    Some([one(left.bounds.max().x, 1.0), one(right.bounds.min.x, -1.0)])
}

/// The box between two opposite corners, in either order.
fn boxed(a: Vec3, b: Vec3) -> Aabb {
    let min = Vec3::new(a.x.min(b.x), a.y.min(b.y), a.z.min(b.z));
    let max = Vec3::new(a.x.max(b.x), a.y.max(b.y), a.z.max(b.z));
    Aabb::new(min, Vec3::new(max.x - min.x, max.y - min.y, max.z - min.z))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{DoorOverlay, DrawerSystem, FrontLayout, HingeSide};
    use crate::{Cabinet, CabinetKind, build_cabinet};

    #[test]
    fn every_hinge_gets_a_stand_in_reaching_from_its_cup_to_the_hinge_side() {
        let mut cab = Cabinet::new("B01", CabinetKind::Base);
        cab.fronts = FrontLayout::Door {
            hinge_side: HingeSide::Right,
        };
        let list = build_cabinet(&cab).unwrap();
        let hinges = list
            .hardware
            .iter()
            .filter(|h| h.item.starts_with("blum.clip-top"))
            .count();
        let s: Vec<_> = stand_ins(&list)
            .into_iter()
            .filter(|s| s.name.contains("Hinge"))
            .collect();
        assert_eq!(s.len(), hinges);
        assert_eq!(s[0].name, "B01.Hinge1");
        let side = list.part("B01.RightSide").unwrap();
        let door = list.part("B01.Door1").unwrap();
        for h in &s {
            let b = h.bounds();
            // Into the door's back, never through its face, and onto the side's inner face.
            assert!(b.min.y > door.bounds.min.y && b.min.y < door.bounds.max().y);
            // The plate stands on the side's inner face; the cup may sit in front of the side.
            let inner = side.bounds.min.x;
            let Shape::Box(plate) = h.shapes[2] else {
                panic!("the plate is a box")
            };
            assert!((plate.max().x - inner).abs() < 1e-6);
            assert!(b.max().y > 37.0 - 1e-6);
        }
    }

    #[test]
    fn a_drawer_bank_gets_a_runner_each_side_of_every_drawer_inside_the_carcass() {
        let list = build_cabinet(&Cabinet::new("D01", CabinetKind::DrawerBank)).unwrap();
        let drawers = list.by_role(PartRole::DrawerFront).count();
        let s: Vec<_> = stand_ins(&list)
            .into_iter()
            .filter(|s| s.name.contains("Runner"))
            .collect();
        assert_eq!(s.len(), 2 * drawers);
        assert_eq!(s[1].name, "D01.Runner1R");
        let (l, r) = (
            list.part("D01.LeftSide").unwrap(),
            list.part("D01.RightSide").unwrap(),
        );
        for runner in &s {
            let b = runner.bounds();
            assert!(b.min.x >= l.bounds.max().x - 1e-6);
            assert!(b.max().x <= r.bounds.min.x + 1e-6);
            assert!(b.max().y < l.bounds.max().y);
            assert!(b.min.y >= 0.0);
        }
    }

    #[test]
    fn stand_ins_stay_out_of_every_panel_except_the_cup_in_its_bore() {
        for kind in CabinetKind::ALL {
            for (overlay, drawers) in [DoorOverlay::Full, DoorOverlay::Half, DoorOverlay::Inset]
                .into_iter()
                .flat_map(|o| {
                    [
                        DrawerSystem::Legrabox,
                        DrawerSystem::Merivobox,
                        DrawerSystem::Tandembox,
                        DrawerSystem::Movento,
                        DrawerSystem::Tandem,
                    ]
                    .map(|d| (o, d))
                })
            {
                let mut cab = Cabinet::new("C01", kind);
                cab.standard.overlay = overlay;
                cab.standard.drawer_system = drawers;
                let list = build_cabinet(&cab).unwrap();
                for s in stand_ins(&list) {
                    for shape in &s.shapes {
                        let Shape::Box(b) = shape else { continue };
                        for p in &list.parts {
                            assert!(
                                !b.overlaps(&p.bounds),
                                "{kind:?} {overlay:?} {drawers:?}: {} runs into {}",
                                s.name,
                                p.name
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn an_open_cabinet_has_no_stand_ins() {
        let mut cab = Cabinet::new("B01", CabinetKind::Base);
        cab.fronts = FrontLayout::Open;
        assert!(
            stand_ins(&build_cabinet(&cab).unwrap())
                .iter()
                .all(|s| s.name.contains("Leg"))
        );
    }

    #[test]
    fn base_cabinets_on_legs_stand_on_the_floor_and_reach_the_carcass_bottom() {
        let list = build_cabinet(&Cabinet::new("B01", CabinetKind::Base)).unwrap();
        let bottom = list.part("B01.Bottom").unwrap().bounds.min.z;
        let legs: Vec<_> = stand_ins(&list)
            .into_iter()
            .filter(|s| s.name.contains("Leg"))
            .collect();
        assert_eq!(legs.len(), 4);
        for l in &legs {
            let b = l.bounds();
            assert!(b.min.z.abs() < 1e-9, "{} starts at the floor", l.name);
            assert!(
                (b.max().z - bottom).abs() < 1e-9,
                "{} reaches the bottom",
                l.name
            );
        }
        // Wall cabinets hang: no legs.
        let wall = build_cabinet(&Cabinet::new("W01", CabinetKind::Wall)).unwrap();
        assert!(!stand_ins(&wall).iter().any(|s| s.name.contains("Leg")));
    }
}
