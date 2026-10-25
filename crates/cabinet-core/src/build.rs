//! Cabinet → parts, machining and hardware.

use crate::geom::{Aabb, Axis, Vec3};
use crate::hinges::{self, HingeNeed};
use crate::joinery::ButtJoint;
use crate::model::{
    BackStyle, BottomJoint, Cabinet, CabinetKind, DoorOverlay, FrontLayout, HingeSide, Opening,
    ToeKick, TopJoint, WallHanging,
};
use crate::part::{
    BorePurpose, Dir, EdgeBanding, HardwarePlacement, Operation, Part, PartList, PartRole,
};
use crate::{Error, Result};

/// Depth of the grooves an inset back sits in.
const BACK_GROOVE_DEPTH: f64 = 8.0;
/// Distance from the carcass rear to the rear face of an inset back (the wall-hanging void).
const BACK_INSET: f64 = 16.0;
/// The narrowest door a blind corner may be left with: a hinged door narrower than this has no
/// room for hinge cups and a handle.
const MIN_BLIND_DOOR: f64 = 250.0;
/// A sink cabinet's pipe cut-out in the back: how high it reaches above the bottom panel, and how
/// far it stays in from the sides (room for the back's fixings and stiffness). Wastes and stop
/// valves sit low, under about 400 mm.
const SINK_CUTOUT_HEIGHT: f64 = 400.0;
const SINK_CUTOUT_SIDE_MARGIN: f64 = 50.0;
/// Doors taller than this (mm) get a TIP-ON unit near each end so they open square.
const TALL_DOOR: f64 = 1200.0;
/// Screw centres along each edge an applied back is screwed to.
const BACK_SCREW_SPACING: f64 = 150.0;
/// Pilot for a 4 × 20 screw through a 6 mm back: short enough to stay clear of the shelf-pin row
/// 37 mm in from the rear.
const BACK_SCREW_PILOT_DEPTH: f64 = 15.0;
/// Clearance each side of an adjustable shelf.
const SHELF_SIDE_CLEARANCE: f64 = 1.0;
/// 32 mm system: shelf-pin and hinge-plate rows sit 37 mm in from the front and back.
const SYSTEM32_SETBACK: f64 = 37.0;
const SYSTEM32_PITCH: f64 = 32.0;
const SHELF_PIN_DIAMETER: f64 = 5.0;
const SHELF_PIN_DEPTH: f64 = 10.0;
/// Our choice, not Blum's: the end hinges sit this far in from the door's top and bottom. The
/// hinge itself, its cup and plate come from the hardware catalog (see `hinges`).
const HINGE_END_OFFSET: f64 = 100.0;

/// Builds the part list for one cabinet.
pub fn build_cabinet(cab: &Cabinet) -> Result<PartList> {
    validate(cab)?;
    let mut b = Builder::new(cab);
    b.carcass();
    b.fronts();
    // After the fronts, so shelf-pin holes can make way for hinge plates.
    b.shelves();
    b.plinth();
    b.wall_hanging();
    b.sink();
    b.cover_caps();
    b.opening();
    let mut list = b.list;
    for p in &mut list.parts {
        let o = cab.origin;
        p.bounds.min = p.bounds.min + o;
    }
    offset_operations(&mut list, cab.origin);
    // Whatever the rules above missed shows to the user rather than at the machine.
    let clashes = crate::collisions::check(&list);
    list.warnings
        .extend(clashes.into_iter().map(|c| format!("clash: {c}")));
    Ok(list)
}

fn validate(cab: &Cabinet) -> Result<()> {
    let s = &cab.standard;
    let t = s.carcass_thickness;
    for (name, v) in [
        ("width", cab.width),
        ("depth", cab.depth),
        ("height", cab.height),
    ] {
        if !positive(v) {
            return Err(Error::Invalid(format!(
                "{name} must be a positive number, got {v}"
            )));
        }
    }
    if !(positive(t)
        && positive(s.front_thickness)
        && s.back_thickness.is_finite()
        && s.back_thickness >= 0.0)
    {
        return Err(Error::Invalid("board thicknesses must be positive".into()));
    }
    if cab.width < 2.0 * t + 50.0 {
        return Err(Error::TooSmall(format!(
            "width {} leaves no opening",
            cab.width
        )));
    }
    if cab.depth < 100.0 + s.back_thickness + BACK_INSET {
        return Err(Error::TooSmall(format!(
            "depth {} is too shallow",
            cab.depth
        )));
    }
    if cab.kind == CabinetKind::BlindCorner {
        let b = &cab.blind;
        if !(positive(b.filler) && b.width > b.filler) {
            return Err(Error::Invalid(format!(
                "blind width {} must be more than the filler {}",
                b.width, b.filler
            )));
        }
        if cab.width - b.width < MIN_BLIND_DOOR {
            return Err(Error::TooSmall(format!(
                "a {} mm blind leaves less than {MIN_BLIND_DOOR} mm for the door",
                b.width
            )));
        }
    }
    let kick = if cab.has_toe_kick() {
        s.toe_kick_height
    } else {
        0.0
    };
    if cab.height - kick < 2.0 * t + 50.0 {
        return Err(Error::TooSmall(format!(
            "height {} leaves no opening",
            cab.height
        )));
    }
    Ok(())
}

/// True for a finite size above zero (NaN is not positive).
fn positive(v: f64) -> bool {
    v.is_finite() && v > 0.0
}

fn offset_operations(list: &mut PartList, o: Vec3) {
    for op in &mut list.operations {
        match op {
            Operation::Bore { at, .. } => *at = *at + o,
            Operation::BiscuitSlot { bounds, .. } | Operation::Groove { bounds, .. } => {
                bounds.min = bounds.min + o
            }
        }
    }
    for h in &mut list.hardware {
        h.at = h.at + o;
    }
}

fn aabb(x0: f64, y0: f64, z0: f64, x1: f64, y1: f64, z1: f64) -> Aabb {
    Aabb::new(Vec3::new(x0, y0, z0), Vec3::new(x1 - x0, y1 - y0, z1 - z0))
}

struct Builder<'a> {
    cab: &'a Cabinet,
    list: PartList,
    /// Bottom of the carcass (top of the toe kick).
    carcass_z0: f64,
    /// Top face of the bottom panel and underside of the top (or rails): the opening.
    opening_z: (f64, f64),
    /// Where the interior ends at the back (front face of the back panel).
    interior_back: f64,
}

impl<'a> Builder<'a> {
    fn new(cab: &'a Cabinet) -> Self {
        Self {
            cab,
            list: PartList {
                cabinet: cab.id.clone(),
                ..PartList::default()
            },
            carcass_z0: 0.0,
            opening_z: (0.0, 0.0),
            interior_back: 0.0,
        }
    }

    fn name(&self, role: PartRole, n: Option<usize>) -> String {
        match n {
            Some(n) => format!("{}.{}{}", self.cab.id, role.code(), n),
            None => format!("{}.{}", self.cab.id, role.code()),
        }
    }

    fn add(
        &mut self,
        role: PartRole,
        n: Option<usize>,
        bounds: Aabb,
        thickness_axis: Axis,
        grain: Axis,
        edges: EdgeBanding,
    ) {
        let name = self.name(role, n);
        let s = &self.cab.standard;
        let board = match role {
            _ if role.is_front() => s.front_board.clone(),
            PartRole::Back if s.back != BackStyle::Structural => s.back_board.clone(),
            _ => s.carcass_board.clone(),
        };
        self.list.parts.push(Part {
            name,
            role,
            bounds,
            thickness_axis,
            grain_axis: Some(grain),
            board,
            edges,
        });
    }

    fn part(&self, role: PartRole) -> Option<Part> {
        self.list.by_role(role).next().cloned()
    }

    fn joint(&mut self, face: PartRole, end: PartRole, normal: Dir, along: Axis) {
        let (Some(face), Some(end)) = (self.part(face), self.part(end)) else {
            return;
        };
        self.joint_parts(&face, &end, normal, along);
    }

