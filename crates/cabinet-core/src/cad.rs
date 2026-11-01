//! Where the suppliers' own 3D models go (BLUM-9).
//!
//! Blum draws its CAD in the mounted position, in one frame for every product: X = 0 is the inner
//! face of the cabinet side the part is fixed to (hinges and plates sit at negative X, inside the
//! cabinet), Y is up, Z points to the front, and the cabinet's front edge is at Z =
//! [`BLUM_FRONT_PLANE`]. A part's height reference is Y = 0: the hinge's centre line, or the
//! underside of the drawer base for runners and box sides. Hinges and plates come as one model;
//! runners, profiles and drawer sides as a left and a right one.
//!
//! So a model is placed by mapping that frame onto ours: the side's inner face, the cabinet front
//! (y = 0) and the part's height. Nothing here needs the model files; the app and the SolidCraft
//! bridge look them up by article number.
//!
//! Häfele draws an AXILO 78 leg set upright in our own axes (Z up), with the leg's axis at X = Y
//! = 0: the tube hangs from Z = 0 down its length and the mounting plate sits above it. The tube
//! clips up into the plate, so the tube stands on the floor and the plate is moved up under the
//! carcass bottom.

use serde::{Deserialize, Serialize};

use crate::geom::Vec3;
use crate::part::{PartList, PartRole};

/// The cabinet front edge in Blum's CAD frame (Z), measured from CLIP top 110° 71B3550: its cup
/// mouth (the door's back face, which a full overlay door closes onto the front edge) is at
/// Z = 38.3. Runners and LEGRABOX profiles in the same frame start 3–5 mm behind it.
pub const BLUM_FRONT_PLANE: f64 = 38.3;

/// The cabinet side a part is fixed to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    Left,
    Right,
}

impl Side {
    /// The cabinet's inside is towards +x from the left side and −x from the right.
    pub fn interior_sign(self) -> f64 {
        match self {
            Side::Left => 1.0,
            Side::Right => -1.0,
        }
    }
}

/// How a supplier's model frame maps onto the cabinet's.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Frame {
    /// Blum's mounted frame, turned onto [`CadPlacement::side`] ([`transform`]).
    #[default]
    Blum,
    /// Our own axes: the model's origin goes to [`CadPlacement::origin`] as it is.
    Upright,
    /// Turned: the model's X along `x`, its Y along `y` (unit vectors in the cabinet frame), its
    /// Z along their cross product. Our generic handles: holes on X, standing out towards -Y.
    Oriented { x: Vec3, y: Vec3 },
}

/// One supplier model to place.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CadPlacement {
    /// `B01.Hinge1`, `D01.Runner2L`: stable across rebuilds.
    pub name: String,
    /// The catalog item it shows.
    pub item: String,
    /// The supplier's article number, which names the model file.
    pub article: String,
    pub side: Side,
    /// The cabinet-frame point the model's frame origin maps to. For Blum: the side's inner face
    /// (x), the cabinet front (y = 0) and the part's height (z).
    pub origin: Vec3,
    #[serde(default)]
    pub frame: Frame,
}

/// A plinth leg, generic or a Häfele AXILO 78 set.
pub fn is_leg(item: &str) -> bool {
    item.starts_with("hafele.leg") || item.starts_with("hafele.axilo.set.")
}

