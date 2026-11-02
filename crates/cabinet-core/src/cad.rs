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
use crate::part::{BorePurpose, Operation, PartList, PartRole};

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

/// How far up a side a light can run: from the bottom panel to the top, or the lowest fixed shelf,
/// which it can't pass.
pub(crate) fn side_light_span(list: &PartList) -> (f64, f64) {
    let floor = list
        .by_role(PartRole::Bottom)
        .next()
        .map_or(0.0, |p| p.bounds.max().z);
    let adjustable = |name: &str| {
        list.hardware
            .iter()
            .any(|h| h.item == crate::build::SHELF_SUPPORT && h.part == name)
    };
    let top = list
        .parts
        .iter()
        .filter(|p| p.role == PartRole::Top || (p.role == PartRole::Shelf && !adjustable(&p.name)))
        .map(|p| p.bounds.min.z)
        .filter(|&z| z > floor)
        .fold(f64::MAX, f64::min);
    (floor, if top == f64::MAX { floor } else { top })
}

/// How far a recessed profile's face stands off the panel: its flange, and what lets it be seen.
const LIGHT_FLANGE: f64 = 0.5;

/// A light's profile: Häfele's model is a short sample of it, so samples lie end to end along the
/// strip, overlapping to fit. Every one lights towards its +Z: down, or across from a side.
fn light_profile(
    list: &PartList,
    h: &crate::part::HardwarePlacement,
    item: &cabinet_catalog::HardwareItem,
    next: &mut impl FnMut(&'static str) -> usize,
) -> Vec<CadPlacement> {
    let Some(panel) = list.part(&h.part) else {
        return Vec::new();
    };
    let b = panel.bounds;
    let (Ok(section), Ok(z_min), Ok(z_max)) = (
        item.param("section"),
        item.param("model_z_min"),
        item.param("model_z_max"),
    ) else {
        return Vec::new();
    };
    let side = matches!(panel.role, PartRole::LeftSide | PartRole::RightSide);
    // Up a side from the bottom panel to the top, or along the panel.
    let (start, length) = if side {
        let (floor, top) = side_light_span(list);
        (
            floor + crate::lighting::STRIP_END_CLEARANCE,
            top - floor - 2.0 * crate::lighting::STRIP_END_CLEARANCE,
        )
    } else {
        (
            b.min.x + crate::lighting::STRIP_END_CLEARANCE,
            b.size.x - 2.0 * crate::lighting::STRIP_END_CLEARANCE,
        )
    };
    let count = (length / section).ceil().max(1.0);
    let pitch = if count > 1.0 {
        (length - section) / (count - 1.0)
    } else {
        0.0
    };
    // A surface profile's back on the panel; a recessed one's flange just proud of it.
    let surface = item.param("surface").unwrap_or(1.0) == 1.0;
    let off = if surface { z_min } else { z_max - LIGHT_FLANGE };
    let along_y = item.param("model_along_y").unwrap_or(0.0) == 1.0;
    let left = panel.role == PartRole::LeftSide;
    let (ox, oy) = (Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0));
    let (up, flip_y) = (Vec3::new(0.0, 0.0, 1.0), Vec3::new(0.0, -1.0, 0.0));
    let frame = match (side, along_y) {
        (false, true) => Frame::Oriented { x: oy, y: ox },
        (false, false) => Frame::Oriented { x: ox, y: flip_y },
        (true, true) => Frame::Oriented {
            x: if left { oy } else { flip_y },
            y: up,
        },
        (true, false) => Frame::Oriented {
            x: up,
            y: if left { flip_y } else { oy },
        },
    };
    let side_of = if b.center().x < h.at.x {
        Side::Right
    } else {
        Side::Left
    };
    (0..count as usize)
        .map(|i| {
            let along = start + i as f64 * pitch;
            let origin = if side {
                let sign = if left { 1.0 } else { -1.0 };
                Vec3::new(h.at.x - sign * off, h.at.y, along)
            } else {
                Vec3::new(along, h.at.y, b.min.z + off)
            };
            CadPlacement {
                name: format!("{}.LightProfile{}", list.cabinet, next("LightProfile")),
                item: h.item.clone(),
                article: item.article.clone(),
                side: side_of,
                origin,
                frame,
            }
        })
        .collect()
}