    fn joint_parts(&mut self, face: &Part, end: &Part, normal: Dir, along: Axis) {
        // Joints running back to front stop at the interior's back: behind it is the back
        // panel and its grooves.
        let limit = (along == Axis::Y).then_some((f64::NEG_INFINITY, self.interior_back));
        // Through an exposed side, screws would show from the room.
        let hidden = match face.role {
            PartRole::LeftSide => self.cab.exposed.left,
            PartRole::RightSide => self.cab.exposed.right,
            _ => false,
        };
        let joint = ButtJoint {
            face_part: face,
            end_part: end,
            normal,
            along,
            limit,
            hidden,
        };
        let ops = joint.operations(&self.cab.standard, &mut self.list.warnings);
        self.list.operations.extend(ops);
    }

    /// Handleless opening hardware: a TIP-ON unit per door (two on tall doors) and per drawer, or
    /// a SERVO-DRIVE unit per drawer plus the cabinet's power supply. Generic ids until the
    /// articles are in the hardware catalog (BLUM-2).
    fn opening(&mut self) {
        let opening = self.cab.standard.opening;
        if opening == Opening::Handles {
            return;
        }
        let fronts: Vec<Part> = self
            .list
            .parts
            .iter()
            .filter(|p| matches!(p.role, PartRole::Door | PartRole::DrawerFront))
            .cloned()
            .collect();
        let mut servo = 0;
        for f in &fronts {
            let (item, description, quantity) = match (f.role, opening) {
                (PartRole::DrawerFront, Opening::ServoDrive) => {
                    servo += 1;
                    (
                        "blum.servo-drive.drawer",
                        "SERVO-DRIVE drive unit for a drawer",
                        1,
                    )
                }
                (PartRole::DrawerFront, _) => (
                    "blum.tip-on.drawer",
                    "TIP-ON BLUMOTION unit set for a drawer",
                    1,
                ),
                _ => {
                    let n = if f.bounds.size.z > TALL_DOOR { 2 } else { 1 };
                    (
                        "blum.tip-on.door",
                        "TIP-ON unit for a door (its hinges have no spring)",
                        n,
                    )
                }
            };
            self.list.hardware.push(HardwarePlacement {
                item: item.into(),
                description: description.into(),
                at: f.bounds.center(),
                part: f.name.clone(),
                quantity,
            });
        }
        if opening == Opening::ServoDrive && fronts.iter().any(|f| f.role == PartRole::Door) {
            self.list
                .warnings
                .push("SERVO-DRIVE does not open hinged doors: they get TIP-ON".into());
        }
        if servo > 0 {
            let bottom = self.name(PartRole::Bottom, None);
            self.list.hardware.push(HardwarePlacement {
                item: "blum.servo-drive.power-supply".into(),
                description: "SERVO-DRIVE power supply and cable set".into(),
                at: Vec3::new(self.cab.width / 2.0, self.interior_back, self.carcass_z0),
                part: bottom,
                quantity: 1,
            });
        }
    }

    /// A cover cap for every screw head left showing, per panel.
    fn cover_caps(&mut self) {
        let mut per_part: Vec<(String, u32)> = Vec::new();
        for op in &self.list.operations {
            if let Operation::Bore {
                part,
                purpose: BorePurpose::Countersink,
                ..
            } = op
            {
                match per_part.iter_mut().find(|(p, _)| p == part) {
                    Some((_, n)) => *n += 1,
                    None => per_part.push((part.clone(), 1)),
                }
            }
        }
        for (part, quantity) in per_part {
            let at = self
                .list
                .part(&part)
                .map(|p| p.bounds.center())
                .unwrap_or_default();
            self.list.hardware.push(HardwarePlacement {
                item: "screw-cover-cap".into(),
                description: "Screw cover cap, colour to match the carcass".into(),
                at,
                part,
                quantity,
            });
        }
    }

    fn carcass(&mut self) {
        let cab = self.cab;
        let s = &cab.standard;
        let (w, d, h, t, bt) = (
            cab.width,
            cab.depth,
            cab.height,
            s.carcass_thickness,
            s.back_thickness,
        );
        let kick = if cab.has_toe_kick() {
            s.toe_kick_height
        } else {
            0.0
        };
        let notch = cab.has_toe_kick() && s.toe_kick == ToeKick::IntegratedNotch;
        self.carcass_z0 = kick;

        let side_depth = if s.back == BackStyle::Applied {
            d - bt
        } else {
            d
        };
        // The front face of the back panel, which is where the interior ends.
        let (back_y0, interior_back) = match s.back {
            BackStyle::InsetGroove => (d - BACK_INSET - bt, d - BACK_INSET - bt),
            BackStyle::Applied => (d - bt, d - bt),
            BackStyle::Structural => (d - t, d - t),
            BackStyle::None => (d, d),
        };
        self.interior_back = interior_back;
        // Horizontal panels stop at a structural back, otherwise run the full side depth.
        let horiz_depth = if s.back == BackStyle::Structural {
            d - t
        } else {
            side_depth
        };

        let bottom_under = s.bottom_joint == BottomJoint::UnderSides && !notch;
        let top_over = s.top_joint == TopJoint::OverSides;
        let side_z0 = if notch {
            0.0
        } else if bottom_under {
            kick + t
        } else {
            kick
        };
        let side_z1 = if top_over { h - t } else { h };

        let side_edges = EdgeBanding::front_only();
        self.add(
            PartRole::LeftSide,
            None,
            aabb(0.0, 0.0, side_z0, t, side_depth, side_z1),
            Axis::X,
            Axis::Z,
            side_edges,
        );
        self.add(
            PartRole::RightSide,
            None,
            aabb(w - t, 0.0, side_z0, w, side_depth, side_z1),
            Axis::X,
            Axis::Z,
            side_edges,
        );
        if notch {
            for (role, x0) in [(PartRole::LeftSide, 0.0), (PartRole::RightSide, w - t)] {
                let part = self.name(role, None);
                self.list.operations.push(Operation::Groove {
                    part,
                    bounds: aabb(x0, 0.0, 0.0, x0 + t, s.toe_kick_setback, kick),
                });
            }
        }

        let (bx0, bx1) = if bottom_under { (0.0, w) } else { (t, w - t) };
        self.add(
            PartRole::Bottom,
            None,
            aabb(bx0, 0.0, kick, bx1, horiz_depth, kick + t),
            Axis::Z,
            Axis::X,
            EdgeBanding::front_only(),
        );

        match s.top_joint {
            TopJoint::Rails => {
                let rw = s.rail_width.min(back_y0 / 2.0);
                self.add(
                    PartRole::FrontRail,
                    None,
                    aabb(t, 0.0, h - t, w - t, rw, h),
                    Axis::Z,
                    Axis::X,
                    EdgeBanding::front_only(),
                );
                // A sink bowl and its plumbing take the back rail's place.
                if cab.kind != CabinetKind::Sink {
                    self.add(
                        PartRole::BackRail,
                        None,
                        aabb(t, back_y0 - rw, h - t, w - t, back_y0, h),
                        Axis::Z,
                        Axis::X,
                        EdgeBanding::default(),
                    );
                }
            }
            TopJoint::BetweenSides => {
                self.add(
                    PartRole::Top,
                    None,
                    aabb(t, 0.0, h - t, w - t, horiz_depth, h),
                    Axis::Z,
                    Axis::X,
                    EdgeBanding::front_only(),
                );
            }
            TopJoint::OverSides => {
                let edges = EdgeBanding {
                    left: true,
                    right: true,
                    ..EdgeBanding::front_only()
                };
                self.add(
                    PartRole::Top,
                    None,
                    aabb(0.0, 0.0, h - t, w, horiz_depth, h),
                    Axis::Z,
                    Axis::X,
                    edges,
                );
            }
        }
        self.opening_z = (kick + t, h - t);

        // Back panel.
        let groove = BACK_GROOVE_DEPTH.min(t / 2.0);
        match s.back {
            BackStyle::InsetGroove => {
                let z0 = kick + t - groove;
                let z1 = if s.top_joint == TopJoint::Rails {
                    h
                } else {
                    h - t + groove
                };
                self.add(
                    PartRole::Back,
                    None,
                    aabb(t - groove, back_y0, z0, w - t + groove, back_y0 + bt, z1),
                    Axis::Y,
                    Axis::Z,
                    EdgeBanding::default(),
                );
                let left = self.name(PartRole::LeftSide, None);
                let right = self.name(PartRole::RightSide, None);
                let bottom = self.name(PartRole::Bottom, None);
                self.list.operations.push(Operation::Groove {
                    part: left,
                    bounds: aabb(t - groove, back_y0, side_z0, t, back_y0 + bt, side_z1),
                });
                self.list.operations.push(Operation::Groove {
                    part: right,
                    bounds: aabb(
                        w - t,
                        back_y0,
                        side_z0,
                        w - t + groove,
                        back_y0 + bt,
                        side_z1,
                    ),
                });
                self.list.operations.push(Operation::Groove {
                    part: bottom,
                    bounds: aabb(t, back_y0, kick + t - groove, w - t, back_y0 + bt, kick + t),
                });
                if s.top_joint != TopJoint::Rails {
                    let top = self.name(PartRole::Top, None);
                    self.list.operations.push(Operation::Groove {
                        part: top,
                        bounds: aabb(t, back_y0, h - t, w - t, back_y0 + bt, h - t + groove),
                    });
                }
            }
            BackStyle::Applied => {
                self.add(
                    PartRole::Back,
                    None,
                    aabb(0.0, d - bt, side_z0.min(kick), w, d, h),
                    Axis::Y,
                    Axis::Z,
                    EdgeBanding::default(),
                );
            }
            BackStyle::Structural => {
                self.add(
                    PartRole::Back,
                    None,
                    aabb(t, d - t, kick + t, w - t, d, h - t),
                    Axis::Y,
                    Axis::Z,
                    EdgeBanding::default(),
                );
            }
            BackStyle::None => {}
        }

        // Joints.
        let px = Dir::new(Axis::X, true);
        let nx = Dir::new(Axis::X, false);
        if bottom_under {
            self.joint(
                PartRole::Bottom,
                PartRole::LeftSide,
                Dir::new(Axis::Z, true),
                Axis::Y,
            );
            self.joint(
                PartRole::Bottom,
                PartRole::RightSide,
                Dir::new(Axis::Z, true),
                Axis::Y,
            );
        } else {
            self.joint(PartRole::LeftSide, PartRole::Bottom, px, Axis::Y);
            self.joint(PartRole::RightSide, PartRole::Bottom, nx, Axis::Y);
        }
        match s.top_joint {
            TopJoint::Rails => {
                for rail in [PartRole::FrontRail, PartRole::BackRail] {
                    self.joint(PartRole::LeftSide, rail, px, Axis::Y);
                    self.joint(PartRole::RightSide, rail, nx, Axis::Y);
                }
            }
            TopJoint::BetweenSides => {
                self.joint(PartRole::LeftSide, PartRole::Top, px, Axis::Y);
                self.joint(PartRole::RightSide, PartRole::Top, nx, Axis::Y);
            }
            TopJoint::OverSides => {
                self.joint(
                    PartRole::Top,
                    PartRole::LeftSide,
                    Dir::new(Axis::Z, false),
                    Axis::Y,
                );
                self.joint(
                    PartRole::Top,
                    PartRole::RightSide,
                    Dir::new(Axis::Z, false),
                    Axis::Y,
                );
            }
        }
        if s.back == BackStyle::Structural {
            self.joint(PartRole::LeftSide, PartRole::Back, px, Axis::Z);
            self.joint(PartRole::RightSide, PartRole::Back, nx, Axis::Z);
        }
        if s.back == BackStyle::Applied {
            self.back_screws();
        }
    }