/// Every hinge, mounting plate, runner, LEGRABOX profile, drawer side and AXILO leg in `list` that
/// has an article number, in the order of its hardware.
pub fn cad_placements(list: &PartList) -> Vec<CadPlacement> {
    let mut out = Vec::new();
    let mut count = std::collections::BTreeMap::<&str, usize>::new();
    let mut next = |kind: &'static str| {
        let n = count.entry(kind).or_default();
        *n += 1;
        *n
    };
    let left = list.by_role(PartRole::LeftSide).next();
    let right = list.by_role(PartRole::RightSide).next();
    let (Some(left), Some(right)) = (left, right) else {
        return out;
    };
    let inner = |side: Side| match side {
        Side::Left => left.bounds.max().x,
        Side::Right => right.bounds.min.x,
    };
    // Inner faces of the bay at x: between the sides, or a divider in a sectioned cabinet.
    let bay = |x: f64| {
        list.bay_around(x)
            .map(|(l, r)| (l.bounds.max().x, r.bounds.min.x))
            .unwrap_or((inner(Side::Left), inner(Side::Right)))
    };
    let face = |x: f64, side: Side| match side {
        Side::Left => bay(x).0,
        Side::Right => bay(x).1,
    };
    for h in &list.hardware {
        let Ok(item) = crate::hardware::item(&h.item) else {
            continue;
        };
        if item.article.is_empty() {
            continue;
        }
        let place = |name: String, side: Side, x: f64, z: f64| CadPlacement {
            name,
            item: h.item.clone(),
            article: item.article.clone(),
            side,
            origin: Vec3::new(face(x, side), 0.0, z),
            frame: Frame::Blum,
        };
        let side_of = |x: f64| {
            let (l, r) = bay(x);
            if x < (l + r) / 2.0 {
                Side::Left
            } else {
                Side::Right
            }
        };
        if h.item.starts_with("handle.") && h.item != "handle.edge-pull" {
            // Between its screw holes on the front's face, the bar along them, standing out of
            // the face the screws come through.
            let holes: Vec<(Vec3, crate::part::Dir, f64)> = list
                .operations
                .iter()
                .filter_map(|o| match o {
                    crate::part::Operation::Bore {
                        part,
                        at,
                        direction,
                        depth,
                        purpose: crate::part::BorePurpose::HandleScrew,
                        ..
                    } if *part == h.part => Some((*at, *direction, *depth)),
                    _ => None,
                })
                .collect();
            let (Some(&(first, dir, depth)), Some(&(last, ..))) = (holes.first(), holes.last())
            else {
                continue;
            };
            let n = next("Handle");
            let sign = dir.sign();
            let mut origin = Vec3::new(
                (first.x + last.x) / 2.0,
                (first.y + last.y) / 2.0,
                (first.z + last.z) / 2.0,
            );
            origin = origin.with(dir.axis, first.get(dir.axis) + sign * depth);
            let d = Vec3::new(last.x - first.x, last.y - first.y, last.z - first.z);
            let len = (d.x * d.x + d.y * d.y + d.z * d.z).sqrt();
            let x = if len > 1e-6 {
                Vec3::new(d.x / len, d.y / len, d.z / len)
            } else {
                // A knob: any way across the face.
                let across = if dir.axis == crate::geom::Axis::X {
                    crate::geom::Axis::Y
                } else {
                    crate::geom::Axis::X
                };
                Vec3::default().with(across, 1.0)
            };
            // The model stands out towards its -Y: its Y points into the front.
            let y = Vec3::default().with(dir.axis, -sign);
            out.push(CadPlacement {
                name: format!("{}.Handle{n}", list.cabinet),
                item: h.item.clone(),
                article: item.article.clone(),
                side: side_of(origin.x),
                origin,
                frame: Frame::Oriented { x, y },
            });
        } else if h.item.starts_with("hafele.axilo.set.") {
            // Both pieces under the leg's name, so they replace its stand-in together.
            let n = next("Leg");
            let (Ok(height), Some(bottom)) = (item.param("nominal_height"), list.part(&h.part))
            else {
                continue;
            };
            let fixing = if item.param("screw_on").ok() == Some(1.0) {
                "screw-on"
            } else {
                "knock-in"
            };
            let tube = crate::hardware::item(&format!("hafele.axilo.tube.{height}"));
            let glide = crate::hardware::item(&format!("hafele.axilo.glide.{fixing}"));
            let upright = |piece: &cabinet_catalog::HardwareItem, z: f64| CadPlacement {
                name: format!("{}.Leg{n}", list.cabinet),
                item: piece.id.clone(),
                article: piece.article.clone(),
                side: side_of(h.at.x),
                origin: Vec3::new(h.at.x, h.at.y, z),
                frame: Frame::Upright,
            };
            // The tube's foot on the floor; the plate's top against the carcass bottom.
            if let Ok(t) = tube {
                out.push(upright(t, height));
            }
            if let Ok(g) = glide
                && let Ok(top) = g.param("model_top")
            {
                out.push(upright(g, bottom.bounds.min.z - top));
            }
        } else if h.item.starts_with("blum.clip-top") {
            let n = next("Hinge");
            out.push(place(
                format!("{}.Hinge{n}", list.cabinet),
                side_of(h.at.x),
                h.at.x,
                h.at.z,
            ));
        } else if h.item.starts_with("blum.mounting-plate") {
            let n = next("Plate");
            out.push(place(
                format!("{}.Plate{n}", list.cabinet),
                side_of(h.at.x),
                h.at.x,
                h.at.z,
            ));
        } else if is_runner(&h.item) {
            // One placement per hand, at the underside of that drawer's base.
            let Some(z) = drawer_base(list, &h.part) else {
                continue;
            };
            let kind = if h.item.contains("drawer-side") {
                "BoxSide"
            } else {
                "Runner"
            };
            let n = next(kind);
            let x = list.part(&h.part).map_or(h.at.x, |p| p.bounds.center().x);
            for (suffix, side) in [("L", Side::Left), ("R", Side::Right)] {
                out.push(place(
                    format!("{}.{kind}{n}{suffix}", list.cabinet),
                    side,
                    x,
                    z,
                ));
            }
        } else if h.item == "blum.servo-drive.drive-unit" {
            // WF-2c: Blum's own models, one per drawer. Their frame is the cabinet's own: X
            // across from the mounting centre (the bay centre from LW 387, else 184 from the
            // left side), Z = 0 on the bracket line (NL + 19), Y = 0 at the datum the unit
            // measures from (A = 45 above the cabinet profile's bottom edge, the drawer base's
            // underside). Names match the stand-ins they replace.
            let Some(unit) = crate::hardware::item(&h.item).ok() else {
                continue;
            };
            let frame = servo_frame();
            for (n, b) in list.by_role(PartRole::DrawerBottom).enumerate() {
                let Some(num) = b
                    .name
                    .rsplit_once(PartRole::DrawerBottom.code())
                    .map(|t| t.1)
                else {
                    continue;
                };
                let front = format!("{}.{}{num}", list.cabinet, PartRole::DrawerFront.code());
                let Some(nl) = crate::standins::nominal_length(list, &front) else {
                    continue;
                };
                let lift = unit.param("lift_above_profile_bottom").unwrap_or(45.0);
                let from_front = unit.param("bracket_from_front").unwrap_or(19.0);
                let Some(x) = servo_centre(list, unit, b.bounds.center().x) else {
                    continue;
                };
                // Keep the stand-ins' numbering: a drawer without room behind its box gets no
                // unit drawn at all.
                if crate::standins::behind_drawer(list, num).is_none() {
                    continue;
                }
                out.push(CadPlacement {
                    name: format!("{}.Servo{}", list.cabinet, n + 1),
                    item: h.item.clone(),
                    article: item.article.clone(),
                    side: side_of(x),
                    origin: Vec3::new(x, nl + from_front, b.bounds.min.z + lift),
                    frame,
                });
            }
        } else if h.item.starts_with("blum.servo-drive.bracket-profile.") {
            // The upright profile once: the same mounting centre and bracket line as the units,
            // its datum 20 above the floor (measured on Blum's own session output).
            let Some(unit) = crate::hardware::item("blum.servo-drive.drive-unit").ok() else {
                continue;
            };
            let Some(first) = list.by_role(PartRole::DrawerBottom).next() else {
                continue;
            };
            let Some(num) = first
                .name
                .rsplit_once(PartRole::DrawerBottom.code())
                .map(|t| t.1)
            else {
                continue;
            };
            let front = format!("{}.{}{num}", list.cabinet, PartRole::DrawerFront.code());
            let Some(nl) = crate::standins::nominal_length(list, &front) else {
                continue;
            };
            let Some(x) = servo_centre(list, unit, first.bounds.center().x) else {
                continue;
            };
            let floor = list
                .by_role(PartRole::Bottom)
                .map(|b| b.bounds.max().z)
                .fold(f64::MIN, f64::max);
            if floor <= f64::MIN / 2.0 {
                continue;
            }
            let lift = unit.param("profile_origin_above_floor").unwrap_or(20.0);
            let from_front = unit.param("bracket_from_front").unwrap_or(19.0);
            out.push(CadPlacement {
                name: format!("{}.ServoProfile", list.cabinet),
                item: h.item.clone(),
                article: item.article.clone(),
                side: side_of(x),
                origin: Vec3::new(x, nl + from_front, floor + lift),
                frame: servo_frame(),
            });
        } else if h.item == "blum.servo-drive.distance-bumper" {
            // In the front's thickness, its datum 4.5 behind the back face (measured on Blum's
            // session output; the pin keeps Blum's 3.6 trigger gap): 2 bumpers at the front's
            // centre height, 4 near its top and bottom (the heavy profile counts as high), 8 in
            // from the side edges. Its model +Z points into the cabinet, unlike the units'.
            let Some(bumper) = crate::hardware::item(&h.item).ok() else {
                continue;
            };
            let inset = bumper.param("inset_from_front_edge").unwrap_or(8.0);
            let face_offset = bumper.param("datum_behind_back_face").unwrap_or(4.5);
            let mut bumpers = 0;
            for f in list
                .parts
                .iter()
                .filter(|p| p.role == PartRole::DrawerFront)
            {
                let heavy = list
                    .hardware
                    .iter()
                    .find(|h| h.part == f.name && is_runner(&h.item))
                    .and_then(|h| crate::hardware::item(&h.item).ok())
                    .and_then(|i| i.param("max_load_kg").ok())
                    .is_some_and(|kg| kg >= 70.0);
                let high = f.bounds.size.z > 300.0 || heavy;
                let zs: Vec<f64> = if high {
                    vec![f.bounds.min.z + inset, f.bounds.max().z - inset]
                } else {
                    vec![f.bounds.center().z]
                };
                for z in zs {
                    for sx in [f.bounds.min.x + inset, f.bounds.max().x - inset] {
                        bumpers += 1;
                        out.push(CadPlacement {
                            name: format!("{}.ServoBumper{bumpers}", list.cabinet),
                            item: h.item.clone(),
                            article: item.article.clone(),
                            side: side_of(sx),
                            origin: Vec3::new(sx, f.bounds.min.y - face_offset, z),
                            frame: Frame::Oriented {
                                x: Vec3::new(-1.0, 0.0, 0.0),
                                y: Vec3::new(0.0, 0.0, 1.0),
                            },
                        });
                    }
                }
            }
        }
    }
    out
}