/// The part Blum models a lift set by, for the sets it only models as parts: a look at the
/// mechanism, whichever strength was chosen, as they share one shape.
fn lift_model_article<'a>(item: &'a str, article: &'a str) -> &'a str {
    if item.starts_with("blum.aventos.hs-top.mechanism.") {
        "20S2A00.05"
    } else if item.starts_with("blum.aventos.hl-top.mechanism.") {
        "20L2700.05"
    } else {
        match article {
            "22S3500" => "20S3500.06",
            "22L3200" => "20L3200.06",
            "22L3500" => "20L3500.06",
            "22L3800" => "20L3800.06",
            "22L3900" => "20L3900.06",
            other => other,
        }
    }
}

/// The lift system whose holes a mechanism or lever arm model is placed by, if we place it.
fn lift_family(item: &str) -> Option<&'static str> {
    if item.starts_with("blum.aventos.hk-top.mechanism.") {
        Some("blum.aventos.hk-top")
    } else if item.starts_with("blum.aventos.hk-s.mechanism.")
        || item.starts_with("blum.aventos.hk-s.tip-on.mechanism.")
    {
        Some("blum.aventos.hk-s")
    } else if item.starts_with("blum.aventos.hf-top.mechanism.")
        || item.starts_with("blum.aventos.hf-top.lever-arm.")
    {
        Some("blum.aventos.hf-top")
    } else if item.starts_with("blum.aventos.hs-top.mechanism.")
        || item == "blum.aventos.hs-top.lever-arm"
    {
        Some("blum.aventos.hs-top")
    } else if item.starts_with("blum.aventos.hl-top.mechanism.")
        || item.starts_with("blum.aventos.hl-top.lever-arm.")
    {
        Some("blum.aventos.hl-top")
    } else {
        None
    }
}

/// An AXILO plate is longer on its +X side, so it turns to keep that towards the cabinet's middle:
/// otherwise a leg by the right side would have its fixings off the end of the bottom panel.
pub(crate) fn leg_plate_turned(x: f64, panel_centre: f64) -> bool {
    x > panel_centre + 1.0
}