    /// An applied back is screwed onto every rear edge it covers; a grooved back needs none.
    fn back_screws(&mut self) {
        let Some(back) = self.part(PartRole::Back) else {
            return;
        };
        // From the back towards the carcass in front of it.
        let forward = Dir::new(Axis::Y, false);
        for (role, along) in [
            (PartRole::LeftSide, Axis::Z),
            (PartRole::RightSide, Axis::Z),
            (PartRole::Bottom, Axis::X),
            (PartRole::Top, Axis::X),
            (PartRole::BackRail, Axis::X),
        ] {
            let Some(edge) = self.part(role) else {
                continue;
            };
            let joint = ButtJoint {
                face_part: &back,
                end_part: &edge,
                normal: forward,
                along,
                limit: None,
                hidden: false,
            };
            let ops = joint.screws_every(BACK_SCREW_SPACING, BACK_SCREW_PILOT_DEPTH);
            self.list.operations.extend(ops);
        }
    }

    fn shelves(&mut self) {
        let cab = self.cab;
        let n = cab.adjustable_shelves as usize;
        let s = &cab.standard;
        let (w, t) = (cab.width, s.carcass_thickness);
        let (z0, z1) = self.opening_z;
        let y1 = self.interior_back - 2.0;
        let mut fixed_z = Vec::new();
        if n > 0 {
            let step = (z1 - z0) / (n + 1) as f64;
            for i in 1..=n {
                // Snap to the 32 mm grid so a shelf rests on real pin holes.
                let raw = z0 + step * i as f64;
                let z = z0 + ((raw - z0) / SYSTEM32_PITCH).round() * SYSTEM32_PITCH;
                let fixed = cab.fixed_shelves.contains(&(i as u32));
                // A fixed shelf fills the width and reaches the back, like a bottom.
                let bounds = if fixed {
                    aabb(
                        t,
                        s.shelf_front_setback,
                        z,
                        w - t,
                        self.interior_back,
                        z + t,
                    )
                } else {
                    aabb(
                        t + SHELF_SIDE_CLEARANCE,
                        s.shelf_front_setback,
                        z,
                        w - t - SHELF_SIDE_CLEARANCE,
                        y1,
                        z + t,
                    )
                };
                self.add(
                    PartRole::Shelf,
                    Some(i),
                    bounds,
                    Axis::Z,
                    Axis::X,
                    EdgeBanding::front_only(),
                );
                if fixed {
                    fixed_z.push(z + t / 2.0);
                    let shelf = self.list.parts.last().cloned().unwrap();
                    let (left, right) = (
                        self.part(PartRole::LeftSide).unwrap(),
                        self.part(PartRole::RightSide).unwrap(),
                    );
                    self.joint_parts(&left, &shelf, Dir::new(Axis::X, true), Axis::Y);
                    self.joint_parts(&right, &shelf, Dir::new(Axis::X, false), Axis::Y);
                }
            }
            if fixed_z.len() < n {
                self.shelf_pin_rows(&fixed_z);
            }
        }
    }

    /// Shelf-pin holes on the inside of both sides: a front and a back row on the 32 mm grid,
    /// left out within a pitch of a fixed shelf (centres in `fixed_z`), where its fixings are.
    fn shelf_pin_rows(&mut self, fixed_z: &[f64]) {
        let cab = self.cab;
        let t = cab.standard.carcass_thickness;
        let (z0, z1) = self.opening_z;
        let rows = [SYSTEM32_SETBACK, self.interior_back - SYSTEM32_SETBACK];
        let first = z0 + 2.0 * SYSTEM32_PITCH;
        let left = self.name(PartRole::LeftSide, None);
        let right = self.name(PartRole::RightSide, None);
        let mut z = first;
        let mut skipped = 0;
        while z <= z1 - 2.0 * SYSTEM32_PITCH {
            if fixed_z.iter().any(|f| (z - f).abs() < SYSTEM32_PITCH) {
                z += SYSTEM32_PITCH;
                continue;
            }
            for y in rows {
                for (part, x, positive) in [(&left, t, false), (&right, cab.width - t, true)] {
                    let at = Vec3::new(x, y, z);
                    if self.bore_clashes(part, at, SHELF_PIN_DIAMETER) {
                        skipped += 1;
                        continue;
                    }
                    self.list.operations.push(Operation::Bore {
                        part: part.clone(),
                        at: Vec3::new(x, y, z),
                        direction: Dir::new(Axis::X, positive),
                        diameter: SHELF_PIN_DIAMETER,
                        depth: SHELF_PIN_DEPTH,
                        purpose: BorePurpose::ShelfPin,
                    });
                }
            }
            z += SYSTEM32_PITCH;
        }
        if skipped > 0 {
            self.list.warnings.push(format!(
                "{skipped} shelf-pin holes left out where they met hinge plates"
            ));
        }
    }

