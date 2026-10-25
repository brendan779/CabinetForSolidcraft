//! Cabinet → parts, machining and hardware.

use crate::corner;
use crate::drawers::{self, BoxSystem, WoodRunner};
use crate::geom::{Aabb, Axis, Vec3};
use crate::hinges::{self, HingeNeed};
use crate::joinery::ButtJoint;
use crate::lift;
use crate::model::{
    BackStyle, BottomJoint, Cabinet, CabinetKind, DoorOverlay, DrawerSystem, Exposed, FrontCore,
    FrontLayout, HingeSide, LiftKind, Opening, ToeKick, TopJoint, WallHanging,
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
/// The narrowest door either leg of an L-corner may be left with beside the other leg.
const MIN_CORNER_DOOR: f64 = 250.0;
/// A sink cabinet's pipe cut-out in the back: how high it reaches above the bottom panel, and how
/// far it stays in from the sides (room for the back's fixings and stiffness). Wastes and stop
/// valves sit low, under about 400 mm.
const SINK_CUTOUT_HEIGHT: f64 = 400.0;
const SINK_CUTOUT_SIDE_MARGIN: f64 = 50.0;
/// Doors taller than this (mm) get a TIP-ON unit near each end so they open square.
const TALL_DOOR: f64 = 1200.0;
/// Our call, not Blum's: a wooden drawer with sides at least this high is a deep drawer for pots
/// and gets the TIP-ON BLUMOTION unit for 15–40 kg.
const DEEP_WOOD_DRAWER: f64 = 150.0;
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
    let mut list = if matches!(cab.kind, CabinetKind::CornerBase | CabinetKind::CornerWall) {
        l_corner(cab)?
    } else {
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
        b.list
    };
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

/// An L-shaped corner base, from two straight legs: the main leg as a base cabinet along X, the
/// second built the same way, its end against the main leg opened, and turned to face +X in
/// front of the main leg's end. Bi-fold doors close both fronts. A corner wall's legs are wall
/// cabinets: the main leg hangs on its two hangers, the second on one at its outer end and, at
/// its open end, on the main leg's corner hanger (owner, 2026-10-26).
fn l_corner(cab: &Cabinet) -> Result<PartList> {
    let (w, d, w2) = (cab.width, cab.depth, cab.corner.width);
    for (name, v) in [("width", w), ("corner width", w2)] {
        if !(v.is_finite() && v >= d + MIN_CORNER_DOOR) {
            return Err(Error::TooSmall(format!(
                "an L-corner {name} of {v} leaves less than {MIN_CORNER_DOOR} mm of door beside \
                 the other leg"
            )));
        }
    }
    let kind = match cab.kind {
        CabinetKind::CornerWall => CabinetKind::Wall,
        _ => CabinetKind::Base,
    };
    let leg = |width: f64, exposed: Exposed| Cabinet {
        kind,
        width,
        fronts: FrontLayout::Open,
        origin: Vec3::default(),
        exposed,
        ..cab.clone()
    };
    // Built as the left-hand corner; a right-hand one is its mirror.
    let (out_a, out_b) = match cab.corner.side {
        HingeSide::Left => (cab.exposed.right, cab.exposed.left),
        HingeSide::Right => (cab.exposed.left, cab.exposed.right),
    };
    let main = leg(
        w,
        Exposed {
            left: false,
            right: out_a,
        },
    );
    let mut second = leg(
        w2 - d,
        Exposed {
            left: out_b,
            right: false,
        },
    );
    second.adjustable_shelves = 0;
    second.fixed_shelves.clear();
    let mut a = Builder::new(&main);
    a.carcass();
    // Before the shelves, so shelf-pin holes make way for the hinge plates.
    a.corner_doors();
    a.shelves();
    a.plinth();
    a.wall_hanging();
    let mut b = Builder::new(&second);
    b.carcass();
    b.plinth();
    // The hanger at the open end goes with its side.
    b.wall_hanging();
    let mut leg_b = b.list;
    corner::open_end(&mut leg_b, &format!("{}.RightSide", cab.id), w2 - d);
    corner::rename(&mut leg_b, |n| format!("{n}2"));
    corner::turn_to_face_x(&mut leg_b, d, -(w2 - d));
    // The plinths meet at the inside corner: the main one stops at the second's back face, the
    // second runs on under the main leg to meet it.
    let s = &cab.standard;
    let (sb, ft) = (s.toe_kick_setback, s.front_thickness);
    for p in a
        .list
        .parts
        .iter_mut()
        .filter(|p| p.role == PartRole::Plinth)
    {
        let end = p.bounds.max().x;
        p.bounds.min.x = d - sb;
        p.bounds.size.x = end - p.bounds.min.x;
    }
    for p in leg_b
        .parts
        .iter_mut()
        .filter(|p| p.role == PartRole::Plinth)
    {
        p.bounds.size.y = sb + ft - p.bounds.min.y;
    }
    a.list.parts.extend(leg_b.parts);
    a.list.operations.extend(leg_b.operations);
    a.list.hardware.extend(leg_b.hardware);
    a.list.warnings.extend(leg_b.warnings);
    a.cover_caps();
    a.opening();
    let mut list = a.list;
    if cab.corner.side == HingeSide::Right {
        corner::mirror_x(&mut list, w);
    }
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
            Operation::BiscuitSlot { bounds, .. }
            | Operation::Groove { bounds, .. }
            | Operation::Mortise { bounds, .. } => bounds.min = bounds.min + o,
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
    /// The drawer box behind each drawer front, for the opening hardware.
    drawer_fits: Vec<DrawerFit>,
}

/// What the opening hardware needs to know about a drawer's box.
struct DrawerFit {
    front: String,
    /// `legrabox`, `merivobox`, `tandembox`, `movento` or `tandem`.
    system: &'static str,
    nominal_length: f64,
    /// On the 70 kg LEGRABOX profile or 60/70 kg MOVENTO runner.
    heavy_runner: bool,
    /// A deep box (LEGRABOX K, wooden sides 150 mm or more): Blum's 15–40 kg TIP-ON unit.
    heavy_load: bool,
    front_height: f64,
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
            drawer_fits: Vec::new(),
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

    /// Handleless opening hardware: a TIP-ON unit on an adapter plate per door (two on tall
    /// doors) and per drawer, or a SERVO-DRIVE unit per drawer plus the cabinet's power supply.
    /// Drawer units keep generic ids until BLUM-13 catalogues them.
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
        let lw = self.cab.width - 2.0 * self.cab.standard.carcass_thickness;
        let mut servo_fronts: Vec<f64> = Vec::new();
        let mut servo_heavy = false;
        for f in &fronts {
            let fit = self.drawer_fits.iter().find(|d| d.front == f.name);
            let rows: Vec<(String, String, u32)> = match (f.role, opening, fit) {
                // SERVO-DRIVE is one system for the box systems and MOVENTO/TANDEM (p.364, p.464);
                // only the box systems' heavy profiles take extra bumpers.
                (PartRole::DrawerFront, Opening::ServoDrive, Some(d)) => {
                    servo_fronts.push(d.front_height);
                    servo_heavy |= d.heavy_runner
                        && matches!(d.system, "legrabox" | "merivobox" | "tandembox");
                    Vec::new()
                }
                (PartRole::DrawerFront, _, Some(d)) => match self.drawer_tip_on(d, lw) {
                    Ok(items) => items,
                    Err(e) => {
                        self.list.warnings.push(format!("{}: {e}", f.name));
                        Vec::new()
                    }
                },
                (PartRole::DrawerFront, _, None) => Vec::new(),
                _ => {
                    let n = if f.bounds.size.z > TALL_DOOR { 2 } else { 1 };
                    match self.tip_on_door(f) {
                        Ok(items) => items
                            .into_iter()
                            .map(|i| (i.id.clone(), format!("{} {}", i.family, i.variant), n))
                            .collect(),
                        Err(e) => {
                            self.list.warnings.push(format!("{}: {e}", f.name));
                            Vec::new()
                        }
                    }
                }
            };
            for (item, description, quantity) in rows {
                self.list.hardware.push(HardwarePlacement {
                    item,
                    description,
                    at: f.bounds.center(),
                    part: f.name.clone(),
                    quantity,
                });
            }
        }
        if opening == Opening::ServoDrive && fronts.iter().any(|f| f.role == PartRole::Door) {
            self.list
                .warnings
                .push("SERVO-DRIVE does not open hinged doors: they get TIP-ON".into());
        }
        if !servo_fronts.is_empty() {
            let bottom = self.name(PartRole::Bottom, None);
            let at = Vec3::new(self.cab.width / 2.0, self.interior_back, self.carcass_z0);
            let inside = self.opening_z.1 - self.opening_z.0;
            match drawers::servo_drive(inside, &servo_fronts, servo_heavy) {
                Ok(items) => {
                    for (i, quantity) in items {
                        self.list.hardware.push(HardwarePlacement {
                            item: i.id.clone(),
                            description: format!("{} {}", i.family, i.variant),
                            at,
                            part: bottom.clone(),
                            quantity,
                        });
                    }
                }
                Err(e) => self.list.warnings.push(e),
            }
        }
    }

    /// TIP-ON for a drawer: a TIP-ON BLUMOTION set and its synchronisation on the box systems
    /// and MOVENTO; TANDEM has TIP-ON in its runners and takes a synchronisation linkage.
    fn drawer_tip_on(
        &self,
        d: &DrawerFit,
        lw: f64,
    ) -> std::result::Result<Vec<(String, String, u32)>, String> {
        let row = |i: &cabinet_catalog::HardwareItem, n: u32| {
            (i.id.clone(), format!("{} {}", i.family, i.variant), n)
        };
        if d.system == "tandem" {
            let sync = crate::hardware::item("blum.tandem.tip-on-sync")?;
            return Ok(if lw <= sync.param("lw_max")? {
                vec![row(sync, 1)]
            } else {
                Vec::new()
            });
        }
        Ok(
            drawers::tip_on_blumotion(
                d.system,
                d.nominal_length,
                d.heavy_runner,
                d.heavy_load,
                lw,
            )?
            .into_iter()
            .map(|(i, n)| row(i, n))
            .collect(),
        )
    }

    /// A door's TIP-ON unit and its adapter plate: Blum's short version for overlay fronts up to
    /// its height limit, the long one for taller and inset fronts (p.192).
    fn tip_on_door(
        &self,
        door: &Part,
    ) -> std::result::Result<[&'static cabinet_catalog::HardwareItem; 2], String> {
        let short = crate::hardware::item("blum.tip-on.door.short")?;
        let long = self.cab.standard.overlay == DoorOverlay::Inset
            || door.bounds.size.z > short.param("front_height_max")?;
        let v = if long { "long" } else { "short" };
        Ok([
            crate::hardware::item(&format!("blum.tip-on.door.{v}"))?,
            crate::hardware::item(&format!("blum.tip-on.adapter.{v}"))?,
        ])
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
            FrontLayout::BiFoldLift { handle_weight } => {
                self.bi_fold_lift(aabb(x0, y0, z0, x1, y1, z1), *handle_weight);
            }
            FrontLayout::Lift {
                kind,
                handle_weight,
            } => {
                self.single_lift(*kind, aabb(x0, y0, z0, x1, y1, z1), *handle_weight);
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
                let mut fronts = Vec::new();
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
                    fronts.push(b);
                    top -= hgt + g;
                }
                // Each drawer's runners stand where its front's bottom edge meets the carcass
                // the way the lowest one meets the bottom panel; a drawer has up to the next
                // drawer's datum (or the underside of the top) for its box.
                let lift = self.opening_z.0 - z0;
                let datums: Vec<f64> = fronts.iter().map(|b| b.min.z + lift).collect();
                for (i, b) in fronts.iter().enumerate() {
                    let ceiling = if i == 0 {
                        self.opening_z.1
                    } else {
                        datums[i - 1]
                    };
                    self.drawer_box(i + 1, *b, datums[i], ceiling - datums[i]);
                }
            }
        }
    }

    /// The drawer system's box behind drawer front `n`: base and back parts, front fixing bores
    /// and the system's hardware. `datum` is the height its runners stand on.
    fn drawer_box(&mut self, n: usize, front: Aabb, datum: f64, space: f64) {
        let runner = match self.cab.standard.drawer_system {
            DrawerSystem::Legrabox => {
                return self.steel_box(BoxSystem::Legrabox, n, front, datum, space);
            }
            DrawerSystem::Merivobox => {
                return self.steel_box(BoxSystem::Merivobox, n, front, datum, space);
            }
            DrawerSystem::Tandembox => {
                return self.steel_box(BoxSystem::Tandembox, n, front, datum, space);
            }
            DrawerSystem::Movento => WoodRunner::Movento,
            DrawerSystem::Tandem => WoodRunner::Tandem,
        };
        self.wood_box(runner, n, front, datum, space);
    }

    /// A wooden box on MOVENTO or TANDEM runners: two sides, a base between them, and a front and
    /// back standing on the base.
    fn wood_box(&mut self, runner: WoodRunner, n: usize, front: Aabb, datum: f64, space: f64) {
        let cab = self.cab;
        let s = &cab.standard;
        let t = s.carcass_thickness;
        let front_name = self.name(PartRole::DrawerFront, Some(n));
        let fy = front.max().y;
        let b = match drawers::wood(
            runner,
            cab.width - 2.0 * t,
            self.interior_back - fy,
            space,
            s.drawer_board_thickness,
            s.opening == Opening::TipOn,
        ) {
            Ok(b) => b,
            Err(e) => {
                self.list
                    .warnings
                    .push(format!("{front_name}: {e}; no drawer box"));
                return;
            }
        };
        self.drawer_fits.push(DrawerFit {
            front: front_name.clone(),
            system: match runner {
                WoodRunner::Movento => "movento",
                WoodRunner::Tandem => "tandem",
            },
            nominal_length: b.nominal_length,
            heavy_runner: b.runner.id.contains(".766h."),
            heavy_load: b.side_height >= DEEP_WOOD_DRAWER,
            front_height: front.size.z,
        });
        let (st, mid) = (b.board, cab.width / 2.0);
        let (x0, x1) = (mid - b.inside_width / 2.0, mid + b.inside_width / 2.0);
        let (y0, y1) = (fy, fy + b.length);
        let z0 = datum + b.side_lift;
        let base_z = z0 + b.base_recess;
        let top = z0 + b.side_height;
        self.add_drawer_part(
            PartRole::DrawerLeftSide,
            n,
            aabb(x0 - st, y0, z0, x0, y1, top),
            Axis::X,
        );
        self.add_drawer_part(
            PartRole::DrawerRightSide,
            n,
            aabb(x1, y0, z0, x1 + st, y1, top),
            Axis::X,
        );
        self.add_drawer_part(
            PartRole::DrawerBottom,
            n,
            aabb(x0, y0, base_z, x1, y1, base_z + st),
            Axis::Z,
        );
        self.add_drawer_part(
            PartRole::DrawerInnerFront,
            n,
            aabb(x0, y0, base_z + st, x1, y0 + st, top),
            Axis::Y,
        );
        self.add_drawer_part(
            PartRole::DrawerBack,
            n,
            aabb(x0, y1 - st, base_z + st, x1, y1, top),
            Axis::Y,
        );
        let at = front.center();
        for (item, quantity) in [(b.runner, 1), (b.locking_device, 2)] {
            self.list.hardware.push(HardwarePlacement {
                item: item.id.clone(),
                description: format!("{} {}", item.family, item.variant),
                at,
                part: front_name.clone(),
                quantity,
            });
        }
    }

    /// A LEGRABOX, MERIVOBOX or TANDEMBOX: Blum's steel sides, with the base and back cut to
    /// Blum's sizes.
    fn steel_box(&mut self, kind: BoxSystem, n: usize, front: Aabb, datum: f64, space: f64) {
        let cab = self.cab;
        let t = cab.standard.carcass_thickness;
        let front_name = self.name(PartRole::DrawerFront, Some(n));
        let inner_width = cab.width - 2.0 * t;
        let fy = front.max().y;
        let lb = match drawers::steel_box(kind, inner_width, self.interior_back - fy, space) {
            Ok(lb) => lb,
            Err(e) => {
                self.list
                    .warnings
                    .push(format!("{front_name}: {e}; no drawer box"));
                return;
            }
        };
        let numbers = (|| {
            let p = |k: &str| lb.system.param(k);
            Ok::<_, String>((p("base_above_datum")?, p("front_hole_inset")?))
        })();
        let Ok((base_lift, hole_inset)) = numbers else {
            self.list
                .warnings
                .push(format!("{front_name}: {}", numbers.unwrap_err()));
            return;
        };
        self.drawer_fits.push(DrawerFit {
            front: front_name.clone(),
            system: kind.key(),
            nominal_length: lb.nominal_length,
            heavy_runner: lb.heavy_profile,
            heavy_load: lb.height == "K",
            front_height: front.size.z,
        });
        let bt = lb.board_thickness;
        // Placed for the 3D view and the cut list; the sizes are Blum's.
        let (bw, bl) = lb.base;
        let (kw, kh) = lb.back;
        let mid = cab.width / 2.0;
        let z = datum + base_lift;
        self.add_drawer_part(
            PartRole::DrawerBottom,
            n,
            aabb(mid - bw / 2.0, fy, z, mid + bw / 2.0, fy + bl, z + bt),
            Axis::Z,
        );
        self.add_drawer_part(
            PartRole::DrawerBack,
            n,
            aabb(
                mid - kw / 2.0,
                fy + bl - bt,
                z + bt,
                mid + kw / 2.0,
                fy + bl,
                z + bt + kh,
            ),
            Axis::Y,
        );
        // Front fixing brackets: their bores in the back of the front, in from each side.
        for x in [t + hole_inset, cab.width - t - hole_inset] {
            for hole in &lb.system.drilling {
                self.list.operations.push(Operation::Bore {
                    part: front_name.clone(),
                    at: Vec3::new(x, fy, datum + hole.y),
                    direction: Dir::new(Axis::Y, false),
                    diameter: hole.diameter,
                    depth: hole.depth.min(front.size.y - 3.0),
                    purpose: BorePurpose::FrontFixing,
                });
            }
        }
        let at = front.center();
        for (item, quantity) in [
            (lb.profile, 1),
            (lb.sides, 1),
            (lb.back_fixing, 1),
            (lb.front_fixing, 2),
        ] {
            self.list.hardware.push(HardwarePlacement {
                item: item.id.clone(),
                description: format!("{} {}", item.family, item.variant),
                at,
                part: front_name.clone(),
                quantity,
            });
        }
    }

    /// A drawer box part in the carcass board when it is the system's thickness, else generic.
    fn add_drawer_part(&mut self, role: PartRole, n: usize, b: Aabb, thickness_axis: Axis) {
        let s = &self.cab.standard;
        let board = if (b.size.get(thickness_axis) - s.carcass_thickness).abs() < 1e-9 {
            s.carcass_board.clone()
        } else {
            String::new()
        };
        self.list.parts.push(Part {
            name: self.name(role, Some(n)),
            role,
            bounds: b,
            thickness_axis,
            grain_axis: Some(Axis::X),
            board,
            // Blum's sizes are cut sizes; tape would take them off.
            edges: EdgeBanding::default(),
        });
    }

    /// One front on AVENTOS HK, HS or HL top: no hinges, the lift holds it.
    fn single_lift(&mut self, kind: LiftKind, front: Aabb, handle_weight: f64) {
        let cab = self.cab;
        let s = &cab.standard;
        self.add(
            PartRole::LiftFront,
            Some(1),
            front,
            Axis::Y,
            Axis::X,
            EdgeBanding::all(),
        );
        let name = self.name(PartRole::LiftFront, Some(1));
        let chosen = lift::reference_density(s.front_core == FrontCore::Mdf).and_then(|density| {
            lift::single_front(
                kind,
                &lift::LiftNeed {
                    cabinet_height: cab.height,
                    cabinet_width: cab.width,
                    inside_height: self.opening_z.1 - self.opening_z.0,
                    inside_depth: self.interior_back - front.max().y.max(0.0),
                    front_weight: lift::front_weight(
                        front.size.z,
                        front.size.x,
                        front.size.y,
                        density,
                    ),
                    handle_weight,
                },
            )
        });
        let lift = match chosen {
            Ok(l) => l,
            Err(e) => {
                self.list
                    .warnings
                    .push(format!("{name}: {e}; no lift fitted"));
                return;
            }
        };
        for w in &lift.warnings {
            self.list.warnings.push(format!("{name}: {w}"));
        }
        if s.opening == Opening::Handles && handle_weight == 0.0 {
            self.list.warnings.push(format!(
                "{name}: the lift is chosen without a handle weight"
            ));
        }
        for item in lift.parts {
            self.list.hardware.push(HardwarePlacement {
                item: item.id.clone(),
                description: format!("{} {}", item.family, item.variant),
                at: front.center(),
                part: name.clone(),
                quantity: 1,
            });
        }
    }

    /// Two equal fronts on AVENTOS HF top: the upper hinged to the cabinet top with 120° hinges,
    /// the lower to the upper with centre hinges, lifted by a mechanism on each side.
    fn bi_fold_lift(&mut self, region: Aabb, handle_weight: f64) {
        let cab = self.cab;
        let s = &cab.standard;
        let centre = crate::hardware::item("blum.aventos.hf-top.centre-hinge");
        // Blum's centre hinge works with a 3–6 mm gap between the fronts.
        let fm = match centre.and_then(|c| Ok((c.param("gap_min")?, c.param("gap_max")?))) {
            Ok((lo, hi)) => s.front_gap.clamp(lo, hi),
            Err(_) => s.front_gap,
        };
        let fh = (region.size.z - fm) / 2.0;
        let (x0, y0, x1, y1) = (region.min.x, region.min.y, region.max().x, region.max().y);
        let top = region.max().z;
        let upper = aabb(x0, y0, top - fh, x1, y1, top);
        let lower = aabb(x0, y0, region.min.z, x1, y1, region.min.z + fh);
        for (n, b) in [(1, upper), (2, lower)] {
            self.add(
                PartRole::LiftFront,
                Some(n),
                b,
                Axis::Y,
                Axis::X,
                EdgeBanding::all(),
            );
        }
        let upper_name = self.name(PartRole::LiftFront, Some(1));
        let lower_name = self.name(PartRole::LiftFront, Some(2));
        let chosen = (|| {
            let density = lift::reference_density(s.front_core == FrontCore::Mdf)?;
            let fg =
                2.0 * lift::front_weight(fh, region.size.x, region.size.y, density) + handle_weight;
            let hf = lift::hf_top(&lift::BiFoldNeed {
                cabinet_height: cab.height,
                cabinet_width: cab.width,
                inside_height: self.opening_z.1 - self.opening_z.0,
                inside_depth: self.interior_back - y1.max(0.0),
                front_weight: fg,
            })?;
            let cup = |h: &cabinet_catalog::HardwareItem| {
                Ok::<_, String>((h.param("cup_diameter")?, h.param("cup_depth")?))
            };
            let (hinge_cup, centre_cup) = (cup(hf.hinge)?, cup(hf.centre_hinge)?);
            // 120° hinge: TB = upper front overlay − boss overlay (plates at 0 mm).
            let fao = top - self.opening_z.1;
            let tb = fao - hf.hinge.param("boss_overlay")?;
            let tb_range =
                hf.hinge.param("boring_distance_min")?..=hf.hinge.param("boring_distance_max")?;
            let tb_centre = hf.centre_hinge.param("boring_distance_plus_gap")? - fm;
            Ok::<_, String>((hf, tb, tb_range, tb_centre, hinge_cup, centre_cup))
        })();
        let (hf, tb, tb_range, tb_centre, hinge_cup, centre_cup) = match chosen {
            Ok(c) => c,
            Err(e) => {
                self.list
                    .warnings
                    .push(format!("{upper_name}: {e}; no lift fitted"));
                return;
            }
        };
        for w in &hf.warnings {
            self.list.warnings.push(format!("{upper_name}: {w}"));
        }
        if !tb_range.contains(&tb) {
            self.list.warnings.push(format!(
                "{upper_name}: the 120° hinges would need a boring distance of {tb:.1} mm \
                 (Blum: {}–{})",
                tb_range.start(),
                tb_range.end()
            ));
        }
        if s.opening == Opening::Handles && handle_weight == 0.0 {
            self.list.warnings.push(format!(
                "{upper_name}: the lift is chosen without a handle weight"
            ));
        }
        // Hinges across the width, as doors space them up the height.
        let n = hf.hinges as usize;
        let span = region.size.x - 2.0 * HINGE_END_OFFSET;
        let xs: Vec<f64> = (0..n)
            .map(|i| x0 + HINGE_END_OFFSET + span * i as f64 / (n - 1) as f64)
            .collect();
        let back = y1;
        let mut cups = Vec::new();
        for &x in &xs {
            let r = hinge_cup.0 / 2.0;
            cups.push((upper_name.clone(), top - tb - r, hinge_cup));
            let rc = centre_cup.0 / 2.0;
            cups.push((upper_name.clone(), upper.min.z + tb_centre + rc, centre_cup));
            cups.push((
                lower_name.clone(),
                lower.max().z - tb_centre - rc,
                centre_cup,
            ));
            for (part, z, (diameter, depth)) in cups.drain(..) {
                self.list.operations.push(Operation::Bore {
                    part,
                    at: Vec3::new(x, back, z),
                    direction: Dir::new(Axis::Y, false),
                    diameter,
                    depth: depth.min(region.size.y - 3.0),
                    purpose: BorePurpose::HingeCup,
                });
            }
        }
        let at = upper.center();
        let hinges = hf.hinges;
        for (item, quantity) in [
            (hf.mechanism, hf.mechanisms),
            (hf.lever_arm, 1),
            (hf.cover_caps, 1),
            (hf.hinge, hinges),
            (hf.centre_hinge, hinges),
            // One per 120° hinge and one per lever arm.
            (hf.plate, hinges + 2),
        ] {
            self.list.hardware.push(HardwarePlacement {
                item: item.id.clone(),
                description: format!("{} {}", item.family, item.variant),
                at,
                part: upper_name.clone(),
                quantity,
            });
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
        self.add(
            PartRole::Door,
            Some(n),
            b,
            Axis::Y,
            Axis::Z,
            EdgeBanding::all(),
        );
        let door = self.name(PartRole::Door, Some(n));
        self.hang_door(&door, b, hinge, None);
    }

    /// Hinges `door` (in the plane of the front, bounds `b`) to the side on `hinge`: the
    /// standard 110° hinge for the overlay, or hinge `hinge_id`. Returns the hinges' heights.
    fn hang_door(
        &mut self,
        door: &str,
        b: Aabb,
        hinge: HingeSide,
        hinge_id: Option<&str>,
    ) -> Vec<f64> {
        let cab = self.cab;
        let s = &cab.standard;
        let door = door.to_string();
        let side_role = match hinge {
            HingeSide::Left => PartRole::LeftSide,
            HingeSide::Right => PartRole::RightSide,
        };
        let side = self.name(side_role, None);
        let height = b.size.z;
        let weight = lift::reference_density(s.front_core == FrontCore::Mdf)
            .map(|density| lift::front_weight(height, b.size.x, b.size.y, density));
        let count = match weight.and_then(|w| hinges::hinge_count(height, w)) {
            Ok((n, note)) => {
                if let Some(note) = note {
                    self.list.warnings.push(format!("{door}: {note}"));
                }
                n as usize
            }
            Err(e) => {
                self.list
                    .warnings
                    .push(format!("{door}: {e}; no hinges placed"));
                return Vec::new();
            }
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
        let chosen = match hinge_id {
            Some(id) => hinges::select_hinge(id, &need),
            None => hinges::select(&need),
        };
        let choice = match chosen {
            Ok(c) => c,
            Err(e) => {
                self.list
                    .warnings
                    .push(format!("{door}: {e}; no hinges placed"));
                return Vec::new();
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
            return Vec::new();
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
        let zs: Vec<f64> = (0..count)
            .map(|i| b.min.z + HINGE_END_OFFSET + span * i as f64 / (count - 1) as f64)
            .collect();
        for &z in &zs {
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
        zs
    }

    /// An L-corner's bi-fold pair: door 1 across the main leg, hung on its end side with 155°
    /// hinges; door 2 across the second leg, hung on door 1 with Blum's corner bi-fold hinges
    /// (bosses in door 1, plates on door 2). Full overlay.
    fn corner_doors(&mut self) {
        let cab = self.cab;
        let s = &cab.standard;
        let (w, d, w2) = (cab.width, cab.depth, cab.corner.width);
        let (g, ft) = (s.front_gap, s.front_thickness);
        if s.overlay != DoorOverlay::Full {
            self.list
                .warnings
                .push("an L-corner's bi-fold doors are always full overlay".into());
        }
        let (z0, z1) = (self.carcass_z0, cab.height - g);
        // Door 2 covers the second leg's front up to the corner; door 1 starts a gap past it.
        let one = aabb(d + ft + g, -ft, z0, w - g / 2.0, 0.0, z1);
        let two = aabb(d, -(w2 - d) + g / 2.0, z0, d + ft, 0.0, z1);
        self.add(
            PartRole::Door,
            Some(1),
            one,
            Axis::Y,
            Axis::Z,
            EdgeBanding::all(),
        );
        self.add(
            PartRole::Door,
            Some(2),
            two,
            Axis::X,
            Axis::Z,
            EdgeBanding::all(),
        );
        let (n1, n2) = (
            self.name(PartRole::Door, Some(1)),
            self.name(PartRole::Door, Some(2)),
        );
        let zs = self.hang_door(
            &n1,
            one,
            HingeSide::Right,
            Some("blum.clip-top-blumotion.155.overlay"),
        );
        let fold = (|| {
            let bi = crate::hardware::item("blum.corner-bi-fold-hinge")?;
            let plate =
                crate::hardware::item("blum.mounting-plate.cruciform-37-32.chipboard-screw.0")?;
            let p = |k: &str| bi.param(k);
            Ok::<_, String>((
                bi,
                plate,
                p("boring_distance")? + p("cup_diameter")? / 2.0,
                p("cup_diameter")?,
                p("cup_depth")?,
                p("front_thickness_min")?..=p("front_thickness_max")?,
            ))
        })();
        let (bi, plate, inset, diameter, depth, thick) = match fold {
            Ok(f) => f,
            Err(e) => {
                self.list.warnings.push(format!("{n2}: {e}; not hung"));
                return;
            }
        };
        if !thick.contains(&ft) {
            self.list.warnings.push(format!(
                "{n2}: Blum's corner bi-fold hinge takes fronts {}–{} mm, not {ft}",
                thick.start(),
                thick.end()
            ));
        }
        for z in zs {
            let at = Vec3::new(one.min.x + inset, 0.0, z);
            self.list.operations.push(Operation::Bore {
                part: n1.clone(),
                at,
                direction: Dir::new(Axis::Y, false),
                diameter,
                depth: depth.min(ft - 3.0),
                purpose: BorePurpose::HingeCup,
            });
            for (item, part, at) in [(bi, &n1, at), (plate, &n2, Vec3::new(d, -inset, z))] {
                self.list.hardware.push(HardwarePlacement {
                    item: item.id.clone(),
                    description: format!("{} {}", item.family, item.variant),
                    at,
                    part: part.clone(),
                    quantity: 1,
                });
            }
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
        for (role, x, hand) in [
            (PartRole::LeftSide, t, "left"),
            (PartRole::RightSide, w - t, "right"),
        ] {
            self.list.hardware.push(HardwarePlacement {
                item: format!("blum.wall-hanging-bracket.{hand}"),
                description: format!("Blum wall hanging bracket, screw-on, {hand}"),
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
    use crate::model::{ConstructionStandard, DrawerSystem, FixingRecipe};

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
            Operation::Bore { part, .. }
            | Operation::Groove { part, .. }
            | Operation::Mortise { part, .. } => *part == shelf.name,
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
            .filter(|h| h.item.starts_with("blum.wall-hanging-bracket."))
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
            assert!(!list.hardware.iter().any(|h| h.item.contains("hang")));
        }
    }

    #[test]
    fn hangers_without_an_inset_back_are_left_out_with_a_warning() {
        let mut cab = Cabinet::new("W01", CabinetKind::Wall);
        cab.standard.back = BackStyle::Applied;
        let list = build_cabinet(&cab).unwrap();
        assert!(!list.hardware.iter().any(|h| h.item.contains("hang")));
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
            .filter(|h| h.item.starts_with("blum.tip-on.door."))
            .map(|h| h.quantity)
            .sum();
        assert_eq!(units, 2, "one tall door, two units");
        // Over Blum's 1300 mm the long version, on its own adapter plate.
        let long = |id: &str| list.hardware.iter().any(|h| h.item == id);
        assert!(long("blum.tip-on.door.long") && long("blum.tip-on.adapter.long"));
        let mut bank = Cabinet::new("D01", CabinetKind::DrawerBank);
        bank.standard.opening = Opening::TipOn;
        let list = build_cabinet(&bank).unwrap();
        // K boxes on NL 550: the 15–40 kg unit, synchronised with the round linkage.
        let count = |id: &str| {
            list.hardware
                .iter()
                .filter(|h| h.item == id)
                .map(|h| h.quantity)
                .sum::<u32>()
        };
        assert_eq!(count("blum.tip-on-blumotion.l3"), 3);
        assert_eq!(count("blum.tip-on-blumotion.sync-round"), 3);
        assert_eq!(count("blum.tip-on-blumotion.sync-adapter"), 6);
        // TANDEM drawers take TIP-ON runners and a synchronisation linkage.
        bank.standard.drawer_system = DrawerSystem::Tandem;
        let list = build_cabinet(&bank).unwrap();
        assert_eq!(
            list.hardware
                .iter()
                .filter(|h| h.item == "blum.tandem.560h-tip-on.550")
                .count(),
            3
        );
        assert!(
            list.hardware
                .iter()
                .any(|h| h.item == "blum.tandem.tip-on-sync")
        );
    }

    #[test]
    fn tip_on_doors_hang_on_unsprung_hinges_and_inset_doors_take_the_long_unit() {
        let mut base = Cabinet::new("B01", CabinetKind::Base);
        base.standard.opening = Opening::TipOn;
        let list = build_cabinet(&base).unwrap();
        let has = |l: &PartList, id: &str| l.hardware.iter().any(|h| h.item == id);
        assert!(has(&list, "blum.clip-top.110.overlay.unsprung"));
        assert!(has(&list, "blum.tip-on.door.short"));
        base.standard.overlay = DoorOverlay::Inset;
        let list = build_cabinet(&base).unwrap();
        assert!(has(&list, "blum.tip-on.door.long"));
    }

    #[test]
    fn servo_drive_drives_drawers_and_leaves_doors_to_tip_on() {
        let mut bank = Cabinet::new("D01", CabinetKind::DrawerBank);
        bank.standard.opening = Opening::ServoDrive;
        let list = build_cabinet(&bank).unwrap();
        let count = |id: &str| {
            list.hardware
                .iter()
                .filter(|h| h.item == id)
                .map(|h| h.quantity)
                .sum::<u32>()
        };
        assert_eq!(count("blum.servo-drive.drive-unit"), 3);
        assert_eq!(count("blum.servo-drive.power-supply.au"), 1);
        assert_eq!(count("blum.servo-drive.bracket-attachment"), 2);
        // Wooden drawers on MOVENTO take the same SERVO-DRIVE parts.
        bank.standard.drawer_system = DrawerSystem::Movento;
        let wood = build_cabinet(&bank).unwrap();
        assert_eq!(
            wood.hardware
                .iter()
                .filter(|h| h.item == "blum.servo-drive.drive-unit")
                .map(|h| h.quantity)
                .sum::<u32>(),
            3
        );
        assert!(
            !wood.warnings.iter().any(|w| w.contains("SERVO")),
            "{:?}",
            wood.warnings
        );
        let mut base = Cabinet::new("B01", CabinetKind::Base);
        base.standard.opening = Opening::ServoDrive;
        let list = build_cabinet(&base).unwrap();
        assert!(
            list.hardware
                .iter()
                .any(|h| h.item == "blum.tip-on.door.short")
        );
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

    /// BLUM-4: a LEGRABOX drawer bank puts each drawer's base and back in the cut list at
    /// Blum's sizes, and drills the front fixing brackets.
    #[test]
    fn a_legrabox_drawer_bank_cuts_a_base_and_back_per_drawer() {
        let cab = Cabinet::new("D01", CabinetKind::DrawerBank);
        let list = build_cabinet(&cab).unwrap();
        let cut = crate::cutlist::cut_list(std::slice::from_ref(&list), "white", 1.0);
        let rd = |v: f64| (v * 1e6).round() / 1e6;
        let sizes = |role: &str| -> Vec<(f64, f64, f64)> {
            cut.rows
                .iter()
                .filter(|r| r.part.contains(role))
                .flat_map(|r| {
                    std::iter::repeat_n(
                        (rd(r.cut_length), rd(r.cut_width), r.thickness),
                        r.quantity as usize,
                    )
                })
                .collect()
        };
        let lw = cab.width - 2.0 * cab.standard.carcass_thickness;
        assert_eq!(sizes("DrawerBottom"), vec![(lw - 35.0, 540.0, 16.0); 3]);
        assert_eq!(sizes("DrawerBack"), vec![(lw - 38.0, 101.0, 16.0); 3]);
        let fixings = list
            .operations
            .iter()
            .filter(|o| matches!(o, Operation::Bore { purpose: BorePurpose::FrontFixing, diameter, .. } if *diameter == 10.0))
            .count();
        assert_eq!(fixings, 3 * 4);
        assert!(list.warnings.is_empty(), "{:?}", list.warnings);
    }

    /// CORE-8: a three-drawer bank of wooden boxes on TANDEM runners cuts two sides, a base, a
    /// box front and a back per drawer, sized by Blum's runner rules.
    #[test]
    fn a_wooden_drawer_bank_cuts_sides_base_front_and_back_per_drawer() {
        let mut cab = Cabinet::new("D01", CabinetKind::DrawerBank);
        cab.standard.drawer_system = DrawerSystem::Tandem;
        let list = build_cabinet(&cab).unwrap();
        assert!(list.warnings.is_empty(), "{:?}", list.warnings);
        let lw = cab.width - 2.0 * cab.standard.carcass_thickness;
        let skw = lw - 42.0;
        for n in 1..=3 {
            let part = |role: &str| list.part(&format!("D01.{role}{n}")).unwrap();
            let (l, r) = (part("DrawerLeftSide"), part("DrawerRightSide"));
            // Inside width between the sides, sides as long as NL − 10.
            assert!((r.bounds.min.x - l.bounds.max().x - skw).abs() < 1e-9);
            assert_eq!(l.bounds.size.y, 540.0);
            assert_eq!(l.thickness(), 16.0);
            let base = part("DrawerBottom");
            assert!((base.bounds.size.x - skw).abs() < 1e-9);
            // The base clears the runners: 27.5 above the runners' datum.
            let datum = l.bounds.min.z - 15.5;
            assert!((base.bounds.min.z - datum - 27.5).abs() < 1e-9);
            for role in ["DrawerInnerFront", "DrawerBack"] {
                let p = part(role);
                assert!((p.bounds.min.z - base.bounds.max().z).abs() < 1e-9);
                assert!((p.bounds.max().z - l.bounds.max().z).abs() < 1e-9);
            }
        }
        let cut = crate::cutlist::cut_list(std::slice::from_ref(&list), "white", 1.0);
        let drawer_parts: u32 = cut
            .rows
            .iter()
            .filter(|r| r.part.contains(".Drawer") && !r.part.contains("DrawerFront"))
            .map(|r| r.quantity)
            .sum();
        assert_eq!(drawer_parts, 3 * 5);
        assert!(
            list.hardware
                .iter()
                .any(|h| h.item == "blum.tandem.560h.550")
        );
    }

    #[test]
    fn eighteen_mm_drawer_sides_are_too_thick_for_tandem() {
        let mut cab = Cabinet::new("D01", CabinetKind::DrawerBank);
        cab.standard.drawer_system = DrawerSystem::Tandem;
        cab.standard.drawer_board_thickness = 18.0;
        let list = build_cabinet(&cab).unwrap();
        assert!(
            list.warnings.iter().any(|w| w.contains("11–16 mm")),
            "{:?}",
            list.warnings
        );
        assert!(list.by_role(PartRole::DrawerLeftSide).next().is_none());
    }

    #[test]
    fn a_drawer_too_low_for_any_legrabox_warns_and_gets_no_box() {
        let mut cab = Cabinet::new("D01", CabinetKind::DrawerBank);
        cab.fronts = FrontLayout::Drawers {
            ratios: vec![1.0; 12],
        };
        let list = build_cabinet(&cab).unwrap();
        assert!(list.by_role(PartRole::DrawerBottom).next().is_none());
        assert!(list.warnings.iter().any(|w| w.contains("no drawer box")));
    }

    /// BLUM-7: a wall cabinet with a bi-fold lift gets two equal fronts, Blum's mechanism for
    /// its power factor, the lever arm for its height, and three cups per hinge position.
    #[test]
    fn a_bi_fold_lift_wall_cabinet_gets_two_fronts_and_its_aventos_parts() {
        let mut cab = Cabinet::new("W01", CabinetKind::Wall);
        cab.fronts = FrontLayout::BiFoldLift { handle_weight: 0.4 };
        let list = build_cabinet(&cab).unwrap();
        assert!(list.warnings.is_empty(), "{:?}", list.warnings);
        let (up, low) = (
            list.part("W01.LiftFront1").unwrap(),
            list.part("W01.LiftFront2").unwrap(),
        );
        assert!((up.bounds.size.z - low.bounds.size.z).abs() < 1e-9);
        // The centre hinge's 3 mm minimum gap between the fronts.
        assert!((up.bounds.min.z - low.bounds.max().z - 3.0).abs() < 1e-9);
        let items: Vec<(String, u32)> = list
            .hardware
            .iter()
            .filter_map(|h| {
                Some((
                    crate::hardware::item(&h.item).ok()?.article.clone(),
                    h.quantity,
                ))
            })
            .collect();
        for expected in [
            ("22F2510", 2),
            ("22F3500", 1),
            ("70T5550.TL", 2),
            ("78Z5500T", 2),
        ] {
            assert!(
                items.contains(&(expected.0.to_string(), expected.1)),
                "{expected:?} in {items:?}"
            );
        }
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
        assert_eq!(cups, 2 * 3);
    }

    #[test]
    fn a_bi_fold_lift_on_a_cabinet_too_low_for_hf_top_warns() {
        let mut cab = Cabinet::new("W01", CabinetKind::Wall);
        cab.height = 400.0;
        cab.fronts = FrontLayout::BiFoldLift { handle_weight: 0.4 };
        let list = build_cabinet(&cab).unwrap();
        assert!(
            list.warnings.iter().any(|w| w.contains("480–1200")),
            "{:?}",
            list.warnings
        );
        assert_eq!(list.by_role(PartRole::LiftFront).count(), 2);
    }

    /// CORE-4b: a 900 × 900 L-corner: two legs sharing one interior, each panel once, nothing
    /// overlapping, bi-fold doors on Blum's 155° and corner bi-fold hinges.
    #[test]
    fn an_l_corner_900_by_900_builds_with_every_panel_once() {
        let cab = Cabinet::new("C01", CabinetKind::CornerBase);
        let list = build_cabinet(&cab).unwrap();
        assert!(
            list.warnings
                .iter()
                .all(|w| w.contains("shelf-pin holes left out")),
            "{:?}",
            list.warnings
        );
        // Inset backs sit in grooves in the panels around them.
        let in_groove = |n: &str| n.ends_with(".Back") || n.ends_with(".Back2");
        let bad: Vec<_> = overlapping(&list)
            .into_iter()
            .filter(|(a, b)| !in_groove(a) && !in_groove(b))
            .collect();
        assert_eq!(bad, []);
        let mut names: Vec<&str> = list.parts.iter().map(|p| p.name.as_str()).collect();
        names.sort_unstable();
        let unique = names.len();
        names.dedup();
        assert_eq!(names.len(), unique);
        for name in [
            "C01.LeftSide",
            "C01.RightSide",
            "C01.Bottom",
            "C01.Back",
            "C01.Plinth",
            "C01.LeftSide2",
            "C01.Bottom2",
            "C01.Back2",
            "C01.Plinth2",
            "C01.Door1",
            "C01.Door2",
        ] {
            assert!(list.part(name).is_some(), "{name} in {names:?}");
        }
        // The second leg is open where it meets the main leg.
        assert!(list.part("C01.RightSide2").is_none());
        let cut = crate::cutlist::cut_list(std::slice::from_ref(&list), "white", 1.0);
        assert_eq!(
            cut.rows.iter().map(|r| r.quantity).sum::<u32>() as usize,
            list.parts.len()
        );
        // The second leg comes forward 900 − 580 from the main leg's front, facing +X.
        let end = list.part("C01.LeftSide2").unwrap();
        assert!((end.bounds.min.y + 320.0).abs() < 1e-9);
        let bottom2 = list.part("C01.Bottom2").unwrap();
        assert!((bottom2.bounds.max().y - 0.0).abs() < 1e-9);
        assert!((bottom2.bounds.max().x - 580.0).abs() < 1e-9);
        let door2 = list.part("C01.Door2").unwrap();
        assert_eq!(door2.thickness_axis, Axis::X);
        let count = |id: &str| {
            list.hardware
                .iter()
                .filter(|h| h.item == id)
                .map(|h| h.quantity)
                .sum::<u32>()
        };
        assert_eq!(count("blum.clip-top-blumotion.155.overlay"), 2);
        assert_eq!(count("blum.corner-bi-fold-hinge"), 2);
    }

    #[test]
    fn a_right_hand_l_corner_mirrors_the_left() {
        let mut cab = Cabinet::new("C01", CabinetKind::CornerBase);
        cab.corner.side = HingeSide::Right;
        let list = build_cabinet(&cab).unwrap();
        assert!(
            list.warnings
                .iter()
                .all(|w| w.contains("shelf-pin holes left out")),
            "{:?}",
            list.warnings
        );
        // Inset backs sit in grooves in the panels around them.
        let in_groove = |n: &str| n.ends_with(".Back") || n.ends_with(".Back2");
        let bad: Vec<_> = overlapping(&list)
            .into_iter()
            .filter(|(a, b)| !in_groove(a) && !in_groove(b))
            .collect();
        assert_eq!(bad, []);
        let leg = list.part("C01.Bottom2").unwrap();
        assert!((leg.bounds.min.x - (900.0 - 580.0)).abs() < 1e-9);
        // The second leg's end side is now on its right.
        assert!(list.part("C01.RightSide2").is_some());
    }

    /// CORE-4c: a 600 × 600 corner wall: wall-height legs, bi-fold pair, two hangers per leg with
    /// the corner one shared, and a rail behind each leg.
    #[test]
    fn a_corner_wall_hangs_each_leg_on_two_hangers_sharing_the_corner_one() {
        for side in [HingeSide::Left, HingeSide::Right] {
            let mut cab = Cabinet::new("C01", CabinetKind::CornerWall);
            cab.corner.side = side;
            let list = build_cabinet(&cab).unwrap();
            assert!(
                list.warnings
                    .iter()
                    .all(|w| w.contains("shelf-pin holes left out")),
                "{:?}",
                list.warnings
            );
            assert!(list.by_role(PartRole::Plinth).next().is_none());
            assert!(list.part("C01.Door1").is_some() && list.part("C01.Door2").is_some());
            let tops = list
                .parts
                .iter()
                .filter(|p| p.bounds.max().z > 719.0)
                .count();
            assert!(tops > 0);
            let hangers: Vec<_> = list
                .hardware
                .iter()
                .filter(|h| h.item.starts_with("blum.wall-hanging-bracket."))
                .collect();
            let on = |part: &str| hangers.iter().filter(|h| h.part == part).count();
            // Main leg: the corner side and its outer end. Second leg: its outer end.
            assert_eq!(hangers.len(), 3, "{hangers:?}");
            let (corner, outer, outer2) = match side {
                HingeSide::Left => ("C01.LeftSide", "C01.RightSide", "C01.LeftSide2"),
                HingeSide::Right => ("C01.RightSide", "C01.LeftSide", "C01.RightSide2"),
            };
            assert_eq!((on(corner), on(outer), on(outer2)), (1, 1, 1));
            // Each hanger is handed for the side it is on.
            for h in &hangers {
                let hand = if h.item.ends_with(".left") {
                    "Left"
                } else {
                    "Right"
                };
                assert!(h.part.contains(hand), "{} on {}", h.item, h.part);
            }
            // Each hanger sits at the top, in the void behind its leg's back.
            for h in &hangers {
                assert!((h.at.z - 720.0).abs() < 1e-9);
            }
            let rails: Vec<_> = list
                .hardware
                .iter()
                .filter(|h| h.item == "wall-hanger.rail")
                .map(|h| h.part.as_str())
                .collect();
            assert_eq!(rails, ["C01.Back", "C01.Back2"]);
        }
    }

    #[test]
    fn a_corner_wall_is_square_by_default() {
        let cab = Cabinet::new("C01", CabinetKind::CornerWall);
        assert_eq!(
            (cab.width, cab.corner.width, cab.depth),
            (600.0, 600.0, 330.0)
        );
        assert!(!cab.has_toe_kick());
    }

    #[test]
    fn an_l_corner_leg_too_short_for_a_door_is_refused() {
        let mut cab = Cabinet::new("C01", CabinetKind::CornerBase);
        cab.corner.width = 700.0;
        assert!(matches!(build_cabinet(&cab), Err(Error::TooSmall(_))));
    }

    /// JOIN-8: DOMINO fixings: a mortise in each part per tenon, no screws, nothing clashing.
    #[test]
    fn domino_joints_put_a_mortise_in_each_part_and_no_screws() {
        let mut cab = Cabinet::new("B01", CabinetKind::Base);
        cab.standard.fixings = FixingRecipe::Domino;
        let list = build_cabinet(&cab).unwrap();
        assert!(
            list.warnings
                .iter()
                .all(|w| w.contains("shelf-pin holes left out")),
            "{:?}",
            list.warnings
        );
        let in_part = |name: &str| {
            list.operations
                .iter()
                .filter(|o| matches!(o, Operation::Mortise { part, .. } if part == name))
                .count()
        };
        let (side, bottom) = (in_part("B01.LeftSide"), in_part("B01.Bottom"));
        assert!(bottom >= 4, "{bottom}");
        assert!(side >= bottom / 2);
        assert!(!list.operations.iter().any(|o| matches!(o,
            Operation::Bore { purpose: BorePurpose::Screw | BorePurpose::ScrewPilot, part, .. }
                if !part.ends_with(".Back"))));
        // 5 × 30 in 16 mm board: 12 into the side's face, 20 into the bottom's end.
        let depth = |name: &str, axis: Axis| {
            list.operations.iter().find_map(|o| match o {
                Operation::Mortise { part, bounds, .. } if part == name => {
                    Some(bounds.size.get(axis))
                }
                _ => None,
            })
        };
        assert_eq!(depth("B01.LeftSide", Axis::X), Some(12.0));
        assert_eq!(depth("B01.Bottom", Axis::X), Some(20.0));
        // Tight in the face part, the middle setting (+6) in the end part for assembly play.
        let (tight, loose) = (
            depth("B01.LeftSide", Axis::Y).unwrap(),
            depth("B01.Bottom", Axis::Y).unwrap(),
        );
        assert!(
            (loose - tight - crate::joinery::DOMINO_PLAY).abs() < 1e-9,
            "{tight} vs {loose}"
        );
    }

    #[test]
    fn lift_fronts_without_a_handle_weight_assume_the_owners_400_g_handle() {
        let lift: FrontLayout =
            serde_json::from_str(r#"{"type":"lift","kind":"stay_lift"}"#).unwrap();
        let bi_fold: FrontLayout = serde_json::from_str(r#"{"type":"bi_fold_lift"}"#).unwrap();
        assert!(matches!(lift, FrontLayout::Lift { handle_weight, .. } if handle_weight == 0.4));
        assert!(
            matches!(bi_fold, FrontLayout::BiFoldLift { handle_weight } if handle_weight == 0.4)
        );
    }

    #[test]
    fn a_domino_too_long_for_the_board_warns_and_cuts_nothing() {
        let mut cab = Cabinet::new("B01", CabinetKind::Base);
        cab.standard.fixings = FixingRecipe::Domino;
        cab.standard.domino = crate::model::DominoSize::D10x50;
        let list = build_cabinet(&cab).unwrap();
        assert!(
            list.warnings.iter().any(|w| w.contains("DF 500")),
            "{:?}",
            list.warnings
        );
        assert!(
            !list
                .operations
                .iter()
                .any(|o| matches!(o, Operation::Mortise { .. }))
        );
    }

    /// BLUM-7 (HK top): a 360 mm wall cabinet with a stay lift: one front, no hinges, the
    /// mechanism for its power factor.
    #[test]
    fn a_stay_lift_wall_cabinet_gets_one_front_and_an_hk_top_mechanism() {
        let mut cab = Cabinet::new("W01", CabinetKind::Wall);
        cab.height = 360.0;
        cab.fronts = FrontLayout::Lift {
            kind: LiftKind::StayLift,
            handle_weight: 0.3,
        };
        let list = build_cabinet(&cab).unwrap();
        assert!(list.warnings.is_empty(), "{:?}", list.warnings);
        assert_eq!(list.by_role(PartRole::LiftFront).count(), 1);
        assert!(
            !list
                .hardware
                .iter()
                .any(|h| h.item.starts_with("blum.clip-top"))
        );
        assert!(
            list.hardware
                .iter()
                .any(|h| h.item.starts_with("blum.aventos.hk-top.mechanism."))
        );
    }

    /// CORE-10: each overlay gets the hinge cranked for it, the cup at Blum's boring distance
    /// for that overlay, and inset doors move the plate back with the door.
    #[test]
    fn each_overlay_gets_its_cranked_hinge_boring_distance_and_plate_position() {
        let ft = ConstructionStandard::default().front_thickness;
        // overlay, carcass thickness → hinge article, TB, plate row from the front
        let rows = [
            (DoorOverlay::Full, 16.0, "71B3550", 4.0, 37.0),
            (DoorOverlay::Full, 18.0, "71B3550", 6.0, 37.0),
            (DoorOverlay::Half, 16.0, "71B3650", 6.5, 37.0),
            (DoorOverlay::Inset, 16.0, "71B3750", 5.0, 37.0 + ft + 1.5),
        ];
        for (overlay, t, article, tb, plate_y) in rows {
            let mut cab = Cabinet::new("B01", CabinetKind::Base);
            cab.standard.overlay = overlay;
            cab.standard.carcass_thickness = t;
            cab.fronts = FrontLayout::Door {
                hinge_side: HingeSide::Left,
            };
            let list = build_cabinet(&cab).unwrap();
            let hinge = list
                .hardware
                .iter()
                .find(|h| h.item.starts_with("blum.clip-top"))
                .unwrap();
            let item = crate::hardware::item(&hinge.item).unwrap();
            assert_eq!(item.article, article, "{overlay:?} {t}");
            let door = list.part("B01.Door1").unwrap();
            assert!(
                (hinge.at.x - door.bounds.min.x - (tb + 17.5)).abs() < 1e-9,
                "{overlay:?} {t}: cup at {}",
                hinge.at.x - door.bounds.min.x
            );
            let plate_holes: Vec<f64> = list
                .operations
                .iter()
                .filter_map(|o| match o {
                    Operation::Bore {
                        part,
                        at,
                        purpose: BorePurpose::HingePlate,
                        ..
                    } if part == "B01.LeftSide" => Some(at.y),
                    _ => None,
                })
                .collect();
            assert!(!plate_holes.is_empty());
            assert!(
                plate_holes.iter().all(|y| (y - plate_y).abs() < 1e-9),
                "{overlay:?}"
            );
            assert!(
                !list.warnings.iter().any(|w| w.contains("Door1")),
                "{overlay:?}: {:?}",
                list.warnings
            );
        }
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