/// Blum's SERVO-DRIVE models come in the cabinet's own axes: X across, Y up, Z towards the
/// front. In our frame Y runs to the back, so the model's Z flips: model X → our X, model Y →
/// our Z, model Z → our −Y.
fn servo_frame() -> Frame {
    Frame::Oriented {
        x: Vec3::new(1.0, 0.0, 0.0),
        y: Vec3::new(0.0, 0.0, 1.0),
    }
}

/// Where a SERVO-DRIVE part mounts across the bay at `x`: centred from LW 387, else 184 in
/// from the left side (p.366).
fn servo_centre(list: &PartList, unit: &cabinet_catalog::HardwareItem, x: f64) -> Option<f64> {
    let (l, r) = list.bay_around(x)?;
    let (left, right) = (l.bounds.max().x, r.bounds.min.x);
    let lw = right - left;
    Some(
        if lw >= unit.param("mount_centred_from_lw").unwrap_or(387.0) {
            (left + right) / 2.0
        } else {
            left + unit.param("mount_from_left_side").unwrap_or(184.0)
        },
    )
}

/// Runners, steel box cabinet profiles and drawer sides: the parts drawn per hand along a drawer.
pub(crate) fn is_runner(item: &str) -> bool {
    item.starts_with("blum.movento.7")
        || item.starts_with("blum.tandem.5")
        || item.contains(".cabinet-profile.")
        || item.contains(".drawer-side.")
}