    /// True when a bore of `diameter` at `at` on `part` would run into an existing bore there.
    fn bore_clashes(&self, part: &str, at: Vec3, diameter: f64) -> bool {
        const WALL: f64 = 3.0;
        self.list.operations.iter().any(|op| match op {
            Operation::Bore {
                part: p,
                at: o,
                diameter: d,
                ..
            } if p == part => {
                let dist =
                    ((at.x - o.x).powi(2) + (at.y - o.y).powi(2) + (at.z - o.z).powi(2)).sqrt();
                dist < (diameter + d) / 2.0 + WALL
            }
            _ => false,
        })
    }

    /// The area the fronts cover, as (x0, x1, z0, z1, y0, y1).
    fn front_region(&self) -> (f64, f64, f64, f64, f64, f64) {
        let cab = self.cab;
        let s = &cab.standard;
        let (w, h, t, g, ft) = (
            cab.width,
            cab.height,
            s.carcass_thickness,
            s.front_gap,
            s.front_thickness,
        );
        let under_bench = matches!(
            cab.kind,
            CabinetKind::Base
                | CabinetKind::DrawerBank
                | CabinetKind::BlindCorner
                | CabinetKind::Sink
        );
        let top_gap = if under_bench { g } else { 0.0 };
        match s.overlay {
            DoorOverlay::Full => (g / 2.0, w - g / 2.0, self.carcass_z0, h - top_gap, -ft, 0.0),
            DoorOverlay::Half => (t / 2.0, w - t / 2.0, self.carcass_z0, h - top_gap, -ft, 0.0),
            DoorOverlay::Inset => {
                let (z0, z1) = self.opening_z;
                (t + g, w - t - g, z0 + g, z1 - g, 0.0, ft)
            }
        }
    }

    fn fronts(&mut self) {
        let cab = self.cab;
        let g = cab.standard.front_gap;
        let (mut x0, mut x1, z0, z1, y0, y1) = self.front_region();
        if cab.kind == CabinetKind::BlindCorner {
            (x0, x1) = self.blind_front(x0, x1, z0, z1, y0, y1);
        }
        match &cab.fronts {
            FrontLayout::Open => {}
            FrontLayout::Door { hinge_side } => {
                let b = aabb(x0, y0, z0, x1, y1, z1);
                self.door(1, b, *hinge_side);
            }
            FrontLayout::DoorPair => {
                let mid = (x0 + x1) / 2.0;
                self.door(1, aabb(x0, y0, z0, mid - g / 2.0, y1, z1), HingeSide::Left);
                self.door(2, aabb(mid + g / 2.0, y0, z0, x1, y1, z1), HingeSide::Right);
            }
            FrontLayout::Drawers { ratios } => {
                let ratios: Vec<f64> = ratios.iter().copied().filter(|r| *r > 0.0).collect();
                if ratios.is_empty() {
                    self.list
                        .warnings
                        .push("drawer layout has no drawers".into());
                    return;
                }
                let total: f64 = ratios.iter().sum();
                let usable = (z1 - z0) - g * (ratios.len() - 1) as f64;
                // Top to bottom.
                let mut top = z1;
                for (i, r) in ratios.iter().enumerate() {
                    let hgt = usable * r / total;
                    let b = aabb(x0, y0, top - hgt, x1, y1, top);
                    self.add(
                        PartRole::DrawerFront,
                        Some(i + 1),
                        b,
                        Axis::Y,
                        Axis::X,
                        EdgeBanding::all(),
                    );
                    self.list.hardware.push(HardwarePlacement {
                        item: "blum.legrabox.runner".into(),
                        description: "Drawer runner set (pair), length to suit".into(),
                        at: b.center(),
                        part: self.name(PartRole::DrawerFront, Some(i + 1)),
                        quantity: 1,
                    });
                    top -= hgt + g;
                }
            }
        }
    }

    /// The blind panel and filler of a blind corner cabinet, from its blind side; returns the
    /// front region left for the door.
    fn blind_front(&mut self, x0: f64, x1: f64, z0: f64, z1: f64, y0: f64, y1: f64) -> (f64, f64) {
        let cab = self.cab;
        let g = cab.standard.front_gap;
        let b = cab.blind;
        // Measured from the cabinet's edge on the blind side, so it matches the other run.
        let (panel, filler, open) = match b.side {
            HingeSide::Left => (
                (x0, b.width - b.filler),
                (b.width - b.filler, b.width),
                (b.width + g, x1),
            ),
            HingeSide::Right => (
                (cab.width - b.width + b.filler, x1),
                (cab.width - b.width, cab.width - b.width + b.filler),
                (x0, cab.width - b.width - g),
            ),
        };
        for (role, (a, c)) in [(PartRole::BlindPanel, panel), (PartRole::Filler, filler)] {
            self.add(
                role,
                None,
                aabb(a, y0, z0, c, y1, z1),
                Axis::Y,
                Axis::Z,
                EdgeBanding::all(),
            );
        }
        open
    }

    fn door(&mut self, n: usize, b: Aabb, hinge: HingeSide) {
        let cab = self.cab;
        let s = &cab.standard;
        self.add(
            PartRole::Door,
            Some(n),
            b,
            Axis::Y,
            Axis::Z,
            EdgeBanding::all(),
        );
        let door = self.name(PartRole::Door, Some(n));
        let side_role = match hinge {
            HingeSide::Left => PartRole::LeftSide,
            HingeSide::Right => PartRole::RightSide,
        };
        let side = self.name(side_role, None);
        let height = b.size.z;
        let count = match height {
            h if h <= 900.0 => 2,
            h if h <= 1600.0 => 3,
            h if h <= 2000.0 => 4,
            _ => 5,
        };
        // How far the door covers its hinge side's front edge (negative: inset, its gap).
        let t = s.carcass_thickness;
        let front_overlay = match hinge {
            HingeSide::Left => t - b.min.x,
            HingeSide::Right => b.max().x - (cab.width - t),
        };
        let need = HingeNeed {
            overlay: s.overlay,
            front_overlay,
            door_thickness: b.size.y,
            side_thickness: t,
            gap: s.front_gap,
            sprung: s.opening == Opening::Handles,
        };
        let choice = match hinges::select(&need) {
            Ok(c) => c,
            Err(e) => {
                self.list
                    .warnings
                    .push(format!("{door}: {e}; no hinges placed"));
                return;
            }
        };
        let numbers = (|| {
            Ok::<_, String>((
                choice.cup_inset()?,
                choice.hinge.param("cup_diameter")?,
                choice.hinge.param("cup_depth")?,
                choice.plate.param("setback")?,
            ))
        })();
        let Ok((cup_inset, cup_diameter, cup_depth, pattern_setback)) = numbers else {
            self.list
                .warnings
                .push(format!("{door}: {}", numbers.unwrap_err()));
            return;
        };
        for w in &choice.warnings {
            self.list.warnings.push(format!("{door}: {w}"));
        }
        let cup_x = match hinge {
            HingeSide::Left => b.min.x + cup_inset,
            HingeSide::Right => b.max().x - cup_inset,
        };
        // The cup is bored into the door's back face, towards the front.
        let back_y = b.max().y;
        let (inner_x, toward_side) = match hinge {
            HingeSide::Left => (t, false),
            HingeSide::Right => (cab.width - t, true),
        };
        // The carcass front is Y = 0 whatever the overlay; the plate is set out from it.
        let plate_y = choice.plate_setback;
        let span = height - 2.0 * HINGE_END_OFFSET;
        for i in 0..count {
            let z = b.min.z + HINGE_END_OFFSET + span * i as f64 / (count - 1) as f64;
            self.list.operations.push(Operation::Bore {
                part: door.clone(),
                at: Vec3::new(cup_x, back_y, z),
                direction: Dir::new(Axis::Y, false),
                diameter: cup_diameter,
                depth: cup_depth.min(b.size.y - 3.0),
                purpose: BorePurpose::HingeCup,
            });
            // The plate's pattern: x along the side (up), y in from its front edge.
            for hole in &choice.plate.drilling {
                self.list.operations.push(Operation::Bore {
                    part: side.clone(),
                    at: Vec3::new(inner_x, plate_y + hole.y - pattern_setback, z + hole.x),
                    direction: Dir::new(Axis::X, toward_side),
                    diameter: hole.diameter,
                    depth: hole.depth.min(t - 3.0),
                    purpose: BorePurpose::HingePlate,
                });
            }
            self.list.hardware.push(HardwarePlacement {
                item: choice.hinge.id.clone(),
                description: format!("{} {}", choice.hinge.family, choice.hinge.variant),
                at: Vec3::new(cup_x, back_y, z),
                part: door.clone(),
                quantity: 1,
            });
            self.list.hardware.push(HardwarePlacement {
                item: choice.plate.id.clone(),
                description: format!("{} {}", choice.plate.family, choice.plate.variant),
                at: Vec3::new(inner_x, plate_y, z),
                part: side.clone(),
                quantity: 1,
            });
        }
    }