/// Where an AXILO plate under a leg at (`x`, `y`) fixes to the bottom panel: its four tips or
/// screws, as (x, y).
pub(crate) fn leg_plate_fixings(
    plate: &cabinet_catalog::HardwareItem,
    x: f64,
    y: f64,
    panel_centre: f64,
) -> Vec<(f64, f64)> {
    let (Ok(a), Ok(b), Ok(across)) = (
        plate.param("fix_x_a"),
        plate.param("fix_x_b"),
        plate.param("fix_y"),
    ) else {
        return Vec::new();
    };
    let sign = if leg_plate_turned(x, panel_centre) {
        -1.0
    } else {
        1.0
    };
    [a, b]
        .iter()
        .flat_map(|fx| [(x + sign * fx, y - across), (x + sign * fx, y + across)])
        .collect()
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
    let mut brackets = 0;
    let left = list.by_role(PartRole::LeftSide).next();
    let right = list.by_role(PartRole::RightSide).next();
    let (Some(left), Some(right)) = (left, right) else {
        // A floating shelf has no sides, but may have a light under it.
        for h in &list.hardware {
            if h.item.starts_with("hafele.loox5.profile.")
                && let Ok(item) = crate::hardware::item(&h.item)
            {
                out.extend(light_profile(list, h, item, &mut next));
            }
        }
        return out;
    };
    // Blum's models measure from the back of the front: inset, that stands inside the carcass.
    let inset = list
        .parts
        .iter()
        .filter(|p| matches!(p.role, PartRole::Door | PartRole::DrawerFront))
        .map(|p| p.bounds.max().y)
        .fold(0.0, f64::max);
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
            origin: Vec3::new(face(x, side), inset, z),
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
        if item.parameters.contains_key("edge_mount") {
            // Its frame: x along it, y to the cabinet, z up, the origin on the back and edge of
            // the front. Under a wall door's bottom edge it hangs upside down.
            let top = list
                .part(&h.part)
                .is_none_or(|p| (h.at.z - p.bounds.max().z).abs() < 1e-6);
            let n = next("Handle");
            out.push(CadPlacement {
                name: format!("{}.Handle{n}", list.cabinet),
                item: h.item.clone(),
                article: item.article.clone(),
                side: side_of(h.at.x),
                origin: h.at,
                frame: Frame::Oriented {
                    x: Vec3::new(if top { 1.0 } else { -1.0 }, 0.0, 0.0),
                    y: Vec3::new(0.0, 1.0, 0.0),
                },
            });
        } else if h.item.starts_with("handle.") && h.item != "handle.edge-pull" {
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
        } else if h.item.starts_with("hafele.minifix15.") {
            let n = next("Cam");
            if let Some((into, to_bolt, shank)) = crate::standins::cam_axes(list, h) {
                // Häfele draws the housing with its mouth at Z = 0 and Z into the wood; its cup
                // opens 60° clockwise of +X. Turn that opening to the bolt.
                let across = Vec3::new(
                    into.y * to_bolt.z - into.z * to_bolt.y,
                    into.z * to_bolt.x - into.x * to_bolt.z,
                    into.x * to_bolt.y - into.y * to_bolt.x,
                );
                let (cos, sin) = (0.5, 3f64.sqrt() / 2.0);
                let blend = |a: f64, b: f64| {
                    Vec3::new(
                        a * to_bolt.x + b * across.x,
                        a * to_bolt.y + b * across.y,
                        a * to_bolt.z + b * across.z,
                    )
                };
                let frame = Frame::Oriented {
                    x: blend(cos, sin),
                    y: blend(-sin, cos),
                };
                // Häfele publishes no model for 262.26.034: the next size up stands in.
                let article = match item.article.as_str() {
                    "262.26.034" => "262.26.035".to_string(),
                    a => a.to_string(),
                };
                let name = format!("{}.Cam{n}", list.cabinet);
                let piece =
                    |item_id: &str, article: String, origin: Vec3, frame: Frame| CadPlacement {
                        name: name.clone(),
                        item: item_id.to_string(),
                        article,
                        side: side_of(h.at.x),
                        origin,
                        frame,
                    };
                out.push(piece(&h.item, article, h.at, frame));
                // The cap shares the housing's frame.
                if let Ok(cap) = crate::hardware::item(crate::joinery::minifix_cap(item)) {
                    out.push(piece(&cap.id, cap.article.clone(), h.at, frame));
                }
                // The bolt lies along its X, thread towards -X from X = 0 on the joint's plane.
                if let Ok(bolt) = crate::hardware::item(crate::joinery::MINIFIX_BOLT) {
                    let along = Vec3::new(-to_bolt.x, -to_bolt.y, -to_bolt.z);
                    out.push(piece(
                        &bolt.id,
                        bolt.article.clone(),
                        shank,
                        Frame::Oriented { x: along, y: into },
                    ));
                }
            }
        } else if let Some(family) = lift_family(&h.item) {
            // Blum draws a mechanism where it mounts: its screws in the side, at a known point of
            // its frame. So it goes where the side's own holes are: on each side, by the front
            // top hole. A lever arm shares its mechanism's frame.
            let Ok(sys) = crate::hardware::item(family) else {
                continue;
            };
            let measured = match (sys.param("model_hole_y"), sys.param("model_hole_z")) {
                (Ok(y), Ok(z)) => Some((y, z)),
                _ => None,
            };
            let read = match (
                sys.param("model_origin_below_top"),
                sys.param("model_origin_from_front"),
            ) {
                (Ok(below), Ok(from)) => Some((below, from)),
                _ => None,
            };
            let hole_of = |part: &str| {
                list.operations
                    .iter()
                    .filter_map(|o| match o {
                        Operation::Bore {
                            part: p,
                            at,
                            purpose: BorePurpose::LiftMechanism,
                            ..
                        } if p == part => Some(*at),
                        _ => None,
                    })
                    .reduce(|a, b| {
                        // The highest, then the nearest the front.
                        if b.z > a.z + 1e-6 || ((b.z - a.z).abs() < 1e-6 && b.y < a.y) {
                            b
                        } else {
                            a
                        }
                    })
            };
            let kind = if h.item.contains("lever-arm") {
                "LiftArm"
            } else {
                "LiftMechanism"
            };
            for (side, panel) in [(Side::Left, left), (Side::Right, right)] {
                // A hole we know the model's place on, or where its drawing puts it.
                let at = if let Some((hole_y, hole_z)) = measured {
                    let Some(hole) = hole_of(&panel.name) else {
                        continue;
                    };
                    (hole.y + hole_z, hole.z - hole_y)
                } else if let Some((below, from)) = read {
                    (from, panel.bounds.max().z - below)
                } else {
                    continue;
                };
                let n = next(kind);
                out.push(CadPlacement {
                    name: format!("{}.{kind}{n}", list.cabinet),
                    item: h.item.clone(),
                    article: lift_model_article(&h.item, &item.article).to_string(),
                    side,
                    // transform() adds the front plane; undo it, as this frame is the hole's.
                    origin: Vec3::new(inner(side), at.0 - BLUM_FRONT_PLANE, at.1),
                    frame: Frame::Blum,
                });
            }
        } else if h.item == crate::build::SHELF_SUPPORT {
            // One row of the hardware list stands for all the shelf's supports: the holes it
            // rests on say where they are. 282.12.405's pin runs down -X from the origin, on the
            // hole's centre, and its seat is a pin's radius above (as the build's shelf is).
            let Some(shelf) = list.part(&h.part).map(|p| p.bounds) else {
                continue;
            };
            let hole_z = shelf.min.z - 2.5;
            for op in &list.operations {
                let Operation::Bore {
                    at,
                    purpose: BorePurpose::ShelfPin,
                    ..
                } = op
                else {
                    continue;
                };
                if (at.z - hole_z).abs() > 1e-6 {
                    continue;
                }
                let (x, y) = if (at.x - shelf.min.x).abs() < 2.0 {
                    (Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0))
                } else if (at.x - shelf.max().x).abs() < 2.0 {
                    (Vec3::new(-1.0, 0.0, 0.0), Vec3::new(0.0, -1.0, 0.0))
                } else {
                    continue;
                };
                let n = next("ShelfSupport");
                out.push(CadPlacement {
                    name: format!("{}.ShelfSupport{n}", list.cabinet),
                    item: h.item.clone(),
                    article: item.article.clone(),
                    side: side_of(at.x),
                    origin: *at,
                    frame: Frame::Oriented { x, y },
                });
            }
        } else if h.item.starts_with("hafele.loox5.profile.") {
            out.extend(light_profile(list, h, item, &mut next));
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
            // The tube's foot on the floor; the plate's face against the carcass bottom, its tips
            // up in their holes.
            if let Ok(t) = tube {
                out.push(upright(t, height));
            }
            if let Ok(g) = glide
                && let Ok(top) = g.param("model_top")
            {
                let mut plate = upright(g, bottom.bounds.min.z - top);
                if leg_plate_turned(h.at.x, bottom.bounds.center().x) {
                    plate.frame = Frame::Oriented {
                        x: Vec3::new(-1.0, 0.0, 0.0),
                        y: Vec3::new(0.0, -1.0, 0.0),
                    };
                }
                out.push(plate);
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
            let x = list
                .front_box(&h.part)
                .map_or(h.at.x, |p| p.bounds.center().x);
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
            let Some(num) = crate::part::drawer_number(&h.part) else {
                continue;
            };
            let bottom = format!("{}.{}{num}", list.cabinet, PartRole::DrawerBottom.code());
            let (Some(b), Some(nl)) = (
                list.part(&bottom),
                crate::standins::nominal_length(list, &h.part),
            ) else {
                continue;
            };
            let lift = unit.param("lift_above_profile_bottom").unwrap_or(45.0);
            let from_front = unit.param("bracket_from_front").unwrap_or(19.0);
            let Some(x) = servo_centre(list, unit, b.bounds.center().x) else {
                continue;
            };
            // Keep the stand-ins' numbering: a drawer without room behind its box gets no unit
            // drawn at all.
            if crate::standins::behind_drawer(list, num).is_none() {
                continue;
            }
            let n = next("Servo");
            out.push(CadPlacement {
                name: format!("{}.Servo{n}", list.cabinet),
                item: h.item.clone(),
                article: item.article.clone(),
                side: side_of(x),
                origin: Vec3::new(x, nl + from_front, b.bounds.min.z + lift),
                frame: servo_frame(),
            });
        } else if h.item.starts_with("blum.servo-drive.bracket-profile.") {
            // A compartment's upright: the same mounting centre and bracket line as its units,
            // its datum 4 above its floor (Blum's session: 20 over a 16 base, 591.17 over a shelf
            // top at 587.17).
            let bracket = brackets;
            brackets += 1;
            let Some(unit) = crate::hardware::item("blum.servo-drive.drive-unit").ok() else {
                continue;
            };
            let compartments = crate::standins::servo_compartments(list);
            let Some(c) = compartments.get(bracket) else {
                continue;
            };
            let Some(nl) = crate::standins::nominal_length(list, &c.front) else {
                continue;
            };
            let Some(x) = servo_centre(list, unit, c.x) else {
                continue;
            };
            let lift = unit.param("profile_origin_above_floor").unwrap_or(4.0);
            // Only the 670 has a model standing up (Z10T1143B lies along its own X), and a model
            // can't be cut: a shorter compartment, or another length, keeps the stand-in.
            let fits = item
                .param("length")
                .is_ok_and(|l| c.floor + lift + l <= c.ceiling);
            if !h.item.ends_with(".670") || !fits {
                continue;
            }
            let from_front = unit.param("bracket_from_front").unwrap_or(19.0);
            out.push(CadPlacement {
                name: crate::standins::servo_profile_name(&list.cabinet, bracket),
                item: h.item.clone(),
                article: item.article.clone(),
                side: side_of(x),
                origin: Vec3::new(x, nl + from_front, c.floor + lift),
                frame: servo_frame(),
            });
        } else if h.item == "blum.servo-drive.distance-bumper" {
            // Its datum on the front's back face, the shank in the front and the head towards the
            // cabinet (Blum's session: 582.5, the fronts' back): 2 bumpers at the front's centre
            // height, 4 near its top and bottom (the heavy profile counts as high), 8 in from the
            // side edges. Its model +Z points into the cabinet, unlike the units'.
            let Some(bumper) = crate::hardware::item(&h.item).ok() else {
                continue;
            };
            let inset = bumper.param("inset_from_front_edge").unwrap_or(8.0);
            let Some(f) = list.front_box(&h.part) else {
                continue;
            };
            // As the builder counted them for this front: 2 in one row, 4 in two.
            let zs: Vec<f64> = if h.quantity > 2 {
                vec![f.bounds.min.z + inset, f.bounds.max().z - inset]
            } else {
                vec![f.bounds.center().z]
            };
            for z in zs {
                for sx in [f.bounds.min.x + inset, f.bounds.max().x - inset] {
                    let n = next("ServoBumper");
                    out.push(CadPlacement {
                        name: format!("{}.ServoBumper{n}", list.cabinet),
                        item: h.item.clone(),
                        article: item.article.clone(),
                        side: side_of(sx),
                        origin: Vec3::new(sx, f.bounds.max().y, z),
                        frame: Frame::Oriented {
                            x: Vec3::new(-1.0, 0.0, 0.0),
                            y: Vec3::new(0.0, 0.0, 1.0),
                        },
                    });
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
    let n = crate::part::drawer_number(front)?;
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
        assert_eq!(plate.origin.z, bottom - 25.0);
        let m = transform(tube, true, BLUM_FRONT_PLANE);
        assert_eq!(apply(&m, Vec3::new(0.0, 0.0, -150.0)).z, 0.0);
    }

    #[test]
    fn a_lift_mechanism_model_has_its_screws_in_the_sides_own_holes() {
        use crate::model::LiftKind;
        let layouts = [
            (
                "blum.aventos.hk-top",
                FrontLayout::Lift {
                    kind: LiftKind::StayLift,
                    handle_weight: 0.4,
                },
                300.0,
            ),
            (
                "blum.aventos.hk-s",
                FrontLayout::Lift {
                    kind: LiftKind::SmallStayLift,
                    handle_weight: 0.4,
                },
                300.0,
            ),
            (
                "blum.aventos.hf-top",
                FrontLayout::BiFoldLift { handle_weight: 0.4 },
                720.0,
            ),
        ];
        for (family, fronts, height) in layouts {
            let mut cab = Cabinet::new("W01", CabinetKind::Wall);
            cab.height = height;
            cab.fronts = fronts;
            let list = build_cabinet(&cab).unwrap();
            let sys = crate::hardware::item(family).unwrap();
            let hole = Vec3::new(
                0.0,
                sys.param("model_hole_y").unwrap(),
                sys.param("model_hole_z").unwrap(),
            );
            let placed: Vec<_> = cad_placements(&list)
                .into_iter()
                .filter(|c| c.name.contains("LiftMechanism"))
                .collect();
            assert_eq!(placed.len(), 2, "{family}");
            for c in &placed {
                let side = if c.side == Side::Left {
                    "W01.LeftSide"
                } else {
                    "W01.RightSide"
                };
                let at = apply(&transform(c, c.side == Side::Left, BLUM_FRONT_PLANE), hole);
                let top = list
                    .operations
                    .iter()
                    .filter_map(|o| match o {
                        Operation::Bore {
                            part,
                            at,
                            purpose: BorePurpose::LiftMechanism,
                            ..
                        } if part == side => Some(*at),
                        _ => None,
                    })
                    .fold(f64::MIN, |z, a| z.max(a.z));
                let holes: Vec<Vec3> = list
                    .operations
                    .iter()
                    .filter_map(|o| match o {
                        Operation::Bore {
                            part,
                            at,
                            purpose: BorePurpose::LiftMechanism,
                            ..
                        } if part == side => Some(*at),
                        _ => None,
                    })
                    .collect();
                assert_eq!(at.z, top, "{family}");
                assert!(
                    holes
                        .iter()
                        .any(|h| (h.y - at.y).abs() < 1e-6 && (h.z - at.z).abs() < 1e-6),
                    "{family}: {at:?} not in {holes:?}"
                );
            }
        }
    }

    /// The owner's report: SERVO-DRIVE and TIP-ON vanished when fronts were shaker.
    #[test]
    fn framed_fronts_keep_the_same_hardware_as_slabs() {
        use crate::model::{DoorStyle, Opening};
        for (kind, opening) in [
            (CabinetKind::DrawerBank, Opening::ServoDrive),
            (CabinetKind::DrawerBank, Opening::TipOn),
            (CabinetKind::Base, Opening::TipOn),
        ] {
            let names = |style| {
                let mut cab = Cabinet::new("C1", kind);
                cab.standard.opening = opening;
                cab.standard.door_style = style;
                let list = build_cabinet(&cab).unwrap();
                let mut a: Vec<String> = cad_placements(&list)
                    .into_iter()
                    .filter(|c| !c.name.contains("Handle"))
                    .map(|c| c.name)
                    .chain(
                        crate::standins::stand_ins(&list)
                            .into_iter()
                            .map(|s| s.name),
                    )
                    .collect();
                a.sort();
                a
            };
            let slab = names(DoorStyle::Slab);
            assert!(!slab.is_empty());
            assert_eq!(slab, names(DoorStyle::Shaker), "{kind:?} {opening:?}");
        }
    }

    #[test]
    fn up_and_over_and_lift_up_mechanisms_are_placed_with_their_arms_and_holes() {
        use crate::model::LiftKind;
        for (kind, height, mech, arm) in [
            (LiftKind::UpAndOver, 720.0, "20S2A00.05", "20S3500.06"),
            (LiftKind::LiftUp, 450.0, "20L2700.05", "20L3"),
        ] {
            let mut cab = Cabinet::new("W01", CabinetKind::Wall);
            cab.height = height;
            cab.fronts = FrontLayout::Lift {
                kind,
                handle_weight: 0.4,
            };
            let list = build_cabinet(&cab).unwrap();
            let placed = cad_placements(&list);
            let count = |a: &str| placed.iter().filter(|c| c.article.starts_with(a)).count();
            assert_eq!(
                (count(mech), count(arm)),
                (2, 2),
                "{kind:?} {:?}",
                list.warnings
            );
            let holes = list
                .operations
                .iter()
                .filter(|o| matches!(o, Operation::Bore { part, purpose: BorePurpose::LiftMechanism, .. } if part == "W01.LeftSide"))
                .count();
            assert_eq!(holes, 4);
        }
    }

    #[test]
    fn a_light_up_a_side_runs_upright_and_lights_across_to_the_other_side() {
        let mut cab = Cabinet::new("R01", CabinetKind::Wardrobe);
        for id in ["hafele.loox5.profile.2190", "hafele.loox5.profile.1104"] {
            cab.lights = vec![crate::model::Light {
                zone: crate::model::LightZone::SideRear,
                voltage: crate::model::Voltage::V24,
                watts_per_metre: 9.6,
                profile: id.into(),
            }];
            let list = build_cabinet(&cab).unwrap();
            let item = crate::hardware::item(id).unwrap();
            let z_max = item.param("model_z_max").unwrap();
            let along_y = item.param("model_along_y").unwrap() == 1.0;
            let section = item.param("section").unwrap();
            let left = list.part("R01.LeftSide").unwrap().bounds;
            let pieces: Vec<_> = cad_placements(&list)
                .into_iter()
                .filter(|c| c.name.contains("LightProfile"))
                .collect();
            assert!(pieces.len() >= 4, "{id}");
            for c in &pieces {
                let m = transform(c, true, BLUM_FRONT_PLANE);
                let end = if along_y {
                    Vec3::new(0.0, section, 0.0)
                } else {
                    Vec3::new(section, 0.0, 0.0)
                };
                let (a, b) = (apply(&m, Vec3::new(0.0, 0.0, 0.0)), apply(&m, end));
                assert!((b.z - a.z - section).abs() < 1e-6, "{id} runs upright");
                let light = apply(&m, Vec3::new(0.0, 0.0, z_max));
                let back = apply(&m, Vec3::new(0.0, 0.0, item.param("model_z_min").unwrap()));
                let on_left = c.origin.x < left.max().x + 40.0;
                assert_eq!(light.x > back.x, on_left, "{id} lights across");
            }
        }
    }

    #[test]
    fn a_turned_leg_plate_still_stands_with_its_face_on_the_panel() {
        let mut cab = Cabinet::new("B01", CabinetKind::Base);
        cab.standard.toe_kick = crate::model::ToeKick::LegsAndPlinth;
        let list = build_cabinet(&cab).unwrap();
        let bottom = list.part("B01.Bottom").unwrap().bounds;
        let plates: Vec<_> = cad_placements(&list)
            .into_iter()
            .filter(|c| c.item.contains(".glide."))
            .collect();
        assert!(plates.iter().any(|c| c.frame == Frame::Upright));
        for c in plates.iter().filter(|c| c.frame != Frame::Upright) {
            let m = transform(c, true, BLUM_FRONT_PLANE);
            assert_eq!(apply(&m, Vec3::new(0.0, 0.0, 25.0)).z, bottom.min.z);
            // The plate's long side points back to the middle.
            assert!(apply(&m, Vec3::new(52.7, 0.0, 0.0)).x < c.origin.x);
        }
    }

    #[test]
    fn a_light_profile_runs_along_the_panels_underside_lighting_down() {
        for (id, recessed) in [
            ("hafele.loox5.profile.1104", true),
            ("hafele.loox5.profile.2190", false),
        ] {
            let mut cab = Cabinet::new("W01", CabinetKind::Wall);
            cab.lights = vec![crate::model::Light {
                zone: crate::model::LightZone::UnderCabinet,
                voltage: crate::model::Voltage::V24,
                watts_per_metre: 9.6,
                profile: id.into(),
            }];
            let list = build_cabinet(&cab).unwrap();
            let bottom = list.part("W01.Bottom").unwrap().bounds;
            let item = crate::hardware::item(id).unwrap();
            let (z_min, z_max) = (
                item.param("model_z_min").unwrap(),
                item.param("model_z_max").unwrap(),
            );
            let along_y = item.param("model_along_y").unwrap() == 1.0;
            let section = item.param("section").unwrap();
            let pieces: Vec<_> = cad_placements(&list)
                .into_iter()
                .filter(|c| c.name.contains("LightProfile"))
                .collect();
            assert!(pieces.len() >= 2);
            for c in &pieces {
                let m = transform(c, true, BLUM_FRONT_PLANE);
                let light = apply(&m, Vec3::new(0.0, 0.0, z_max)).z;
                let back = apply(&m, Vec3::new(0.0, 0.0, z_min)).z;
                assert!(light < back, "lights downwards");
                let end = if along_y {
                    Vec3::new(0.0, section, 0.0)
                } else {
                    Vec3::new(section, 0.0, 0.0)
                };
                let (a, b) = (apply(&m, Vec3::new(0.0, 0.0, 0.0)), apply(&m, end));
                assert!(b.x > a.x, "runs along the cabinet");
                assert!(b.x <= bottom.max().x + 1e-6);
                if recessed {
                    assert_eq!(
                        light,
                        bottom.min.z - LIGHT_FLANGE,
                        "just proud of the panel"
                    );
                } else {
                    assert_eq!(back, bottom.min.z, "its back on the panel");
                }
            }
        }
    }

    #[test]
    fn shelf_supports_stand_on_the_shelfs_own_holes_with_their_pins_in_the_side() {
        let mut cab = Cabinet::new("B01", CabinetKind::Base);
        cab.adjustable_shelves = 2;
        let list = build_cabinet(&cab).unwrap();
        let supports: Vec<_> = cad_placements(&list)
            .into_iter()
            .filter(|c| c.name.contains("ShelfSupport"))
            .collect();
        assert_eq!(supports.len(), 8, "four under each of the two shelves");
        for c in &supports {
            assert_eq!(c.article, "282.12.405");
            let m = transform(c, true, BLUM_FRONT_PLANE);
            let tip = apply(&m, Vec3::new(-7.4, 0.0, 0.0));
            let seat = apply(&m, Vec3::new(5.0, 0.0, 2.5));
            assert_eq!(seat.z - tip.z, 2.5, "upright on both sides");
            let inner = list
                .by_role(if tip.x < cab.width / 2.0 {
                    PartRole::LeftSide
                } else {
                    PartRole::RightSide
                })
                .next()
                .unwrap()
                .bounds;
            assert!(
                tip.x > inner.min.x && tip.x < inner.max().x,
                "pin in the side"
            );
            assert!(
                list.by_role(PartRole::Shelf)
                    .any(|s| (s.bounds.min.z - seat.z).abs() < 1e-6),
                "a shelf rests on the seat"
            );
        }
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
        // The profile once, its datum 4 above the floor.
        let profile = p.iter().find(|c| c.name == "D01.ServoProfile").unwrap();
        let floor = list.part("D01.Bottom").unwrap().bounds.max().z;
        assert!((profile.origin.x - centre).abs() < 1e-9);
        assert!((profile.origin.y - (nl + 19.0)).abs() < 1e-9);
        assert!((profile.origin.z - (floor + 4.0)).abs() < 1e-9);
        // Bumpers: 2 per front at its centre, 8 in from the side edges, their datum on the back
        // face, head towards the interior (the model's +Z is our +Y).
        let bumpers: Vec<_> = p
            .iter()
            .filter(|c| c.name.contains("ServoBumper"))
            .collect();
        assert_eq!(bumpers.len(), 6, "three fronts, two bumpers each");
        let front = list.part("D01.DrawerFront1").unwrap();
        for b in &bumpers {
            assert!((b.origin.y - front.bounds.max().y).abs() < 1e-9);
            let from_left = b.origin.x - front.bounds.min.x;
            let from_right = front.bounds.max().x - b.origin.x;
            assert!((from_left - 8.0).abs() < 1e-9 || (from_right - 8.0).abs() < 1e-9);
            let m = transform(b, true, BLUM_FRONT_PLANE);
            assert_eq!(apply(&m, Vec3::new(0.0, 0.0, 2.6)).y, b.origin.y + 2.6);
        }
        assert!((bumpers[0].origin.z - front.bounds.center().z).abs() < 1e-9);
    }
}