/// The underside of the base of the drawer behind `front` (`B01.DrawerFront2` →
/// `B01.DrawerBottom2`).
fn drawer_base(list: &PartList, front: &str) -> Option<f64> {
    let n = front.rsplit_once(PartRole::DrawerFront.code())?.1;
    let cab = &list.cabinet;
    list.part(&format!("{cab}.{}{n}", PartRole::DrawerBottom.code()))
        .map(|p| p.bounds.min.z)
}

/// The model-to-cabinet transform (mm) for a model whose parts sit on the +X side of its frame
/// origin when `model_interior_positive`, placed on `p.side`: a row-major 3 × 4 matrix, so
/// `cabinet = M · [x y z 1]ᵀ`.
///
/// Blum's Y (up) becomes our z and its Z (to the front) our −y. Its X becomes ±x so the model's
/// inside faces the cabinet's inside; a one-handed model on the other side turns half a turn
/// about the depth axis, which flips it upside down too (hinges and plates are symmetric).
pub fn transform(p: &CadPlacement, model_interior_positive: bool, front_plane: f64) -> [f64; 12] {
    let o = p.origin;
    match p.frame {
        Frame::Upright => {
            return [
                1.0, 0.0, 0.0, o.x, //
                0.0, 1.0, 0.0, o.y, //
                0.0, 0.0, 1.0, o.z,
            ];
        }
        Frame::Oriented { x, y } => {
            let z = Vec3::new(
                x.y * y.z - x.z * y.y,
                x.z * y.x - x.x * y.z,
                x.x * y.y - x.y * y.x,
            );
            return [
                x.x, y.x, z.x, o.x, //
                x.y, y.y, z.y, o.y, //
                x.z, y.z, z.z, o.z,
            ];
        }
        Frame::Blum => {}
    }
    let model = if model_interior_positive { 1.0 } else { -1.0 };
    let flip = model * p.side.interior_sign();
    // Columns: where the model's X, Y and Z axes point in the cabinet frame.
    let (mx, my, mz) = ([flip, 0.0, 0.0], [0.0, 0.0, flip], [0.0, -1.0, 0.0]);
    // The model's origin, moved so its front plane lands on our front (y = 0).
    let o = p.origin;
    let t = [o.x, o.y + front_plane, o.z];
    [
        mx[0], my[0], mz[0], t[0], //
        mx[1], my[1], mz[1], t[1], //
        mx[2], my[2], mz[2], t[2],
    ]
}