    /// Hangers for a wall cabinet: one in each top back corner, fixed to the inside of the side in
    /// the void behind an inset back, and a wall rail across the width for them to hook onto.
    /// `at` is the corner of the void each hanger sits in; the hanger's own drilling comes from its
    /// catalog record (BLUM-1 / HAF data), not from here.
    fn wall_hanging(&mut self) {
        let cab = self.cab;
        let s = &cab.standard;
        if cab.kind != CabinetKind::Wall || s.wall_hanging == WallHanging::None {
            return;
        }
        if s.back != BackStyle::InsetGroove {
            self.list.warnings.push(
                "wall hangers need the void behind an inset back; with this back they are \
                 left out"
                    .into(),
            );
            return;
        }
        let (w, d, h, t) = (cab.width, cab.depth, cab.height, s.carcass_thickness);
        let void_y = d - BACK_INSET;
        for (role, x) in [(PartRole::LeftSide, t), (PartRole::RightSide, w - t)] {
            self.list.hardware.push(HardwarePlacement {
                item: "wall-hanger.concealed".into(),
                description: "Concealed adjustable wall cabinet hanger".into(),
                at: Vec3::new(x, void_y, h),
                part: self.name(role, None),
                quantity: 1,
            });
        }
        self.list.hardware.push(HardwarePlacement {
            item: "wall-hanger.rail".into(),
            description: format!("Wall rail for cabinet hangers, cut to {w:.0} mm"),
            at: Vec3::new(w / 2.0, d, h),
            part: self.name(PartRole::Back, None),
            quantity: 1,
        });
    }

    /// A sink cabinet's pipe cut-out in the back and its waterproof liner.
    fn sink(&mut self) {
        let cab = self.cab;
        if cab.kind != CabinetKind::Sink {
            return;
        }
        let t = cab.standard.carcass_thickness;
        let (z0, _) = self.opening_z;
        if cab.sink.back_cutout {
            if let Some(back) = self.part(PartRole::Back) {
                // Through the back, from the bottom panel up, leaving the sides' fixings clear.
                let b = &back.bounds;
                let x0 = (t + SINK_CUTOUT_SIDE_MARGIN).max(b.min.x);
                let x1 = (cab.width - t - SINK_CUTOUT_SIDE_MARGIN).min(b.max().x);
                let z1 = (z0 + SINK_CUTOUT_HEIGHT).min(b.max().z);
                self.list.operations.push(Operation::Groove {
                    part: back.name.clone(),
                    bounds: aabb(x0, b.min.y, z0, x1, b.max().y, z1),
                });
            } else {
                self.list
                    .warnings
                    .push("no back to cut the pipe opening in".into());
            }
        }
        if cab.sink.liner {
            let bottom = self
                .part(PartRole::Bottom)
                .expect("every carcass has a bottom");
            let (iw, id) = (cab.width - 2.0 * t, self.interior_back);
            self.list.hardware.push(HardwarePlacement {
                item: "sink-base-liner".into(),
                description: format!("Waterproof sink base liner, cut to {iw:.0} × {id:.0} mm"),
                at: Vec3::new(cab.width / 2.0, id / 2.0, bottom.bounds.max().z),
                part: bottom.name,
                quantity: 1,
            });
        }
    }

    fn plinth(&mut self) {
        let cab = self.cab;
        let s = &cab.standard;
        if !cab.has_toe_kick() || s.toe_kick == ToeKick::IntegratedNotch {
            return;
        }
        let ft = s.front_thickness;
        let (w, kick, sb) = (cab.width, s.toe_kick_height, s.toe_kick_setback);
        self.add(
            PartRole::Plinth,
            None,
            aabb(0.0, sb, 0.0, w, sb + ft, kick),
            Axis::Y,
            Axis::X,
            EdgeBanding {
                top: true,
                ..EdgeBanding::default()
            },
        );
        if s.toe_kick == ToeKick::LegsAndPlinth {
            let xs: Vec<f64> = if w > 900.0 {
                vec![50.0, w / 2.0, w - 50.0]
            } else {
                vec![50.0, w - 50.0]
            };
            let bottom = self.name(PartRole::Bottom, None);
            for x in xs {
                for y in [sb + ft + 50.0, cab.depth - 50.0] {
                    self.list.hardware.push(HardwarePlacement {
                        item: "hafele.leg.adjustable".into(),
                        description: "Adjustable cabinet leg with plinth clip".into(),
                        at: Vec3::new(x, y, 0.0),
                        part: bottom.clone(),
                        quantity: 1,
                    });
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ConstructionStandard, FixingRecipe};

    fn base() -> Cabinet {
        Cabinet::new("B01", CabinetKind::Base)
    }

    fn overlapping(list: &PartList) -> Vec<(String, String)> {
        let mut out = Vec::new();
        for (i, a) in list.parts.iter().enumerate() {
            for b in &list.parts[i + 1..] {
                if a.bounds.overlaps(&b.bounds) {
                    out.push((a.name.clone(), b.name.clone()));
                }
            }
        }
        out
    }

    #[test]
    fn base_cabinet_has_the_expected_parts() {
        let list = build_cabinet(&base()).unwrap();
        let names: Vec<&str> = list.parts.iter().map(|p| p.name.as_str()).collect();
        for n in [
            "B01.LeftSide",
            "B01.RightSide",
            "B01.Bottom",
            "B01.FrontRail",
            "B01.BackRail",
            "B01.Back",
            "B01.Shelf1",
            "B01.Door1",
            "B01.Plinth",
        ] {
            assert!(names.contains(&n), "missing {n} in {names:?}");
        }
    }

    #[test]
    fn panels_do_not_overlap_except_backs_in_grooves() {
        let list = build_cabinet(&base()).unwrap();
        let bad: Vec<_> = overlapping(&list)
            .into_iter()
            .filter(|(a, b)| !a.ends_with(".Back") && !b.ends_with(".Back"))
            .collect();
        assert!(bad.is_empty(), "{bad:?}");
    }

    #[test]
    fn every_top_and_bottom_style_builds_without_overlaps() {
        for top in [TopJoint::Rails, TopJoint::BetweenSides, TopJoint::OverSides] {
            for bottom in [BottomJoint::BetweenSides, BottomJoint::UnderSides] {
                for back in [
                    BackStyle::InsetGroove,
                    BackStyle::Applied,
                    BackStyle::Structural,
                    BackStyle::None,
                ] {
                    let mut c = base();
                    c.standard.top_joint = top;
                    c.standard.bottom_joint = bottom;
                    c.standard.back = back;
                    let list = build_cabinet(&c).unwrap();
                    let bad: Vec<_> = overlapping(&list)
                        .into_iter()
                        .filter(|(a, b)| {
                            back != BackStyle::InsetGroove
                                || (!a.ends_with(".Back") && !b.ends_with(".Back"))
                        })
                        .collect();
                    assert!(bad.is_empty(), "{top:?} {bottom:?} {back:?}: {bad:?}");
                }
            }
        }
    }

    #[test]
    fn carcass_fits_the_envelope() {
        let c = base();
        let list = build_cabinet(&c).unwrap();
        for p in list
            .parts
            .iter()
            .filter(|p| !matches!(p.role, PartRole::Door | PartRole::DrawerFront))
        {
            let max = p.bounds.max();
            assert!(
                p.bounds.min.x >= -1e-9 && max.x <= c.width + 1e-9,
                "{} x",
                p.name
            );
            assert!(
                p.bounds.min.y >= -1e-9 && max.y <= c.depth + 1e-9,
                "{} y",
                p.name
            );
            assert!(
                p.bounds.min.z >= -1e-9 && max.z <= c.height + 1e-9,
                "{} z",
                p.name
            );
        }
    }

    #[test]
    fn bottom_is_biscuited_and_screwed_to_both_sides() {
        let list = build_cabinet(&base()).unwrap();
        let slots = list
            .operations
            .iter()
            .filter(
                |o| matches!(o, Operation::BiscuitSlot { parts, .. } if parts[1] == "B01.Bottom"),
            )
            .count();
        let screws = list.operations.iter().filter(|o| matches!(o, Operation::Bore { part, purpose: BorePurpose::ScrewPilot, .. } if part == "B01.Bottom")).count();
        assert!(slots >= 4, "slots {slots}");
        assert_eq!(
            screws,
            slots - 2,
            "one screw between each pair of biscuits per side"
        );
    }

    #[test]
    fn a_fixed_shelf_is_joined_to_both_sides_like_a_bottom() {
        let mut cab = Cabinet::new("T01", CabinetKind::Tall);
        cab.fixed_shelves = vec![2];
        let list = build_cabinet(&cab).unwrap();
        let shelf = list.part("T01.Shelf2").unwrap();
        let t = cab.standard.carcass_thickness;
        assert_eq!(shelf.bounds.size.x, cab.width - 2.0 * t, "fills the width");
        for side in ["T01.LeftSide", "T01.RightSide"] {
            let slots = list
                .operations
                .iter()
                .filter(|o| {
                    matches!(o, Operation::BiscuitSlot { parts, .. }
                    if parts.contains(&side.to_string()) && parts.contains(&shelf.name))
                })
                .count();
            assert!(slots >= 2, "{side}: {slots} biscuits");
        }
        let screws = list
            .operations
            .iter()
            .filter(|o| {
                matches!(o, Operation::Bore { part, purpose: BorePurpose::ScrewPilot, .. }
                if *part == shelf.name)
            })
            .count();
        assert!(screws >= 2, "{screws} screws");
    }

    #[test]
    fn adjustable_shelves_have_no_joints_and_keep_their_clearance() {
        let list = build_cabinet(&Cabinet::new("T01", CabinetKind::Tall)).unwrap();
        let shelf = list.part("T01.Shelf2").unwrap();
        assert!(!list.operations.iter().any(|o| match o {
            Operation::BiscuitSlot { parts, .. } => parts.contains(&shelf.name),
            Operation::Bore { part, .. } => *part == shelf.name,
            Operation::Groove { part, .. } => *part == shelf.name,
        }));
        assert_eq!(shelf.bounds.min.x, 16.0 + SHELF_SIDE_CLEARANCE);
    }

    #[test]
    fn no_shelf_pins_beside_a_fixed_shelf_and_none_at_all_when_every_shelf_is_fixed() {
        let mut cab = Cabinet::new("T01", CabinetKind::Tall);
        cab.fixed_shelves = vec![2];
        let list = build_cabinet(&cab).unwrap();
        let shelf = list.part("T01.Shelf2").unwrap();
        let mid = shelf.bounds.center().z;
        assert!(list.operations.iter().any(|o| matches!(
            o,
            Operation::Bore {
                purpose: BorePurpose::ShelfPin,
                ..
            }
        )));
        assert!(!list.operations.iter().any(|o| matches!(o,
            Operation::Bore { purpose: BorePurpose::ShelfPin, at, .. } if (at.z - mid).abs() < SYSTEM32_PITCH)));

        cab.fixed_shelves = vec![1, 2, 3, 4];
        let list = build_cabinet(&cab).unwrap();
        assert!(!list.operations.iter().any(|o| matches!(
            o,
            Operation::Bore {
                purpose: BorePurpose::ShelfPin,
                ..
            }
        )));
    }

    #[test]
    fn fixed_shelf_fixings_stay_clear_of_other_bores_in_the_sides() {
        for kind in [CabinetKind::Base, CabinetKind::Wall, CabinetKind::Tall] {
            let mut cab = Cabinet::new("C01", kind);
            cab.adjustable_shelves = 3;
            cab.fixed_shelves = vec![1, 2, 3];
            let list = build_cabinet(&cab).unwrap();
            let side_bores: Vec<_> = list
                .operations
                .iter()
                .filter_map(|o| match o {
                    Operation::Bore {
                        purpose: BorePurpose::Countersink,
                        ..
                    } => None,
                    Operation::Bore {
                        part,
                        at,
                        diameter,
                        direction,
                        ..
                    } if part.ends_with("Side") && direction.axis == Axis::X => {
                        Some((part, at, diameter))
                    }
                    _ => None,
                })
                .collect();
            for (i, a) in side_bores.iter().enumerate() {
                for b in &side_bores[i + 1..] {
                    if a.0 != b.0 {
                        continue;
                    }
                    let gap = ((a.1.y - b.1.y).powi(2) + (a.1.z - b.1.z).powi(2)).sqrt();
                    assert!(
                        gap >= (a.2 + b.2) / 2.0,
                        "{kind:?}: bores {gap:.1} apart at {:?}",
                        a.1
                    );
                }
            }
        }
    }

    #[test]
    fn a_wall_cabinet_hangs_on_two_hangers_in_its_top_back_corners_and_a_rail() {
        let cab = Cabinet::new("W01", CabinetKind::Wall);
        let list = build_cabinet(&cab).unwrap();
        let hangers: Vec<_> = list
            .hardware
            .iter()
            .filter(|h| h.item == "wall-hanger.concealed")
            .collect();
        assert_eq!(hangers.len(), 2);
        assert_eq!(hangers[0].part, "W01.LeftSide");
        assert_eq!(hangers[1].part, "W01.RightSide");
        for h in &hangers {
            assert_eq!(h.at.z, cab.height, "at the top");
            assert_eq!(
                h.at.y,
                cab.depth - BACK_INSET,
                "in the void behind the back"
            );
        }
        // The back sits in front of the void the hangers and rail use.
        let back = list.part("W01.Back").unwrap();
        assert!(back.bounds.max().y <= cab.depth - BACK_INSET);
        assert_eq!(
            list.hardware
                .iter()
                .filter(|h| h.item == "wall-hanger.rail")
                .count(),
            1
        );
    }

    #[test]
    fn base_cabinets_and_unhung_wall_cabinets_have_no_hangers() {
        let base = build_cabinet(&Cabinet::new("B01", CabinetKind::Base)).unwrap();
        let mut wall = Cabinet::new("W01", CabinetKind::Wall);
        wall.standard.wall_hanging = WallHanging::None;
        let wall = build_cabinet(&wall).unwrap();
        for list in [base, wall] {
            assert!(
                !list
                    .hardware
                    .iter()
                    .any(|h| h.item.starts_with("wall-hanger"))
            );
        }
    }

    #[test]
    fn hangers_without_an_inset_back_are_left_out_with_a_warning() {
        let mut cab = Cabinet::new("W01", CabinetKind::Wall);
        cab.standard.back = BackStyle::Applied;
        let list = build_cabinet(&cab).unwrap();
        assert!(
            !list
                .hardware
                .iter()
                .any(|h| h.item.starts_with("wall-hanger"))
        );
        assert!(list.warnings.iter().any(|w| w.contains("wall hangers")));
    }

    #[test]
    fn a_blind_corner_has_a_blind_panel_a_filler_and_one_door_on_the_open_width() {
        let cab = Cabinet::new("C01", CabinetKind::BlindCorner);
        assert_eq!(cab.width, 1000.0);
        let list = build_cabinet(&cab).unwrap();
        let panel = list.part("C01.BlindPanel").unwrap();
        let filler = list.part("C01.Filler").unwrap();
        let doors: Vec<_> = list.by_role(PartRole::Door).collect();
        assert_eq!(doors.len(), 1);
        let door = doors[0];
        let (b, g) = (cab.blind, cab.standard.front_gap);
        // From the left: blind panel, filler, gap, door to the right edge's overlay gap.
        assert_eq!(panel.bounds.max().x, b.width - b.filler);
        assert_eq!(filler.bounds.min.x, panel.bounds.max().x);
        assert_eq!(filler.bounds.max().x, b.width);
        assert_eq!(door.bounds.min.x, b.width + g);
        assert_eq!(door.bounds.max().x, cab.width - g / 2.0);
        assert_eq!(door.bounds.size.x, cab.width - b.width - g - g / 2.0);
        // Hinged away from the corner, on the right side.
        assert!(
            list.hardware
                .iter()
                .any(|h| h.item.contains("clip-top") && h.part == door.name)
        );
        assert!(list.operations.iter().any(|o| matches!(o,
            Operation::Bore { part, purpose: BorePurpose::HingePlate, .. } if part == "C01.RightSide")));
        for p in [panel, filler] {
            assert!(p.role.is_front());
            assert_eq!(p.thickness(), cab.standard.front_thickness);
            assert_eq!(
                (p.bounds.min.z, p.bounds.max().z),
                (door.bounds.min.z, door.bounds.max().z)
            );
        }
    }

    #[test]
    fn a_right_hand_blind_corner_mirrors_and_hinges_on_the_left() {
        let mut cab = Cabinet::new("C01", CabinetKind::BlindCorner);
        cab.blind.side = HingeSide::Right;
        cab.fronts = FrontLayout::Door {
            hinge_side: HingeSide::Left,
        };
        let list = build_cabinet(&cab).unwrap();
        let filler = list.part("C01.Filler").unwrap();
        assert_eq!(filler.bounds.min.x, cab.width - cab.blind.width);
        let door = list.by_role(PartRole::Door).next().unwrap();
        assert_eq!(
            door.bounds.max().x,
            cab.width - cab.blind.width - cab.standard.front_gap
        );
    }

    #[test]
    fn a_blind_too_wide_for_a_door_is_refused() {
        let mut cab = Cabinet::new("C01", CabinetKind::BlindCorner);
        cab.blind.width = 900.0;
        assert!(matches!(build_cabinet(&cab), Err(Error::TooSmall(_))));
        cab.blind.width = 40.0;
        assert!(matches!(build_cabinet(&cab), Err(Error::Invalid(_))));
    }

    #[test]
    fn a_sink_cabinet_has_no_back_rail_a_pipe_cut_out_and_a_liner() {
        let cab = Cabinet::new("S01", CabinetKind::Sink);
        let list = build_cabinet(&cab).unwrap();
        assert!(list.part("S01.FrontRail").is_some());
        assert!(list.part("S01.BackRail").is_none());
        assert_eq!(list.by_role(PartRole::Shelf).count(), 0);
        let back = list.part("S01.Back").unwrap();
        let cutout = list
            .operations
            .iter()
            .find_map(|o| match o {
                Operation::Groove { part, bounds } if *part == back.name => Some(bounds),
                _ => None,
            })
            .expect("a pipe cut-out in the back");
        assert_eq!(cutout.size.y, back.thickness(), "through the back");
        assert_eq!(
            cutout.min.z,
            list.part("S01.Bottom").unwrap().bounds.max().z
        );
        let liner = list
            .hardware
            .iter()
            .find(|h| h.item == "sink-base-liner")
            .expect("liner in the hardware list");
        assert_eq!(liner.part, "S01.Bottom");
        assert!(
            liner.description.contains("868 × 558"),
            "{}",
            liner.description
        );
    }

    #[test]
    fn sink_options_can_be_turned_off() {
        let mut cab = Cabinet::new("S01", CabinetKind::Sink);
        cab.sink.back_cutout = false;
        cab.sink.liner = false;
        let list = build_cabinet(&cab).unwrap();
        assert!(!list.hardware.iter().any(|h| h.item == "sink-base-liner"));
        assert!(
            !list
                .operations
                .iter()
                .any(|o| matches!(o, Operation::Groove { part, .. } if part == "S01.Back"))
        );
    }

    #[test]
    fn each_part_is_made_from_its_slots_board_and_thickness() {
        use crate::model::BoardSlot;
        let mut cab = Cabinet::new("B01", CabinetKind::Base);
        cab.standard
            .set_board(BoardSlot::Carcass, "carcass-18", 18.0);
        cab.standard.set_board(BoardSlot::Front, "front-16", 16.0);
        cab.standard.set_board(BoardSlot::Back, "back-9", 9.0);
        let list = build_cabinet(&cab).unwrap();
        for p in &list.parts {
            let (board, t) = match p.role {
                r if r.is_front() => ("front-16", 16.0),
                PartRole::Back => ("back-9", 9.0),
                _ => ("carcass-18", 18.0),
            };
            assert_eq!(p.board, board, "{}", p.name);
            assert_eq!(p.thickness(), t, "{}", p.name);
        }
    }

    #[test]
    fn every_screw_is_countersunk_and_capped() {
        let list = build_cabinet(&Cabinet::new("B01", CabinetKind::Base)).unwrap();
        let screws: Vec<_> = list
            .operations
            .iter()
            .filter_map(|o| match o {
                Operation::Bore {
                    part,
                    at,
                    purpose: BorePurpose::Screw,
                    ..
                } => Some((part, at)),
                _ => None,
            })
            .collect();
        assert!(!screws.is_empty());
        for (part, at) in &screws {
            assert!(
                list.operations.iter().any(|o| matches!(o,
                Operation::Bore { part: p, at: a, purpose: BorePurpose::Countersink, diameter, .. }
                    if p == *part && a == *at && *diameter == 8.0)),
                "{part} {at:?}"
            );
        }
        let caps: u32 = list
            .hardware
            .iter()
            .filter(|h| h.item == "screw-cover-cap")
            .map(|h| h.quantity)
            .sum();
        assert_eq!(caps as usize, screws.len());
    }

    #[test]
    fn an_exposed_end_has_no_screws_through_it() {
        let mut cab = Cabinet::new("B01", CabinetKind::Base);
        cab.exposed.left = true;
        let list = build_cabinet(&cab).unwrap();
        let through = |side: &str| {
            list.operations
                .iter()
                .filter(|o| matches!(o, Operation::Bore { part, purpose: BorePurpose::Screw, .. } if part == side))
                .count()
        };
        assert_eq!(through("B01.LeftSide"), 0);
        assert!(
            through("B01.RightSide") > 0,
            "the hidden side keeps its screws"
        );
        // Its joints are still biscuited.
        assert!(list.operations.iter().any(|o| matches!(o,
            Operation::BiscuitSlot { parts, .. } if parts[0] == "B01.LeftSide")));
    }

    #[test]
    fn an_applied_back_is_screwed_to_every_rear_edge_at_150_centres() {
        let mut cab = Cabinet::new("B01", CabinetKind::Base);
        cab.standard.back = BackStyle::Applied;
        let list = build_cabinet(&cab).unwrap();
        let back = list.part("B01.Back").unwrap();
        for (edge, along) in [
            ("B01.LeftSide", Axis::Z),
            ("B01.RightSide", Axis::Z),
            ("B01.Bottom", Axis::X),
            ("B01.BackRail", Axis::X),
        ] {
            let edge_part = list.part(edge).unwrap();
            // Pilots into this edge, and the matching holes through the back.
            let mut at: Vec<f64> = list
                .operations
                .iter()
                .filter_map(|o| match o {
                    Operation::Bore {
                        part,
                        at,
                        purpose: BorePurpose::ScrewPilot,
                        ..
                    } if part == edge && at.y == back.bounds.min.y => Some(at.get(along)),
                    _ => None,
                })
                .collect();
            at.sort_by(f64::total_cmp);
            assert!(at.len() >= 2, "{edge}: {at:?}");
            assert!(
                at.windows(2)
                    .all(|w| w[1] - w[0] <= BACK_SCREW_SPACING + 1e-9),
                "{edge}: {at:?}"
            );
            assert!(
                at.iter().all(|p| *p > edge_part.bounds.min.get(along)
                    && *p < edge_part.bounds.max().get(along))
            );
        }
        let through_back = list
            .operations
            .iter()
            .filter(|o| matches!(o, Operation::Bore { part, purpose: BorePurpose::Screw, .. } if *part == back.name))
            .count();
        assert!(through_back >= 8);
    }

    #[test]
    fn a_grooved_back_has_no_fixings() {
        let list = build_cabinet(&Cabinet::new("B01", CabinetKind::Base)).unwrap();
        assert!(!list.operations.iter().any(|o| matches!(o,
            Operation::Bore { part, .. } if part == "B01.Back")));
    }

    #[test]
    fn tip_on_puts_a_unit_on_every_front() {
        let mut tall = Cabinet::new("T01", CabinetKind::Tall);
        tall.standard.opening = Opening::TipOn;
        let list = build_cabinet(&tall).unwrap();
        let units: u32 = list
            .hardware
            .iter()
            .filter(|h| h.item == "blum.tip-on.door")
            .map(|h| h.quantity)
            .sum();
        assert_eq!(units, 2, "one tall door, two units");
        let mut bank = Cabinet::new("D01", CabinetKind::DrawerBank);
        bank.standard.opening = Opening::TipOn;
        let list = build_cabinet(&bank).unwrap();
        assert_eq!(
            list.hardware
                .iter()
                .filter(|h| h.item == "blum.tip-on.drawer")
                .count(),
            3
        );
    }

    #[test]
    fn servo_drive_drives_drawers_and_leaves_doors_to_tip_on() {
        let mut bank = Cabinet::new("D01", CabinetKind::DrawerBank);
        bank.standard.opening = Opening::ServoDrive;
        let list = build_cabinet(&bank).unwrap();
        assert_eq!(
            list.hardware
                .iter()
                .filter(|h| h.item == "blum.servo-drive.drawer")
                .count(),
            3
        );
        assert_eq!(
            list.hardware
                .iter()
                .filter(|h| h.item == "blum.servo-drive.power-supply")
                .count(),
            1
        );
        let mut base = Cabinet::new("B01", CabinetKind::Base);
        base.standard.opening = Opening::ServoDrive;
        let list = build_cabinet(&base).unwrap();
        assert!(list.hardware.iter().any(|h| h.item == "blum.tip-on.door"));
        assert!(
            !list
                .hardware
                .iter()
                .any(|h| h.item.starts_with("blum.servo-drive"))
        );
        assert!(list.warnings.iter().any(|w| w.contains("SERVO-DRIVE")));
    }

    #[test]
    fn handles_add_no_opening_hardware() {
        let list = build_cabinet(&Cabinet::new("B01", CabinetKind::Base)).unwrap();
        assert!(
            !list
                .hardware
                .iter()
                .any(|h| h.item.contains("tip-on") || h.item.contains("servo"))
        );
    }

    #[test]
    fn rails_are_screwed_not_biscuited() {
        let list = build_cabinet(&base()).unwrap();
        assert!(!list.operations.iter().any(
            |o| matches!(o, Operation::BiscuitSlot { parts, .. } if parts[1].ends_with("Rail"))
        ));
        assert!(
            list.operations
                .iter()
                .any(|o| matches!(o, Operation::Bore { part, .. } if part.ends_with("FrontRail")))
        );
    }

    #[test]
    fn hinges_scale_with_door_height() {
        let mut tall = Cabinet::new("T01", CabinetKind::Tall);
        tall.fronts = FrontLayout::Door {
            hinge_side: HingeSide::Right,
        };
        let list = build_cabinet(&tall).unwrap();
        let cups = list
            .operations
            .iter()
            .filter(|o| {
                matches!(
                    o,
                    Operation::Bore {
                        purpose: BorePurpose::HingeCup,
                        ..
                    }
                )
            })
            .count();
        assert_eq!(cups, 4);
    }

    #[test]
    fn wide_base_gets_a_door_pair() {
        let mut c = base();
        c.width = 900.0;
        c.fronts = CabinetKind::Base.default_fronts_for(900.0);
        let list = build_cabinet(&c).unwrap();
        assert_eq!(list.by_role(PartRole::Door).count(), 2);
    }

    #[test]
    fn drawer_fronts_fill_the_front_with_gaps() {
        let c = Cabinet::new("D01", CabinetKind::DrawerBank);
        let list = build_cabinet(&c).unwrap();
        let fronts: Vec<_> = list.by_role(PartRole::DrawerFront).collect();
        assert_eq!(fronts.len(), 3);
        let g = c.standard.front_gap;
        for w in fronts.windows(2) {
            let gap = w[0].bounds.min.z - w[1].bounds.max().z;
            assert!((gap - g).abs() < 1e-9, "gap {gap}");
        }
    }

    #[test]
    fn origin_moves_everything() {
        let mut c = base();
        c.origin = Vec3::new(1000.0, 0.0, 0.0);
        let list = build_cabinet(&c).unwrap();
        assert_eq!(list.part("B01.LeftSide").unwrap().bounds.min.x, 1000.0);
    }

    #[test]
    fn screw_only_recipe_has_no_slots() {
        let mut c = base();
        c.standard = ConstructionStandard {
            fixings: FixingRecipe::ScrewOnly,
            ..ConstructionStandard::default()
        };
        let list = build_cabinet(&c).unwrap();
        assert!(
            !list
                .operations
                .iter()
                .any(|o| matches!(o, Operation::BiscuitSlot { .. }))
        );
    }

    #[test]
    fn no_two_bores_on_a_part_collide() {
        for kind in CabinetKind::ALL {
            let list = build_cabinet(&Cabinet::new("C", kind)).unwrap();
            let bores: Vec<_> = list
                .operations
                .iter()
                .filter_map(|o| match o {
                    // A countersink is its screw hole's seat, not another hole.
                    Operation::Bore {
                        purpose: BorePurpose::Countersink,
                        ..
                    } => None,
                    Operation::Bore {
                        part, at, diameter, ..
                    } => Some((part, *at, *diameter)),
                    _ => None,
                })
                .collect();
            for (i, a) in bores.iter().enumerate() {
                for b in &bores[i + 1..] {
                    if a.0 != b.0 {
                        continue;
                    }
                    let d = ((a.1.x - b.1.x).powi(2)
                        + (a.1.y - b.1.y).powi(2)
                        + (a.1.z - b.1.z).powi(2))
                    .sqrt();
                    assert!(
                        d >= (a.2 + b.2) / 2.0,
                        "{kind:?}: bores on {} at {:?} and {:?}",
                        a.0,
                        a.1,
                        b.1
                    );
                }
            }
        }
    }

    #[test]
    fn too_narrow_is_refused() {
        let mut c = base();
        c.width = 60.0;
        assert!(matches!(build_cabinet(&c), Err(Error::TooSmall(_))));
    }
}