/// Applies a [`transform`] to a model point.
pub fn apply(m: &[f64; 12], p: Vec3) -> Vec3 {
    Vec3::new(
        m[0] * p.x + m[1] * p.y + m[2] * p.z + m[3],
        m[4] * p.x + m[5] * p.y + m[6] * p.z + m[7],
        m[8] * p.x + m[9] * p.y + m[10] * p.z + m[11],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{DrawerSystem, FrontLayout, HingeSide};
    use crate::{Cabinet, CabinetKind, build_cabinet};

    fn det(m: &[f64; 12]) -> f64 {
        let r = |i: usize, j: usize| m[i * 4 + j];
        r(0, 0) * (r(1, 1) * r(2, 2) - r(1, 2) * r(2, 1))
            - r(0, 1) * (r(1, 0) * r(2, 2) - r(1, 2) * r(2, 0))
            + r(0, 2) * (r(1, 0) * r(2, 1) - r(1, 1) * r(2, 0))
    }

    #[test]
    fn a_base_cabinet_places_a_hinge_and_a_plate_per_hinge_position_on_the_hinge_side() {
        let mut cab = Cabinet::new("B01", CabinetKind::Base);
        cab.fronts = FrontLayout::Door {
            hinge_side: HingeSide::Right,
        };
        let list = build_cabinet(&cab).unwrap();
        let p = cad_placements(&list);
        let hinges: Vec<_> = p.iter().filter(|c| c.name.contains("Hinge")).collect();
        let plates: Vec<_> = p.iter().filter(|c| c.name.contains("Plate")).collect();
        assert_eq!(hinges.len(), 2);
        assert_eq!(plates.len(), 2);
        let inner = list.part("B01.RightSide").unwrap().bounds.min.x;
        for c in hinges.iter().chain(&plates) {
            assert_eq!(c.side, Side::Right);
            assert_eq!(c.origin.x, inner);
            assert_eq!(c.origin.y, 0.0);
        }
        assert_eq!(hinges[0].article, "71B3550");
        assert_eq!(plates[0].article, "175H9100");
    }

    #[test]
    fn drawers_get_a_runner_each_side_at_the_underside_of_their_base() {
        for system in [
            DrawerSystem::Legrabox,
            DrawerSystem::Movento,
            DrawerSystem::Tandem,
        ] {
            let mut cab = Cabinet::new("D01", CabinetKind::DrawerBank);
            cab.standard.drawer_system = system;
            let list = build_cabinet(&cab).unwrap();
            let p = cad_placements(&list);
            let runners: Vec<_> = p.iter().filter(|c| c.name.contains("Runner")).collect();
            assert_eq!(runners.len(), 6, "{system:?}");
            let base = list.part("D01.DrawerBottom1").unwrap().bounds.min.z;
            assert_eq!(runners[0].origin.z, base);
            assert_eq!(
                (runners[0].side, runners[1].side),
                (Side::Left, Side::Right)
            );
        }
    }

    #[test]
    fn transforms_are_rotations_that_put_the_front_plane_on_the_cabinet_front() {
        let list = build_cabinet(&Cabinet::new("B01", CabinetKind::Base)).unwrap();
        for c in cad_placements(&list)
            .iter()
            .filter(|c| c.frame == Frame::Blum)
        {
            for interior in [true, false] {
                let m = transform(c, interior, BLUM_FRONT_PLANE);
                assert!(
                    (det(&m) - 1.0).abs() < 1e-12,
                    "{} is not a rotation",
                    c.name
                );
                let front = apply(&m, Vec3::new(0.0, 0.0, BLUM_FRONT_PLANE));
                assert!((front.y).abs() < 1e-9 && (front.x - c.origin.x).abs() < 1e-9);
                // The model's inside always faces the cabinet's inside.
                let inside = apply(
                    &m,
                    Vec3::new(if interior { 10.0 } else { -10.0 }, 0.0, BLUM_FRONT_PLANE),
                );
                assert_eq!((inside.x - c.origin.x).signum(), c.side.interior_sign());
            }
        }
    }

    #[test]
    fn an_axilo_leg_stands_its_tube_on_the_floor_and_its_plate_under_the_carcass() {
        let mut cab = Cabinet::new("B01", CabinetKind::Base);
        cab.standard.toe_kick = crate::model::ToeKick::LegsAndPlinth;
        let list = build_cabinet(&cab).unwrap();
        let bottom = list.part("B01.Bottom").unwrap().bounds.min.z;
        let legs: Vec<_> = cad_placements(&list)
            .into_iter()
            .filter(|c| c.name == "B01.Leg1")
            .collect();
        let tube = legs.iter().find(|c| c.item.contains(".tube.")).unwrap();
        let plate = legs.iter().find(|c| c.item.contains(".glide.")).unwrap();
        assert_eq!(tube.frame, Frame::Upright);
        assert_eq!(tube.article, "637.76.355");
        assert_eq!(plate.article, "637.38.287");
        // The 150 mm tube hangs from its origin, so its foot is on the floor.
        assert_eq!(tube.origin.z, 150.0);
        assert_eq!(plate.origin.z, bottom - 25.6);
        let m = transform(tube, true, BLUM_FRONT_PLANE);
        assert_eq!(apply(&m, Vec3::new(0.0, 0.0, -150.0)).z, 0.0);
    }

    /// WF-7: a handle model sits between its screw holes on the front's face: a door's bar
    /// upright, a drawer's across, both standing out in front of the face; a knob on its hole.
    #[test]
    fn handles_sit_on_their_holes_standing_out_of_the_front() {
        let door = build_cabinet(&Cabinet::new("B01", CabinetKind::Base)).unwrap();
        let bank = build_cabinet(&Cabinet::new("D01", CabinetKind::DrawerBank)).unwrap();
        for (list, upright) in [(&door, true), (&bank, false)] {
            let handles: Vec<_> = cad_placements(list)
                .into_iter()
                .filter(|c| c.name.contains(".Handle"))
                .collect();
            assert!(!handles.is_empty());
            for h in &handles {
                assert_eq!(h.article, "GEN-H-CC128");
                let m = transform(h, true, BLUM_FRONT_PLANE);
                assert!((det(&m) - 1.0).abs() < 1e-12);
                // The holes, ±64 along the model's X, land on the front's face, upright on a
                // door and across on a drawer.
                let (a, b) = (
                    apply(&m, Vec3::new(-64.0, 0.0, 0.0)),
                    apply(&m, Vec3::new(64.0, 0.0, 0.0)),
                );
                if upright {
                    assert!((b.z - a.z).abs() > 127.0 && (b.x - a.x).abs() < 1e-9);
                } else {
                    assert!((b.x - a.x).abs() > 127.0 && (b.z - a.z).abs() < 1e-9);
                }
                let front = list
                    .parts
                    .iter()
                    .filter(|p| p.role.is_front())
                    .find(|p| {
                        let q = p.bounds;
                        a.x >= q.min.x - 1e-6
                            && a.x <= q.max().x + 1e-6
                            && a.z >= q.min.z
                            && a.z <= q.max().z
                    })
                    .unwrap();
                assert!((a.y - front.bounds.min.y).abs() < 1e-9, "{}", h.name);
                // The bar 32 in front of it.
                assert!(
                    (apply(&m, Vec3::new(0.0, -32.0, 0.0)).y - (front.bounds.min.y - 32.0)).abs()
                        < 1e-9
                );
            }
        }
    }

    /// WF-2c: SERVO-DRIVE places Blum's own models per drawer plus the bracket profile once, in
    /// the frame measured on Blum's session output: centred across a wide bay, the datum depth
    /// NL + 19 (the bracket line), the unit A = 45 above the drawer base's underside, the
    /// profile 20 above the floor. Bumpers sit on each front's back face.
    #[test]
    fn servo_drive_models_mount_on_the_bracket_line_at_blums_datums() {
        use crate::model::Opening;
        let mut bank = Cabinet::new("D01", CabinetKind::DrawerBank);
        bank.standard.opening = Opening::ServoDrive;
        let list = build_cabinet(&bank).unwrap();
        let p = cad_placements(&list);
        let units: Vec<_> = p
            .iter()
            .filter(|c| {
                c.name.starts_with("D01.Servo")
                    && !c.name.contains("Profile")
                    && !c.name.contains("Bumper")
            })
            .collect();
        assert_eq!(units.len(), 3, "one drive unit per drawer: {units:?}");
        let nl = crate::standins::nominal_length(&list, "D01.DrawerFront1").unwrap();
        let (l, r) = list
            .bay_around(list.part("D01.DrawerBottom1").unwrap().bounds.center().x)
            .unwrap();
        let centre = (l.bounds.max().x + r.bounds.min.x) / 2.0;
        let base = list.part("D01.DrawerBottom1").unwrap().bounds.min.z;
        for c in units.iter() {
            assert_eq!(
                c.frame,
                Frame::Oriented {
                    x: Vec3::new(1.0, 0.0, 0.0),
                    y: Vec3::new(0.0, 0.0, 1.0)
                }
            );
            assert!((c.origin.x - centre).abs() < 1e-9, "centred across the bay");
            assert!(
                (c.origin.y - (nl + 19.0)).abs() < 1e-9,
                "on the bracket line NL + 19"
            );
        }
        let unit = units[0];
        assert!(
            (unit.origin.z - (base + 45.0)).abs() < 1e-9,
            "A = 45 above the base"
        );
        // The frame keeps Blum's model upright: its Y (up) is our +Z, its Z (to the front) our
        // −Y.
        let m = transform(unit, true, BLUM_FRONT_PLANE);
        assert_eq!(apply(&m, Vec3::new(0.0, 10.0, 0.0)).z, unit.origin.z + 10.0);
        assert_eq!(apply(&m, Vec3::new(0.0, 0.0, 10.0)).y, unit.origin.y - 10.0);
        // The profile once, its datum 20 above the floor.
        let profile = p.iter().find(|c| c.name == "D01.ServoProfile").unwrap();
        let floor = list.part("D01.Bottom").unwrap().bounds.max().z;
        assert!((profile.origin.x - centre).abs() < 1e-9);
        assert!((profile.origin.y - (nl + 19.0)).abs() < 1e-9);
        assert!((profile.origin.z - (floor + 20.0)).abs() < 1e-9);
        // Bumpers: 2 per front at its centre, 8 in from the side edges, their datum 4.5 behind
        // the back face, pin towards the interior (the model's +Z is our +Y).
        let bumpers: Vec<_> = p
            .iter()
            .filter(|c| c.name.contains("ServoBumper"))
            .collect();
        assert_eq!(bumpers.len(), 6, "three fronts, two bumpers each");
        let front = list.part("D01.DrawerFront1").unwrap();
        for b in &bumpers {
            assert!((b.origin.y - (front.bounds.min.y - 4.5)).abs() < 1e-9);
            let from_left = b.origin.x - front.bounds.min.x;
            let from_right = front.bounds.max().x - b.origin.x;
            assert!((from_left - 8.0).abs() < 1e-9 || (from_right - 8.0).abs() < 1e-9);
            let m = transform(b, true, BLUM_FRONT_PLANE);
            assert_eq!(apply(&m, Vec3::new(0.0, 0.0, 2.6)).y, b.origin.y + 2.6);
        }
        assert!((bumpers[0].origin.z - front.bounds.center().z).abs() < 1e-9);
    }
}
