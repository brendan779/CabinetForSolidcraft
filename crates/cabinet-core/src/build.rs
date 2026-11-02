//! Cabinet → parts, machining and hardware.

use cabinet_catalog::HardwareItem;

use crate::corner;
use crate::drawers::{self, BoxRange, BoxSystem, SteelBox, WoodRunner};
use crate::geom::{Aabb, Axis, Vec3};
use crate::hinges::{self, HingeNeed};
use crate::joinery::ButtJoint;
use crate::lift;
use crate::model::{
    BackStyle, BottomJoint, Cabinet, CabinetKind, DoorOverlay, DoorStyle, DrawerFrame, DrawerSides,
    DrawerSystem, Exposed, FrontCore, FrontGrain, FrontLayout, HandlePlace, Hanging, HingeSide,
    InsertDesign, LegFixing, LiftKind, LightZone, Opening, Section, SectionColumn, SectionFront,
    SectionInside, SectionKind, SharknoseEdge, StackSection, ToeKick, TopJoint, WallHanging,
};
use crate::part::{
    BorePurpose, Dir, EdgeBanding, HardwarePlacement, Operation, Part, PartList, PartRole, Tilt,
};
use crate::{Error, Result};

/// `edge_mount` in the catalog: a Rappana's back leg, rather than a Scalo's plate on the edge.
const EDGE_MOUNT_BACK_LEG: f64 = 1.0;
/// The screws in a Rappana's box (ABI's specification).
const EDGE_PULL_SCREW: f64 = 16.0;
/// Our choice: the wood left over a screw's tip, and round a screw in an edge.
const EDGE_PULL_SCREW_COVER: f64 = 1.0;
const EDGE_PULL_WOOD_BEHIND: f64 = 3.0;
/// Our choices for a sloped shoe shelf: clear of the back and the floor, room left for a shoe over
/// it, and its screws.
const SHOE_SHELF_BACK_GAP: f64 = 2.0;
const SHOE_SHELF_FLOOR_GAP: f64 = 2.0;
const MIN_SHOE_ROOM: f64 = 100.0;
const SHOE_SHELF_SCREW_CLEARANCE: f64 = 4.5;
const SHOE_SHELF_PILOT: f64 = 3.0;
const SHOE_SHELF_PILOT_DEPTH: f64 = 25.0;
/// A cruciform plate's system screws, as a door's (Blum's session, check 1).
const PLATE_HOLE_OFFSET: f64 = 16.0;
const PLATE_HOLE_DIAMETER: f64 = 5.0;
const PLATE_HOLE_DEPTH: f64 = 11.5;
/// Our choice: a pilot for the chipboard screws (609.1x00) in a lift bracket.
const LIFT_BRACKET_PILOT: f64 = 2.5;
const LIFT_BRACKET_PILOT_DEPTH: f64 = 10.0;
/// Our choice: a sliding door narrower than this is no use as a door.
const MIN_SLIDING_DOOR: f64 = 200.0;
/// Our choice: the least wood in front of a runner's screw.
const MIN_SCREW_FROM_EDGE: f64 = 8.0;
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

/// Every screw hole sits on the front, clear of its edges, and so does the handle's body:
/// `body` is its reach along the front's width and up its height.
fn handle_fits(
    part: &Part,
    holes: &[(f64, f64)],
    wa: Axis,
    body: ((f64, f64), (f64, f64)),
) -> bool {
    let b = part.bounds;
    let margin = HANDLE_SCREW_DIAMETER;
    let (w, z) = body;
    holes.iter().all(|(w, z)| {
        *w >= b.min.get(wa) + margin
            && *w <= b.max().get(wa) - margin
            && *z >= b.min.z + margin
            && *z <= b.max().z - margin
    }) && w.0 >= b.min.get(wa)
        && w.1 <= b.max().get(wa)
        && z.0 >= b.min.z
        && z.1 <= b.max().z
}
/// Pilot for a 4 × 20 screw through a 6 mm back: short enough to stay clear of the shelf-pin row
/// 37 mm in from the rear.
const BACK_SCREW_PILOT_DEPTH: f64 = 15.0;
/// The hidden batten behind a kick board between notched sides: our rule of thumb.
const PLINTH_BLOCK_HEIGHT: f64 = 40.0;
/// Its screws, at most this far apart.
const PLINTH_BLOCK_SPACING: f64 = 300.0;
/// How far each screw goes into the board it fixes: both are at most 18, and stop short of showing.
const PLINTH_BLOCK_PILOT: f64 = 12.0;
/// Clearance each side of an adjustable shelf.
const SHELF_SIDE_CLEARANCE: f64 = 1.0;
/// 32 mm system: shelf-pin and hinge-plate rows sit 37 mm in from the front and back.
const SYSTEM32_SETBACK: f64 = 37.0;
const SYSTEM32_PITCH: f64 = 32.0;
const SHELF_PIN_DIAMETER: f64 = 5.0;
const SHELF_PIN_DEPTH: f64 = 10.0;
/// Our choice (owner, 2026-10-27): shelf-pin holes only this many grid steps above and below each
/// adjustable shelf, so it can still move ±96 mm, not full-height rows.
const SHELF_PIN_REACH: f64 = 3.0;
/// Our choices for handles: the screw hole, the handle's centre line in from a door's free edge,
/// the nearest hole in from the top or bottom edge, the height of a tall door's handle centre,
/// and how deep a drawer front is before its handle moves up from the middle.
const HANDLE_SCREW_DIAMETER: f64 = 5.0;
const HANDLE_EDGE_INSET: f64 = 35.0;
const HANDLE_END_INSET: f64 = 50.0;
/// Our choice: wood left between a supplier handle's body and the end or edge of its front. Long
/// overhangs (Pedra Curve's body runs 79 past its screws) move the screws in to keep it.
const HANDLE_BODY_CLEARANCE: f64 = 20.0;
const HANDLE_SIDE_CLEARANCE: f64 = 10.0;
const TALL_HANDLE_HEIGHT: f64 = 1050.0;
const DEEP_FRONT: f64 = 200.0;
/// Our choice: a canopy bottom keeps at least this much board round the insert's cut-out.
const CANOPY_CUTOUT_MARGIN: f64 = 20.0;
/// How far a sharknose bevel goes into the front, our choice, and the nose it must leave at the
/// face so the edge doesn't chip.
const SHARKNOSE_INTO: f64 = 12.0;
const SHARKNOSE_NOSE: f64 = 4.0;
/// Our choice: double hanging's lower rail needs at least this much drop above whatever is under
/// it. The wardrobe's other sizes are the standard's.
const MIN_LOWER_HANGING: f64 = 300.0;
/// Our choice: a shaker panel's play in its groove each side. The frame, panel and groove sizes
/// are the standard's.
const SHAKER_PLAY: f64 = 1.0;
/// Glass frames: the glass, and the rebate at the frame's back that takes it and its bead.
const GLASS_THICKNESS: f64 = 4.0;
const GLASS_REBATE_DEPTH: f64 = 12.0;
/// Our choice: legs stand this far in from the cabinet's sides, so an AXILO plate's fixings clear
/// the joints in the ends of the bottom panel, the widest being a cam housing 47.5 in.
const LEG_INSET: f64 = 80.0;
/// Our choice: and this far in from the back, so a knock-in plate's tips clear the back's groove.
const LEG_BACK_INSET: f64 = 70.0;
/// Our choice: a light's profile runs this far back from the panel's front edge, to light the
/// bench rather than the wall.
const LIGHT_PROFILE_SETBACK: f64 = 50.0;
/// The owner's: a light at the rear stands this far from the rear.
const LIGHT_PROFILE_REAR_SETBACK: f64 = 50.0;
/// Our choice: how far from a strip's end its cable leaves through the panel.
const LIGHT_CABLE_FROM_END: f64 = 15.0;
/// Our choice: a channel for a light's cable, wide and deep.
const LIGHT_CHANNEL: (f64, f64) = (8.0, 6.0);
/// As `lighting`'s strips: short of each end of what they run along.
const LIGHT_PROFILE_END_CLEARANCE: f64 = crate::lighting::STRIP_END_CLEARANCE;
/// Our choice: the least board to leave behind a routed profile.
const LIGHT_PROFILE_MIN_LEFT: f64 = 3.0;
/// Generic until a supplier's shelf support is catalogued (HAF-3).
pub(crate) const SHELF_SUPPORT: &str = "shelf-support.5mm";
/// Our choice, not Blum's: an AVENTOS HF bi-fold's end hinges sit this far in from the fronts'
/// sides. A door's come from [`HINGE_POSITION`].
const HINGE_END_OFFSET: f64 = 100.0;
/// Where a door's end hinges sit, from Blum's configurator.
const HINGE_POSITION: &str = "blum.clip-top.hinge-position";

/// The arm cover cap for a 110° CLIP top hinge (catalogue p.76); other hinges take none here.
fn cover_cap(hinge: &str) -> Option<&'static str> {
    let rest = hinge
        .strip_prefix("blum.clip-top-blumotion.110.")
        .or_else(|| hinge.strip_prefix("blum.clip-top.110."))?;
    Some(if rest.starts_with("overlay") {
        "blum.hinge-cover-cap.overlay"
    } else {
        "blum.hinge-cover-cap.dual-inset"
    })
}
/// The least a hinge's cup centre sits from its door's top or bottom when it moves off a joint
/// (the 35 mm cup and some wood beyond it).
const HINGE_EDGE_CLEARANCE: f64 = 40.0;
/// Wood left between a plate screw and a joint beside it.
const JOINT_CLEARANCE: f64 = 2.0;
/// Our choice: a light's cable hole, its diameter and how far it sits in from the side and in
/// front of the back (clear of the side joint's screws and the back's groove).
const CABLE_HOLE_DIAMETER: f64 = 10.0;
const CABLE_HOLE_SIDE_INSET: f64 = 60.0;
const CABLE_HOLE_BACK_INSET: f64 = 25.0;
/// Blum's 155° hinge, 0-protrusion: for doors in front of inner drawers.
const INNER_DRAWER_HINGE: &str = "blum.clip-top-blumotion.155.overlay";

/// Builds the part list for one cabinet.
/// How sizes share a length with gaps between them: 0 takes an even part of what the rest leave.
pub(crate) fn share(total: f64, asks: &[f64], gap: f64) -> Option<Vec<f64>> {
    Builder::share(total, asks, gap)
}

pub fn build_cabinet(cab: &Cabinet) -> Result<PartList> {
    validate(cab)?;
    let mut frame = 0.0;
    let mut list = if matches!(cab.kind, CabinetKind::CornerBase | CabinetKind::CornerWall) {
        l_corner(cab)?
    } else if cab.kind == CabinetKind::Rangehood {
        let mut b = Builder::new(cab);
        b.canopy();
        b.cladding();
        b.cover_caps();
        b.list
    } else {
        // A drawer frame takes its thickness off the carcass, which moves back behind it: the
        // fronts and the back stay where they are. Built with the frame's back on the usual
        // front plane, then all of it moved back.
        frame = Builder::new(cab).frame_shift();
        let mut carcass = cab.clone();
        carcass.depth -= frame;
        let mut b = Builder::new(&carcass);
        b.carcass();
        b.fronts();
        b.drawer_frame();
        // After the fronts, so shelf-pin holes can make way for hinge plates.
        b.shelves();
        b.plinth();
        b.wall_hanging();
        b.sink();
        b.lights();
        b.cladding();
        b.cover_caps();
        b.opening();
        b.door_rails();
        b.drawer_inserts();
        b.lock();
        b.arched_top();
        b.grain_match();
        // Last: hinges, handles and drawer fixings are placed on the slab, then move to the
        // frame member they fall in.
        b.shaker_fronts();
        b.lift_arm_warnings();
        b.list
    };
    let o = cab.origin + Vec3::new(0.0, frame, 0.0);
    for p in &mut list.parts {
        p.bounds.min = p.bounds.min + o;
    }
    for tilt in list.tilts.values_mut() {
        tilt.pivot_y += o.y;
        tilt.pivot_z += o.z;
    }
    offset_operations(&mut list, o);
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
    // Its hidden block stops with it, and loses the screws that fell in the part cut away.
    let start = d - sb;
    for p in a
        .list
        .parts
        .iter_mut()
        .filter(|p| p.role == PartRole::PlinthBlock)
    {
        let end = p.bounds.max().x;
        p.bounds.min.x = start;
        p.bounds.size.x = end - start;
    }
    let block_name = format!("{}.{}", cab.id, PartRole::PlinthBlock.code());
    let plinth_name = format!("{}.{}", cab.id, PartRole::Plinth.code());
    let bottom_name = format!("{}.{}", cab.id, PartRole::Bottom.code());
    let block_y = sb + ft + s.carcass_thickness / 2.0;
    a.list.operations.retain(|o| match o {
        Operation::Bore {
            part, at, purpose, ..
        } => {
            let gone = at.x < start + 20.0
                && (*part == block_name
                    || (*part == plinth_name && *purpose == BorePurpose::ScrewPilot)
                    || (*part == bottom_name
                        && *purpose == BorePurpose::ScrewPilot
                        && (at.y - block_y).abs() < 1e-6));
            !gone
        }
        _ => true,
    });
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
    a.shaker_fronts();
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
            Operation::Bore { at, .. } | Operation::Round { at, .. } => *at = *at + o,
            Operation::Arch { centre, .. } => *centre = *centre + o,
            Operation::Chamfer { at, bounds, .. } => {
                *at = *at + o;
                bounds.min = bounds.min + o;
            }
            Operation::BiscuitSlot { bounds, .. }
            | Operation::Groove { bounds, .. }
            | Operation::Cutout { bounds, .. }
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
    /// While a section's fronts are built: its own opening and drawer system, if it has them.
    opening_now: Option<Opening>,
    system_now: Option<DrawerSystem>,
    /// That section's box height and length, where the maker picked them.
    box_pick_now: drawers::BoxPick,
    /// Fronts whose section opens differently from the job, by name.
    front_openings: std::collections::BTreeMap<String, Opening>,
    /// Doors that slide: they have neither hinges nor a handle or opening hardware of their own.
    sliding_fronts: std::collections::BTreeSet<String>,
    /// The drawer frame's openings, collected as the fronts are built.
    frame_openings: Vec<Aabb>,
    /// A section's pin rows, drilled after the fronts so they keep off the hinge plates.
    pending_pins: Vec<PinRows>,
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
    /// The space being fitted out when it is a column of sections, not the whole carcass.
    bay: Option<Bay>,
    /// Doors being hung now have inner drawers behind them (in a section).
    inner_hinge: bool,
}

/// Numbers already given in a sectioned cabinet, so part names stay unique across columns.
#[derive(Default)]
struct SectionCounts {
    lifts: usize,
    doors: usize,
    drawers: usize,
    shelves: usize,
}

/// The space doors hinge in and drawers run in: the inner faces of the panels either side and
/// those panels (a side or a divider).
#[derive(Clone)]
struct Bay {
    left: f64,
    right: f64,
    left_panel: String,
    right_panel: String,
    /// The panel on that hand is a divider, which doors share (half overlay).
    left_divider: bool,
    right_divider: bool,
}

/// The shelf pin holes one bay of shelves asks for.
struct PinRows {
    bay: Bay,
    z: (f64, f64),
    /// Centres of its fixed shelves, and its adjustable shelves' pin holes.
    fixed: Vec<f64>,
    loose: Vec<f64>,
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
    /// The steel box's side set (N, M, K, C, F); empty for wooden drawers.
    height: String,
    /// The front's top edge: the highest drawer takes the cutlery insert.
    front_top: f64,
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
            bay: None,
            inner_hinge: false,
            opening_now: None,
            system_now: None,
            box_pick_now: drawers::BoxPick::default(),
            front_openings: Default::default(),
            sliding_fronts: Default::default(),
            frame_openings: Vec::new(),
            pending_pins: Vec::new(),
        }
    }

    /// The bay being fitted out: a column of sections, or the carcass between its sides.
    fn bay(&self) -> Bay {
        self.bay.clone().unwrap_or_else(|| {
            let t = self.cab.standard.carcass_thickness;
            Bay {
                left: t,
                right: self.cab.width - t,
                left_panel: self.name(PartRole::LeftSide, None),
                right_panel: self.name(PartRole::RightSide, None),
                left_divider: false,
                right_divider: false,
            }
        })
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
            PartRole::ShoeShelf | PartRole::ShoeLip => s.front_board.clone(),
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
            PartRole::LeftSide => self.cab.seen_left(),
            PartRole::RightSide => self.cab.seen_right(),
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
        for op in &ops {
            if let Operation::Bore {
                part,
                at,
                purpose: BorePurpose::CamHousing,
                ..
            } = op
                && let Ok(housing) = crate::joinery::minifix_housing(end.thickness())
            {
                for item in [
                    housing.id.as_str(),
                    crate::joinery::MINIFIX_BOLT,
                    crate::joinery::minifix_cap(housing),
                ] {
                    let row = crate::hardware::item(item);
                    self.list.hardware.push(HardwarePlacement {
                        item: item.into(),
                        description: row
                            .map(|r| format!("{} {}, {}", r.family, r.article, r.variant))
                            .unwrap_or_else(|_| item.into()),
                        at: *at,
                        part: part.clone(),
                        quantity: 1,
                    });
                }
            }
        }
        self.list.operations.extend(ops);
    }

    /// How the fronts being built open: their section's way, or the job's.
    fn opening_now(&self) -> Opening {
        self.opening_now.unwrap_or(self.cab.standard.opening)
    }

    /// How a built front opens.
    fn opening_of(&self, front: &str) -> Opening {
        self.front_openings
            .get(front)
            .copied()
            .unwrap_or(self.cab.standard.opening)
    }

    /// How each front opens: handles on those with handles, and handleless hardware on the rest:
    /// a TIP-ON unit on an adapter plate per door (two on tall doors) and per drawer, or a
    /// SERVO-DRIVE unit per drawer plus the cabinet's power supply. Drawer units keep generic ids
    /// until BLUM-13 catalogues them.
    fn opening(&mut self) {
        let by_section = !self.front_openings.is_empty();
        if self.cab.standard.opening == Opening::Handles || by_section {
            self.handles();
        }
        let fronts: Vec<Part> = self
            .list
            .parts
            .iter()
            .filter(|p| matches!(p.role, PartRole::Door | PartRole::DrawerFront))
            .filter(|p| self.opening_of(&p.name) != Opening::Handles)
            .filter(|p| !self.sliding_fronts.contains(&p.name))
            .cloned()
            .collect();
        let lw = self.cab.width - 2.0 * self.cab.standard.carcass_thickness;
        let mut any_servo = false;
        for f in &fronts {
            let opening = self.opening_of(&f.name);
            let fit = self.drawer_fits.iter().find(|d| d.front == f.name);
            let rows: Vec<(String, String, u32)> = match (f.role, opening, fit) {
                // SERVO-DRIVE is one system for the box systems and MOVENTO/TANDEM (p.364, p.464);
                // only the box systems' heavy profiles take extra bumpers.
                (PartRole::DrawerFront, Opening::ServoDrive, Some(d)) => {
                    any_servo = true;
                    let heavy = d.heavy_runner
                        && matches!(d.system, "legrabox" | "merivobox" | "tandembox");
                    match self.servo_drawer(d.front_height, heavy) {
                        Ok(items) => items,
                        Err(e) => {
                            self.list.warnings.push(format!("{}: {e}", f.name));
                            Vec::new()
                        }
                    }
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
        let servo_door = fronts
            .iter()
            .any(|f| f.role == PartRole::Door && self.opening_of(&f.name) == Opening::ServoDrive);
        if servo_door {
            self.list
                .warnings
                .push("SERVO-DRIVE does not open hinged doors: they get TIP-ON".into());
        }
        if any_servo {
            let bottom = self.name(PartRole::Bottom, None);
            let at = Vec3::new(self.cab.width / 2.0, self.interior_back, self.carcass_z0);
            self.servo_brackets(&bottom);
            let supply = crate::hardware::item("blum.servo-drive.power-supply.au");
            match supply {
                Ok(i) => self.list.hardware.push(HardwarePlacement {
                    item: i.id.clone(),
                    description: format!("{} {}", i.family, i.variant),
                    at,
                    part: bottom,
                    quantity: 1,
                }),
                Err(e) => self.list.warnings.push(e),
            }
        }
    }

    /// What one driven drawer carries: its drive unit and distance bumpers.
    fn servo_drawer(
        &self,
        front_height: f64,
        heavy: bool,
    ) -> std::result::Result<Vec<(String, String, u32)>, String> {
        let row = |i: &cabinet_catalog::HardwareItem, n: u32| {
            (i.id.clone(), format!("{} {}", i.family, i.variant), n)
        };
        Ok(vec![
            row(crate::hardware::item("blum.servo-drive.drive-unit")?, 1),
            row(
                crate::hardware::item("blum.servo-drive.distance-bumper")?,
                drawers::servo_bumpers(front_height, heavy)?,
            ),
        ])
    }

    /// A bracket, with its attachment, cable and node, per compartment holding driven drawers. An
    /// adjustable shelf in the way of the upright asks for a fixed shelf to stop it.
    fn servo_brackets(&mut self, bottom: &str) {
        for c in crate::standins::servo_compartments(&self.list) {
            let clear = c.ceiling - c.floor;
            let at = Vec3::new(c.x, self.interior_back, c.floor);
            match drawers::servo_bracket(clear) {
                Ok(items) => {
                    for (i, quantity) in items {
                        let cut = if i.id.contains("bracket-profile") {
                            format!(", cut to {:.1} mm", clear - 10.0)
                        } else {
                            String::new()
                        };
                        self.list.hardware.push(HardwarePlacement {
                            item: i.id.clone(),
                            description: format!("{} {}{cut}", i.family, i.variant),
                            at,
                            part: bottom.to_string(),
                            quantity,
                        });
                    }
                }
                Err(e) => self.list.warnings.push(e),
            }
            let line = crate::standins::nominal_length(&self.list, &c.front)
                .map(|nl| nl + 19.0 - self.frame_shift());
            let blocked: Vec<String> = self
                .list
                .hardware
                .iter()
                .filter(|h| h.item == SHELF_SUPPORT)
                .filter_map(|h| self.list.part(&h.part))
                .filter(|p| {
                    p.bounds.min.x <= c.x
                        && c.x <= p.bounds.max().x
                        && (c.floor..c.ceiling).contains(&p.bounds.min.z)
                        && line.is_some_and(|y| p.bounds.max().y >= y)
                })
                .map(|p| p.name.clone())
                .collect();
            for shelf in blocked {
                self.list.warnings.push(format!(
                    "{shelf} is in the way of the SERVO-DRIVE bracket behind the drawers: put a \
                     fixed shelf above them"
                ));
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
        // A stacked front's or sectioned cabinet's sections bring their own fixed shelves.
        if let FrontLayout::Sections { columns } = &cab.fronts {
            let wardrobe = columns
                .iter()
                .flat_map(|c| &c.rows)
                .any(|r| r.inside == SectionInside::Wardrobe);
            if wardrobe {
                if columns.len() > 1 {
                    self.list
                        .warnings
                        .push("a wardrobe interior needs the whole cabinet; left out".into());
                } else {
                    self.wardrobe();
                }
            } else if cab.adjustable_shelves > 0 || cab.inner_drawers > 0 {
                self.list
                    .warnings
                    .push("shelves and inner drawers left out: each section sets its own".into());
            }
            return;
        }
        if cab.kind == CabinetKind::Wardrobe {
            self.wardrobe();
            return;
        }
        if matches!(cab.fronts, FrontLayout::Stack { .. }) {
            if cab.adjustable_shelves > 0 {
                self.list
                    .warnings
                    .push("shelves left out: the stacked sections have their own".into());
            }
            return;
        }
        if cab.inner_drawers > 0 {
            if cab.adjustable_shelves > 0 {
                self.list
                    .warnings
                    .push("shelves left out: the inner drawers take the opening".into());
            }
            self.inner_drawers();
            return;
        }
        let bay = self.bay();
        let z = self.opening_z;
        let pins = self.shelves_in(
            1,
            cab.adjustable_shelves as usize,
            &cab.fixed_shelves,
            &bay,
            z,
        );
        self.shelf_pin_rows(&pins);
    }

    /// `n` shelves evenly spaced on the 32 mm grid over `z`, numbered from `first`, across `bay`:
    /// on pins, or jointed where `fixed` says (from 1, bottom up). Returns the holes they want.
    fn shelves_in(
        &mut self,
        first: usize,
        n: usize,
        fixed: &[u32],
        bay: &Bay,
        (z0, z1): (f64, f64),
    ) -> PinRows {
        let s = &self.cab.standard;
        let t = s.carcass_thickness;
        let y1 = self.interior_back - 2.0;
        let mut fixed_z = Vec::new();
        let mut loose_z = Vec::new();
        let step = (z1 - z0) / (n + 1) as f64;
        for i in 1..=n {
            // Snap to the 32 mm grid so a shelf rests on real pin holes: on top of the pins,
            // a pin's radius above the holes' centres (Blum's configurator, 2026-10-27).
            let raw = z0 + step * i as f64;
            let hole = z0 + ((raw - z0) / SYSTEM32_PITCH).round() * SYSTEM32_PITCH;
            let is_fixed = fixed.contains(&(i as u32));
            let z = if is_fixed {
                hole
            } else {
                hole + SHELF_PIN_DIAMETER / 2.0
            };
            // A fixed shelf fills the width and reaches the back, like a bottom.
            let bounds = if is_fixed {
                aabb(
                    bay.left,
                    self.shelf_setback(),
                    z,
                    bay.right,
                    self.interior_back,
                    z + t,
                )
            } else {
                aabb(
                    bay.left + SHELF_SIDE_CLEARANCE,
                    self.shelf_setback(),
                    z,
                    bay.right - SHELF_SIDE_CLEARANCE,
                    y1,
                    z + t,
                )
            };
            let number = first + i - 1;
            self.add(
                PartRole::Shelf,
                Some(number),
                bounds,
                Axis::Z,
                Axis::X,
                EdgeBanding::front_only(),
            );
            if is_fixed {
                fixed_z.push(z + t / 2.0);
                let shelf = self.list.parts.last().cloned().unwrap();
                let panel = |b: &Self, name: &str| {
                    b.list
                        .parts
                        .iter()
                        .find(|p| p.name == name)
                        .cloned()
                        .unwrap()
                };
                let (left, right) = (panel(self, &bay.left_panel), panel(self, &bay.right_panel));
                self.joint_parts(&left, &shelf, Dir::new(Axis::X, true), Axis::Y);
                self.joint_parts(&right, &shelf, Dir::new(Axis::X, false), Axis::Y);
            } else {
                // One in each front and back pin row, both sides.
                loose_z.push(hole);
                self.list.hardware.push(HardwarePlacement {
                    item: SHELF_SUPPORT.into(),
                    description: "Shelf support for 5 mm holes".into(),
                    at: bounds.center(),
                    part: self.name(PartRole::Shelf, Some(number)),
                    quantity: 4,
                });
            }
        }
        PinRows {
            bay: bay.clone(),
            z: (z0, z1),
            fixed: fixed_z,
            loose: loose_z,
        }
    }

    /// Shelf-pin holes on the inside of a bay's panels: a front and a back row on the 32 mm grid,
    /// within [`SHELF_PIN_REACH`] steps of each adjustable shelf, and left out within a pitch of a
    /// fixed shelf, where its fixings are.
    fn shelf_pin_rows(&mut self, pins: &PinRows) {
        if pins.loose.is_empty() {
            return;
        }
        let t = self.cab.standard.carcass_thickness;
        let bay = &pins.bay;
        let (z0, z1) = pins.z;
        let rows = [SYSTEM32_SETBACK, self.interior_back - SYSTEM32_SETBACK];
        let mut z = z0 + 2.0 * SYSTEM32_PITCH;
        let mut skipped = 0;
        let hands = [
            (&bay.left_panel, bay.left, false, bay.left_divider),
            (&bay.right_panel, bay.right, true, bay.right_divider),
        ];
        while z <= z1 - 2.0 * SYSTEM32_PITCH {
            let near_shelf = pins
                .loose
                .iter()
                .any(|l| (z - l).abs() <= SHELF_PIN_REACH * SYSTEM32_PITCH + 1e-6);
            if !near_shelf || pins.fixed.iter().any(|f| (z - f).abs() < SYSTEM32_PITCH) {
                z += SYSTEM32_PITCH;
                continue;
            }
            for y in rows {
                for (part, x, positive, divider) in hands {
                    let at = Vec3::new(x, y, z);
                    // A plate screw's hole on the row serves as this pin hole too.
                    let shared = self.list.operations.iter().any(|o| match o {
                        Operation::Bore {
                            part: p,
                            at: h,
                            diameter: d,
                            ..
                        } => {
                            p == part
                                && Axis::ALL
                                    .iter()
                                    .all(|&a| (h.get(a) - at.get(a)).abs() < 1e-6)
                                && (d - SHELF_PIN_DIAMETER).abs() < 1e-6
                        }
                        _ => false,
                    });
                    if shared {
                        continue;
                    }
                    // A divider is drilled from both faces: what is behind this hole counts.
                    let behind = divider
                        && self.list.operations.iter().any(|o| match o {
                            Operation::Bore {
                                part: p,
                                at: h,
                                diameter: d,
                                ..
                            } if p == part && (h.x - x).abs() > 1e-6 => {
                                let dist = ((h.y - y).powi(2) + (h.z - z).powi(2)).sqrt();
                                dist < (SHELF_PIN_DIAMETER + d) / 2.0 + 3.0
                                    && (d - SHELF_PIN_DIAMETER).abs() > 1e-6
                            }
                            _ => false,
                        });
                    if behind || self.bore_clashes(part, at, SHELF_PIN_DIAMETER) {
                        skipped += 1;
                        continue;
                    }
                    self.list.operations.push(Operation::Bore {
                        part: part.clone(),
                        at,
                        direction: Dir::new(Axis::X, positive),
                        diameter: SHELF_PIN_DIAMETER,
                        // Holes meet from both faces of a divider only if they run over half.
                        depth: if divider {
                            SHELF_PIN_DEPTH.min((t - 1.0) / 2.0)
                        } else {
                            SHELF_PIN_DEPTH
                        },
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
                // The same hole already there (a plate screw's on the pin row) is shared.
                let same = dist < 1e-6 && (diameter - d).abs() < 1e-6;
                !same && dist < (diameter + d) / 2.0 + WALL
            }
            // A pocket in the part (AVENTOS HKi's): no hole in or beside it.
            Operation::Groove { part: p, bounds } if p == part => {
                let reach = diameter / 2.0 + WALL;
                Axis::ALL.iter().all(|&a| {
                    at.get(a) > bounds.min.get(a) - reach && at.get(a) < bounds.max().get(a) + reach
                })
            }
            _ => false,
        })
    }

    /// How much a drawer frame takes off the carcass's depth: 0 without one. Hinged and lifting
    /// fronts need the carcass edge the board would cover, and inset fronts sit inside it.
    fn drawer_frame_thickness(&self) -> f64 {
        let s = &self.cab.standard;
        if s.drawer_frame == DrawerFrame::None || !self.takes_drawer_frame() {
            return 0.0;
        }
        s.drawer_frame_thickness
    }

    /// How far a drawer frame moves the carcass back. Inset fronts have the board inside the
    /// carcass, behind them, so nothing moves.
    fn frame_shift(&self) -> f64 {
        if self.cab.standard.overlay == DoorOverlay::Inset {
            0.0
        } else {
            self.drawer_frame_thickness()
        }
    }

    /// The front and back of the drawer frame, before the carcass moves back.
    fn frame_y(&self) -> (f64, f64) {
        let f = self.drawer_frame_thickness();
        if self.cab.standard.overlay == DoorOverlay::Inset {
            let ft = self.cab.standard.front_thickness;
            (ft, ft + f)
        } else {
            (-f, 0.0)
        }
    }

    fn takes_drawer_frame(&self) -> bool {
        let cab = self.cab;
        cab.kind != CabinetKind::BlindCorner
            && match &cab.fronts {
                FrontLayout::Drawers { .. } => true,
                FrontLayout::Sections { columns } => {
                    // Inset, the board stands inside the carcass, where dividers would cross it.
                    if cab.standard.overlay == DoorOverlay::Inset && columns.len() > 1 {
                        return false;
                    }
                    let rows = || columns.iter().flat_map(|c| &c.rows);
                    rows().any(|r| matches!(r.front, SectionFront::Drawers { .. }))
                        && rows().all(|r| {
                            matches!(
                                r.front,
                                SectionFront::Drawers { .. }
                                    | SectionFront::Open
                                    | SectionFront::Niche
                            )
                        })
                }
                _ => false,
            }
    }

    /// The drawer frame: one board over the carcass's front, behind the fronts, with an opening
    /// cut for each drawer and each open section.
    fn drawer_frame(&mut self) {
        let cab = self.cab;
        if cab.standard.drawer_frame == DrawerFrame::None {
            return;
        }
        let f = self.drawer_frame_thickness();
        if f <= 0.0 {
            self.list.warnings.push(
                "a drawer frame needs fronts that are all drawers, or open sections, and inset in \
                 one column: left off"
                    .into(),
            );
            return;
        }
        let s = &cab.standard;
        if s.is_sharknose() {
            let into = SHARKNOSE_INTO.min(s.front_thickness - SHARKNOSE_NOSE);
            let down = into / s.sharknose_angle.degrees().to_radians().tan();
            if s.drawer_frame_overlap < down {
                self.list.warnings.push(format!(
                    "the drawer frame's rails stop above the {down:.0} mm sharknose bevel: the \
                     inside shows through the gap"
                ));
            }
        }
        let (y0, y1) = self.frame_y();
        // Inset, the openings run from side to side: what is left is separate rails between them.
        if cab.standard.overlay == DoorOverlay::Inset {
            let t = cab.standard.carcass_thickness;
            let (z0, z1) = self.opening_z;
            let mut openings = std::mem::take(&mut self.frame_openings);
            openings.sort_by(|a, b| a.min.z.total_cmp(&b.min.z));
            let mut from = z0;
            let mut rails = Vec::new();
            for o in &openings {
                if o.min.z > from + 1e-6 {
                    rails.push((from, o.min.z));
                }
                from = from.max(o.max().z);
            }
            if z1 > from + 1e-6 {
                rails.push((from, z1));
            }
            for (i, (a, b)) in rails.into_iter().enumerate() {
                self.add(
                    PartRole::DrawerFrame,
                    Some(i + 1),
                    aabb(t, y0, a, cab.width - t, y1, b),
                    Axis::Y,
                    Axis::X,
                    EdgeBanding::all(),
                );
            }
            return;
        }
        let bounds = aabb(0.0, y0, self.carcass_z0, cab.width, y1, cab.height);
        self.add(
            PartRole::DrawerFrame,
            None,
            bounds,
            Axis::Y,
            Axis::Z,
            EdgeBanding::all(),
        );
        let part = self.name(PartRole::DrawerFrame, None);
        for cut in std::mem::take(&mut self.frame_openings) {
            self.list.operations.push(Operation::Cutout {
                part: part.clone(),
                bounds: cut,
                radius: 0.0,
                note: String::new(),
            });
        }
    }

    /// The rail behind a sharknose door's bevelled edge, and only there: without it the inside
    /// shows down the bevel. It stands inside the opening, behind the door, so the hinges and the
    /// carcass stay as they are.
    fn door_rails(&mut self) {
        let s = &self.cab.standard;
        let (t, overlap) = (s.drawer_frame_thickness, s.drawer_frame_overlap);
        let half_gap = s.sharknose_gap / 2.0;
        let cuts: Vec<(Part, Vec3)> = self
            .list
            .operations
            .iter()
            .filter_map(|o| match o {
                Operation::Chamfer { part, at, .. } => {
                    let p = self.list.part(part)?;
                    (p.role == PartRole::Door && p.thickness_axis == Axis::Y)
                        .then(|| (p.clone(), *at))
                }
                _ => None,
            })
            .collect();
        let mut n = 0;
        let mut short = false;
        for (door, at) in cuts {
            let d = door.bounds;
            let mid = d.center();
            let Some((l, r)) = self.list.bay_around(mid.x) else {
                continue;
            };
            let (left, right) = (l.bounds.max().x, r.bounds.min.x);
            let Some((floor, ceiling)) = self.list.compartment_around(mid.x, mid.z) else {
                continue;
            };
            let (x0, x1) = (d.min.x.max(left), d.max().x.min(right));
            let (z0, z1) = (d.min.z.max(floor), d.max().z.min(ceiling));
            let (box_x, box_z) = if (at.z - d.max().z).abs() < 1e-6 {
                let top = d.max().z;
                let reach = (top + s.sharknose_gap).min(ceiling);
                ((x0, x1), (top - overlap, reach))
            } else if (at.z - d.min.z).abs() < 1e-6 {
                let bottom = d.min.z;
                let reach = (bottom - s.sharknose_gap).max(floor);
                ((x0, x1), (reach, bottom + overlap))
            } else if (at.x - d.max().x).abs() < 1e-6 {
                let edge = d.max().x;
                (
                    ((edge - overlap).max(left), (edge + half_gap).min(right)),
                    (z0, z1),
                )
            } else {
                let edge = d.min.x;
                (
                    ((edge - half_gap).max(left), (edge + overlap).min(right)),
                    (z0, z1),
                )
            };
            let into = SHARKNOSE_INTO.min(s.front_thickness - SHARKNOSE_NOSE);
            short |= overlap < into / s.sharknose_angle.degrees().to_radians().tan();
            // Nothing to cover where the carcass already reaches past the bevel.
            if box_x.1 - box_x.0 < 1.0 || box_z.1 - box_z.0 < 1.0 {
                continue;
            }
            n += 1;
            let y = d.max().y;
            self.add(
                PartRole::GripRail,
                Some(n),
                aabb(box_x.0, y, box_z.0, box_x.1, y + t, box_z.1),
                Axis::Y,
                if box_x.1 - box_x.0 > box_z.1 - box_z.0 {
                    Axis::X
                } else {
                    Axis::Z
                },
                EdgeBanding::all(),
            );
        }
        if short {
            self.list.warnings.push(
                "the rails behind the sharknose doors stop short of the bevel: the inside shows \
                 through the gap"
                    .into(),
            );
        }
    }

    /// Whether the top front needs a finger gap over it: drawers do, and doors gripped there.
    fn finger_gap_on_top(&self) -> bool {
        let cab = self.cab;
        let s = &cab.standard;
        if !s.is_sharknose() {
            return false;
        }
        let drawers = match &cab.fronts {
            FrontLayout::Drawers { .. } => true,
            FrontLayout::Stack { sections } => sections
                .iter()
                .any(|x| matches!(x.kind, SectionKind::Drawers { .. })),
            FrontLayout::Sections { columns } => columns
                .iter()
                .flat_map(|c| &c.rows)
                .any(|r| matches!(r.front, SectionFront::Drawers { .. })),
            _ => false,
        };
        drawers
            || !matches!(
                s.sharknose_door_edge,
                SharknoseEdge::Bottom | SharknoseEdge::Opening
            )
    }

    /// How far a shelf stands back from the cabinet's front: clear of a rail down the door's
    /// opening side.
    fn shelf_setback(&self) -> f64 {
        let s = &self.cab.standard;
        // A bi-fold's lever arms run down the sides just behind the front, past any shelf.
        let arms = crate::hardware::item("blum.aventos.hf-top")
            .ok()
            .filter(|_| self.cab.has_bi_fold())
            .and_then(|hf| hf.param("arm_clear_from_front").ok())
            .unwrap_or(0.0);
        if arms > 0.0 {
            return s.shelf_front_setback.max(arms);
        }
        if s.is_sharknose() && s.sharknose_door_edge == SharknoseEdge::Opening {
            let inset = if s.overlay == DoorOverlay::Inset {
                s.front_thickness
            } else {
                0.0
            };
            s.shelf_front_setback
                .max(inset + s.drawer_frame_thickness + 2.0)
        } else {
            s.shelf_front_setback
        }
    }

    /// An opening in the drawer frame, across the bay from `z0` to `z1`.
    fn frame_opening(&mut self, z0: f64, z1: f64) {
        let f = self.drawer_frame_thickness();
        if f > 0.0 {
            let bay = self.bay();
            self.frame_openings.push(aabb(
                bay.left,
                self.frame_y().0,
                z0,
                bay.right,
                self.frame_y().1,
                z1,
            ));
        }
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
        // Blum's configurator sets these apart from the gap between fronts.
        let top_gap = if under_bench && self.finger_gap_on_top() {
            s.top_gap.max(s.sharknose_gap)
        } else if under_bench {
            s.top_gap.max(s.edge_pull_gap().unwrap_or(0.0))
        } else {
            0.0
        };
        let z0 = self.carcass_z0 + s.bottom_gap;
        match s.overlay {
            DoorOverlay::Full => {
                let f = self.frame_shift();
                (g / 2.0, w - g / 2.0, z0, h - top_gap, -ft - f, -f)
            }
            DoorOverlay::Half => {
                let f = self.frame_shift();
                (t / 2.0, w - t / 2.0, z0, h - top_gap, -ft - f, -f)
            }
            DoorOverlay::Inset => {
                let (z0, z1) = self.opening_z;
                let over = if self.finger_gap_on_top() {
                    s.sharknose_gap.max(g)
                } else {
                    g.max(s.edge_pull_gap().unwrap_or(0.0))
                };
                (t + g, w - t - g, z0 + g, z1 - over, 0.0, ft)
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
                let (floor, ceiling) = self.opening_z;
                self.drawer_fronts(1, ratios, (x0, x1, y0, y1), (z0, z1), (floor, ceiling));
            }
            FrontLayout::Stack { sections } => {
                self.stack_fronts(sections, (x0, x1, y0, y1), (z0, z1));
            }
            FrontLayout::Sections { columns } => {
                self.section_fronts(columns, (x0, x1, y0, y1), (z0, z1));
            }
        }
    }

    /// Shares a length among parts asking for `asks` each (0: a share of what is left), with
    /// `gap` between them; with none at 0 the last takes the rest. None when they need more.
    fn share(total: f64, asks: &[f64], gap: f64) -> Option<Vec<f64>> {
        let fixed: f64 = asks.iter().map(|a| a.max(0.0)).sum();
        let rest = total - gap * asks.len().saturating_sub(1) as f64 - fixed;
        if rest < -1e-6 {
            return None;
        }
        let free = asks.iter().filter(|a| **a <= 0.0).count();
        let mut out: Vec<f64> = asks.to_vec();
        if free > 0 {
            for a in out.iter_mut().filter(|a| **a <= 0.0) {
                *a = rest / free as f64;
            }
        } else if let Some(last) = out.last_mut() {
            *last += rest;
        }
        Some(out)
    }

    /// A sectioned cabinet (SEC-1): columns between full-height dividers, each split into rows
    /// by fixed shelves, every row with its own fronts and inside. Doors hinge on the side or
    /// divider beside them; drawers run in their column.
    fn section_fronts(
        &mut self,
        columns: &[SectionColumn],
        (x0, x1, y0, y1): (f64, f64, f64, f64),
        (z0, z1): (f64, f64),
    ) {
        let cab = self.cab;
        let s = &cab.standard;
        let (g, t) = (s.front_gap, s.carcass_thickness);
        if columns.is_empty() {
            return;
        }
        self.list.section_zone = Some((x1 - x0, z1 - z0));
        let asks: Vec<f64> = columns.iter().map(|c| c.width).collect();
        let Some(widths) = Self::share(x1 - x0, &asks, g) else {
            self.list.warnings.push(
                "the columns need more width than the cabinet's fronts have; left out".into(),
            );
            return;
        };
        // The dividers stand in the gaps between the columns' fronts, bottom to top.
        let (oz0, oz1) = self.opening_z;
        let mut bays = Vec::new();
        let mut left = (t, self.name(PartRole::LeftSide, None), false);
        let mut fx0 = x0;
        for (i, w) in widths.iter().enumerate() {
            let fx1 = fx0 + w;
            let right = if i + 1 == widths.len() {
                (cab.width - t, self.name(PartRole::RightSide, None), false)
            } else {
                let c = fx1 + g / 2.0;
                self.add(
                    PartRole::Divider,
                    Some(i + 1),
                    aabb(c - t / 2.0, 0.0, oz0, c + t / 2.0, self.interior_back, oz1),
                    Axis::X,
                    Axis::Z,
                    EdgeBanding::front_only(),
                );
                let divider = self.list.parts.last().cloned().unwrap();
                if let Some(bottom) = self.part(PartRole::Bottom) {
                    self.joint_parts(&bottom, &divider, Dir::new(Axis::Z, true), Axis::Y);
                }
                for role in [PartRole::Top, PartRole::FrontRail, PartRole::BackRail] {
                    if let Some(over) = self.part(role) {
                        self.joint_parts(&over, &divider, Dir::new(Axis::Z, false), Axis::Y);
                    }
                }
                (c - t / 2.0, divider.name.clone(), true)
            };
            bays.push((
                Bay {
                    left: left.0,
                    right: right.0,
                    left_panel: left.1.clone(),
                    right_panel: right.1.clone(),
                    left_divider: left.2,
                    right_divider: right.2,
                },
                (fx0, fx1),
            ));
            left = (right.0 + if right.2 { t } else { 0.0 }, right.1, right.2);
            fx0 = fx1 + g;
        }
        // Every column's insides and shelves before any front, so each hinge can keep clear of
        // the joints on its panel, a divider's other face included.
        let mut counts = SectionCounts::default();
        for fronts in [false, true] {
            for (column, (bay, (fx0, fx1))) in columns.iter().zip(bays.iter().cloned()) {
                self.bay = Some(bay);
                let at = ((fx0, fx1, y0, y1), (z0, z1));
                self.column_rows(&column.rows, at, &mut counts, fronts);
            }
        }
        self.bay = None;
        for pins in std::mem::take(&mut self.pending_pins) {
            self.shelf_pin_rows(&pins);
        }
    }

    /// One column's rows, bottom to top, as `stack_fronts` lays a stack out: what is inside each
    /// row and a fixed shelf over it (but the top one), or, with `fronts`, each row's fronts.
    fn column_rows(
        &mut self,
        rows: &[Section],
        ((x0, x1, y0, y1), (z0, z1)): ((f64, f64, f64, f64), (f64, f64)),
        counts: &mut SectionCounts,
        fronts: bool,
    ) {
        let cab = self.cab;
        let s = &cab.standard;
        let (g, t) = (s.row_gap(), s.carcass_thickness);
        if rows.is_empty() {
            return;
        }
        // A niche's height is its clear opening: add the shelf under it, less the gap fronts have.
        let asks: Vec<f64> = rows
            .iter()
            .map(|r| match r.front {
                SectionFront::Niche if r.height > 0.0 => r.height + t - g,
                _ => r.height,
            })
            .collect();
        let Some(heights) = Self::share(z1 - z0, &asks, g) else {
            if !fronts {
                self.list.warnings.push(
                    "a column's sections need more height than the cabinet has; left out".into(),
                );
            }
            return;
        };
        let bay = self.bay();
        let panel = |b: &Self, name: &str| b.list.parts.iter().find(|p| p.name == name).cloned();
        let lift = self.opening_z.0 - z0;
        let mut fz0 = z0;
        let mut floor = self.opening_z.0;
        for (i, (row, h)) in rows.iter().zip(&heights).enumerate() {
            let fz1 = fz0 + h;
            let last = i + 1 == rows.len();
            // A shelf over it has its top where the next fronts start, as the bottom panel sits
            // under the lowest front. Without one, the next section's floor is this one's top.
            let next_floor = fz1 + g + lift;
            let ceiling = if last {
                self.opening_z.1
            } else if row.shelf_above {
                next_floor - t
            } else {
                fz1 + g
            };
            let behind_drawers = matches!(row.front, SectionFront::Drawers { .. });
            if fronts {
                // Inner drawers take the 0-protrusion hinge on the doors in front of them.
                self.inner_hinge =
                    matches!(row.inside, SectionInside::InnerDrawers { .. }) && !behind_drawers;
                self.opening_now = row.opening;
                self.system_now = match row.front {
                    SectionFront::Drawers { system, .. } => system,
                    _ => None,
                };
                self.box_pick_now = match row.front {
                    SectionFront::Drawers {
                        box_height,
                        nominal_length,
                        ..
                    } => drawers::BoxPick {
                        height: box_height,
                        nominal_length,
                    },
                    _ => drawers::BoxPick::default(),
                };
                let before = self.list.parts.len();
                self.row_fronts(row, (x0, x1, y0, y1), (fz0, fz1), (floor, ceiling), counts);
                if matches!(row.front, SectionFront::Open | SectionFront::Niche) {
                    self.frame_opening(floor, ceiling);
                }
                if let Some(opening) = row.opening {
                    for p in &self.list.parts[before..] {
                        self.front_openings.insert(p.name.clone(), opening);
                    }
                }
                self.inner_hinge = false;
                self.opening_now = None;
                self.system_now = None;
                self.box_pick_now = drawers::BoxPick::default();
                if !last {
                    floor = if row.shelf_above { next_floor } else { fz1 + g };
                }
                fz0 = fz1 + g;
                continue;
            }
            match &row.inside {
                SectionInside::Empty => {}
                SectionInside::Shelves { count, fixed } => {
                    let n = *count as usize;
                    let pins =
                        self.shelves_in(counts.shelves + 1, n, fixed, &bay, (floor, ceiling));
                    counts.shelves += n;
                    self.pending_pins.push(pins);
                }
                // Built with the cabinet's shelves: it is the whole cabinet's interior.
                SectionInside::Wardrobe => {}
                SectionInside::ShoeShelves { count, angle, lip } if *angle > 0.0 => {
                    self.sloped_shoe_shelves(
                        (*count, *angle, *lip),
                        &bay,
                        (floor, ceiling),
                        counts,
                    );
                }
                SectionInside::ShoeShelves { count, .. } => {
                    let n = *count as usize;
                    for k in 1..=n {
                        let z = floor + (ceiling - floor) * (k - 1) as f64 / n as f64;
                        if k == 1 && (z - floor).abs() < 1e-9 {
                            // The floor is the first shoe shelf.
                            continue;
                        }
                        counts.shelves += 1;
                        self.fixed_shelf(
                            counts.shelves,
                            &bay,
                            z,
                            panel(self, &bay.left_panel),
                            panel(self, &bay.right_panel),
                        );
                    }
                }
                SectionInside::Hanging => {
                    let length = bay.right - bay.left - 2.0;
                    let at = Vec3::new(
                        (bay.left + bay.right) / 2.0,
                        self.interior_back / 2.0,
                        ceiling - s.wardrobe_rail_drop,
                    );
                    // Its end supports screw to the bay's panels.
                    let part = bay.left_panel.clone();
                    for (item, description, quantity) in [
                        (
                            "wardrobe-rail.oval",
                            format!("Wardrobe rail, cut to {length:.0} mm"),
                            1,
                        ),
                        (
                            "wardrobe-rail.end-support",
                            "Wardrobe rail end support".to_string(),
                            2,
                        ),
                    ] {
                        self.list.hardware.push(HardwarePlacement {
                            item: item.into(),
                            description,
                            at,
                            part: part.clone(),
                            quantity,
                        });
                    }
                }
                SectionInside::InnerDrawers { count } => {
                    if behind_drawers {
                        self.list.warnings.push(
                            "inner drawers go behind doors or in the open; left out behind drawers"
                                .into(),
                        );
                    } else {
                        counts.drawers += self.inner_drawers_from(
                            counts.drawers + 1,
                            *count as usize,
                            floor,
                            ceiling,
                        );
                    }
                }
            }
            if !last {
                if row.shelf_above {
                    counts.shelves += 1;
                    self.fixed_shelf(
                        counts.shelves,
                        &bay,
                        ceiling,
                        panel(self, &bay.left_panel),
                        panel(self, &bay.right_panel),
                    );
                }
                floor = if row.shelf_above { next_floor } else { fz1 + g };
            }
            fz0 = fz1 + g;
        }
    }

    /// AVENTOS is sized from the cabinet and fitted to its sides: one lift to a cabinet, across the
    /// top of it. Says why not.
    fn lift_section_fits(&mut self, counts: &mut SectionCounts, top: f64) -> bool {
        let bay = self.bay();
        let whole_width = !bay.left_divider && !bay.right_divider;
        let at_top = (top - self.front_region().3).abs() < 1e-6;
        if counts.lifts > 0 || !whole_width || !at_top {
            self.list.warnings.push(
                "a lift goes across the top of the cabinet, one to a cabinet; left out".into(),
            );
            return false;
        }
        counts.lifts += 1;
        true
    }

    /// A section row's fronts across `x`, `fz` high, opening onto `inside` (floor, ceiling).
    fn row_fronts(
        &mut self,
        row: &Section,
        (x0, x1, y0, y1): (f64, f64, f64, f64),
        (fz0, fz1): (f64, f64),
        (floor, ceiling): (f64, f64),
        counts: &mut SectionCounts,
    ) {
        let g = self.cab.standard.front_gap;
        match &row.front {
            SectionFront::Open => {}
            SectionFront::Door { hinge_side } => {
                counts.doors += 1;
                self.door(counts.doors, aabb(x0, y0, fz0, x1, y1, fz1), *hinge_side);
            }
            SectionFront::DoorPair => {
                let mid = (x0 + x1) / 2.0;
                self.door(
                    counts.doors + 1,
                    aabb(x0, y0, fz0, mid - g / 2.0, y1, fz1),
                    HingeSide::Left,
                );
                self.door(
                    counts.doors + 2,
                    aabb(mid + g / 2.0, y0, fz0, x1, y1, fz1),
                    HingeSide::Right,
                );
                counts.doors += 2;
            }
            SectionFront::Niche => {}
            SectionFront::Sliding {
                count,
                overlap,
                track,
            } => self.sliding_doors(
                (*count, *overlap, track),
                (x0, x1, y0, y1),
                (fz0, fz1),
                counts,
            ),
            SectionFront::Lift {
                kind,
                handle_weight,
            } => {
                if self.lift_section_fits(counts, fz1) {
                    self.single_lift(*kind, aabb(x0, y0, fz0, x1, y1, fz1), *handle_weight);
                }
            }
            SectionFront::BiFold { handle_weight } => {
                if self.lift_section_fits(counts, fz1) {
                    self.bi_fold_lift(aabb(x0, y0, fz0, x1, y1, fz1), *handle_weight);
                }
            }
            SectionFront::Drawers { count, ratios, .. } => {
                let ratios = if ratios.is_empty() {
                    vec![1.0; (*count).max(1) as usize]
                } else {
                    ratios.clone()
                };
                counts.drawers += self.drawer_fronts(
                    counts.drawers + 1,
                    &ratios,
                    (x0, x1, y0, y1),
                    (fz0, fz1),
                    (floor, ceiling),
                );
            }
        }
    }

    /// Sloped shoe shelves of decorative board at an even pitch, each with a toe lip along its low
    /// front edge, screwed through the panels either side. Each is the flat board as it's cut,
    /// turned up about the bottom of its front edge.
    fn sloped_shoe_shelves(
        &mut self,
        (count, angle, lip): (u32, f64, f64),
        bay: &Bay,
        (floor, ceiling): (f64, f64),
        counts: &mut SectionCounts,
    ) {
        let s = &self.cab.standard;
        let t = s.front_thickness;
        let (sin, cos) = angle.to_radians().sin_cos();
        let y0 = self.shelf_setback() + t;
        // The back's lower corner reaches furthest: the board's length comes from that.
        let depth = self.interior_back - SHOE_SHELF_BACK_GAP - y0;
        let length = depth / cos;
        let n = count.max(1) as usize;
        let pitch = (ceiling - floor) / n as f64;
        // The lip's front corner dips below the pivot as it turns.
        let dip = t * sin + SHOE_SHELF_FLOOR_GAP;
        // The shelves are parallel, so the slope takes no room from the shoes: only the board does.
        // The slope does have to fit under the ceiling over the top one.
        let rise = length * sin + t * cos;
        if pitch - t / cos < MIN_SHOE_ROOM {
            self.list.warnings.push(format!(
                "{n} shoe shelves leave too little room for shoes between them: try fewer"
            ));
        }
        if pitch < dip + rise {
            self.list.warnings.push(format!(
                "the top shoe shelf's back rises into what is above it at {angle:.0}°: try fewer \
                 shelves or a shallower slope"
            ));
        }
        let (left, right) = (bay.left_panel.clone(), bay.right_panel.clone());
        for k in 0..n {
            let z = floor + k as f64 * pitch + dip;
            counts.shelves += 1;
            let tilt = Tilt {
                degrees: angle,
                pivot_y: y0,
                pivot_z: z,
            };
            let (bx0, bx1) = (bay.left, bay.right);
            self.add(
                PartRole::ShoeShelf,
                Some(counts.shelves),
                aabb(bx0, y0, z, bx1, y0 + length, z + t),
                Axis::Z,
                Axis::X,
                EdgeBanding::default(),
            );
            let shelf = self.name(PartRole::ShoeShelf, Some(counts.shelves));
            self.list.tilts.insert(shelf.clone(), tilt);
            self.add(
                PartRole::ShoeLip,
                Some(counts.shelves),
                aabb(bx0, y0 - t, z, bx1, y0, z + t + lip),
                Axis::Y,
                Axis::X,
                EdgeBanding {
                    top: true,
                    ..EdgeBanding::default()
                },
            );
            let toe = self.name(PartRole::ShoeLip, Some(counts.shelves));
            self.list.tilts.insert(toe, tilt);
            // Two screws each end, through the panel into the shelf's end grain, where the flat
            // board's centre line lands once turned.
            for along in [0.25, 0.75] {
                let flat_y = y0 + length * along;
                let flat_z = z + t / 2.0;
                let world = tilt.apply(Vec3::new(bx0, flat_y, flat_z));
                let sides = [
                    (&left, bx0, true, bay.left_divider),
                    (&right, bx1, false, bay.right_divider),
                ];
                for (panel, face, positive, divider) in sides {
                    let wall = if divider {
                        self.cab.standard.carcass_thickness
                    } else {
                        self.list
                            .part(panel)
                            .map_or(self.cab.standard.carcass_thickness, |p| p.thickness())
                    };
                    let sign = if positive { -1.0 } else { 1.0 };
                    self.list.operations.push(Operation::Bore {
                        part: panel.clone(),
                        at: Vec3::new(face + sign * wall, world.y, world.z),
                        direction: Dir::new(Axis::X, positive),
                        diameter: SHOE_SHELF_SCREW_CLEARANCE,
                        depth: wall,
                        purpose: BorePurpose::Screw,
                    });
                    self.list.operations.push(Operation::Bore {
                        part: shelf.clone(),
                        at: Vec3::new(face, flat_y, flat_z),
                        direction: Dir::new(Axis::X, positive),
                        diameter: SHOE_SHELF_PILOT,
                        depth: SHOE_SHELF_PILOT_DEPTH,
                        purpose: BorePurpose::ScrewPilot,
                    });
                }
            }
        }
        if !self.list.notes.iter().any(|n| n.contains("toe lip")) {
            self.list
                .notes
                .push("Shoe shelves: glue and pin each toe lip to its shelf's front edge".into());
        }
    }

    /// Sliding doors across a section: every other one in the front plane, overlapping its
    /// neighbours, between a top track and a bottom guide. They hang on the track, so no hinges,
    /// and take a flush pull, as a handle would foul the door passing it.
    fn sliding_doors(
        &mut self,
        (count, overlap, track): (u32, f64, &str),
        (x0, x1, y0, y1): (f64, f64, f64, f64),
        (z0, z1): (f64, f64),
        counts: &mut SectionCounts,
    ) {
        let numbers = (|| {
            let top = crate::hardware::item(&format!("{track}.top"))?;
            let bottom = crate::hardware::item(&format!("{track}.bottom"))?;
            let roller = crate::hardware::item(&format!("{track}.roller"))?;
            let pull = crate::hardware::item(&format!("{track}.pull"))?;
            Ok::<_, String>((
                (top, bottom, roller, pull),
                top.param("height")?,
                bottom.param("height")?,
                top.param("plane_gap")?,
            ))
        })();
        let ((top, bottom, roller, pull), over, under, plane_gap) = match numbers {
            Ok(n) => n,
            Err(e) => {
                self.list.warnings.push(format!("sliding doors: {e}"));
                return;
            }
        };
        let n = count.max(2) as f64;
        let w = ((x1 - x0) + (n - 1.0) * overlap) / n;
        if w < MIN_SLIDING_DOOR {
            self.list.warnings.push(format!(
                "{n} sliding doors {w:.0} wide are too narrow: left out"
            ));
            return;
        }
        let ft = y1 - y0;
        let (door_z0, door_z1) = (z0 + under, z1 - over);
        let mut names = Vec::new();
        for i in 0..count.max(2) as usize {
            let x = x0 + i as f64 * (w - overlap);
            let plane = if i % 2 == 1 { ft + plane_gap } else { 0.0 };
            counts.doors += 1;
            let b = aabb(x, y0 - plane, door_z0, x + w, y1 - plane, door_z1);
            self.add(
                PartRole::Door,
                Some(counts.doors),
                b,
                Axis::Y,
                Axis::Z,
                EdgeBanding::all(),
            );
            let name = self.name(PartRole::Door, Some(counts.doors));
            self.sliding_fronts.insert(name.clone());
            names.push((name, b));
        }
        let mid = (x0 + x1) / 2.0;
        let bottom_panel = self.name(PartRole::Bottom, None);
        for (item, z) in [(top, z1 - over / 2.0), (bottom, z0 + under / 2.0)] {
            self.list.hardware.push(HardwarePlacement {
                item: item.id.clone(),
                description: format!("{} {}; cut to {:.0} mm", item.family, item.variant, x1 - x0),
                at: Vec3::new(mid, (y1 + y0 - ft - plane_gap) / 2.0, z),
                part: bottom_panel.clone(),
                quantity: 1,
            });
        }
        for (name, b) in names {
            for item in [roller, pull] {
                self.list.hardware.push(HardwarePlacement {
                    item: item.id.clone(),
                    description: format!("{} {}", item.family, item.variant),
                    at: b.center(),
                    part: name.clone(),
                    quantity: 1,
                });
            }
        }
    }

    /// A fixed shelf across `bay` with its top at `z + t`, jointed to the panels either side.
    fn fixed_shelf(
        &mut self,
        n: usize,
        bay: &Bay,
        z: f64,
        left: Option<Part>,
        right: Option<Part>,
    ) {
        let t = self.cab.standard.carcass_thickness;
        self.add(
            PartRole::Shelf,
            Some(n),
            aabb(bay.left, 0.0, z, bay.right, self.interior_back, z + t),
            Axis::Z,
            Axis::X,
            EdgeBanding::front_only(),
        );
        let shelf = self.list.parts.last().cloned().unwrap();
        if let Some(l) = left {
            self.joint_parts(&l, &shelf, Dir::new(Axis::X, true), Axis::Y);
        }
        if let Some(r) = right {
            self.joint_parts(&r, &shelf, Dir::new(Axis::X, false), Axis::Y);
        }
    }

    /// Drawer fronts `first`, `first + 1`... top to bottom in proportion to `ratios`, filling the
    /// fronts' height range `fz` with gaps between, and their boxes: each drawer's runners stand
    /// where its front's bottom edge meets the carcass the way the lowest one meets `inside.0`
    /// (the panel under it); a drawer has up to the next drawer's datum (or `inside.1`, the
    /// underside of the panel above) for its box. Returns how many it made.
    fn drawer_fronts(
        &mut self,
        first: usize,
        ratios: &[f64],
        (x0, x1, y0, y1): (f64, f64, f64, f64),
        (z0, z1): (f64, f64),
        (floor, ceiling): (f64, f64),
    ) -> usize {
        let g = self.cab.standard.row_gap();
        let ratios: Vec<f64> = ratios.iter().copied().filter(|r| *r > 0.0).collect();
        if ratios.is_empty() {
            self.list
                .warnings
                .push("drawer layout has no drawers".into());
            return 0;
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
                Some(first + i),
                b,
                Axis::Y,
                Axis::X,
                EdgeBanding::all(),
            );
            fronts.push(b);
            top -= hgt + g;
        }
        // Blum's session: the lowest drawer stands on the floor, and one above another on the level
        // of its own front's bottom edge (its front holes 51 up, against the lowest's 67).
        let lift = floor - z0;
        let datums: Vec<f64> = fronts
            .iter()
            .enumerate()
            .map(|(i, b)| {
                if i + 1 == fronts.len() {
                    b.min.z + lift
                } else {
                    b.min.z
                }
            })
            .collect();
        for (i, b) in fronts.iter().enumerate() {
            let above = if i == 0 { ceiling } else { datums[i - 1] };
            // A frame's rail comes down behind the front, past any bevel, and the box stays under
            // it.
            let framed = self.drawer_frame_thickness() > 0.0;
            let opening_top = b.max().z - self.cab.standard.drawer_frame_overlap;
            let space = if framed {
                (above - datums[i]).min(opening_top - datums[i])
            } else {
                above - datums[i]
            };
            self.drawer_box(first + i, *b, datums[i], space);
            self.frame_opening(datums[i], opening_top.min(above));
        }
        fronts.len()
    }

    /// A stacked front: sections bottom to top, each over the fronts' width, with a fixed shelf
    /// between one section and the next. A niche's height is its clear opening; other sections'
    /// heights are their fronts'; one section of height 0 takes what is left.
    fn stack_fronts(
        &mut self,
        sections: &[StackSection],
        (x0, x1, y0, y1): (f64, f64, f64, f64),
        (z0, z1): (f64, f64),
    ) {
        let cab = self.cab;
        let s = &cab.standard;
        let (g, t) = (s.row_gap(), s.carcass_thickness);
        if sections.is_empty() {
            return;
        }
        // A section's share of the front height: a niche's clear opening plus the shelf under
        // it, less the gap the fronts would have.
        let zone = |sec: &StackSection| match sec.kind {
            SectionKind::Niche if sec.height > 0.0 => sec.height + t - g,
            _ => sec.height,
        };
        let fixed: f64 = sections.iter().map(zone).sum();
        let gaps = g * (sections.len() - 1) as f64;
        let rest = (z1 - z0) - gaps - fixed;
        if rest < -1e-6 {
            self.list.warnings.push(format!(
                "the stacked sections need {:.0} mm more height than the cabinet has",
                -rest
            ));
            return;
        }
        // Sections of height 0 share what is left; with none, the top one stretches.
        let free = sections.iter().filter(|x| x.height <= 0.0).count();
        let mut heights: Vec<f64> = sections.iter().map(zone).collect();
        if free > 0 {
            for (h, sec) in heights.iter_mut().zip(sections) {
                if sec.height <= 0.0 {
                    *h = rest / free as f64;
                }
            }
        } else {
            *heights.last_mut().unwrap() += rest;
        }
        let lift = self.opening_z.0 - z0;
        let (left, right) = match (
            self.part(PartRole::LeftSide),
            self.part(PartRole::RightSide),
        ) {
            (Some(l), Some(r)) => (l, r),
            _ => return,
        };
        let mut fz0 = z0;
        let mut floor = self.opening_z.0;
        let (mut doors, mut drawers, mut shelves) = (0, 0, 0);
        for (i, (sec, h)) in sections.iter().zip(&heights).enumerate() {
            let fz1 = fz0 + h;
            let last = i + 1 == sections.len();
            // The shelf over this section: its top where the next section's fronts start, as
            // the bottom panel sits under the lowest front.
            let ceiling = if last {
                self.opening_z.1
            } else {
                fz1 + g + lift - t
            };
            match &sec.kind {
                SectionKind::Drawers { count } => {
                    let ratios = vec![1.0; *count as usize];
                    drawers += self.drawer_fronts(
                        drawers + 1,
                        &ratios,
                        (x0, x1, y0, y1),
                        (fz0, fz1),
                        (floor, ceiling),
                    );
                }
                SectionKind::Door { hinge_side } => {
                    doors += 1;
                    self.door(doors, aabb(x0, y0, fz0, x1, y1, fz1), *hinge_side);
                }
                SectionKind::DoorPair => {
                    let mid = (x0 + x1) / 2.0;
                    self.door(
                        doors + 1,
                        aabb(x0, y0, fz0, mid - g / 2.0, y1, fz1),
                        HingeSide::Left,
                    );
                    self.door(
                        doors + 2,
                        aabb(mid + g / 2.0, y0, fz0, x1, y1, fz1),
                        HingeSide::Right,
                    );
                    doors += 2;
                }
                SectionKind::Niche => {}
            }
            if !last {
                shelves += 1;
                let bounds = aabb(
                    t,
                    0.0,
                    ceiling,
                    cab.width - t,
                    self.interior_back,
                    ceiling + t,
                );
                self.add(
                    PartRole::Shelf,
                    Some(shelves),
                    bounds,
                    Axis::Z,
                    Axis::X,
                    EdgeBanding::front_only(),
                );
                let shelf = self.list.parts.last().cloned().unwrap();
                self.joint_parts(&left, &shelf, Dir::new(Axis::X, true), Axis::Y);
                self.joint_parts(&right, &shelf, Dir::new(Axis::X, false), Axis::Y);
                floor = ceiling + t;
            }
            fz0 = fz1 + g;
        }
    }

    /// The drawer system's box behind drawer front `n`: base and back parts, front fixing bores
    /// and the system's hardware. `datum` is the height its runners stand on.
    fn drawer_box(&mut self, n: usize, front: Aabb, datum: f64, space: f64) {
        let system = self.system_now.unwrap_or(self.cab.standard.drawer_system);
        let runner = match system {
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
        let front_name = self.name(PartRole::DrawerFront, Some(n));
        let bay = self.bay();
        let fy = front.max().y;
        let b = match drawers::wood(
            runner,
            bay.right - bay.left,
            self.interior_back - fy,
            space,
            s.drawer_board_thickness,
            self.opening_now() == Opening::TipOn,
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
            height: String::new(),
            front_top: front.max().z,
        });
        self.profile_screws(b.runner, &bay, fy, datum);
        let (st, mid) = (b.board, (bay.left + bay.right) / 2.0);
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
        // TANDEM's rear hooks need wood below the base: its back runs down behind the base.
        let hook = (|| {
            let p = |k: &str| b.system.param(k);
            Some((
                p("hook_hole_diameter").ok()?,
                p("hook_hole_depth").ok()?,
                p("hook_hole_above_side_bottom").ok()?,
                p("hook_hole_from_side_inside").ok()?,
                p("back_above_side_bottom").ok()?,
            ))
        })();
        let (base_back, back_bottom) = match hook {
            Some((.., above)) => (y1 - st, z0 + above),
            None => (y1, base_z + st),
        };
        self.add_drawer_part(
            PartRole::DrawerBottom,
            n,
            aabb(x0, y0, base_z, x1, base_back, base_z + st),
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
            aabb(x0, y1 - st, back_bottom, x1, y1, top),
            Axis::Y,
        );
        if let Some((diameter, depth, above, inset, _)) = hook {
            let back = self.name(PartRole::DrawerBack, Some(n));
            for x in [x0 + inset, x1 - inset] {
                self.list.operations.push(Operation::Bore {
                    part: back.clone(),
                    at: Vec3::new(x, y1, z0 + above),
                    direction: Dir::new(Axis::Y, false),
                    diameter,
                    depth,
                    purpose: BorePurpose::RunnerHook,
                });
            }
        }
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
        let front_name = self.name(PartRole::DrawerFront, Some(n));
        let bay = self.bay();
        let inner_width = bay.right - bay.left;
        let fy = front.max().y;
        let range = match cab.standard.drawer_sides {
            DrawerSides::Standard => BoxRange::Drawers,
            DrawerSides::HighFronted => BoxRange::HighFronted,
        };
        let servo = self.opening_now() == Opening::ServoDrive;
        let depth = self.interior_back - fy;
        let pick = self.box_pick_now;
        let found = match drawers::picked_box(kind, range, inner_width, depth, space, servo, pick) {
            Ok(lb) => Ok(lb),
            Err(e) if pick != drawers::BoxPick::default() => {
                self.list
                    .warnings
                    .push(format!("{front_name}: {e}; the box that fits is used"));
                drawers::steel_box(kind, range, inner_width, depth, space, servo)
            }
            Err(e) => Err(e),
        };
        let lb = match found {
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
            heavy_load: matches!(lb.height.as_str(), "K" | "C" | "F"),
            front_height: front.size.z,
            height: lb.height.clone(),
            front_top: front.max().z,
        });
        self.steel_box_parts(n, &lb, fy, datum + base_lift);
        self.profile_screws(lb.profile, &bay, fy, datum);
        // Front fixing brackets: their bores in the back of the front, in from each side.
        for x in [bay.left + hole_inset, bay.right - hole_inset] {
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
        self.steel_box_hardware(&lb, front.center(), &front_name);
    }

    /// The cabinet profile's screw holes in the panels either side of the bay, each from the
    /// front's back face and above the drawer's datum. A hole the moved-back side no longer
    /// reaches is left out: Blum lists the first one as optional.
    fn profile_screws(&mut self, runner: &HardwareItem, bay: &Bay, fy: f64, datum: f64) {
        let t = self.cab.standard.carcass_thickness;
        // A divider is drilled from both sides: each hole stops short of the middle.
        let sides = [
            (bay.left_panel.clone(), bay.left, false, bay.left_divider),
            (bay.right_panel.clone(), bay.right, true, bay.right_divider),
        ];
        for (part, x, positive, divider) in sides {
            for hole in &runner.drilling {
                let y = fy + hole.x;
                if y < MIN_SCREW_FROM_EDGE {
                    continue;
                }
                let depth = if divider {
                    hole.depth.min(t / 2.0 - 0.5)
                } else {
                    hole.depth
                };
                self.list.operations.push(Operation::Bore {
                    part: part.clone(),
                    at: Vec3::new(x, y, datum + hole.y),
                    direction: Dir::new(Axis::X, positive),
                    diameter: hole.diameter,
                    depth,
                    purpose: BorePurpose::RunnerScrew,
                });
            }
        }
    }

    /// A steel box's base and back, placed for the 3D view and the cut list (the sizes are
    /// Blum's), the base's underside at `z`, its front edge at `fy`.
    fn steel_box_parts(&mut self, n: usize, lb: &SteelBox, fy: f64, z: f64) {
        let bt = lb.board_thickness;
        let (bw, bl) = lb.base;
        let (kw, kh) = lb.back;
        let bay = self.bay();
        let mid = (bay.left + bay.right) / 2.0;
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
    }

    fn steel_box_hardware(&mut self, lb: &SteelBox, at: Vec3, part: &str) {
        let mut items = vec![(lb.profile, 1), (lb.sides, 1), (lb.back_fixing, 1)];
        items.extend(lb.front_fixings.iter().copied());
        for (item, quantity) in items {
            self.list.hardware.push(HardwarePlacement {
                item: item.id.clone(),
                description: format!("{} {}", item.family, item.variant),
                at,
                part: part.to_string(),
                quantity,
            });
        }
        if let Some((piece, length)) = lb.front_piece {
            self.list.hardware.push(HardwarePlacement {
                item: piece.id.clone(),
                description: format!("{} {}; cut to {length:.0} mm", piece.family, piece.variant),
                at,
                part: part.to_string(),
                quantity: 1,
            });
        }
    }

    /// A wardrobe's fixed hat shelf and hanging rails (one, or two for double hanging), each rail
    /// cut to the inside width with an end support each side.
    fn wardrobe(&mut self) {
        let cab = self.cab;
        let s = &cab.standard;
        let (w, h, t) = (cab.width, cab.height, s.carcass_thickness);
        let (hat_drop, rail_drop, lower_shelf, pitch) = (
            s.wardrobe_hat_shelf_drop,
            s.wardrobe_rail_drop,
            s.wardrobe_double_hang_shelf,
            s.wardrobe_drawer_pitch,
        );
        let hat = h - hat_drop - t;
        let mut shelves = vec![hat];
        // Inner drawers at the foot, under a fixed shelf: the hanging above it.
        let mut drawers_top = None;
        if cab.inner_drawers > 0 {
            let z0 = self.opening_z.0;
            let top = z0 + cab.inner_drawers as f64 * pitch;
            self.inner_drawers_in(z0, top);
            drawers_top = Some(top);
        }
        if cab.wardrobe == Hanging::Double {
            if drawers_top.is_some_and(|top| top + t > lower_shelf - rail_drop - MIN_LOWER_HANGING)
            {
                self.list
                    .warnings
                    .push("the drawers leave no room for a lower rail; long hanging only".into());
            } else if lower_shelf + t > hat - rail_drop - MIN_LOWER_HANGING {
                self.list.warnings.push(format!(
                    "a lower shelf at {lower_shelf:.0} mm leaves no room for the upper rail; long hanging only"
                ));
            } else {
                shelves.push(lower_shelf);
            }
        }
        let (left, right) = match (
            self.part(PartRole::LeftSide),
            self.part(PartRole::RightSide),
        ) {
            (Some(l), Some(r)) => (l, r),
            _ => return,
        };
        for (i, &z) in shelves.iter().enumerate() {
            let bounds = aabb(t, self.shelf_setback(), z, w - t, self.interior_back, z + t);
            self.add(
                PartRole::Shelf,
                Some(i + 1),
                bounds,
                Axis::Z,
                Axis::X,
                EdgeBanding::front_only(),
            );
            let shelf = self.list.parts.last().cloned().unwrap();
            self.joint_parts(&left, &shelf, Dir::new(Axis::X, true), Axis::Y);
            self.joint_parts(&right, &shelf, Dir::new(Axis::X, false), Axis::Y);
            let rail_z = z - rail_drop;
            let length = w - 2.0 * t - 2.0;
            let at = Vec3::new(w / 2.0, self.interior_back / 2.0, rail_z);
            for (item, description, quantity) in [
                (
                    "wardrobe-rail.oval",
                    format!("Wardrobe rail, cut to {length:.0} mm"),
                    1,
                ),
                (
                    "wardrobe-rail.end-support",
                    "Wardrobe rail end support".to_string(),
                    2,
                ),
            ] {
                self.list.hardware.push(HardwarePlacement {
                    item: item.into(),
                    description,
                    at,
                    part: shelf.name.clone(),
                    quantity,
                });
            }
        }
        if let Some(top) = drawers_top {
            let n = shelves.len() + 1;
            let bounds = aabb(
                t,
                self.shelf_setback(),
                top,
                w - t,
                self.interior_back,
                top + t,
            );
            self.add(
                PartRole::Shelf,
                Some(n),
                bounds,
                Axis::Z,
                Axis::X,
                EdgeBanding::front_only(),
            );
            let shelf = self.list.parts.last().cloned().unwrap();
            self.joint_parts(&left, &shelf, Dir::new(Axis::X, true), Axis::Y);
            self.joint_parts(&right, &shelf, Dir::new(Axis::X, false), Axis::Y);
        }
    }

    /// Blum LEGRABOX inner drawers behind the doors: the opening shared evenly from the bottom,
    /// each the tallest inner drawer (K, else M) its share fits. They have no drawer front, so
    /// the hardware goes on each one's base.
    fn inner_drawers(&mut self) {
        let (z0, z1) = self.opening_z;
        self.inner_drawers_in(z0, z1);
    }

    /// `inner_drawers` sharing the height from `z0` to `z1`.
    fn inner_drawers_in(&mut self, z0: f64, z1: f64) {
        let cab = self.cab;
        let s = &cab.standard;
        let n = cab.inner_drawers as usize;
        match &cab.fronts {
            FrontLayout::Door { .. } | FrontLayout::DoorPair | FrontLayout::Open => {}
            _ => {
                self.list
                    .warnings
                    .push("inner drawers go behind doors; left out with these fronts".into());
                return;
            }
        }
        if s.drawer_system != DrawerSystem::Legrabox {
            self.list
                .warnings
                .push("inner drawers are LEGRABOX only; left out".into());
            return;
        }
        self.inner_drawers_from(1, n, z0, z1);
    }

    /// `n` LEGRABOX inner drawers from `z0` up to `z1` in the bay, numbered from `first`.
    /// Returns how many it made.
    fn inner_drawers_from(&mut self, first: usize, n: usize, z0: f64, z1: f64) -> usize {
        let s = &self.cab.standard;
        if n == 0 {
            return 0;
        }
        if s.drawer_system != DrawerSystem::Legrabox {
            self.list
                .warnings
                .push("inner drawers are LEGRABOX only; left out".into());
            return 0;
        }
        let bay = self.bay();
        let pitch = (z1 - z0) / n as f64;
        // Inset doors fill the front of the opening; the boxes start behind them.
        let fy = if s.overlay == DoorOverlay::Inset {
            s.front_thickness
        } else {
            0.0
        };
        for i in 0..n {
            let datum = z0 + pitch * i as f64;
            let lb = match drawers::steel_box(
                BoxSystem::Legrabox,
                BoxRange::Inner,
                bay.right - bay.left,
                self.interior_back - fy,
                pitch,
                false,
            ) {
                Ok(lb) => lb,
                Err(e) => {
                    self.list
                        .warnings
                        .push(format!("inner drawer {}: {e}; left out", i + 1));
                    return i;
                }
            };
            let base_lift = match lb.system.param("base_above_datum") {
                Ok(v) => v,
                Err(e) => {
                    self.list.warnings.push(e);
                    return i;
                }
            };
            self.steel_box_parts(first + i, &lb, fy, datum + base_lift);
            let base = self.list.parts[self.list.parts.len() - 2].clone();
            self.steel_box_hardware(&lb, base.bounds.center(), &base.name);
        }
        n
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
                    tip_on: s.opening != Opening::Handles,
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
        for (item, quantity) in lift.parts {
            self.list.hardware.push(HardwarePlacement {
                item: item.id.clone(),
                description: format!("{} {}", item.family, item.variant),
                at: front.center(),
                part: name.clone(),
                quantity,
            });
        }
        if kind == LiftKind::IntegratedStayLift {
            self.hki_recesses();
        }
        self.lift_drilling(kind, front, &name);
        if kind == LiftKind::HingedStayLift {
            self.top_hinges(front, &name);
        }
    }

    /// AVENTOS HK-XS hangs its front on ordinary hinges along the top: cups in the front, plates
    /// under the top panel, their count from the lift's parts, spaced as a door's along its edge.
    fn top_hinges(&mut self, front: Aabb, name: &str) {
        let s = &self.cab.standard;
        let count = self
            .list
            .hardware
            .iter()
            .filter(|h| h.part == name && h.item.starts_with("blum.clip-top"))
            .map(|h| h.quantity)
            .sum::<u32>() as usize;
        let need = HingeNeed {
            overlay: DoorOverlay::Full,
            front_overlay: front.max().z - self.opening_z.1,
            door_thickness: front.size.y,
            side_thickness: s.carcass_thickness,
            gap: s.front_gap,
            sprung: true,
            boss: s.hinge_boss,
        };
        let numbers = hinges::select(&need).and_then(|choice| {
            Ok::<_, String>((
                choice.cup_inset()?,
                choice.hinge.param("cup_diameter")?,
                choice.hinge.param("cup_depth")?,
                choice.plate.param("setback")?,
                crate::hardware::item(HINGE_POSITION)?.param("end_distance")?,
            ))
        });
        let (inset, diameter, depth, setback, ends) = match numbers {
            Ok(n) => n,
            Err(e) => {
                self.list
                    .warnings
                    .push(format!("{name}: {e}; hinges not drilled"));
                return;
            }
        };
        let Some(over) = self
            .list
            .parts
            .iter()
            .find(|p| matches!(p.role, PartRole::Top | PartRole::FrontRail))
            .map(|p| p.name.clone())
        else {
            return;
        };
        if count < 2 {
            return;
        }
        let span = front.size.x - 2.0 * ends;
        let underside = self.opening_z.1;
        for k in 0..count {
            let x = front.min.x + ends + span * k as f64 / (count - 1) as f64;
            self.list.operations.push(Operation::Bore {
                part: name.to_string(),
                at: Vec3::new(x, front.max().y, front.max().z - inset),
                direction: Dir::new(Axis::Y, false),
                diameter,
                depth,
                purpose: BorePurpose::HingeCup,
            });
            for along in [-PLATE_HOLE_OFFSET, PLATE_HOLE_OFFSET] {
                self.list.operations.push(Operation::Bore {
                    part: over.clone(),
                    at: Vec3::new(x + along, setback, underside),
                    direction: Dir::new(Axis::Z, true),
                    diameter: PLATE_HOLE_DIAMETER,
                    depth: PLATE_HOLE_DEPTH,
                    purpose: BorePurpose::HingePlate,
                });
            }
        }
    }

    /// A stay lift's mechanism: its holes in the sides, and its bracket's pilots in the back of
    /// the front, from the system's catalog row.
    fn lift_drilling(&mut self, kind: LiftKind, front: Aabb, name: &str) {
        let id = match kind {
            LiftKind::StayLift => "blum.aventos.hk-top",
            LiftKind::SmallStayLift => "blum.aventos.hk-s",
            LiftKind::HingedStayLift => "blum.aventos.hk-xs",
            LiftKind::UpAndOver => "blum.aventos.hs-top",
            LiftKind::LiftUp => "blum.aventos.hl-top",
            _ => return,
        };
        let Ok(sys) = crate::hardware::item(id) else {
            return;
        };
        let t = self.cab.standard.carcass_thickness;
        let param = |k: &str| sys.parameters.get(k).copied();
        // From the opening's top, or the cabinet's plus the top panel's thickness.
        let top = if param("side_hole_from_cabinet_top").is_some() {
            self.cab.height
        } else if param("side_hole_adds_top_thickness").is_some() {
            self.cab.height - t
        } else {
            self.opening_z.1
        };
        let bay = self.bay();
        let holes: Vec<_> = sys
            .drilling
            .iter()
            .map(|h| (h.x, top - h.y, h.diameter, h.depth))
            .collect();
        self.side_holes(&bay, &holes);
        // HL's front brackets sit by its lever arm's size.
        let arm_below = self
            .list
            .hardware
            .iter()
            .filter(|h| h.item.contains(".lever-arm."))
            .find_map(|h| {
                crate::hardware::item(&h.item)
                    .ok()?
                    .param("front_hole_below_top")
                    .ok()
            });
        let (Some(below), Some(pitch), Some(count), Some(inset)) = (
            param("front_hole_below_top").or(arm_below),
            param("front_hole_pitch"),
            param("front_hole_count"),
            param("front_hole_inset"),
        ) else {
            return;
        };
        let overlay = front.max().z - self.opening_z.1;
        let gap = self.opening_z.1 + t - front.max().z;
        let first = below
            + if param("front_hole_adds_overlay").is_some() {
                overlay
            } else if param("front_hole_less_gap").is_some() {
                -gap
            } else {
                0.0
            };
        for x in [bay.left + inset, bay.right - inset] {
            for k in 0..count as usize {
                self.list.operations.push(Operation::Bore {
                    part: name.to_string(),
                    at: Vec3::new(x, front.max().y, front.max().z - first - k as f64 * pitch),
                    direction: Dir::new(Axis::Y, false),
                    diameter: LIFT_BRACKET_PILOT,
                    depth: LIFT_BRACKET_PILOT_DEPTH.min(front.size.y - 3.0),
                    purpose: BorePurpose::FrontFixing,
                });
            }
        }
    }

    /// Holes (from the front, height, diameter, depth) in the panels either side of a bay.
    fn side_holes(&mut self, bay: &Bay, holes: &[(f64, f64, f64, f64)]) {
        for (part, x, positive) in [
            (bay.left_panel.clone(), bay.left, false),
            (bay.right_panel.clone(), bay.right, true),
        ] {
            for &(from_front, z, diameter, depth) in holes {
                self.list.operations.push(Operation::Bore {
                    part: part.clone(),
                    at: Vec3::new(x, from_front, z),
                    direction: Dir::new(Axis::X, positive),
                    diameter,
                    depth,
                    purpose: BorePurpose::LiftMechanism,
                });
            }
        }
    }

    /// AVENTOS HF's mechanism: four holes in each side, at a height set by the cabinet's.
    fn hf_drilling(&mut self) {
        let Ok(sys) = crate::hardware::item("blum.aventos.hf-top") else {
            return;
        };
        let numbers = (|| {
            let p = |k: &str| sys.param(k);
            let below = if self.cab.height < p("mechanism_short_below_height")? {
                p("mechanism_hole_below_top_short")?
            } else {
                p("mechanism_hole_below_top")?
            };
            Ok::<_, String>((
                below,
                p("mechanism_hole_front_setback")?,
                p("mechanism_hole_pitch_back")?,
                p("mechanism_hole_pitch_down")?,
                p("mechanism_hole_diameter")?,
                p("mechanism_hole_depth_min")?,
            ))
        })();
        let (below, setback, back, down, diameter, depth) = match numbers {
            Ok(n) => n,
            Err(e) => {
                self.list.warnings.push(e);
                return;
            }
        };
        let z = self.cab.height - below;
        let holes: Vec<_> = [z, z - down]
            .into_iter()
            .flat_map(|z| [setback, setback + back].map(|y| (y, z, diameter, depth)))
            .collect();
        let bay = self.bay();
        self.side_holes(&bay, &holes);
    }

    /// AVENTOS HKi's pockets: one in each side's inside face, open at the front edge, below the
    /// cabinet top (p.15). Cut as a box: the R25 at the pocket's back lower corner is left to the
    /// router.
    fn hki_recesses(&mut self) {
        let cab = self.cab;
        let t = cab.standard.carcass_thickness;
        let numbers = (|| {
            let sys = crate::hardware::item("blum.aventos.hki")?;
            let p = |k: &str| sys.param(k);
            Ok::<_, String>((
                p("recess_depth")?,
                p("recess_height")?,
                p("recess_length")?,
                p("recess_below_top")?,
                p("side_thickness_min")?..=p("side_thickness_max")?,
            ))
        })();
        let (depth, height, length, below, sides) = match numbers {
            Ok(n) => n,
            Err(e) => {
                self.list.warnings.push(e);
                return;
            }
        };
        if !sides.contains(&t) {
            self.list.warnings.push(format!(
                "AVENTOS HKi needs sides {}–{} mm thick, not {t}",
                sides.start(),
                sides.end()
            ));
            return;
        }
        let z1 = cab.height - below;
        for (role, x0) in [
            (PartRole::LeftSide, t - depth),
            (PartRole::RightSide, cab.width - t),
        ] {
            self.list.operations.push(Operation::Groove {
                part: self.name(role, None),
                bounds: aabb(x0, 0.0, z1 - height, x0 + depth, length, z1),
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
            // On an inset front the cup would fall above its top edge: nothing to drill.
            if tb >= 0.0 {
                cups.push((upper_name.clone(), top - tb - r, hinge_cup));
            }
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
        self.hf_drilling();
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

    fn door(&mut self, n: usize, mut b: Aabb, hinge: HingeSide) {
        let s = &self.cab.standard;
        // Gripped at the opening side: the finger gap comes off that edge, half each for a pair.
        if s.is_sharknose() && s.sharknose_door_edge == SharknoseEdge::Opening {
            let bay = self.bay();
            let free = if hinge == HingeSide::Left {
                b.max().x
            } else {
                b.min.x
            };
            let at_a_side = free > bay.right - s.carcass_thickness - s.front_gap
                || free < bay.left + s.carcass_thickness + s.front_gap;
            let extra = (s.sharknose_gap - s.front_gap).max(0.0);
            let trim = if at_a_side { extra } else { extra / 2.0 };
            b.size.x -= trim;
            if hinge != HingeSide::Left {
                b.min.x += trim;
            }
        }
        self.add(
            PartRole::Door,
            Some(n),
            b,
            Axis::Y,
            Axis::Z,
            EdgeBanding::all(),
        );
        let door = self.name(PartRole::Door, Some(n));
        // Blum's 0-protrusion hinge lets inner drawers pass the open door (p.84).
        let hinge_id =
            (self.cab.inner_drawers > 0 || self.inner_hinge).then_some(INNER_DRAWER_HINGE);
        if hinge_id.is_some() && self.cab.standard.overlay != DoorOverlay::Full {
            self.list.warnings.push(format!(
                "{door}: inner drawers need a 0-protrusion hinge; only the full overlay one is \
                 catalogued, so the standard hinge may foul them"
            ));
            self.hang_door(&door, b, hinge, None);
        } else {
            self.hang_door(&door, b, hinge, hinge_id);
        }
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
        let bay = self.bay();
        let (side, on_divider) = match hinge {
            HingeSide::Left => (bay.left_panel.clone(), bay.left_divider),
            HingeSide::Right => (bay.right_panel.clone(), bay.right_divider),
        };
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
            HingeSide::Left => bay.left - b.min.x,
            HingeSide::Right => b.max().x - bay.right,
        };
        // Two doors share a divider: each covers half of it.
        let overlay = if on_divider && s.overlay == DoorOverlay::Full {
            DoorOverlay::Half
        } else {
            s.overlay
        };
        let need = HingeNeed {
            overlay,
            front_overlay,
            door_thickness: b.size.y,
            side_thickness: t,
            gap: s.front_gap,
            sprung: self.opening_now() == Opening::Handles,
            boss: s.hinge_boss,
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
                crate::hardware::item(HINGE_POSITION)?.param("end_distance")?,
            ))
        })();
        let Ok((cup_inset, cup_diameter, cup_depth, pattern_setback, end_distance)) = numbers
        else {
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
            HingeSide::Left => (bay.left, false),
            HingeSide::Right => (bay.right, true),
        };
        // The carcass front is Y = 0 whatever the overlay; the plate is set out from it.
        let plate_y = choice.plate_setback;
        // An arched top row comes down almost its full rise at the hinged edges: the top hinge
        // keeps its distance from there.
        let arched = cab.arched_top > 0.0 && (b.max().z - self.front_region().3).abs() < 1e-6;
        let top_end = end_distance + if arched { cab.arched_top } else { 0.0 };
        let span = height - end_distance - top_end;
        let holes: Vec<(f64, f64, f64)> = choice
            .plate
            .drilling
            .iter()
            .map(|h| (plate_y + h.y - pattern_setback, h.x, h.diameter / 2.0))
            .collect();
        // Blum's 37/32 system: a middle hinge's plate screws go in the shelf-pin rows' holes,
        // so its holes land on the 32 mm grid the rows are drilled on.
        let pitch = choice.plate.param("hole_pitch").ok();
        let grid = |z: f64| match pitch {
            Some(p) if (pattern_setback - SYSTEM32_SETBACK).abs() < 1e-9 => {
                let g0 = self.opening_z.0 + p / 2.0;
                g0 + ((z - g0) / SYSTEM32_PITCH).round() * SYSTEM32_PITCH
            }
            _ => z,
        };
        let zs: Vec<f64> = (0..count)
            .map(|i| {
                let z = b.min.z + end_distance + span * i as f64 / (count - 1) as f64;
                if i == 0 || i + 1 == count { z } else { grid(z) }
            })
            .map(|z| self.clear_of_joints(&side, z, &holes, (b.min.z, b.max().z)))
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
            // EXPANDO and INSERTA bosses: x along the door, y further from its edge than the cup.
            let away = match hinge {
                HingeSide::Left => 1.0,
                HingeSide::Right => -1.0,
            };
            for hole in choice.hinge.drilling.iter().filter(|h| h.x != 0.0) {
                self.list.operations.push(Operation::Bore {
                    part: door.clone(),
                    at: Vec3::new(cup_x + away * hole.y, back_y, z + hole.x),
                    direction: Dir::new(Axis::Y, false),
                    diameter: hole.diameter,
                    depth: hole.depth.min(b.size.y - 3.0),
                    purpose: BorePurpose::HingeBoss,
                });
            }
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
            if let Some(cap) = cover_cap(&choice.hinge.id) {
                self.list.hardware.push(HardwarePlacement {
                    item: cap.into(),
                    description: "CLIP top hinge arm cover cap".into(),
                    at: Vec3::new(inner_x, plate_y, z),
                    part: side.clone(),
                    quantity: 1,
                });
            }
        }
        zs
    }

    /// The height nearest `z` at which a mounting plate's `holes` (y, z offset, radius) clear the
    /// joints and bores already in `panel`: a fixed shelf or the next bay's door may be there.
    /// The hinge stays on its door, `HINGE_EDGE_CLEARANCE` in from either end.
    fn clear_of_joints(
        &self,
        panel: &str,
        z: f64,
        holes: &[(f64, f64, f64)],
        door: (f64, f64),
    ) -> f64 {
        let taken: Vec<(f64, f64, f64, f64)> = self
            .list
            .operations
            .iter()
            .filter_map(|op| match op {
                Operation::BiscuitSlot { parts, bounds, .. }
                    if parts.iter().any(|p| p == panel) =>
                {
                    Some((bounds.min.y, bounds.max().y, bounds.min.z, bounds.max().z))
                }
                Operation::Bore {
                    part, at, diameter, ..
                } if part == panel => {
                    let r = diameter / 2.0;
                    Some((at.y - r, at.y + r, at.z - r, at.z + r))
                }
                _ => None,
            })
            .collect();
        let clear = |z: f64| {
            holes.iter().all(|&(y, dz, r)| {
                let (y0, y1) = (y - r - JOINT_CLEARANCE, y + r + JOINT_CLEARANCE);
                let (z0, z1) = (z + dz - r - JOINT_CLEARANCE, z + dz + r + JOINT_CLEARANCE);
                taken
                    .iter()
                    .all(|&(ty0, ty1, tz0, tz1)| y1 <= ty0 || ty1 <= y0 || z1 <= tz0 || tz1 <= z0)
            })
        };
        let on_door =
            |z: f64| z >= door.0 + HINGE_EDGE_CLEARANCE && z <= door.1 - HINGE_EDGE_CLEARANCE;
        (0..=40)
            .flat_map(|i| [z - 5.0 * i as f64, z + 5.0 * i as f64])
            .find(|&z| on_door(z) && clear(z))
            .unwrap_or(z)
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

    /// A handle (or knob, or edge pull) on every door, drawer front and lift front, with its
    /// screw holes through the front. A door's handle goes by its free edge: the edge away from
    /// its hinge cups, or, for a door hung on another door (an L-corner's second door), away from
    /// that door. A door with cups by both edges (the first door of a bi-fold pair) has none.
    fn handles(&mut self) {
        let cab = self.cab;
        let s = &cab.standard;
        let fronts: Vec<Part> = self
            .list
            .parts
            .iter()
            .filter(|p| {
                matches!(
                    p.role,
                    PartRole::Door | PartRole::DrawerFront | PartRole::LiftFront
                )
            })
            .filter(|p| self.opening_of(&p.name) == Opening::Handles)
            .filter(|p| !self.sliding_fronts.contains(&p.name))
            .cloned()
            .collect();
        let wall_hung = matches!(
            cab.kind,
            CabinetKind::Wall | CabinetKind::CornerWall | CabinetKind::OpenShelf
        );
        if matches!(
            s.handle_place,
            HandlePlace::FingerPull | HandlePlace::Sharknose
        ) && s.door_style != DoorStyle::Slab
        {
            let grip = if s.handle_place == HandlePlace::Sharknose {
                "sharknose bevels need"
            } else {
                "finger pulls need"
            };
            self.list.warnings.push(format!(
                "{grip} slab fronts: a frame's grooves run where the grip would be; none cut"
            ));
        }
        // An ABI style the catalog does not have falls back to the generic handle, and says so,
        // rather than boring a size we invented.
        let chosen = match crate::hardware::chosen_handle(s) {
            Ok(c) => c,
            Err(e) => {
                self.list.warnings.push(e);
                crate::hardware::generic_handle(s)
            }
        };
        let edge_pull = s.edge_mounted_pull();
        let mut unfit = false;
        let span = chosen.centres.unwrap_or(0.0);
        // One screw (a knob, a T pull): the body lies across the front, level, whatever the front.
        let single = chosen.holes.len() == 1;
        let (below, above) = chosen.across;
        // How far the body runs past the outer screws along the screw line.
        let overhang = ((chosen.length - span) / 2.0).max(0.0);
        for f in &fronts {
            if let Some(pull) = edge_pull {
                self.fit_edge_pull(f, pull, &chosen.description, wall_hung);
                continue;
            }
            let b = f.bounds;
            let ta = f.thickness_axis;
            let wa = if ta == Axis::X { Axis::Y } else { Axis::X };
            let (w0, w1) = (b.min.get(wa), b.max().get(wa));
            let (z0, z1) = (b.min.z, b.max().z);
            // Holes are drilled from the face towards the cabinet's inside.
            let inside = Vec3::new(cab.width / 2.0, cab.depth / 2.0, 0.0).get(ta);
            let back_low = (b.min.get(ta) - inside).abs() < (b.max().get(ta) - inside).abs();
            let back = if back_low {
                b.min.get(ta)
            } else {
                b.max().get(ta)
            };
            let dir = Dir::new(ta, back_low);
            // Which way across the front a door's model's +Z (its straight edge) points.
            let mut toward_free = 1.0;
            // (position along the width, height) of each hole.
            let holes: Vec<(f64, f64)> = if f.role == PartRole::Door {
                let cups: Vec<f64> = self
                    .list
                    .operations
                    .iter()
                    .filter_map(|o| match o {
                        Operation::Bore {
                            part,
                            at,
                            purpose: BorePurpose::HingeCup,
                            ..
                        } if *part == f.name => Some(at.get(wa)),
                        _ => None,
                    })
                    .collect();
                let mid = (w0 + w1) / 2.0;
                let hinge_low = if cups.is_empty() {
                    // Hung on another door: its free edge is the one away from that door.
                    let other = fronts
                        .iter()
                        .filter(|o| o.name != f.name && o.role == PartRole::Door)
                        .map(|o| o.bounds.center().get(wa))
                        .next();
                    match other {
                        Some(c) => c < mid,
                        None => continue,
                    }
                } else if cups.iter().all(|&c| c < mid) {
                    true
                } else if cups.iter().all(|&c| c > mid) {
                    false
                } else {
                    continue;
                };
                // The body stays clear of the free edge and of the door's ends.
                let reach = if single { chosen.length / 2.0 } else { above };
                let edge_inset = HANDLE_EDGE_INSET.max(reach + HANDLE_SIDE_CLEARANCE);
                let x = if hinge_low {
                    w1 - edge_inset
                } else {
                    w0 + edge_inset
                };
                let end_reach = if single { above } else { overhang };
                let end_inset = HANDLE_END_INSET.max(end_reach + HANDLE_BODY_CLEARANCE);
                let (lo, hi) = match s.handle_place {
                    HandlePlace::Centre => {
                        let c = (z0 + z1) / 2.0;
                        (c - span / 2.0, c + span / 2.0)
                    }
                    _ if wall_hung => (z0 + end_inset, z0 + end_inset + span),
                    _ if matches!(cab.kind, CabinetKind::Tall | CabinetKind::Wardrobe) => {
                        // A pull longer than the door has no legal centre; the fit check below
                        // leaves it off. `clamp` panics when its bounds are reversed.
                        let low = z0 + end_inset + span / 2.0;
                        let high = z1 - end_inset - span / 2.0;
                        let c = if low <= high {
                            TALL_HANDLE_HEIGHT.clamp(low, high)
                        } else {
                            (z0 + z1) / 2.0
                        };
                        (c - span / 2.0, c + span / 2.0)
                    }
                    _ => (z1 - end_inset - span, z1 - end_inset),
                };
                if chosen.centres.is_some() {
                    let c = (lo + hi) / 2.0;
                    let mut holes: Vec<(f64, f64)> = chosen
                        .holes
                        .iter()
                        .map(|(along, across)| (x + across, c + along))
                        .collect();
                    // The model's frame follows the holes in order: down the door turns its
                    // straight edge to a left-hung door's free edge, as a pair meets at its gap.
                    if hinge_low {
                        holes.reverse();
                    } else {
                        toward_free = -1.0;
                    }
                    holes
                } else {
                    // A knob sits where the handle's end nearest the edge would.
                    let z = if wall_hung { lo } else { hi };
                    vec![(
                        x,
                        if s.handle_place == HandlePlace::Centre {
                            (lo + hi) / 2.0
                        } else {
                            z
                        },
                    )]
                }
            } else {
                let c = (w0 + w1) / 2.0;
                let z = match (f.role, s.handle_place) {
                    // Lift fronts are pulled up from the bottom.
                    (PartRole::LiftFront, _) => {
                        z0 + HANDLE_END_INSET.max(HANDLE_BODY_CLEARANCE - below)
                    }
                    (_, HandlePlace::Corner) if z1 - z0 > DEEP_FRONT => {
                        z1 - HANDLE_END_INSET.max(HANDLE_BODY_CLEARANCE + above)
                    }
                    // The body centred on the front, not its screw line.
                    _ => (z0 + z1) / 2.0 - (below + above) / 2.0,
                };
                chosen
                    .holes
                    .iter()
                    .map(|(along, across)| (c + along, z + across))
                    .collect()
            };
            // An ABI pull keeps its named centres. One that would break out of the front is left
            // off, once per cabinet, rather than bored past the edge.
            let mean = |v: &dyn Fn(&(f64, f64)) -> f64| {
                holes.iter().map(v).sum::<f64>() / holes.len().max(1) as f64
            };
            let (hw, hz) = (mean(&|h| h.0), mean(&|h| h.1));
            let half = chosen.length / 2.0;
            let body = if f.role == PartRole::Door && !single {
                let (a, b) = (hw + toward_free * below, hw + toward_free * above);
                ((a.min(b), a.max(b)), (hz - half, hz + half))
            } else {
                ((hw - half, hw + half), (hz + below, hz + above))
            };
            if chosen.abi && !handle_fits(f, &holes, wa, body) {
                if !unfit {
                    self.list.warnings.push(format!(
                        "{} does not fit {}; no handle bored",
                        chosen.description, f.name
                    ));
                    unfit = true;
                }
                continue;
            }
            if s.handle_place == HandlePlace::Sharknose {
                if s.door_style != DoorStyle::Slab {
                    continue;
                }
                let into = SHARKNOSE_INTO.min(f.thickness() - SHARKNOSE_NOSE);
                let down = into / s.sharknose_angle.degrees().to_radians().tan();
                // A door may be gripped on the side opposite its hinges instead.
                if f.role == PartRole::Door
                    && s.sharknose_door_edge == SharknoseEdge::Opening
                    && ta == Axis::Y
                {
                    let hinge = self
                        .list
                        .hardware
                        .iter()
                        .find(|h| h.part == f.name && h.item.starts_with("blum.clip-top"));
                    let right = hinge.is_none_or(|h| h.at.x < b.center().x);
                    let (xl, xh, xe) = if right {
                        (w1 - down, w1, w1)
                    } else {
                        (w0, w0 + down, w0)
                    };
                    let (t_lo, t_hi) = if back_low {
                        (back, back + into)
                    } else {
                        (back - into, back)
                    };
                    self.list.operations.push(Operation::Chamfer {
                        part: f.name.clone(),
                        at: Vec3::new(xe, back, (z0 + z1) / 2.0),
                        bounds: aabb(xl, t_lo, z0, xh, t_hi, z1),
                    });
                    continue;
                }
                // The gripping edge's back corner, bevelled across the front's width.
                let top = if f.role == PartRole::Door {
                    match s.sharknose_door_edge {
                        SharknoseEdge::Top => true,
                        SharknoseEdge::Bottom => false,
                        _ => !wall_hung,
                    }
                } else {
                    !(wall_hung && f.role != PartRole::DrawerFront)
                };
                let (zl, zh, ze) = if top {
                    (z1 - down, z1, z1)
                } else {
                    (z0, z0 + down, z0)
                };
                let (t_lo, t_hi) = if back_low {
                    (back, back + into)
                } else {
                    (back - into, back)
                };
                let lo = Vec3::new(0.0, 0.0, zl).with(wa, w0).with(ta, t_lo);
                let hi = Vec3::new(0.0, 0.0, zh).with(wa, w1).with(ta, t_hi);
                let at = Vec3::new(0.0, 0.0, ze)
                    .with(wa, (w0 + w1) / 2.0)
                    .with(ta, back);
                self.list.operations.push(Operation::Chamfer {
                    part: f.name.clone(),
                    at,
                    bounds: Aabb::new(lo, Vec3::new(hi.x - lo.x, hi.y - lo.y, hi.z - lo.z)),
                });
                continue;
            }
            if s.handle_place == HandlePlace::FingerPull {
                // A frame's grooves and tenons run where the grip would be cut.
                if s.door_style != DoorStyle::Slab {
                    continue;
                }
                // Along the back of the gripping edge, across the front's width.
                let (zl, zh) = if wall_hung && f.role != PartRole::DrawerFront {
                    (z0, z0 + s.finger_pull_height)
                } else {
                    (z1 - s.finger_pull_height, z1)
                };
                let (t_lo, t_hi) = if back_low {
                    (back, back + s.finger_pull_depth)
                } else {
                    (back - s.finger_pull_depth, back)
                };
                let lo = Vec3::new(0.0, 0.0, zl).with(wa, w0).with(ta, t_lo);
                let hi = Vec3::new(0.0, 0.0, zh).with(wa, w1).with(ta, t_hi);
                self.list.operations.push(Operation::Groove {
                    part: f.name.clone(),
                    bounds: Aabb::new(lo, Vec3::new(hi.x - lo.x, hi.y - lo.y, hi.z - lo.z)),
                });
                continue;
            }
            let (item, description) = if s.handle_place == HandlePlace::EdgePull {
                (
                    "handle.edge-pull".to_string(),
                    format!("Edge pull, {}", s.handle_finish.label()),
                )
            } else {
                (chosen.item.clone(), chosen.description.clone())
            };
            if s.handle_place != HandlePlace::EdgePull {
                for (w, z) in &holes {
                    let at = Vec3::new(0.0, 0.0, *z).with(wa, *w).with(ta, back);
                    self.list.operations.push(Operation::Bore {
                        part: f.name.clone(),
                        at,
                        direction: dir,
                        diameter: HANDLE_SCREW_DIAMETER,
                        depth: f.thickness(),
                        purpose: BorePurpose::HandleScrew,
                    });
                }
            }
            // An edge pull sits on the gripping edge: the top, or a wall door's bottom (as a
            // finger pull's groove).
            let at = if s.handle_place == HandlePlace::EdgePull {
                let z = if wall_hung && f.role != PartRole::DrawerFront {
                    z0
                } else {
                    z1
                };
                Vec3::new(b.center().x, b.center().y, z)
            } else {
                b.center()
            };
            self.list.hardware.push(HardwarePlacement {
                item,
                description,
                at,
                part: f.name.clone(),
                quantity: 1,
            });
        }
    }

    /// A pull over the front's gripping edge, centred on it: the top, or a wall door's bottom. A
    /// Rappana's leg sits in a rebate in the back of the front and screws into it; a Scalo's plate
    /// sits on the edge, flush with the back, and screws down into it (owner, 2026-11-02).
    fn fit_edge_pull(
        &mut self,
        f: &Part,
        pull: &'static HardwareItem,
        description: &str,
        wall_hung: bool,
    ) {
        let s = &self.cab.standard;
        let b = f.bounds;
        let numbers = (|| {
            Ok::<_, String>((
                pull.param("length")?,
                pull.param("edge_mount")?,
                pull.param("sheet_thickness")?,
                pull.param("hole_offset")?,
                pull.param("leg_height")?,
            ))
        })();
        let (length, mount, sheet, offset, leg) = match numbers {
            Ok(n) => n,
            Err(e) => {
                self.list.warnings.push(e);
                return;
            }
        };
        if s.door_style != DoorStyle::Slab || f.thickness_axis != Axis::Y {
            self.list.warnings.push(format!(
                "{description} needs a slab front in line with the cabinet: left off {}",
                f.name
            ));
            return;
        }
        if length > b.size.x {
            self.list
                .warnings
                .push(format!("{description} does not fit {}: left off", f.name));
            return;
        }
        let top = !(wall_hung && f.role != PartRole::DrawerFront);
        let edge = if top { b.max().z } else { b.min.z };
        // From the edge into the front.
        let inward = if top { -1.0 } else { 1.0 };
        let (mid, back) = (b.center().x, b.max().y);
        let thickness = f.thickness();
        if mount == EDGE_MOUNT_BACK_LEG {
            let (reach, x0, x1) = (
                leg - sheet + 1.0,
                mid - length / 2.0 - 1.0,
                mid + length / 2.0 + 1.0,
            );
            let (z0, z1) = if top {
                (edge - reach, edge)
            } else {
                (edge, edge + reach)
            };
            self.list.operations.push(Operation::Groove {
                part: f.name.clone(),
                bounds: aabb(x0, back - sheet, z0, x1, back, z1),
            });
            let screw_into_wood = EDGE_PULL_SCREW - sheet;
            if screw_into_wood > thickness - sheet - EDGE_PULL_SCREW_COVER {
                self.list.warnings.push(format!(
                    "{}: the 16 mm screws in the box would come too close to its {thickness:.0} mm \
                     face behind the rebate: use shorter ones",
                    f.name
                ));
            }
            for hole in &pull.drilling {
                self.list.operations.push(Operation::Bore {
                    part: f.name.clone(),
                    at: Vec3::new(mid + hole.x, back - sheet, edge + inward * (offset - sheet)),
                    direction: Dir::new(Axis::Y, false),
                    diameter: hole.diameter,
                    depth: hole.depth.min(thickness - sheet - EDGE_PULL_SCREW_COVER),
                    purpose: BorePurpose::EdgePullScrew,
                });
            }
        } else {
            if thickness < offset + EDGE_PULL_WOOD_BEHIND {
                self.list.warnings.push(format!(
                    "{}: its {thickness:.0} mm edge leaves too little wood round the screws of \
                     {description}",
                    f.name
                ));
            }
            for hole in &pull.drilling {
                self.list.operations.push(Operation::Bore {
                    part: f.name.clone(),
                    at: Vec3::new(mid + hole.x, back - offset, edge),
                    direction: Dir::new(Axis::Z, !top),
                    diameter: hole.diameter,
                    depth: hole.depth,
                    purpose: BorePurpose::EdgePullScrew,
                });
            }
        }
        self.list.hardware.push(HardwarePlacement {
            item: pull.id.clone(),
            description: description.to_string(),
            at: Vec3::new(mid, back, edge),
            part: f.name.clone(),
            quantity: 1,
        });
    }

    /// Shaker fronts: every door, lift front and drawer front big enough becomes two stiles
    /// (full height), two rails between them with stub tenons into the stiles' grooves, and a
    /// panel in the grooves, centred in the front's thickness. Bores and hardware on the slab move
    /// to the member they fall in.
    /// CABLOXX (p.594): a locking bar set for the cabinet's height, with a locking unit and a
    /// front locking bracket set per drawer; past 600 wide a second bar on the left, linked by a
    /// synchronisation set, as Blum recommends with a closing function (every Blum runner here
    /// has one).
    fn lock(&mut self) {
        let cab = self.cab;
        if !cab.lock {
            return;
        }
        let fronts: Vec<String> = self.drawer_fits.iter().map(|f| f.front.clone()).collect();
        if fronts.is_empty() {
            self.list
                .warnings
                .push("a CABLOXX lock needs drawers; this cabinet has none".into());
            return;
        }
        // One bar to a column: a bar only reaches the drawers in its bay.
        let mut columns: Vec<(f64, f64, Vec<String>)> = Vec::new();
        for front in fronts {
            let x = self
                .list
                .part(&front)
                .map_or(cab.width / 2.0, |p| p.bounds.center().x);
            let t = cab.standard.carcass_thickness;
            let (left, right) = self
                .list
                .bay_around(x)
                .map_or((t, cab.width - t), |(l, r)| {
                    (l.bounds.max().x, r.bounds.min.x)
                });
            match columns.iter_mut().find(|c| c.0 == left) {
                Some(c) => c.2.push(front),
                None => columns.push((left, right, vec![front])),
            }
        }
        for (left, right, fronts) in &columns {
            self.lock_column(*left, *right, fronts);
        }
        self.list.notes.push(
            "CABLOXX: fit a cylinder lock (Ø16.5–22, 28–48 long; not supplied by Blum); the \
             locking bar's groove and drilling are not machined yet"
                .into(),
        );
    }

    /// A CABLOXX bar and what goes with it, for the drawers of one bay.
    fn lock_column(&mut self, left: f64, right: f64, fronts: &[String]) {
        let cab = self.cab;
        // Blum's limit: up to five pull-outs on one locking bar.
        if fronts.len() > 5 {
            self.list.warnings.push(format!(
                "CABLOXX locks up to five pull-outs on one bar, not {}",
                fronts.len()
            ));
            return;
        }
        let kick = if cab.has_toe_kick() {
            cab.standard.toe_kick_height
        } else {
            0.0
        };
        let kh = cab.height - kick;
        let lh = kh - 2.0 * cab.standard.carcass_thickness;
        let lw = right - left;
        let Some(bar) = [600, 800, 1300]
            .into_iter()
            .find(|&h| kh <= h as f64 + 1e-6)
            .and_then(|h| crate::hardware::item(&format!("blum.cabloxx.bar-set.{h}")).ok())
        else {
            self.list.warnings.push(format!(
                "CABLOXX has no locking bar for a {kh:.0} mm high cabinet (up to 1300)"
            ));
            return;
        };
        // Blum's width limit is the cabinet's: a bay stands as wide as its opening and panels.
        let synced = lw + 2.0 * cab.standard.carcass_thickness > 600.0;
        let at = Vec3::new((left + right) / 2.0, 0.0, kick + kh / 2.0);
        let mut rows = vec![(bar, if synced { 2 } else { 1 }, String::new())];
        let unit = |side: &str| crate::hardware::item(&format!("blum.cabloxx.locking-unit.{side}"));
        let n = fronts.len() as u32;
        for side in if synced {
            &["right", "left"][..]
        } else {
            &["right"][..]
        } {
            if let Ok(u) = unit(side) {
                rows.push((u, n, String::new()));
            }
        }
        if let Ok(b) = crate::hardware::item("blum.cabloxx.front-bracket") {
            rows.push((b, n, String::new()));
        }
        if synced {
            for id in ["blum.cabloxx.sync-set", "blum.cabloxx.sync-linkage"] {
                if let Ok(i) = crate::hardware::item(id) {
                    let cut = if id.ends_with("linkage") {
                        format!(", cut to {:.0}", lw - 7.0)
                    } else {
                        String::new()
                    };
                    rows.push((i, 1, cut));
                }
            }
        }
        for (item, quantity, extra) in rows {
            let cut = if item.id.starts_with("blum.cabloxx.bar-set.") {
                format!(", bar cut to {:.0}", lh - 2.0)
            } else {
                extra
            };
            self.list.hardware.push(HardwarePlacement {
                item: item.id.clone(),
                description: format!("{} {}{cut}", item.family, item.variant),
                at,
                part: fronts[0].clone(),
                quantity,
            });
        }
    }

    /// An arched top: the doors whose tops are the highest of the cabinet's fronts get one arch
    /// across them, `arched_top` high at its middle over the ends. Refused on framed fronts, and
    /// where the arch would cut through a hole in a door (a hinge cup, a handle).
    fn arched_top(&mut self) {
        let rise = self.cab.arched_top;
        if rise <= 0.0 {
            return;
        }
        if self.cab.standard.door_style != DoorStyle::Slab {
            self.list
                .warnings
                .push("arched tops are cut in slab doors only: the top stays square".into());
            return;
        }
        let doors: Vec<Part> = self.list.by_role(PartRole::Door).cloned().collect();
        let Some(top) = doors.iter().map(|d| d.bounds.max().z).reduce(f64::max) else {
            self.list
                .warnings
                .push("an arched top needs doors; this cabinet has none".into());
            return;
        };
        let row: Vec<&Part> = doors
            .iter()
            .filter(|d| (d.bounds.max().z - top).abs() < 1e-6)
            .collect();
        let x0 = row.iter().map(|d| d.bounds.min.x).fold(f64::MAX, f64::min);
        let x1 = row
            .iter()
            .map(|d| d.bounds.max().x)
            .fold(f64::MIN, f64::max);
        let chord = x1 - x0;
        let lowest = row.iter().map(|d| d.bounds.size.z).fold(f64::MAX, f64::min);
        if rise > chord / 2.0 || rise > lowest / 2.0 {
            self.list.warnings.push(format!(
                "a {rise:.0} mm arch is too high for doors {chord:.0} wide: the top stays square"
            ));
            return;
        }
        // The circle through both ends, `rise` under the top, and the top at the middle.
        let radius = chord * chord / (8.0 * rise) + rise / 2.0;
        let (cx, cz) = ((x0 + x1) / 2.0, top - radius);
        let cut = |p: Vec3| p.z > cz && ((p.x - cx).powi(2) + (p.z - cz).powi(2)).sqrt() > radius;
        for d in &row {
            let hit = self.list.operations.iter().find_map(|o| match o {
                Operation::Bore {
                    part, at, diameter, ..
                } if *part == d.name => {
                    // The hole's edges nearest the arch.
                    let r = diameter / 2.0;
                    let up = Vec3::new(at.x, at.y, at.z + r);
                    let side = Vec3::new(at.x + if at.x < cx { -r } else { r }, at.y, at.z);
                    (cut(up) || cut(side)).then_some(*at)
                }
                _ => None,
            });
            if let Some(at) = hit {
                self.list.warnings.push(format!(
                    "{}: a {rise:.0} mm arch would cut through the hole {:.0} mm under its top: \
                     the top stays square",
                    d.name,
                    top - at.z
                ));
                return;
            }
        }
        for d in row {
            self.list.operations.push(Operation::Arch {
                part: d.name.clone(),
                centre: Vec3::new(cx, d.bounds.center().y, cz),
                radius,
            });
        }
    }

    /// Continuous grain: each column of two or more slab fronts (the same left and right edges)
    /// runs its grain upright and is cut as one strip, top first.
    fn grain_match(&mut self) {
        let s = &self.cab.standard;
        // A chosen direction, on every front that shows it.
        if matches!(s.front_grain, FrontGrain::Upright | FrontGrain::Across) {
            let upright = s.front_grain == FrontGrain::Upright;
            for p in &mut self.list.parts {
                if matches!(
                    p.role,
                    PartRole::Door | PartRole::DrawerFront | PartRole::LiftFront
                ) {
                    let across = if p.thickness_axis == Axis::X {
                        Axis::Y
                    } else {
                        Axis::X
                    };
                    p.grain_axis = Some(if upright { Axis::Z } else { across });
                }
            }
            return;
        }
        if s.front_grain != FrontGrain::Matched {
            return;
        }
        if s.door_style != DoorStyle::Slab {
            self.list.warnings.push(
                "continuous grain needs slab fronts: framed fronts are cut piece by piece".into(),
            );
            return;
        }
        let key = |p: &Part| {
            (
                (p.bounds.min.x * 10.0).round() as i64,
                (p.bounds.max().x * 10.0).round() as i64,
            )
        };
        let mut columns: std::collections::BTreeMap<(i64, i64), Vec<(f64, String)>> =
            Default::default();
        for p in self
            .list
            .parts
            .iter()
            .filter(|p| matches!(p.role, PartRole::Door | PartRole::DrawerFront))
        {
            columns
                .entry(key(p))
                .or_default()
                .push((p.bounds.max().z, p.name.clone()));
        }
        for mut column in columns.into_values().filter(|c| c.len() > 1) {
            column.sort_by(|a, b| b.0.total_cmp(&a.0));
            let names: Vec<String> = column.into_iter().map(|(_, n)| n).collect();
            for p in self
                .list
                .parts
                .iter_mut()
                .filter(|p| names.contains(&p.name))
            {
                p.grain_axis = Some(Axis::Z);
            }
            self.list.grain_matched.push(names);
        }
    }

    fn shaker_fronts(&mut self) {
        let s = &self.cab.standard;
        if s.door_style == DoorStyle::Slab {
            return;
        }
        let glass_style = s.door_style == DoorStyle::GlassFrame;
        let front_board = s.front_board.clone();
        let panel_board = s.shaker_panel_board.clone();
        let (f, p, g, play) = (
            s.shaker_frame_width,
            s.shaker_panel_thickness,
            s.shaker_groove_depth,
            SHAKER_PLAY,
        );
        let slab_below = s.shaker_slab_below;
        // The groove must leave frame each side of it, and the panel must leave face each side.
        if g <= 0.0 || f <= g || p <= 0.0 || p >= s.front_thickness - 4.0 {
            self.list.warnings.push(format!(
                "shaker sizes do not fit: a {f:.0} mm frame needs a groove under {f:.0} mm, and a \
                 {:.0} mm front a panel under {:.0} mm; the fronts stay slab",
                s.front_thickness,
                s.front_thickness - 4.0
            ));
            return;
        }
        let fronts: Vec<Part> = self
            .list
            .parts
            .iter()
            .filter(|p| {
                matches!(
                    p.role,
                    PartRole::Door | PartRole::DrawerFront | PartRole::LiftFront
                )
            })
            .cloned()
            .collect();
        for front in fronts {
            let b = front.bounds;
            let ta = front.thickness_axis;
            let wa = if ta == Axis::X { Axis::Y } else { Axis::X };
            let (w0, w1, z0, z1) = (b.min.get(wa), b.max().get(wa), b.min.z, b.max().z);
            // A front narrower or shorter than the threshold, or with no panel left, stays slab.
            let min = slab_below.max(2.0 * f + 2.0 * g);
            if w1 - w0 < min || z1 - z0 < min {
                continue;
            }
            let (t0, t1) = (b.min.get(ta), b.max().get(ta));
            let mid = (t0 + t1) / 2.0;
            let glass = glass_style && front.role != PartRole::DrawerFront;
            // The back face: towards the cabinet's inside.
            let inside = Vec3::new(self.cab.width / 2.0, self.cab.depth / 2.0, 0.0).get(ta);
            let back = if (t0 - inside).abs() < (t1 - inside).abs() {
                (t0, t0 + GLASS_REBATE_DEPTH)
            } else {
                (t1 - GLASS_REBATE_DEPTH, t1)
            };
            // A box from (w, z) ranges, across the thickness range (a, c).
            let boxed = |wr: (f64, f64), zr: (f64, f64), tr: (f64, f64)| {
                let lo = Vec3::new(0.0, 0.0, zr.0).with(wa, wr.0).with(ta, tr.0);
                let hi = Vec3::new(0.0, 0.0, zr.1).with(wa, wr.1).with(ta, tr.1);
                Aabb::new(lo, Vec3::new(hi.x - lo.x, hi.y - lo.y, hi.z - lo.z))
            };
            let full = (t0, t1);
            let slot = (mid - p / 2.0, mid + p / 2.0);
            let mut members = vec![
                (
                    PartRole::FrameStile,
                    1,
                    boxed((w0, w0 + f), (z0, z1), full),
                    Axis::Z,
                ),
                (
                    PartRole::FrameStile,
                    2,
                    boxed((w1 - f, w1), (z0, z1), full),
                    Axis::Z,
                ),
                (
                    PartRole::FrameRail,
                    1,
                    boxed((w0 + f - g, w1 - f + g), (z0, z0 + f), full),
                    wa,
                ),
                (
                    PartRole::FrameRail,
                    2,
                    boxed((w0 + f - g, w1 - f + g), (z1 - f, z1), full),
                    wa,
                ),
                (
                    PartRole::FramePanel,
                    1,
                    boxed(
                        (w0 + f - g + play, w1 - f + g - play),
                        (z0 + f - g + play, z1 - f + g - play),
                        slot,
                    ),
                    Axis::Z,
                ),
            ];
            if glass {
                members.retain(|m| m.0 != PartRole::FramePanel);
            }
            let name = |role: PartRole, n: usize| {
                if role == PartRole::FramePanel {
                    format!("{}{}", front.name, role.code())
                } else {
                    format!("{}{}{n}", front.name, role.code())
                }
            };
            self.list.parts.retain(|x| x.name != front.name);
            for &(role, n, bounds, grain) in &members {
                self.list.parts.push(Part {
                    name: name(role, n),
                    role,
                    bounds,
                    thickness_axis: ta,
                    grain_axis: Some(grain),
                    board: if role == PartRole::FramePanel {
                        panel_board.clone()
                    } else {
                        front_board.clone()
                    },
                    edges: EdgeBanding::default(),
                });
            }
            // The panel's grooves: along each stile's inner edge and each rail's.
            // Glass sits in rebates from the back face instead of the panel's grooves.
            let slot = if glass { back } else { slot };
            let grooves = [
                (
                    name(PartRole::FrameStile, 1),
                    boxed((w0 + f - g, w0 + f), (z0, z1), slot),
                ),
                (
                    name(PartRole::FrameStile, 2),
                    boxed((w1 - f, w1 - f + g), (z0, z1), slot),
                ),
                (
                    name(PartRole::FrameRail, 1),
                    boxed((w0 + f - g, w1 - f + g), (z0 + f - g, z0 + f), slot),
                ),
                (
                    name(PartRole::FrameRail, 2),
                    boxed((w0 + f - g, w1 - f + g), (z1 - f, z1 - f + g), slot),
                ),
            ];
            for (part, bounds) in grooves {
                self.list
                    .operations
                    .push(Operation::Groove { part, bounds });
            }
            if glass {
                let (gw, gh) = (
                    w1 - w0 - 2.0 * (f - g + play),
                    z1 - z0 - 2.0 * (f - g + play),
                );
                for (item, description) in [
                    (
                        "glass.4mm",
                        format!("Glass {GLASS_THICKNESS:.0} mm, cut to {gw:.0} × {gh:.0}"),
                    ),
                    (
                        "glass.bead",
                        format!("Glazing bead, {:.0} mm in all", 2.0 * (gw + gh)),
                    ),
                ] {
                    self.list.hardware.push(HardwarePlacement {
                        item: item.into(),
                        description,
                        at: b.center(),
                        part: name(PartRole::FrameStile, 1),
                        quantity: 1,
                    });
                }
            }
            // Which member a point on the slab falls in: stiles take the full height, rails the
            // rest of the top and bottom, the panel the middle.
            let member = |at: Vec3| {
                let (w, z) = (at.get(wa), at.z);
                if w < w0 + f {
                    name(PartRole::FrameStile, 1)
                } else if w > w1 - f {
                    name(PartRole::FrameStile, 2)
                } else if z < z0 + f {
                    name(PartRole::FrameRail, 1)
                } else if z > z1 - f {
                    name(PartRole::FrameRail, 2)
                } else {
                    name(PartRole::FramePanel, 1)
                }
            };
            let panel = name(PartRole::FramePanel, 1);
            if glass {
                // Nothing is fixed through glass: holes that fell there are left out.
                let before = self.list.operations.len();
                self.list.operations.retain(|o| {
                    !matches!(o, Operation::Bore { part, at, .. }
                        if *part == front.name && member(*at) == panel)
                });
                if self.list.operations.len() < before {
                    self.list
                        .warnings
                        .push(format!("{}: holes in the glass left out", front.name));
                }
            }
            for op in &mut self.list.operations {
                if let Operation::Bore {
                    part, at, depth, ..
                } = op
                    && *part == front.name
                {
                    *part = member(*at);
                    // A through hole in the panel goes through the panel only.
                    if *part == panel {
                        *depth = depth.min(p);
                    }
                }
            }
            // Grooves on the slab (a finger pull) are cut in each member they cross.
            let members_now: Vec<(String, Aabb)> = self
                .list
                .parts
                .iter()
                .filter(|x| x.name.starts_with(&front.name) && x.name != front.name)
                .map(|x| (x.name.clone(), x.bounds))
                .collect();
            let mut split = Vec::new();
            self.list.operations.retain(|o| match o {
                Operation::Groove { part, bounds } if *part == front.name => {
                    for (name, mb) in &members_now {
                        let lo = Vec3::new(
                            bounds.min.x.max(mb.min.x),
                            bounds.min.y.max(mb.min.y),
                            bounds.min.z.max(mb.min.z),
                        );
                        let hi = Vec3::new(
                            bounds.max().x.min(mb.max().x),
                            bounds.max().y.min(mb.max().y),
                            bounds.max().z.min(mb.max().z),
                        );
                        if hi.x - lo.x > 1e-6 && hi.y - lo.y > 1e-6 && hi.z - lo.z > 1e-6 {
                            split.push(Operation::Groove {
                                part: name.clone(),
                                bounds: Aabb::new(
                                    lo,
                                    Vec3::new(hi.x - lo.x, hi.y - lo.y, hi.z - lo.z),
                                ),
                            });
                        }
                    }
                    false
                }
                _ => true,
            });
            self.list.operations.extend(split);
            for h in &mut self.list.hardware {
                if h.part == front.name {
                    h.part = member(h.at);
                    if glass && h.part == panel {
                        h.part = name(PartRole::FrameStile, 1);
                    }
                }
            }
        }
    }

    /// HS and HL arms swing through the top of the cabinet: a shelf up there is in their way.
    fn lift_arm_warnings(&mut self) {
        let reach = self
            .list
            .hardware
            .iter()
            .filter_map(|h| {
                let id = if h.item.starts_with("blum.aventos.hs-top") {
                    "blum.aventos.hs-top"
                } else if h.item.starts_with("blum.aventos.hl-top.lever-arm.") {
                    h.item.as_str()
                } else {
                    return None;
                };
                crate::hardware::item(id)
                    .ok()?
                    .param("arm_reach_below_top")
                    .ok()
            })
            .fold(0.0, f64::max);
        if reach <= 0.0 {
            return;
        }
        let limit = self.cab.origin.z + self.cab.height - reach;
        let high: Vec<String> = self
            .list
            .by_role(PartRole::Shelf)
            .filter(|p| p.bounds.max().z > limit)
            .map(|p| p.name.clone())
            .collect();
        for name in high {
            self.list.warnings.push(format!(
                "{name} is within {reach:.0} of the cabinet's top, in the lift's arms' way: lower it or leave it out"
            ));
        }
    }

    /// A rangehood canopy: two sides, a top between them, a bottom with the insert's cut-out, a
    /// fixed front over all of them; open at the back (the wall). Sides butt-jointed like a
    /// carcass, the front screwed from inside (left to site).
    fn canopy(&mut self) {
        let cab = self.cab;
        let s = &cab.standard;
        let (w, d, h, t, ft) = (
            cab.width,
            cab.depth,
            cab.height,
            s.carcass_thickness,
            s.front_thickness,
        );
        // No back: joints run the full depth.
        self.interior_back = d;
        let sides = [(PartRole::LeftSide, 0.0), (PartRole::RightSide, w - t)];
        for (role, x) in sides {
            self.add(
                role,
                None,
                aabb(x, 0.0, 0.0, x + t, d, h),
                Axis::X,
                Axis::Z,
                EdgeBanding::front_only(),
            );
        }
        self.add(
            PartRole::Top,
            None,
            aabb(t, 0.0, h - t, w - t, d, h),
            Axis::Z,
            Axis::X,
            EdgeBanding::default(),
        );
        self.add(
            PartRole::Bottom,
            None,
            aabb(t, 0.0, 0.0, w - t, d, t),
            Axis::Z,
            Axis::X,
            EdgeBanding::front_only(),
        );
        self.add(
            PartRole::CanopyFront,
            None,
            aabb(0.0, -ft, 0.0, w, 0.0, h),
            Axis::Y,
            Axis::Z,
            EdgeBanding::all(),
        );
        let (left, right) = (
            self.part(PartRole::LeftSide).unwrap(),
            self.part(PartRole::RightSide).unwrap(),
        );
        for role in [PartRole::Top, PartRole::Bottom] {
            let panel = self.part(role).unwrap();
            self.joint_parts(&left, &panel, Dir::new(Axis::X, true), Axis::Y);
            self.joint_parts(&right, &panel, Dir::new(Axis::X, false), Axis::Y);
        }
        // The insert's cut-out, centred across, from the front.
        let ins = cab.insert;
        let inner = w - 2.0 * t;
        if ins.width > inner - 2.0 * CANOPY_CUTOUT_MARGIN
            || ins.depth > d - 2.0 * CANOPY_CUTOUT_MARGIN
        {
            self.list.warnings.push(format!(
                "a {:.0} × {:.0} insert does not fit this canopy's bottom",
                ins.width, ins.depth
            ));
            return;
        }
        let x0 = (w - ins.width) / 2.0;
        let y0 = (d - ins.depth) / 2.0;
        self.list.operations.push(Operation::Groove {
            part: self.name(PartRole::Bottom, None),
            bounds: aabb(x0, y0, 0.0, x0 + ins.width, y0 + ins.depth, t),
        });
        self.list.notes.push(format!(
            "Rangehood insert cut-out {:.0} × {:.0}: check it against the insert's installation drawing",
            ins.width, ins.depth
        ));
    }

    /// Finished boards on the faces seen from the room, standing proud of the carcass: the ends
    /// from the front's face to the back (and over a back board), the back across the width; floor
    /// to top with the grain (any flutes) upright. Glued and pinned on site: no machining.
    fn cladding(&mut self) {
        let cab = self.cab;
        let c = &cab.cladding;
        let (w, d, h) = (cab.width, cab.depth, cab.height);
        let ct = c.thickness;
        // Where the fronts really stand: an inset one is back in the carcass, and a drawer frame
        // moves the whole build back by its thickness afterwards. No fronts: the carcass's edge.
        let face = self
            .list
            .parts
            .iter()
            .filter(|p| {
                matches!(
                    p.role,
                    PartRole::Door
                        | PartRole::DrawerFront
                        | PartRole::LiftFront
                        | PartRole::CanopyFront
                )
            })
            .map(|p| p.bounds.min.y)
            .fold(f64::MAX, f64::min);
        let face = if face == f64::MAX { 0.0 } else { face };
        // A seen side's finished board, from the front's face to the back (and over a back board).
        let back_t = if c.back { ct } else { 0.0 };
        let st = cab.seen_board_thickness();
        let board = if c.board.is_empty() {
            cab.standard.front_board.clone()
        } else {
            c.board.clone()
        };
        let mut pieces = Vec::new();
        if cab.seen_left() {
            pieces.push((
                "SeenEndLeft",
                aabb(-st, face, 0.0, 0.0, d + back_t, h),
                Axis::X,
            ));
        }
        if cab.seen_right() {
            pieces.push((
                "SeenEndRight",
                aabb(w, face, 0.0, w + st, d + back_t, h),
                Axis::X,
            ));
        }
        if c.back {
            pieces.push(("SeenBack", aabb(0.0, d, 0.0, w, d + ct, h), Axis::Y));
        }
        for (name, bounds, axis) in pieces {
            self.list.parts.push(Part {
                name: format!("{}.{name}", cab.id),
                role: PartRole::Cladding,
                bounds,
                thickness_axis: axis,
                grain_axis: Some(Axis::Z),
                board: board.clone(),
                edges: EdgeBanding::default(),
            });
        }
    }

    /// Blum's inner dividing sets in every M or K steel drawer, as Blum recommends for the
    /// cabinet's width: in LEGRABOX / MERIVOBOX, AMBIA-LINE (the top drawer a cutlery insert set,
    /// the rest drawer frames, p.546); in TANDEMBOX, ORGA-LINE (the top drawer a container set,
    /// the rest utensil divider sets, pp. 556-557).
    fn drawer_inserts(&mut self) {
        let cab = self.cab;
        if !cab.drawer_inserts {
            return;
        }
        let fits: Vec<&DrawerFit> = self
            .drawer_fits
            .iter()
            .filter(|f| matches!(f.system, "legrabox" | "merivobox" | "tandembox"))
            .collect();
        if fits.is_empty() {
            self.list.warnings.push(
                "drawer inserts go in LEGRABOX, MERIVOBOX and TANDEMBOX drawers; this cabinet has \
                 none"
                    .into(),
            );
            return;
        }
        let top = fits.iter().map(|f| f.front_top).fold(f64::MIN, f64::max);
        let mut rows = Vec::new();
        for f in fits {
            let cutlery = (f.front_top - top).abs() < 1e-6;
            let items = if f.system == "tandembox" {
                let set = if cutlery {
                    drawers::OrgaSet::Container
                } else {
                    drawers::OrgaSet::Utensil
                };
                drawers::orga_line(cab.width, f.nominal_length, &f.height, set)
            } else {
                let wood = cab.standard.insert_design == InsertDesign::Wood;
                drawers::ambia_line(cab.width, f.nominal_length, &f.height, cutlery, wood)
            };
            match items {
                Ok(items) => {
                    for (item, quantity) in items {
                        rows.push(HardwarePlacement {
                            item: item.id.clone(),
                            description: format!("{} {}", item.family, item.variant),
                            at: Vec3::default(),
                            part: f.front.clone(),
                            quantity,
                        });
                    }
                }
                Err(e) => self.list.warnings.push(format!("{}: {e}", f.front)),
            }
        }
        // Placed at their drawer front's middle.
        for r in &mut rows {
            if let Some(p) = self.list.part(&r.part) {
                r.at = p.bounds.center();
            }
        }
        self.list.hardware.extend(rows);
    }

    /// Each light's strip, and a cable hole through the panel between it and the void behind
    /// the back: down through the bottom for under-cabinet and plinth lights, up through the
    /// top (or back rail) for lights inside.
    fn lights(&mut self) {
        let cab = self.cab;
        let t = cab.standard.carcass_thickness;
        for l in cab
            .lights
            .iter()
            .filter(|l| !crate::lighting::zone_fits(cab, l.zone))
        {
            self.list.warnings.push(format!(
                "a {:?} light does not fit a {:?} cabinet; left out",
                l.zone, cab.kind
            ));
        }
        let fitting = cab
            .lights
            .iter()
            .filter(|l| crate::lighting::zone_fits(cab, l.zone));
        for (light, strip) in fitting.zip(crate::lighting::strips(cab)) {
            if matches!(light.zone, LightZone::SideRear | LightZone::UnderFixedShelf) {
                self.light_on_panels(light, &strip);
                continue;
            }
            let x = t + CABLE_HOLE_SIDE_INSET;
            let y = self.interior_back - CABLE_HOLE_BACK_INSET;
            let roles: &[PartRole] = match light.zone {
                LightZone::InCabinet => &[PartRole::Top, PartRole::BackRail],
                _ => &[PartRole::Bottom],
            };
            let through = self
                .list
                .parts
                .iter()
                .filter(|p| roles.contains(&p.role) && p.thickness_axis == Axis::Z)
                .find(|p| {
                    let b = &p.bounds;
                    (b.min.x..=b.max().x).contains(&x) && (b.min.y..=b.max().y).contains(&y)
                })
                .cloned();
            let Some(panel) = through else {
                self.list.warnings.push(format!(
                    "no panel to take the {:?} light's cable; drill it on site",
                    light.zone
                ));
                continue;
            };
            let b = panel.bounds;
            // Drilled from the inside face.
            let (z, down) = match light.zone {
                LightZone::InCabinet => (b.min.z, false),
                _ => (b.max().z, true),
            };
            self.list.operations.push(Operation::Bore {
                part: panel.name.clone(),
                at: Vec3::new(x, y, z),
                direction: Dir::new(Axis::Z, !down),
                diameter: CABLE_HOLE_DIAMETER,
                depth: b.size.z,
                purpose: BorePurpose::CableHole,
            });
            let v = light.voltage.label();
            self.list.hardware.push(HardwarePlacement {
                item: format!("lighting.strip.{}", v.replace(' ', "").to_lowercase()),
                description: format!(
                    "LED strip {v}, {:.0} mm, {:.1} W ({} W/m)",
                    strip.length, strip.watts, light.watts_per_metre
                ),
                at: Vec3::new(cab.width / 2.0, y, z),
                part: panel.name.clone(),
                quantity: 1,
            });
            self.light_profile(light, strip.length, &panel);
        }
    }

    /// A strip on each panel its zone names, set back from the rear: both sides, or each fixed shelf.
    fn light_on_panels(&mut self, light: &crate::model::Light, strip: &crate::lighting::Strip) {
        let panels: Vec<Part> = match light.zone {
            LightZone::SideRear => self
                .list
                .parts
                .iter()
                .filter(|p| matches!(p.role, PartRole::LeftSide | PartRole::RightSide))
                .cloned()
                .collect(),
            _ => self
                .list
                .parts
                .iter()
                .filter(|p| {
                    p.role == PartRole::Shelf
                        && !self
                            .list
                            .hardware
                            .iter()
                            .any(|h| h.item == SHELF_SUPPORT && h.part == p.name)
                })
                .cloned()
                .collect(),
        };
        if panels.is_empty() {
            self.list
                .warnings
                .push(format!("no panel to take the {:?} light", light.zone));
            return;
        }
        let v = light.voltage.label();
        let back = self.interior_back - LIGHT_PROFILE_REAR_SETBACK;
        for panel in &panels {
            let b = panel.bounds;
            let side = light.zone == LightZone::SideRear;
            let left = panel.role == PartRole::LeftSide;
            // The strip's cable runs back to the void behind the back, in a channel in the same
            // face, from near the strip's end to the panel's rear edge.
            let (channel, at) = if side {
                let z = self.opening_z.0 + LIGHT_CABLE_FROM_END;
                let (x0, x1) = if left {
                    (b.max().x - LIGHT_CHANNEL.1, b.max().x)
                } else {
                    (b.min.x, b.min.x + LIGHT_CHANNEL.1)
                };
                (
                    aabb(x0, back, z, x1, b.max().y, z + LIGHT_CHANNEL.0),
                    Vec3::new(if left { b.max().x } else { b.min.x }, back, z),
                )
            } else {
                let x = b.min.x + LIGHT_CABLE_FROM_END;
                (
                    aabb(
                        x,
                        back,
                        b.min.z,
                        x + LIGHT_CHANNEL.0,
                        b.max().y,
                        b.min.z + LIGHT_CHANNEL.1,
                    ),
                    Vec3::new(x, back, b.min.z),
                )
            };
            self.list.operations.push(Operation::Groove {
                part: panel.name.clone(),
                bounds: channel,
            });
            self.list.hardware.push(HardwarePlacement {
                item: format!("lighting.strip.{}", v.replace(' ', "").to_lowercase()),
                description: format!(
                    "LED strip {v}, {:.0} mm, {:.1} W ({} W/m)",
                    strip.length, strip.watts, light.watts_per_metre
                ),
                at,
                part: panel.name.clone(),
                quantity: 1,
            });
            let length = if side {
                let (floor, top) = crate::cad::side_light_span(&self.list);
                top - floor - 2.0 * LIGHT_PROFILE_END_CLEARANCE
            } else {
                strip.length
            };
            self.light_profile(light, length, panel);
        }
    }

    /// A strip's aluminium profile on the panel's underside, near its front edge. A recessed one
    /// has its groove routed to the catalogue's size, so it is on the cut sheet.
    fn light_profile(&mut self, light: &crate::model::Light, length: f64, panel: &Part) {
        if light.profile.is_empty() {
            return;
        }
        let item = match crate::hardware::item(&light.profile) {
            Ok(item) => item,
            Err(e) => {
                self.list.warnings.push(e);
                return;
            }
        };
        let b = panel.bounds;
        let side = light.zone == LightZone::SideRear;
        let rear = matches!(light.zone, LightZone::SideRear | LightZone::UnderFixedShelf);
        let y = if rear {
            self.interior_back - LIGHT_PROFILE_REAR_SETBACK
        } else {
            b.min.y + LIGHT_PROFILE_SETBACK
        };
        let left = panel.role == PartRole::LeftSide;
        // The face it fixes to, and the board behind that face.
        let thick = if side { b.size.x } else { b.size.z };
        if item.param("surface").unwrap_or(1.0) == 0.0 {
            let (Ok(width), Ok(depth)) = (item.param("routing_width"), item.param("routing_depth"))
            else {
                return;
            };
            if depth > thick - LIGHT_PROFILE_MIN_LEFT {
                self.list.warnings.push(format!(
                    "{} needs a {depth:.1} mm deep groove: {thick:.0} mm thick {} leaves too little. Use a surface mounted profile",
                    item.family, panel.name
                ));
                return;
            }
            let bounds = if side {
                let (x0, x1) = if left {
                    (b.max().x - depth, b.max().x)
                } else {
                    (b.min.x, b.min.x + depth)
                };
                let z0 = crate::cad::side_light_span(&self.list).0 + LIGHT_PROFILE_END_CLEARANCE;
                aabb(x0, y - width / 2.0, z0, x1, y + width / 2.0, z0 + length)
            } else {
                let x0 = b.center().x - length / 2.0;
                aabb(
                    x0,
                    y - width / 2.0,
                    b.min.z,
                    x0 + length,
                    y + width / 2.0,
                    b.min.z + depth,
                )
            };
            self.list.operations.push(Operation::Groove {
                part: panel.name.clone(),
                bounds,
            });
            // The cable leaves from the groove's end, through the panel to the back void.
            if !rear {
                self.list.operations.push(Operation::Bore {
                    part: panel.name.clone(),
                    at: Vec3::new(bounds.min.x + LIGHT_CABLE_FROM_END, y, b.min.z),
                    direction: Dir::new(Axis::Z, true),
                    diameter: CABLE_HOLE_DIAMETER,
                    depth: thick,
                    purpose: BorePurpose::CableHole,
                });
            }
        }
        self.list.hardware.push(HardwarePlacement {
            item: item.id.clone(),
            description: format!("{}, cut to {length:.0} mm", item.family),
            at: if side {
                Vec3::new(if left { b.max().x } else { b.min.x }, y, b.center().z)
            } else {
                Vec3::new(b.center().x, y, b.min.z)
            },
            part: panel.name.clone(),
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
        if !cab.has_toe_kick() {
            return;
        }
        let ft = s.front_thickness;
        let (w, kick, sb) = (cab.width, s.toe_kick_height, s.toe_kick_setback);
        // With notched sides the kick board fits between the sides, behind the notch.
        let (x0, x1) = if s.toe_kick == ToeKick::IntegratedNotch {
            (s.carcass_thickness, w - s.carcass_thickness)
        } else {
            (0.0, w)
        };
        // In line with its neighbours' behind a drawer frame, unless it fits in the sides' notch.
        let forward = if s.toe_kick == ToeKick::IntegratedNotch {
            0.0
        } else {
            self.frame_shift()
        };
        self.add(
            PartRole::Plinth,
            None,
            aabb(x0, sb - forward, 0.0, x1, sb - forward + ft, kick),
            Axis::Y,
            Axis::X,
            EdgeBanding {
                top: true,
                ..EdgeBanding::default()
            },
        );
        if s.toe_kick == ToeKick::IntegratedNotch {
            self.plinth_block(x0, x1, sb + ft);
        }
        if s.toe_kick == ToeKick::LegsAndPlinth {
            let (item, description) = match axilo_set(kick, s.leg_fixing) {
                Some(set) => (
                    set.id.clone(),
                    format!("Häfele AXILO 78 leg, {} ({})", set.variant, set.article),
                ),
                None => {
                    self.list.warnings.push(format!(
                        "no Häfele AXILO 78 leg reaches a {kick:.0} mm plinth: a generic adjustable leg is listed"
                    ));
                    (
                        "hafele.leg.adjustable".to_string(),
                        "Adjustable cabinet leg with plinth clip".to_string(),
                    )
                }
            };
            let xs: Vec<f64> = if w > 900.0 {
                vec![LEG_INSET, w / 2.0, w - LEG_INSET]
            } else {
                vec![LEG_INSET, w - LEG_INSET]
            };
            let bottom = self.name(PartRole::Bottom, None);
            let plinth = self.name(PartRole::Plinth, None);
            let clip = crate::hardware::item("hafele.axilo.plinth-clip.screw").ok();
            if let Some(from) = clip.and_then(|c| c.param("from_height").ok())
                && kick < from
            {
                self.list.warnings.push(format!(
                    "the plinth clip is for a plinth from {from:.0} mm, not {kick:.0} mm"
                ));
            }
            for x in xs {
                let front_leg = sb - forward + ft + 50.0;
                for y in [front_leg, cab.depth - LEG_BACK_INSET] {
                    self.list.hardware.push(HardwarePlacement {
                        item: item.clone(),
                        description: description.clone(),
                        at: Vec3::new(x, y, 0.0),
                        part: bottom.clone(),
                        quantity: 1,
                    });
                    self.leg_plate_holes(&item, &bottom, x, y);
                }
                // The plinth clips to the front legs, screwed to its back where nothing shows, and
                // pulls off for access to them.
                if let Some(c) = clip {
                    self.list.hardware.push(HardwarePlacement {
                        item: c.id.clone(),
                        description: format!("{} {}, {}", c.family, c.article, c.variant),
                        at: Vec3::new(x, sb - forward + ft, kick / 2.0),
                        part: plinth.clone(),
                        quantity: 1,
                    });
                }
            }
        }
    }
}

impl Builder<'_> {
    /// The holes an AXILO plate's knock-in tips or screws go into, up into the bottom panel.
    fn leg_plate_holes(&mut self, set: &str, bottom: &str, x: f64, y: f64) {
        let fixing = if set.ends_with("knock-in") {
            "knock-in"
        } else {
            "screw-on"
        };
        let (Some(panel), Ok(plate)) = (
            self.list.part(bottom).map(|p| p.bounds),
            crate::hardware::item(&format!("hafele.axilo.glide.{fixing}")),
        ) else {
            return;
        };
        if !set.starts_with("hafele.axilo.set.") {
            return;
        }
        let (Ok(diameter), Ok(depth)) = (plate.param("fix_diameter"), plate.param("fix_depth"))
        else {
            return;
        };
        let mut missed = 0;
        for (hx, hy) in crate::cad::leg_plate_fixings(plate, x, y, panel.center().x) {
            let inside = hx - diameter / 2.0 >= panel.min.x
                && hx + diameter / 2.0 <= panel.max().x
                && hy - diameter / 2.0 >= panel.min.y
                && hy + diameter / 2.0 <= panel.max().y;
            if !inside {
                missed += 1;
                continue;
            }
            self.list.operations.push(Operation::Bore {
                part: bottom.to_string(),
                at: Vec3::new(hx, hy, panel.min.z),
                direction: Dir::new(Axis::Z, true),
                diameter,
                depth: depth.min(panel.size.z - 3.0),
                purpose: BorePurpose::LegPlate,
            });
        }
        if missed > 0 {
            self.list.warnings.push(format!(
                "a leg at {x:.0}, {y:.0} has {missed} plate fixings off the bottom panel: fix it on site"
            ));
        }
    }

    /// Between notched sides there are no legs to clip a kick board to, so it screws to a batten
    /// on edge behind it, under the bottom panel: up through the batten into the bottom, and
    /// through the batten into the board's back. Nothing shows in the room or inside the cabinet,
    /// and the pilots stop short of both faces. It goes together on the bench, before the cabinet
    /// stands: the board is then not removable.
    fn plinth_block(&mut self, x0: f64, x1: f64, y0: f64) {
        let s = &self.cab.standard;
        let (t, kick, ft) = (s.carcass_thickness, s.toe_kick_height, s.front_thickness);
        let h = PLINTH_BLOCK_HEIGHT.min(kick);
        let (y1, z0) = (y0 + t, kick - h);
        self.add(
            PartRole::PlinthBlock,
            None,
            aabb(x0, y0, z0, x1, y1, kick),
            Axis::Y,
            Axis::X,
            EdgeBanding::default(),
        );
        let block = self.name(PartRole::PlinthBlock, None);
        let plinth = self.name(PartRole::Plinth, None);
        let bottom = self.name(PartRole::Bottom, None);
        let length = x1 - x0;
        // Staggered, so the two sets of screws do not cross in the batten.
        let mut up = crate::joinery::fixing_positions(
            length,
            crate::joinery::END_SETBACK.min(length / 4.0),
            PLINTH_BLOCK_SPACING,
            20.0,
        );
        if up.len() < 2 {
            up = vec![length / 4.0, 3.0 * length / 4.0];
        }
        let across: Vec<f64> = up.windows(2).map(|w| (w[0] + w[1]) / 2.0).collect();
        let clearance = crate::joinery::SCREW_CLEARANCE;
        let head = crate::joinery::SCREW_HEAD;
        let pilot = crate::joinery::SCREW_PILOT;
        let screw = |list: &mut PartList,
                     through: Vec3,
                     into_part: &str,
                     into: Vec3,
                     dir: Dir,
                     run: f64,
                     pilot_depth: f64| {
            for (d, depth, purpose, part, at) in [
                (clearance, run, BorePurpose::Screw, block.as_str(), through),
                (
                    head,
                    (head - clearance) / 2.0,
                    BorePurpose::Countersink,
                    block.as_str(),
                    through,
                ),
                (pilot, pilot_depth, BorePurpose::ScrewPilot, into_part, into),
            ] {
                list.operations.push(Operation::Bore {
                    part: part.to_string(),
                    at,
                    direction: dir,
                    diameter: d,
                    depth,
                    purpose,
                });
            }
        };
        let y = y0 + t / 2.0;
        for p in up {
            let x = x0 + p;
            screw(
                &mut self.list,
                Vec3::new(x, y, z0),
                &bottom,
                Vec3::new(x, y, kick),
                Dir::new(Axis::Z, true),
                h,
                PLINTH_BLOCK_PILOT.min(t - 3.0),
            );
        }
        let z = kick - h / 2.0;
        for p in across {
            let x = x0 + p;
            screw(
                &mut self.list,
                Vec3::new(x, y1, z),
                &plinth,
                Vec3::new(x, y0, z),
                Dir::new(Axis::Y, false),
                t,
                PLINTH_BLOCK_PILOT.min(ft - 3.0),
            );
        }
    }
}

/// The Häfele AXILO 78 leg set with `fixing` whose adjustment covers a plinth `height`, the
/// nearest nominal height first.
fn axilo_set(height: f64, fixing: LegFixing) -> Option<&'static cabinet_catalog::HardwareItem> {
    let screw_on = if fixing == LegFixing::ScrewOn {
        1.0
    } else {
        0.0
    };
    crate::hardware::items()
        .iter()
        .filter(|i| {
            i.id.starts_with("hafele.axilo.set.") && i.param("screw_on").ok() == Some(screw_on)
        })
        .filter_map(|i| {
            let (h, down, up) = (
                i.param("nominal_height").ok()?,
                i.param("adjust_down").ok()?,
                i.param("adjust_up").ok()?,
            );
            (height >= h - down - 1e-9 && height <= h + up + 1e-9)
                .then_some(((height - h).abs(), i))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, i)| i)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::joinery::{MINIFIX_BOLT, minifix_cap, minifix_housing};
    use crate::model::{ConstructionStandard, DrawerSystem, FixingRecipe};

    fn base() -> Cabinet {
        Cabinet::new("B01", CabinetKind::Base)
    }

    #[test]
    fn legs_are_the_axilo_set_whose_adjustment_covers_the_plinth() {
        for (kick, set) in [
            (150.0, Some("hafele.axilo.set.150.screw-on")),
            (165.0, Some("hafele.axilo.set.150.screw-on")),
            (170.0, Some("hafele.axilo.set.180.screw-on")),
            (100.0, Some("hafele.axilo.set.100.screw-on")),
            (250.0, None),
        ] {
            let mut cab = base();
            cab.standard.toe_kick = ToeKick::LegsAndPlinth;
            cab.standard.toe_kick_height = kick;
            let list = build_cabinet(&cab).unwrap();
            let legs: Vec<_> = list
                .hardware
                .iter()
                .filter(|h| crate::cad::is_leg(&h.item))
                .collect();
            assert_eq!(legs.len(), 4, "{kick}");
            let want = set.unwrap_or("hafele.leg.adjustable");
            assert!(
                legs.iter().all(|h| h.item == want),
                "{kick}: {}",
                legs[0].item
            );
            assert_eq!(
                list.warnings.iter().any(|w| w.contains("AXILO")),
                set.is_none(),
                "{kick}"
            );
        }
    }

    #[test]
    fn knock_in_legs_are_the_knock_in_axilo_set() {
        let mut cab = base();
        cab.standard.toe_kick = ToeKick::LegsAndPlinth;
        cab.standard.leg_fixing = LegFixing::KnockIn;
        let list = build_cabinet(&cab).unwrap();
        let legs: Vec<_> = list
            .hardware
            .iter()
            .filter(|h| crate::cad::is_leg(&h.item))
            .collect();
        assert_eq!(legs.len(), 4);
        assert!(
            legs.iter()
                .all(|h| h.item == "hafele.axilo.set.150.knock-in")
        );
        assert!(legs[0].description.contains("637.76.409"));
        let plates: Vec<_> = crate::cad::cad_placements(&list)
            .into_iter()
            .filter(|c| c.item.contains(".glide."))
            .collect();
        assert_eq!(plates.len(), 4);
        assert!(plates.iter().all(|c| c.article == "637.38.296"));
    }

    #[test]
    fn notched_sides_get_a_kick_board_between_them_behind_the_notch() {
        let mut cab = base();
        cab.standard.toe_kick = ToeKick::IntegratedNotch;
        let list = build_cabinet(&cab).unwrap();
        let t = cab.standard.carcass_thickness;
        let kick = list
            .parts
            .iter()
            .find(|p| p.role == PartRole::Plinth)
            .expect("a kick board");
        assert_eq!(kick.bounds.min.x, t);
        assert_eq!(kick.bounds.max().x, cab.width - t);
        assert_eq!(kick.bounds.min.y, cab.standard.toe_kick_setback);
        let clashes: Vec<_> = overlapping(&list)
            .into_iter()
            .filter(|(a, b)| *a == kick.name || *b == kick.name)
            .collect();
        assert!(clashes.is_empty(), "{clashes:?}");
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
            | Operation::Cutout { part, .. }
            | Operation::Chamfer { part, .. }
            | Operation::Mortise { part, .. }
            | Operation::Round { part, .. }
            | Operation::Arch { part, .. } => *part == shelf.name,
        }));
        assert_eq!(shelf.bounds.min.x, 16.0 + SHELF_SIDE_CLEARANCE);
    }

    /// Owner 2026-10-27: pin holes only a few steps above and below each adjustable shelf.
    #[test]
    fn shelf_pins_are_drilled_only_around_each_adjustable_shelf() {
        let mut cab = Cabinet::new("T01", CabinetKind::Tall);
        cab.adjustable_shelves = 2;
        let list = build_cabinet(&cab).unwrap();
        // Each shelf's pin holes, a pin's radius under it.
        let shelves: Vec<f64> = list
            .by_role(PartRole::Shelf)
            .map(|p| p.bounds.min.z - SHELF_PIN_DIAMETER / 2.0)
            .collect();
        let pins: Vec<f64> = list
            .operations
            .iter()
            .filter_map(|o| match o {
                Operation::Bore {
                    purpose: BorePurpose::ShelfPin,
                    at,
                    ..
                } => Some(at.z),
                _ => None,
            })
            .collect();
        assert!(!pins.is_empty());
        let reach = SHELF_PIN_REACH * SYSTEM32_PITCH + 1e-6;
        assert!(
            pins.iter()
                .all(|z| shelves.iter().any(|s| (z - s).abs() <= reach))
        );
        // Each shelf has holes at its own height and at the ends of its reach.
        for s in &shelves {
            for d in [-reach + 1e-6, 0.0, reach - 1e-6] {
                assert!(pins.iter().any(|z| (z - (s + d)).abs() < 1e-3), "{s} {d}");
            }
        }
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
        assert_eq!(count("blum.servo-drive.bracket-attachment"), 1);
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
            let front = part("DrawerInnerFront");
            assert!((front.bounds.min.z - base.bounds.max().z).abs() < 1e-9);
            // The back comes down behind the base for TANDEM's rear hooks.
            let back = part("DrawerBack");
            assert!((back.bounds.min.z - (l.bounds.min.z + 4.0)).abs() < 1e-9);
            for p in [front, back] {
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

    /// BLUM-4b: inner drawers share the opening behind the doors, which hang on 0-protrusion
    /// hinges; no shelves, no drawer fronts.
    #[test]
    fn inner_drawers_fill_the_opening_behind_doors_on_zero_protrusion_hinges() {
        for (count, side) in [(2, "K"), (5, "M")] {
            let mut cab = Cabinet::new("B01", CabinetKind::Base);
            cab.inner_drawers = count;
            let list = build_cabinet(&cab).unwrap();
            assert!(
                list.warnings
                    .iter()
                    .all(|w| w.contains("shelves left out") || w.contains("shelf-pin")),
                "{:?}",
                list.warnings
            );
            assert_eq!(list.by_role(PartRole::DrawerBottom).count(), count as usize);
            assert_eq!(list.by_role(PartRole::DrawerBack).count(), count as usize);
            assert_eq!(list.by_role(PartRole::Shelf).count(), 0);
            assert_eq!(list.by_role(PartRole::DrawerFront).count(), 0);
            let qty = |id: &str| {
                list.hardware
                    .iter()
                    .filter(|h| h.item == id)
                    .map(|h| h.quantity)
                    .sum::<u32>()
            };
            assert_eq!(qty("blum.clip-top-blumotion.155.overlay"), 2);
            assert_eq!(qty("blum.legrabox.front-piece"), count);
            let sides = format!("blum.legrabox.drawer-side.{}.", side.to_lowercase());
            assert_eq!(
                list.hardware
                    .iter()
                    .filter(|h| h.item.starts_with(&sides))
                    .count(),
                count as usize
            );
            let piece = list
                .hardware
                .iter()
                .find(|h| h.item == "blum.legrabox.front-piece")
                .unwrap();
            assert!(
                piece.description.ends_with("cut to 442 mm"),
                "{}",
                piece.description
            );
            // Each box stays inside the carcass, above the bottom and under the next one.
            let mut bottoms: Vec<_> = list.by_role(PartRole::DrawerBottom).collect();
            bottoms.sort_by(|a, b| a.bounds.min.z.total_cmp(&b.bounds.min.z));
            for pair in bottoms.windows(2) {
                assert!(pair[1].bounds.min.z - pair[0].bounds.min.z >= 104.0);
            }
        }
    }

    #[test]
    fn inner_drawers_need_doors_and_legrabox() {
        let mut cab = Cabinet::new("D01", CabinetKind::DrawerBank);
        cab.inner_drawers = 2;
        let list = build_cabinet(&cab).unwrap();
        assert!(list.warnings.iter().any(|w| w.contains("go behind doors")));
        let mut cab = Cabinet::new("B01", CabinetKind::Base);
        cab.inner_drawers = 2;
        cab.standard.drawer_system = DrawerSystem::Tandem;
        let list = build_cabinet(&cab).unwrap();
        assert!(list.warnings.iter().any(|w| w.contains("LEGRABOX only")));
        assert_eq!(list.by_role(PartRole::DrawerBottom).count(), 0);
    }

    /// BLUM-4b: with high sides, a two-drawer bank's deep drawers are F pull-outs: five Ø10
    /// front bores a side and two bracket pairs.
    #[test]
    fn high_fronted_drawers_get_f_sides_and_their_front_bores() {
        let mut cab = Cabinet::new("D01", CabinetKind::DrawerBank);
        cab.fronts = FrontLayout::Drawers {
            ratios: vec![1.0, 1.0],
        };
        cab.standard.drawer_sides = DrawerSides::HighFronted;
        let list = build_cabinet(&cab).unwrap();
        assert!(list.warnings.is_empty(), "{:?}", list.warnings);
        let bores = list
            .operations
            .iter()
            .filter(|o| {
                matches!(o, Operation::Bore { part, purpose: BorePurpose::FrontFixing, .. }
                    if part == "D01.DrawerFront1")
            })
            .count();
        assert_eq!(bores, 10);
        assert!(
            list.hardware
                .iter()
                .any(|h| h.item == "blum.legrabox.front-fixing.f.upper")
        );
        let back = list.part("D01.DrawerBack1").unwrap();
        assert_eq!(back.bounds.size.z, 212.0);
    }

    /// BLUM-7b: a small wall cabinet on HK-S and on HK-XS gets its mechanism and fixings.
    #[test]
    fn small_lifts_build_with_their_parts() {
        use crate::model::LiftKind;
        for (kind, mechanism, other) in [
            (
                LiftKind::SmallStayLift,
                "blum.aventos.hk-s.mechanism.",
                "blum.mounting-plate.175h3100",
            ),
            (
                LiftKind::HingedStayLift,
                "blum.aventos.hk-xs.mechanism.",
                "blum.clip-top.110.overlay.unsprung",
            ),
        ] {
            let mut cab = Cabinet::new("W01", CabinetKind::Wall);
            cab.height = 360.0;
            cab.fronts = FrontLayout::Lift {
                kind,
                handle_weight: 0.4,
            };
            let list = build_cabinet(&cab).unwrap();
            assert!(
                list.hardware.iter().any(|h| h.item.starts_with(mechanism)),
                "{kind:?}: {:?} {:?}",
                list.hardware,
                list.warnings
            );
            assert!(
                list.hardware
                    .iter()
                    .any(|h| h.item == other && h.quantity >= 2)
            );
        }
    }

    /// BLUM-7b: AVENTOS HKi sits in a 265 × 120 pocket, 12 deep, in each side, 21 below the
    /// top; shelf pins keep clear of it and nothing clashes.
    #[test]
    fn hki_cuts_a_pocket_in_each_side_and_pins_keep_clear() {
        use crate::model::LiftKind;
        let mut cab = Cabinet::new("W01", CabinetKind::Wall);
        cab.height = 450.0;
        cab.fronts = FrontLayout::Lift {
            kind: LiftKind::IntegratedStayLift,
            handle_weight: 0.4,
        };
        let list = build_cabinet(&cab).unwrap();
        assert!(
            !list.warnings.iter().any(|w| w.contains("clash")),
            "{:?}",
            list.warnings
        );
        let pockets: Vec<Aabb> = list
            .operations
            .iter()
            .filter_map(|o| match o {
                Operation::Groove { part, bounds }
                    if part.ends_with("Side") && bounds.size.x == 12.0 =>
                {
                    Some(*bounds)
                }
                _ => None,
            })
            .collect();
        assert_eq!(pockets.len(), 2);
        for p in &pockets {
            assert_eq!(
                (p.size.y, p.size.z, p.max().z),
                (265.0, 120.0, 450.0 - 21.0)
            );
            assert_eq!(p.min.y, 0.0, "open at the front edge");
        }
        assert!(
            list.hardware
                .iter()
                .any(|h| h.item.starts_with("blum.aventos.hki.mechanism."))
        );
    }

    /// SHAK-1: the frame width, panel thickness and groove depth come from the standard, and a
    /// front under the slab threshold stays slab.
    #[test]
    fn shaker_sizes_and_the_slab_threshold_follow_the_standard() {
        let mut cab = Cabinet::new("B01", CabinetKind::Base);
        cab.standard.door_style = DoorStyle::Shaker;
        cab.standard.shaker_frame_width = 90.0;
        cab.standard.shaker_panel_thickness = 6.0;
        cab.standard.shaker_groove_depth = 12.0;
        let list = build_cabinet(&cab).unwrap();
        let get = |n: &str| list.part(n).unwrap_or_else(|| panic!("{n}"));
        let (l, rail) = (get("B01.Door1Stile1"), get("B01.Door1Rail1"));
        assert_eq!(l.bounds.size.x, 90.0);
        assert_eq!(rail.bounds.size.z, 90.0);
        assert_eq!(get("B01.Door1Panel").thickness(), 6.0);
        assert_eq!(rail.bounds.min.x, l.bounds.max().x - 12.0);
        // A threshold over the door's width keeps it slab.
        cab.standard.shaker_slab_below = 700.0;
        let list = build_cabinet(&cab).unwrap();
        assert!(list.part("B01.Door1").is_some());
    }

    #[test]
    fn shaker_sizes_that_do_not_fit_leave_the_fronts_slab_with_a_note() {
        let mut cab = Cabinet::new("B01", CabinetKind::Base);
        cab.standard.door_style = DoorStyle::Shaker;
        cab.standard.shaker_panel_thickness = 16.0;
        let list = build_cabinet(&cab).unwrap();
        assert!(list.part("B01.Door1").is_some());
        assert!(list.warnings.iter().any(|w| w.contains("shaker sizes")));
    }

    /// P2-2: a shaker door is two stiles, two tenoned rails and a grooved panel; its hinge cups
    /// move to the hinge-side stile and its handle to the other; nothing clashes.
    #[test]
    fn a_shaker_door_is_a_frame_and_panel_carrying_its_hinges_and_handle() {
        let mut cab = Cabinet::new("B01", CabinetKind::Base);
        cab.standard.door_style = DoorStyle::Shaker;
        let list = build_cabinet(&cab).unwrap();
        assert!(
            list.warnings.iter().all(|w| w.contains("shelf-pin")),
            "{:?}",
            list.warnings
        );
        assert!(list.part("B01.Door1").is_none());
        let get = |n: &str| list.part(n).unwrap_or_else(|| panic!("{n}"));
        let (l, r) = (get("B01.Door1Stile1"), get("B01.Door1Stile2"));
        let (bottom, top) = (get("B01.Door1Rail1"), get("B01.Door1Rail2"));
        let panel = get("B01.Door1Panel");
        // Rails reach a groove's depth into each stile.
        assert!(
            (bottom.bounds.size.x
                - (r.bounds.min.x - l.bounds.max().x + 2.0 * cab.standard.shaker_groove_depth))
                .abs()
                < 1e-9
        );
        assert!((panel.thickness() - cab.standard.shaker_panel_thickness).abs() < 1e-9);
        assert_eq!(l.bounds.size.z, top.bounds.max().z - bottom.bounds.min.z);
        // A left-hinged door: cups in the left stile, the handle in the right.
        let on = |part: &str, purpose: BorePurpose| {
            list.operations
                .iter()
                .filter(|o| matches!(o, Operation::Bore { part: p, purpose: q, .. } if p == part && *q == purpose))
                .count()
        };
        assert_eq!(on("B01.Door1Stile1", BorePurpose::HingeCup), 2);
        assert_eq!(on("B01.Door1Stile2", BorePurpose::HandleScrew), 2);
        assert!(list.hardware.iter().all(|h| h.part != "B01.Door1"));
        let grooves = list
            .operations
            .iter()
            .filter(
                |o| matches!(o, Operation::Groove { part, .. } if part.starts_with("B01.Door1")),
            )
            .count();
        assert_eq!(grooves, 4);
    }

    #[test]
    fn shaker_drawer_fronts_keep_their_fixings_and_small_ones_stay_slab() {
        let mut cab = Cabinet::new("D01", CabinetKind::DrawerBank);
        cab.standard.door_style = DoorStyle::Shaker;
        cab.fronts = FrontLayout::Drawers {
            ratios: vec![1.0, 2.0, 2.0],
        };
        let list = build_cabinet(&cab).unwrap();
        assert!(
            !list.warnings.iter().any(|w| w.contains("clash")),
            "{:?}",
            list.warnings
        );
        // The top front is small: one board.
        assert!(list.part("D01.DrawerFront1").is_some());
        assert!(list.part("D01.DrawerFront2Stile1").is_some());
        let fixings = list
            .operations
            .iter()
            .filter(|o| matches!(o, Operation::Bore { part, purpose: BorePurpose::FrontFixing, .. } if part.starts_with("D01.DrawerFront2Stile")))
            .count();
        assert_eq!(fixings, 4);
    }

    /// P2-2b: a glass door: the frame without a panel, rebated at the back, with glass and beads
    /// in the hardware list; drawer fronts in the same cabinet stay panelled.
    #[test]
    fn a_glass_door_has_a_rebated_frame_and_its_glass_listed() {
        let mut cab = Cabinet::new("W01", CabinetKind::Wall);
        cab.standard.door_style = DoorStyle::GlassFrame;
        let list = build_cabinet(&cab).unwrap();
        assert!(
            !list.warnings.iter().any(|w| w.contains("clash")),
            "{:?}",
            list.warnings
        );
        assert!(list.part("W01.Door1Panel").is_none());
        let stile = list.part("W01.Door1Stile1").unwrap();
        let rebate = list
            .operations
            .iter()
            .find_map(|o| match o {
                Operation::Groove { part, bounds } if *part == stile.name => Some(*bounds),
                _ => None,
            })
            .unwrap();
        // From the back face (towards the cabinet), 12 deep.
        assert_eq!(rebate.max().y, stile.bounds.max().y);
        assert_eq!(rebate.size.y, GLASS_REBATE_DEPTH);
        let glass = list
            .hardware
            .iter()
            .find(|h| h.item == "glass.4mm")
            .unwrap();
        assert!(
            glass.description.contains("cut to"),
            "{}",
            glass.description
        );
        assert!(list.hardware.iter().all(|h| list.part(&h.part).is_some()));
    }

    /// P2-3a: a wardrobe: hat shelf 300 under the top with a rail under it; double hanging adds
    /// a shelf at 1050 with its own rail. Both shelves are fixed between the sides.
    #[test]
    fn a_wardrobe_hangs_from_rails_under_fixed_shelves() {
        for (hanging, shelves) in [(Hanging::Long, 1), (Hanging::Double, 2)] {
            let mut cab = Cabinet::new("R01", CabinetKind::Wardrobe);
            cab.wardrobe = hanging;
            let list = build_cabinet(&cab).unwrap();
            assert!(
                !list.warnings.iter().any(|w| w.contains("clash")),
                "{:?}",
                list.warnings
            );
            assert_eq!(list.by_role(PartRole::Shelf).count(), shelves);
            let rails: Vec<_> = list
                .hardware
                .iter()
                .filter(|h| h.item == "wardrobe-rail.oval")
                .collect();
            assert_eq!(rails.len(), shelves);
            assert!(
                rails[0].description.ends_with("cut to 866 mm"),
                "{}",
                rails[0].description
            );
            let hat = list.part("R01.Shelf1").unwrap();
            assert_eq!(
                hat.bounds.max().z,
                2100.0 - cab.standard.wardrobe_hat_shelf_drop
            );
            // Fixed: fixings in the sides at the shelf.
            assert!(list.operations.iter().any(|o| matches!(o,
                Operation::BiscuitSlot { parts, .. } if parts.iter().any(|p| p == "R01.Shelf1"))));
        }
    }

    /// Blum's session (check 2): a third hole 37 from the front and 38 above each datum, which
    /// is 54 and 328.33 up the sides; the top one follows the fixed shelf, 7.5 higher in ours.
    #[test]
    fn a_legrabox_drills_its_profile_screws_at_blums_positions() {
        let cab: Cabinet = serde_json::from_value(serde_json::json!({
            "id": "B01", "kind": "base", "width": 600, "depth": 580, "height": 870,
            "adjustable_shelves": 0,
            "fronts": {"type": "sections", "columns": [{"width": 0, "rows": [
                {"front": {"type": "drawers", "count": 1}, "shelf_above": false,
                 "opening": "servo_drive"},
                {"front": {"type": "drawers", "count": 1}, "shelf_above": true,
                 "opening": "servo_drive"},
                {"front": {"type": "drawers", "count": 1}, "opening": "servo_drive"}
            ]}]},
            "standard": {"toe_kick": "none", "top_joint": "rails", "bottom_joint": "under_sides",
                "back": "applied", "back_thickness": 16, "front_thickness": 16,
                "carcass_thickness": 16, "front_gap": 3, "top_gap": 2, "bottom_gap": 0,
                "rail_width": 80, "fixings": "dowel", "overlay": "full",
                "hinge_boss": "expando", "drawer_system": "legrabox"}
        }))
        .unwrap();
        let list = build_cabinet(&cab).unwrap();
        let mut third: Vec<f64> = list
            .operations
            .iter()
            .filter_map(|o| match o {
                Operation::Bore {
                    part,
                    at,
                    purpose: BorePurpose::RunnerScrew,
                    ..
                } if part == "B01.LeftSide" && at.y == 37.0 => Some(at.z),
                _ => None,
            })
            .collect();
        third.sort_by(f64::total_cmp);
        assert_eq!(third.len(), 3);
        assert!((third[0] - 54.0).abs() < 0.01, "{third:?}");
        assert!((third[1] - 328.33).abs() < 0.01, "{third:?}");
        let shelf = list.by_role(PartRole::Shelf).next().unwrap().bounds.max().z;
        assert!((third[2] - (shelf + 38.0)).abs() < 0.01, "{third:?}");
        assert!(
            list.warnings.iter().all(|w| !w.contains("clash")),
            "{:?}",
            list.warnings
        );
    }

    /// Blum's p.592: the screw line's height above the datum, and the third hole 37 from the front.
    #[test]
    fn every_drawer_system_drills_its_runner_screws_at_its_own_height() {
        for (system, height) in [
            (DrawerSystem::Legrabox, 38.0),
            (DrawerSystem::Merivobox, 54.0),
            (DrawerSystem::Tandembox, 36.0),
            (DrawerSystem::Movento, 38.0),
            (DrawerSystem::Tandem, 37.0),
        ] {
            let mut cab = Cabinet::new("D01", CabinetKind::DrawerBank).sectioned();
            cab.standard.drawer_system = system;
            cab.standard.bottom_joint = BottomJoint::BetweenSides;
            let list = build_cabinet(&cab).unwrap();
            let floor = list.part("D01.Bottom").unwrap().bounds.max().z;
            let lowest = list
                .operations
                .iter()
                .filter_map(|o| match o {
                    Operation::Bore {
                        part,
                        at,
                        purpose: BorePurpose::RunnerScrew,
                        ..
                    } if part == "D01.LeftSide" && at.y == 37.0 => Some(at.z),
                    _ => None,
                })
                .fold(f64::MAX, f64::min);
            assert!(
                (lowest - (floor + height)).abs() < 1e-6,
                "{system:?}: {lowest} over {floor}"
            );
            assert!(
                list.warnings.iter().all(|w| !w.contains("clash")),
                "{system:?}: {:?}",
                list.warnings
            );
        }
    }

    #[test]
    fn every_runner_in_the_catalog_says_where_it_screws_on() {
        for item in crate::hardware::items() {
            let runner = crate::cad::is_runner(&item.id) && !item.id.contains(".drawer-side.");
            if runner && item.param("nominal_length").is_ok() && !item.id.contains(".inner") {
                assert!(!item.drilling.is_empty(), "{}", item.id);
                assert!(item.drilling.iter().any(|h| h.x == 37.0), "{}", item.id);
            }
        }
    }

    fn with_abi_pull(kind: CabinetKind, family: &str) -> Cabinet {
        let id = match kind {
            CabinetKind::Wall => "W01",
            _ => "D01",
        };
        let mut cab = Cabinet::new(id, kind).sectioned();
        cab.standard.handle_range = crate::model::HandleRange::Abi;
        cab.standard.handle_style = crate::hardware::items()
            .iter()
            .find(|i| i.id.starts_with(family))
            .expect("the pull")
            .id
            .clone();
        cab
    }

    fn pull_bores(list: &PartList, part: &str) -> Vec<(Vec3, Dir, f64)> {
        list.operations
            .iter()
            .filter_map(|o| match o {
                Operation::Bore {
                    part: p,
                    at,
                    direction,
                    depth,
                    purpose: BorePurpose::EdgePullScrew,
                    ..
                } if p == part => Some((*at, *direction, *depth)),
                _ => None,
            })
            .collect()
    }

    /// ABI's Rappana 100: holes 75 apart, 10 below the plate's top (2 thick), leg 2 thick in a
    /// rebate in the back of the front.
    #[test]
    fn a_rappana_screws_its_leg_into_a_rebate_in_the_back_of_the_front() {
        let cab = with_abi_pull(CabinetKind::DrawerBank, "handle.abi.rappana-100.");
        let list = build_cabinet(&cab).unwrap();
        let front = list.part("D01.DrawerFront1").unwrap().bounds;
        let bores = pull_bores(&list, "D01.DrawerFront1");
        assert_eq!(bores.len(), 2);
        assert!((bores[1].0.x - bores[0].0.x - 75.0).abs() < 1e-9);
        assert_eq!(
            bores[0].0.z,
            front.max().z - 8.0,
            "10 below the plate's top"
        );
        assert_eq!(bores[0].0.y, front.max().y - 2.0, "from the rebate's floor");
        assert!(!bores[0].1.positive, "towards the face");
        assert!(bores[0].2 < front.size.y - 2.0, "not through the face");
        let rebate = list
            .operations
            .iter()
            .find_map(|o| match o {
                Operation::Groove { part, bounds } if part == "D01.DrawerFront1" => Some(*bounds),
                _ => None,
            })
            .expect("the rebate");
        assert_eq!(rebate.size.y, 2.0);
        assert_eq!(rebate.max().z, front.max().z);
        assert!(rebate.size.x > 100.0 && rebate.size.x < 110.0);
        assert!(
            list.warnings.iter().all(|w| !w.contains("clash")),
            "{:?}",
            list.warnings
        );
        assert!(list.hardware.iter().any(|h| h.item.contains("rappana-100")));
    }

    #[test]
    fn a_scalo_sits_on_the_edge_and_screws_down_into_it() {
        let cab = with_abi_pull(CabinetKind::DrawerBank, "handle.abi.scalo-50.");
        let list = build_cabinet(&cab).unwrap();
        let front = list.part("D01.DrawerFront1").unwrap().bounds;
        let bores = pull_bores(&list, "D01.DrawerFront1");
        assert_eq!(bores.len(), 2);
        assert!((bores[1].0.x - bores[0].0.x - 32.0).abs() < 1e-9);
        assert_eq!(bores[0].0.z, front.max().z);
        assert_eq!(
            bores[0].0.y,
            front.max().y - 10.0,
            "10 from the plate's back"
        );
        assert!(!bores[0].1.positive && bores[0].1.axis == Axis::Z, "down");
        assert!(
            list.warnings.iter().all(|w| !w.contains("clash")),
            "{:?}",
            list.warnings
        );
    }

    /// The owner: 2 mm between the plate and whatever is above it, raised automatically.
    #[test]
    fn an_edge_pull_gets_clearance_over_it_at_the_top_and_between_fronts() {
        let cab = with_abi_pull(CabinetKind::DrawerBank, "handle.abi.rappana-100.");
        let list = build_cabinet(&cab).unwrap();
        let fronts: Vec<_> = list
            .by_role(PartRole::DrawerFront)
            .map(|p| p.bounds)
            .collect();
        assert_eq!(cab.height - fronts[0].max().z, 4.0, "plate 2 + clearance 2");
        assert_eq!(fronts[0].min.z - fronts[1].max().z, 4.0);
        let mut plain = cab.clone();
        plain.standard.handle_range = crate::model::HandleRange::Generic;
        let list = build_cabinet(&plain).unwrap();
        let fronts: Vec<_> = list
            .by_role(PartRole::DrawerFront)
            .map(|p| p.bounds)
            .collect();
        assert_eq!(
            fronts[0].min.z - fronts[1].max().z,
            plain.standard.front_gap
        );
    }

    #[test]
    fn a_pull_longer_than_its_front_is_left_off_and_says_so() {
        let mut cab = with_abi_pull(CabinetKind::DrawerBank, "handle.abi.rappana-800.");
        cab.width = 600.0;
        let list = build_cabinet(&cab).unwrap();
        assert!(list.warnings.iter().any(|w| w.contains("does not fit")));
        assert!(pull_bores(&list, "D01.DrawerFront1").is_empty());
    }

    #[test]
    fn a_wall_doors_edge_pull_goes_on_its_bottom_edge() {
        let cab = with_abi_pull(CabinetKind::Wall, "handle.abi.rappana-100.");
        let list = build_cabinet(&cab).unwrap();
        let door = list.part("W01.Door1").unwrap().bounds;
        let bores = pull_bores(&list, "W01.Door1");
        assert_eq!(bores[0].0.z, door.min.z + 8.0);
    }

    #[test]
    fn a_shaker_front_leaves_an_edge_pull_off_and_says_so() {
        let mut cab = with_abi_pull(CabinetKind::DrawerBank, "handle.abi.rappana-100.");
        cab.standard.door_style = DoorStyle::Shaker;
        let list = build_cabinet(&cab).unwrap();
        assert!(
            list.warnings
                .iter()
                .any(|w| w.contains("needs a slab front"))
        );
    }

    fn lift_holes(list: &PartList, part: &str) -> Vec<Vec3> {
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
            .collect()
    }

    fn wall_lift(kind: LiftKind) -> Cabinet {
        let mut cab = Cabinet::new("W01", CabinetKind::Wall);
        cab.height = 400.0;
        cab.fronts = FrontLayout::Lift {
            kind,
            handle_weight: 0.4,
        };
        cab
    }

    /// Blum's p.48: 37, then 32, 64 and 32 along, 36 under the top panel.
    #[test]
    fn an_hk_top_drills_its_mechanism_and_front_brackets() {
        let list = build_cabinet(&wall_lift(LiftKind::StayLift)).unwrap();
        assert!(
            list.warnings.iter().all(|w| !w.contains("no lift")),
            "{:?}",
            list.warnings
        );
        for side in ["W01.LeftSide", "W01.RightSide"] {
            let holes = lift_holes(&list, side);
            let ys: Vec<f64> = holes.iter().map(|h| h.y).collect();
            assert_eq!(ys, [37.0, 69.0, 133.0, 165.0], "{side}");
        }
        let top = list
            .parts
            .iter()
            .filter(|p| matches!(p.role, PartRole::FrontRail | PartRole::Top))
            .map(|p| p.bounds.min.z)
            .fold(f64::MAX, f64::min);
        assert!((lift_holes(&list, "W01.LeftSide")[0].z - (top - 36.0)).abs() < 1e-6);
        let brackets = list
            .operations
            .iter()
            .filter(|o| {
                matches!(o, Operation::Bore { part, purpose: BorePurpose::FrontFixing, .. } if part == "W01.LiftFront1")
            })
            .count();
        assert_eq!(brackets, 8, "four each side");
        assert!(
            list.warnings.iter().all(|w| !w.contains("clash")),
            "{:?}",
            list.warnings
        );
    }

    #[test]
    fn hk_s_and_hk_xs_drill_their_own_patterns() {
        let s = build_cabinet(&wall_lift(LiftKind::SmallStayLift)).unwrap();
        let ys: Vec<f64> = lift_holes(&s, "W01.LeftSide").iter().map(|h| h.y).collect();
        assert_eq!(ys, [37.0, 101.0]);
        assert!(
            s.warnings.iter().all(|w| !w.contains("clash")),
            "{:?}",
            s.warnings
        );
        let xs = build_cabinet(&wall_lift(LiftKind::HingedStayLift)).unwrap();
        let holes = lift_holes(&xs, "W01.LeftSide");
        assert_eq!(holes.len(), 2);
        assert_eq!(holes[0].z - holes[1].z, 32.0, "the plate's holes");
        assert!(
            xs.warnings.iter().all(|w| !w.contains("clash")),
            "{:?}",
            xs.warnings
        );
    }

    #[test]
    fn an_hk_xs_hangs_on_hinges_along_the_top_with_plates_under_it() {
        let list = build_cabinet(&wall_lift(LiftKind::HingedStayLift)).unwrap();
        let front = list.part("W01.LiftFront1").unwrap().bounds;
        let cups: Vec<Vec3> = list
            .operations
            .iter()
            .filter_map(|o| match o {
                Operation::Bore {
                    part,
                    at,
                    purpose: BorePurpose::HingeCup,
                    ..
                } if part == "W01.LiftFront1" => Some(*at),
                _ => None,
            })
            .collect();
        assert_eq!(cups.len(), 2);
        assert!(
            cups.iter().all(|c| c.z > front.max().z - 40.0),
            "along the top"
        );
        assert_eq!(cups[0].x, front.min.x + 80.0);
        assert_eq!(cups[1].x, front.max().x - 80.0);
        let plates = list
            .operations
            .iter()
            .filter(|o| {
                matches!(
                    o,
                    Operation::Bore {
                        purpose: BorePurpose::HingePlate,
                        ..
                    }
                )
            })
            .count();
        assert_eq!(plates, 4, "two screws to a plate");
        assert!(
            list.warnings.iter().all(|w| !w.contains("clash")),
            "{:?}",
            list.warnings
        );
    }

    /// p.24: a rectangle 128 × 64, 101 from the front, H below the cabinet's top by its height.
    #[test]
    fn an_hf_drills_four_holes_each_side_at_the_height_for_its_cabinet() {
        let mut cab = Cabinet::new("W01", CabinetKind::Wall);
        cab.fronts = FrontLayout::BiFoldLift { handle_weight: 0.4 };
        cab.height = 720.0;
        let list = build_cabinet(&cab).unwrap();
        let holes = lift_holes(&list, "W01.RightSide");
        assert_eq!(holes.len(), 4);
        assert_eq!(holes[0].y, 101.0);
        assert_eq!(holes[1].y, 229.0);
        assert_eq!(holes[0].z, 720.0 - 116.0);
        assert_eq!(holes[2].z, 720.0 - 116.0 - 64.0);
        assert!(
            list.warnings.iter().all(|w| !w.contains("clash")),
            "{:?}",
            list.warnings
        );
        cab.height = 500.0;
        let short = build_cabinet(&cab).unwrap();
        assert_eq!(lift_holes(&short, "W01.RightSide")[0].z, 500.0 - 93.5);
    }

    #[test]
    fn a_section_can_pick_its_box_height_and_length() {
        let mut cab = Cabinet::new("D01", CabinetKind::DrawerBank).sectioned();
        let FrontLayout::Sections { columns } = &mut cab.fronts else {
            panic!("sections");
        };
        columns[0].rows[0].front = SectionFront::Drawers {
            count: 1,
            ratios: Vec::new(),
            system: None,
            box_height: Some(crate::model::BoxHeight::C),
            nominal_length: Some(500.0),
        };
        let list = build_cabinet(&cab).unwrap();
        let has = |id: &str| list.hardware.iter().any(|h| h.item == id);
        assert!(has("blum.legrabox.cabinet-profile.750.500"));
        assert!(
            list.hardware
                .iter()
                .any(|h| h.item.contains(".drawer-side.c."))
        );
        assert!(list.warnings.iter().all(|w| !w.contains("box that fits")));
    }

    #[test]
    fn a_box_too_tall_for_its_drawer_says_so_and_fits_one_that_does() {
        let mut cab = Cabinet::new("D01", CabinetKind::DrawerBank).sectioned();
        let FrontLayout::Sections { columns } = &mut cab.fronts else {
            panic!("sections");
        };
        columns[0].rows[0].front = SectionFront::Drawers {
            count: 4,
            ratios: Vec::new(),
            system: None,
            box_height: Some(crate::model::BoxHeight::F),
            nominal_length: None,
        };
        let list = build_cabinet(&cab).unwrap();
        assert!(list.warnings.iter().any(|w| w.contains("box that fits")));
        assert!(list.part("D01.DrawerBottom1").is_some());
    }

    fn shoe_rack(count: u32, angle: f64, lip: f64) -> Cabinet {
        let mut cab = Cabinet::new("R01", CabinetKind::Wardrobe).sectioned();
        let FrontLayout::Sections { columns } = &mut cab.fronts else {
            panic!("sections");
        };
        columns[0].rows[0].inside = SectionInside::ShoeShelves { count, angle, lip };
        columns[0].rows[0].shelf_above = false;
        cab
    }

    #[test]
    fn a_sloped_shoe_shelf_rises_to_the_back_at_its_angle_and_stays_inside() {
        for angle in [5.0, 15.0] {
            let cab = shoe_rack(3, angle, 30.0);
            let list = build_cabinet(&cab).unwrap();
            let shelves: Vec<_> = list.by_role(PartRole::ShoeShelf).collect();
            assert_eq!(shelves.len(), 3);
            let back = list
                .by_role(PartRole::Back)
                .map(|b| b.bounds.min.y)
                .fold(f64::MAX, f64::min);
            let bottom = list.part("R01.Bottom").unwrap().bounds.max().z;
            for shelf in &shelves {
                let tilt = list.tilts[&shelf.name];
                assert_eq!(tilt.degrees, angle);
                let b = shelf.bounds;
                let low = tilt.apply(b.min);
                let high = tilt.apply(Vec3::new(b.min.x, b.max().y, b.min.z));
                let slope = ((high.z - low.z) / (high.y - low.y)).atan().to_degrees();
                assert!((slope - angle).abs() < 1e-9, "{slope}");
                let (min, max) = tilt.reach(b);
                assert!(max.y <= back + 1e-6, "{angle}: {} past {back}", max.y);
                assert!(min.z >= bottom, "{angle}: {} under the floor", min.z);
            }
            assert!(
                list.warnings.iter().all(|w| !w.contains("clash")),
                "{:?}",
                list.warnings
            );
        }
    }

    /// Cut as a flat board: its true length, longer than the depth it spans.
    #[test]
    fn a_sloped_shelfs_cut_size_is_its_length_along_the_slope() {
        let cab = shoe_rack(2, 15.0, 30.0);
        let list = build_cabinet(&cab).unwrap();
        let shelf = list.by_role(PartRole::ShoeShelf).next().unwrap();
        let tilt = list.tilts[&shelf.name];
        let spans = tilt.apply(shelf.bounds.max()).y - tilt.apply(shelf.bounds.min).y;
        assert!(shelf.bounds.size.y > spans);
        assert_eq!(shelf.bounds.size.z, cab.standard.front_thickness);
        assert_eq!(shelf.board, cab.standard.front_board, "decorative board");
    }

    #[test]
    fn each_sloped_shelf_has_a_toe_lip_on_its_low_edge_turned_with_it() {
        let cab = shoe_rack(3, 15.0, 40.0);
        let list = build_cabinet(&cab).unwrap();
        let shelves: Vec<_> = list.by_role(PartRole::ShoeShelf).collect();
        let lips: Vec<_> = list.by_role(PartRole::ShoeLip).collect();
        assert_eq!(lips.len(), shelves.len());
        for (shelf, lip) in shelves.iter().zip(&lips) {
            assert_eq!(list.tilts[&lip.name], list.tilts[&shelf.name]);
            assert_eq!(lip.bounds.max().y, shelf.bounds.min.y, "on its front edge");
            assert_eq!(
                lip.bounds.max().z,
                shelf.bounds.max().z + 40.0,
                "standing 40 above it"
            );
            assert_eq!(lip.bounds.size.x, shelf.bounds.size.x);
        }
    }

    #[test]
    fn sloped_shelves_are_screwed_through_the_sides_where_the_slope_lands() {
        let cab = shoe_rack(2, 15.0, 30.0);
        let list = build_cabinet(&cab).unwrap();
        let shelf = list.by_role(PartRole::ShoeShelf).next().unwrap();
        let tilt = list.tilts[&shelf.name];
        let pilots: Vec<Vec3> = list
            .operations
            .iter()
            .filter_map(|o| match o {
                Operation::Bore {
                    part,
                    at,
                    purpose: BorePurpose::ScrewPilot,
                    ..
                } if *part == shelf.name => Some(*at),
                _ => None,
            })
            .collect();
        assert_eq!(pilots.len(), 4);
        let side: Vec<Vec3> = list
            .operations
            .iter()
            .filter_map(|o| match o {
                Operation::Bore {
                    part,
                    at,
                    purpose: BorePurpose::Screw,
                    ..
                } if part == "R01.LeftSide" => Some(*at),
                _ => None,
            })
            .collect();
        for pilot in pilots.iter().filter(|p| p.x < 100.0) {
            let w = tilt.apply(*pilot);
            assert!(
                side.iter()
                    .any(|s| (s.y - w.y).abs() < 1e-6 && (s.z - w.z).abs() < 1e-6),
                "a hole in the side for the pilot at {w:?}"
            );
        }
    }

    #[test]
    fn too_many_shelves_warn_about_the_room_for_shoes() {
        let cab = shoe_rack(20, 15.0, 30.0);
        let list = build_cabinet(&cab).unwrap();
        assert!(list.warnings.iter().any(|w| w.contains("room for shoes")));
    }

    /// The owner: up to 8, for a 2400 high cabinet.
    #[test]
    fn a_tandem_back_runs_down_behind_the_base_and_takes_the_rear_hooks() {
        let mut cab = Cabinet::new("D01", CabinetKind::DrawerBank);
        cab.standard.drawer_system = DrawerSystem::Tandem;
        let list = build_cabinet(&cab).unwrap();
        let side = list.part("D01.DrawerLeftSide1").unwrap().bounds;
        let base = list.part("D01.DrawerBottom1").unwrap().bounds;
        let back = list.part("D01.DrawerBack1").unwrap().bounds;
        // Below the base, but clear of the runner under it.
        assert_eq!(back.min.z, side.min.z + 4.0);
        assert!(back.min.z < base.min.z);
        assert_eq!(base.max().y, back.min.y);
        assert_eq!(back.max().y, side.max().y);
        let holes: Vec<(Vec3, f64, f64)> = list
            .operations
            .iter()
            .filter_map(|o| match o {
                Operation::Bore {
                    part,
                    at,
                    diameter,
                    depth,
                    purpose: BorePurpose::RunnerHook,
                    ..
                } if part == "D01.DrawerBack1" => Some((*at, *diameter, *depth)),
                _ => None,
            })
            .collect();
        assert_eq!(holes.len(), 2);
        for (at, d, depth) in &holes {
            assert_eq!((*d, *depth), (6.0, 10.0));
            assert_eq!(at.z, side.min.z + 11.0);
            assert_eq!(at.y, back.max().y);
        }
        assert_eq!(holes[0].0.x, side.max().x + 7.0);
        assert_eq!(
            holes[1].0.x,
            list.part("D01.DrawerRightSide1").unwrap().bounds.min.x - 7.0
        );
    }

    #[test]
    fn a_movento_back_still_stands_on_the_base() {
        let mut cab = Cabinet::new("D01", CabinetKind::DrawerBank);
        cab.standard.drawer_system = DrawerSystem::Movento;
        let list = build_cabinet(&cab).unwrap();
        let base = list.part("D01.DrawerBottom1").unwrap().bounds;
        let back = list.part("D01.DrawerBack1").unwrap().bounds;
        assert_eq!(back.min.z, base.max().z);
        assert!(!list.operations.iter().any(|o| matches!(
            o,
            Operation::Bore {
                purpose: BorePurpose::RunnerHook,
                ..
            }
        )));
    }

    #[test]
    fn seen_end_boards_and_cladding_stay_flush_with_the_fronts_in_every_combination() {
        use crate::model::{DoorOverlay, DrawerFrame};
        let mut failures = Vec::new();
        for kind in [
            CabinetKind::Base,
            CabinetKind::DrawerBank,
            CabinetKind::Wall,
            CabinetKind::Tall,
            CabinetKind::Sink,
        ] {
            for overlay in [DoorOverlay::Full, DoorOverlay::Half, DoorOverlay::Inset] {
                for frame in [DrawerFrame::None, DrawerFrame::Board] {
                    for clad in [false, true] {
                        let mut cab = Cabinet::new("C1", kind);
                        cab.standard.overlay = overlay;
                        cab.standard.drawer_frame = frame;
                        cab.exposed.left = !clad;
                        cab.cladding.left = clad;
                        cab.cladding.thickness = 18.0;
                        let list = build_cabinet(&cab).unwrap();
                        let face = list
                            .parts
                            .iter()
                            .filter(|p| {
                                matches!(
                                    p.role,
                                    PartRole::Door | PartRole::DrawerFront | PartRole::LiftFront
                                )
                            })
                            .map(|p| p.bounds.min.y)
                            .fold(f64::MAX, f64::min);
                        let board = list.parts.iter().find(|p| p.role == PartRole::Cladding);
                        let board = board.unwrap();
                        if (board.bounds.min.y - face).abs() > 1e-6 {
                            failures.push(format!(
                                "{kind:?} {overlay:?} {frame:?} clad {clad}: {} at {} but fronts at {face}",
                                board.name, board.bounds.min.y
                            ));
                        }
                    }
                }
            }
        }
        assert!(failures.is_empty(), "{failures:#?}");
    }

    /// Every kind of front, in every mounting and style, with and without a drawer frame: the side
    /// boards reach the fronts' face, nothing stands in front of it, and nothing clashes.
    #[test]
    fn side_boards_meet_every_type_of_front() {
        use crate::model::{DoorOverlay, DoorStyle, DrawerFrame, HandlePlace, LiftKind};
        let door = || FrontLayout::Door {
            hinge_side: HingeSide::Left,
        };
        let drawers = || FrontLayout::Drawers {
            ratios: vec![1.0, 1.0, 2.0],
        };
        let row = |front: SectionFront| Section {
            height: 0.0,
            front,
            ..serde_json::from_str(r#"{}"#).unwrap()
        };
        let layouts: Vec<(&str, FrontLayout)> = vec![
            ("door", door()),
            ("pair", FrontLayout::DoorPair),
            ("drawers", drawers()),
            ("open", FrontLayout::Open),
            (
                "lift",
                FrontLayout::Lift {
                    kind: LiftKind::default(),
                    handle_weight: 0.5,
                },
            ),
            ("bifold", FrontLayout::BiFoldLift { handle_weight: 0.5 }),
            (
                "sections",
                FrontLayout::Sections {
                    columns: vec![
                        SectionColumn {
                            width: 0.0,
                            rows: vec![row(SectionFront::DoorPair), row(SectionFront::Open)],
                        },
                        SectionColumn {
                            width: 0.0,
                            rows: vec![row(SectionFront::Drawers {
                                count: 2,
                                ratios: Vec::new(),
                                system: None,
                                box_height: Default::default(),
                                nominal_length: Default::default(),
                            })],
                        },
                    ],
                },
            ),
        ];
        let mut failures = Vec::new();
        for (name, layout) in &layouts {
            for overlay in [DoorOverlay::Full, DoorOverlay::Half, DoorOverlay::Inset] {
                for frame in [DrawerFrame::None, DrawerFrame::Board] {
                    for style in [DoorStyle::Slab, DoorStyle::Shaker] {
                        for place in [HandlePlace::Corner, HandlePlace::Sharknose] {
                            let mut cab = Cabinet::new("C1", CabinetKind::Tall);
                            cab.height = 1200.0;
                            cab.width = 900.0;
                            cab.fronts = layout.clone();
                            cab.standard.overlay = overlay;
                            cab.standard.drawer_frame = frame;
                            cab.standard.door_style = style;
                            cab.standard.handle_place = place;
                            cab.exposed.left = true;
                            let what = format!("{name} {overlay:?} {frame:?} {style:?} {place:?}");
                            let list = match build_cabinet(&cab) {
                                Ok(l) => l,
                                Err(e) => {
                                    failures.push(format!("{what}: does not build: {e}"));
                                    continue;
                                }
                            };
                            let fronts = || {
                                list.parts.iter().filter(|p| {
                                    matches!(
                                        p.role,
                                        PartRole::Door
                                            | PartRole::DrawerFront
                                            | PartRole::LiftFront
                                            | PartRole::FrameStile
                                            | PartRole::FrameRail
                                    )
                                })
                            };
                            let face = fronts().map(|p| p.bounds.min.y).fold(f64::MAX, f64::min);
                            let carcass = list.part("C1.LeftSide").unwrap().bounds.min.y;
                            let expect = if face == f64::MAX { carcass } else { face };
                            let board = list.part("C1.SeenEndLeft").unwrap();
                            if (board.bounds.min.y - expect).abs() > 1e-6 {
                                failures.push(format!(
                                    "{what}: board at {} but the face at {expect}",
                                    board.bounds.min.y
                                ));
                            }
                            for p in list.parts.iter().filter(|p| p.bounds.min.y < expect - 1e-6) {
                                failures.push(format!(
                                    "{what}: {} at {} proud of {expect}",
                                    p.name, p.bounds.min.y
                                ));
                            }
                            for w in list.warnings.iter().filter(|w| w.starts_with("clash")) {
                                failures.push(format!("{what}: {w}"));
                            }
                        }
                    }
                }
            }
        }
        failures.sort();
        failures.dedup();
        assert!(
            failures.is_empty(),
            "{} failures\n{}",
            failures.len(),
            failures.join("\n")
        );
    }

    #[test]
    fn an_inset_bi_fold_warns_instead_of_drilling_above_its_top_edge() {
        let mut cab = Cabinet::new("W01", CabinetKind::Wall);
        cab.width = 900.0;
        cab.fronts = FrontLayout::BiFoldLift { handle_weight: 0.4 };
        cab.standard.overlay = crate::model::DoorOverlay::Inset;
        let list = build_cabinet(&cab).unwrap();
        assert!(
            list.warnings.iter().any(|w| w.contains("120° hinges")),
            "{:?}",
            list.warnings
        );
        assert!(
            !list.warnings.iter().any(|w| w.starts_with("clash")),
            "{:?}",
            list.warnings
        );
    }

    #[test]
    fn a_kick_board_between_notched_sides_screws_to_a_hidden_block() {
        let mut cab = Cabinet::new("B01", CabinetKind::Base);
        cab.standard.toe_kick = crate::model::ToeKick::IntegratedNotch;
        let list = build_cabinet(&cab).unwrap();
        let block = list.part("B01.PlinthBlock").unwrap();
        let (plinth, bottom) = (
            list.part("B01.Plinth").unwrap().bounds,
            list.part("B01.Bottom").unwrap().bounds,
        );
        // Against the board's back, under the bottom, between the sides.
        assert_eq!(block.bounds.min.y, plinth.max().y);
        assert_eq!(block.bounds.max().z, bottom.min.z);
        assert_eq!(block.bounds.min.x, plinth.min.x);
        assert_eq!(block.bounds.max().x, plinth.max().x);
        let pilots = |part: &str| -> Vec<(Dir, f64)> {
            list.operations
                .iter()
                .filter_map(|o| match o {
                    Operation::Bore {
                        part: p,
                        direction,
                        depth,
                        purpose: BorePurpose::ScrewPilot,
                        ..
                    } if p == part => Some((*direction, *depth)),
                    _ => None,
                })
                .collect()
        };
        // Blind in both: nothing comes through the board's face or into the cabinet.
        let into_plinth = pilots("B01.Plinth");
        assert!(!into_plinth.is_empty());
        assert!(into_plinth.iter().all(|(_, d)| *d < plinth.size.y));
        let into_bottom: Vec<_> = pilots("B01.Bottom")
            .into_iter()
            .filter(|(dir, _)| dir.axis == Axis::Z && dir.positive)
            .collect();
        assert!(!into_bottom.is_empty());
        assert!(into_bottom.iter().all(|(_, d)| *d < bottom.size.z));
        assert!(
            !list.warnings.iter().any(|w| w.starts_with("clash")),
            "{:?}",
            list.warnings
        );
    }

    #[test]
    fn a_plinth_block_fits_every_kind_mounting_and_frame_without_a_clash() {
        use crate::model::{DoorOverlay, DrawerFrame, ToeKick};
        let mut failures = Vec::new();
        for kind in [
            CabinetKind::Base,
            CabinetKind::DrawerBank,
            CabinetKind::Sink,
            CabinetKind::Tall,
            CabinetKind::BlindCorner,
            CabinetKind::CornerBase,
            CabinetKind::Wardrobe,
        ] {
            for width in [300.0, 600.0, 1200.0] {
                for overlay in [DoorOverlay::Full, DoorOverlay::Inset] {
                    for frame in [DrawerFrame::None, DrawerFrame::Board] {
                        let mut cab = Cabinet::new("B01", kind);
                        cab.width = width;
                        cab.standard.toe_kick = ToeKick::IntegratedNotch;
                        cab.standard.overlay = overlay;
                        cab.standard.drawer_frame = frame;
                        let Ok(list) = build_cabinet(&cab) else {
                            continue;
                        };
                        for w in list.warnings.iter().filter(|w| w.starts_with("clash")) {
                            failures.push(format!("{kind:?} {width} {overlay:?} {frame:?}: {w}"));
                        }
                    }
                }
            }
        }
        failures.sort();
        failures.dedup();
        assert!(
            failures.is_empty(),
            "{}\n{}",
            failures.len(),
            failures.join("\n")
        );
    }

    #[test]
    fn legs_take_no_plinth_block() {
        let list = build_cabinet(&Cabinet::new("B01", CabinetKind::Base)).unwrap();
        assert!(list.part("B01.PlinthBlock").is_none());
    }

    #[test]
    fn a_plinth_on_legs_clips_to_each_front_leg_from_behind() {
        let cab = Cabinet::new("B01", CabinetKind::Base);
        let list = build_cabinet(&cab).unwrap();
        let clips: Vec<_> = list
            .hardware
            .iter()
            .filter(|h| h.item == "hafele.axilo.plinth-clip.screw")
            .collect();
        let legs = list
            .hardware
            .iter()
            .filter(|h| h.item.starts_with("hafele.axilo.set."))
            .count();
        assert_eq!(clips.len(), legs / 2);
        let plinth = list.part("B01.Plinth").unwrap().bounds;
        assert!(clips.iter().all(|c| c.part == "B01.Plinth"));
        // On the plinth's back face, where it does not show.
        assert!(clips.iter().all(|c| c.at.y == plinth.max().y));
    }

    #[test]
    fn a_job_saved_with_a_plinth_box_opens_on_legs_and_plinth() {
        let kick: crate::model::ToeKick = serde_json::from_str(r#""plinth_box""#).unwrap();
        assert_eq!(kick, crate::model::ToeKick::LegsAndPlinth);
    }

    #[test]
    fn nothing_stands_proud_of_the_fronts_in_any_combination() {
        use crate::model::{DoorOverlay, DrawerFrame};
        let mut failures = Vec::new();
        for kind in [
            CabinetKind::Base,
            CabinetKind::DrawerBank,
            CabinetKind::Wall,
            CabinetKind::Tall,
            CabinetKind::Sink,
        ] {
            for overlay in [DoorOverlay::Full, DoorOverlay::Half, DoorOverlay::Inset] {
                for frame in [DrawerFrame::None, DrawerFrame::Board] {
                    let mut cab = Cabinet::new("C1", kind);
                    cab.standard.overlay = overlay;
                    cab.standard.drawer_frame = frame;
                    let list = build_cabinet(&cab).unwrap();
                    let face = list
                        .parts
                        .iter()
                        .filter(|p| {
                            matches!(
                                p.role,
                                PartRole::Door | PartRole::DrawerFront | PartRole::LiftFront
                            )
                        })
                        .map(|p| p.bounds.min.y)
                        .fold(f64::MAX, f64::min);
                    for p in &list.parts {
                        if p.bounds.min.y < face - 1e-6 {
                            failures.push(format!(
                                "{kind:?} {overlay:?} {frame:?}: {} at {} proud of {face}",
                                p.name, p.bounds.min.y
                            ));
                        }
                    }
                    for w in list.warnings.iter().filter(|w| w.starts_with("clash")) {
                        failures.push(format!("{kind:?} {overlay:?} {frame:?}: {w}"));
                    }
                }
            }
        }
        assert!(failures.is_empty(), "{failures:#?}");
    }

    #[test]
    fn a_seen_end_gets_a_finished_board_flush_with_the_fronts_face() {
        let mut cab = Cabinet::new("D01", CabinetKind::DrawerBank);
        cab.exposed.left = true;
        let list = build_cabinet(&cab).unwrap();
        let board = list.part("D01.SeenEndLeft").unwrap();
        let front = list.part("D01.DrawerFront1").unwrap().bounds;
        assert_eq!(board.bounds.min.y, front.min.y);
        assert_eq!(board.bounds.max().x, 0.0);
        assert_eq!(board.bounds.size.x, cab.standard.front_thickness);
        assert_eq!(board.bounds.max().y, cab.depth);
        assert_eq!(board.board, cab.standard.front_board);
        assert!(list.part("D01.SeenEndRight").is_none());
    }

    #[test]
    fn cam_locks_are_bored_from_hafeles_table_with_bolt_housing_and_cap_counted() {
        let mut cab = base();
        cab.standard.fixings = FixingRecipe::CamLock;
        let list = build_cabinet(&cab).unwrap();
        assert!(
            !list
                .warnings
                .iter()
                .any(|w| w.contains("cam") || w.contains("MINIFIX")),
            "{:?}",
            list.warnings
        );
        let bores = |want: BorePurpose| -> Vec<(String, f64, f64)> {
            list.operations
                .iter()
                .filter_map(|o| match o {
                    Operation::Bore {
                        part,
                        diameter,
                        depth,
                        purpose,
                        ..
                    } if *purpose == want => Some((part.clone(), *diameter, *depth)),
                    _ => None,
                })
                .collect()
        };
        let (cups, shanks, threads) = (
            bores(BorePurpose::CamHousing),
            bores(BorePurpose::CamShank),
            bores(BorePurpose::CamThread),
        );
        assert!(!cups.is_empty());
        assert_eq!(cups.len(), shanks.len());
        assert_eq!(cups.len(), threads.len());
        // 16 mm board: p. 3.6's 262.26.033.
        assert!(cups.iter().all(|c| (c.1, c.2) == (15.0, 12.5)), "{cups:?}");
        assert!(shanks.iter().all(|c| (c.1, c.2) == (8.0, 24.0)));
        assert!(threads.iter().all(|c| (c.1, c.2) == (5.0, 11.0)));
        assert!(threads.iter().all(|c| c.0.ends_with("Side")));
        let housing = minifix_housing(16.0).unwrap();
        for article in [
            "hafele.minifix15.from-16",
            MINIFIX_BOLT,
            minifix_cap(housing),
        ] {
            let n: u32 = list
                .hardware
                .iter()
                .filter(|h| h.item == article)
                .map(|h| h.quantity)
                .sum();
            assert_eq!(n as usize, cups.len(), "{article}");
        }
    }

    #[test]
    fn a_cam_housing_keeps_its_bolt_line_a_from_the_face_it_is_drilled_in() {
        let mut cab = base();
        cab.standard.fixings = FixingRecipe::CamLock;
        let list = build_cabinet(&cab).unwrap();
        let bottom = list.part(&format!("{}.Bottom", cab.id)).unwrap();
        let (mouth, shank) = list.operations.iter().fold((None, None), |acc, o| match o {
            Operation::Bore {
                part, at, purpose, ..
            } if part == &bottom.name => match purpose {
                BorePurpose::CamHousing if acc.0.is_none() => (Some(*at), acc.1),
                BorePurpose::CamShank if acc.1.is_none() => (acc.0, Some(*at)),
                _ => acc,
            },
            _ => acc,
        });
        let (mouth, shank) = (mouth.unwrap(), shank.unwrap());
        // Drilled from below, so nothing shows inside the cabinet.
        assert_eq!(mouth.z, bottom.bounds.min.z);
        assert_eq!(shank.z, bottom.bounds.min.z + 8.0);
        assert_eq!((mouth.x - shank.x).abs(), 24.0);
    }

    #[test]
    fn an_18_mm_board_takes_the_next_housing_and_a_15_the_one_below() {
        assert_eq!(minifix_housing(18.0).unwrap().article, "262.26.034");
        // 262.26.032 is struck out of the catalogue.
        assert_eq!(minifix_housing(15.0).unwrap().article, "262.26.031");
        assert_eq!(minifix_housing(33.0).unwrap().article, "262.26.036");
        // The 13 mm housing drives with PZ2 only, so takes the other cap.
        assert_eq!(
            minifix_cap(minifix_housing(15.0).unwrap()),
            "hafele.minifix.cap.pz2.white"
        );
    }

    #[test]
    fn eight_sloped_shelves_fit_a_2400_high_cabinet() {
        for angle in [5.0, 15.0] {
            let mut cab = shoe_rack(8, angle, 30.0);
            cab.height = 2400.0;
            let list = build_cabinet(&cab).unwrap();
            assert_eq!(list.by_role(PartRole::ShoeShelf).count(), 8);
            assert!(
                list.warnings
                    .iter()
                    .all(|w| !w.contains("shoe") && !w.contains("clash")),
                "{angle}: {:?}",
                list.warnings
            );
            let (_, ceiling) = (0.0, list.part("R01.LeftSide").unwrap().bounds.max().z);
            let top = list
                .by_role(PartRole::ShoeShelf)
                .map(|s| list.tilts[&s.name].reach(s.bounds).1.z)
                .fold(f64::MIN, f64::max);
            assert!(top <= ceiling, "{angle}: {top} over {ceiling}");
        }
    }

    #[test]
    fn a_flat_shoe_shelf_stays_a_fixed_shelf() {
        let cab = shoe_rack(3, 0.0, 30.0);
        let list = build_cabinet(&cab).unwrap();
        assert_eq!(list.by_role(PartRole::ShoeShelf).count(), 0);
        assert!(list.tilts.is_empty());
    }

    #[test]
    fn a_saved_job_with_flat_shoe_shelves_still_loads() {
        let inside: SectionInside =
            serde_json::from_str(r#"{"type":"shoe_shelves","count":2}"#).unwrap();
        assert_eq!(
            inside,
            SectionInside::ShoeShelves {
                count: 2,
                angle: 0.0,
                lip: 30.0
            }
        );
    }

    fn sliding_wardrobe(count: u32, overlap: f64) -> Cabinet {
        let mut cab = Cabinet::new("R01", CabinetKind::Wardrobe);
        cab.width = 1800.0;
        cab = cab.sectioned();
        let FrontLayout::Sections { columns } = &mut cab.fronts else {
            panic!("sections");
        };
        columns[0].rows[0].front = SectionFront::Sliding {
            count,
            overlap,
            track: "generic.sliding-track".into(),
        };
        cab
    }

    #[test]
    fn sliding_doors_overlap_on_two_planes_and_share_the_opening() {
        let list = build_cabinet(&sliding_wardrobe(2, 30.0)).unwrap();
        let doors: Vec<_> = list.by_role(PartRole::Door).map(|p| p.bounds).collect();
        assert_eq!(doors.len(), 2);
        assert!(
            (doors[0].max().x - doors[1].min.x - 30.0).abs() < 1e-9,
            "overlap"
        );
        assert!(
            doors[1].max().y <= doors[0].min.y,
            "the second passes in front"
        );
        assert_eq!(doors[0].size.x, doors[1].size.x);
        assert!(
            list.warnings.iter().all(|w| !w.contains("clash")),
            "{:?}",
            list.warnings
        );
    }

    #[test]
    fn three_sliding_doors_put_the_middle_one_in_front() {
        let list = build_cabinet(&sliding_wardrobe(3, 40.0)).unwrap();
        let doors: Vec<_> = list.by_role(PartRole::Door).map(|p| p.bounds).collect();
        assert_eq!(doors.len(), 3);
        assert_eq!(doors[0].min.y, doors[2].min.y);
        assert!(doors[1].max().y <= doors[0].min.y);
    }

    #[test]
    fn sliding_doors_hang_on_a_track_with_no_hinges_or_handles() {
        let mut cab = sliding_wardrobe(2, 30.0);
        cab.standard.opening = Opening::TipOn;
        let list = build_cabinet(&cab).unwrap();
        let count = |prefix: &str| {
            list.hardware
                .iter()
                .filter(|h| h.item.starts_with(prefix))
                .count()
        };
        assert_eq!(count("blum.clip-top"), 0);
        assert_eq!(count("blum.tip-on"), 0);
        assert_eq!(count("generic.sliding-track.top"), 1);
        assert_eq!(count("generic.sliding-track.bottom"), 1);
        assert_eq!(count("generic.sliding-track.roller"), 2);
        assert_eq!(count("generic.sliding-track.pull"), 2);
        let mut with_handles = sliding_wardrobe(2, 30.0);
        with_handles.standard.opening = Opening::Handles;
        let list = build_cabinet(&with_handles).unwrap();
        assert!(list.operations.iter().all(|o| !matches!(
            o,
            Operation::Bore {
                purpose: BorePurpose::HandleScrew,
                ..
            }
        )));
        assert!(
            list.hardware
                .iter()
                .any(|h| h.description.contains("cut to"))
        );
    }

    #[test]
    fn sliding_doors_too_narrow_are_left_out_and_say_so() {
        let mut cab = sliding_wardrobe(3, 30.0);
        cab.width = 500.0;
        let list = build_cabinet(&cab).unwrap();
        assert_eq!(list.by_role(PartRole::Door).count(), 0);
        assert!(list.warnings.iter().any(|w| w.contains("too narrow")));
    }

    fn framed_bank() -> Cabinet {
        let mut cab = Cabinet::new("D01", CabinetKind::DrawerBank).sectioned();
        cab.standard.drawer_frame = DrawerFrame::Board;
        cab
    }

    fn frame_openings(list: &PartList) -> Vec<Aabb> {
        list.operations
            .iter()
            .filter_map(|o| match o {
                Operation::Cutout { part, bounds, .. } if part == "D01.DrawerFrame" => {
                    Some(*bounds)
                }
                _ => None,
            })
            .collect()
    }

    /// The owner's installed sharknose drawers: one board, an opening per drawer, the rails
    /// closing the gaps between the fronts.
    #[test]
    fn a_drawer_frame_is_one_board_with_an_opening_for_each_drawer() {
        let cab = framed_bank();
        let list = build_cabinet(&cab).unwrap();
        let frame = list.part("D01.DrawerFrame").expect("the frame");
        let t = cab.standard.drawer_frame_thickness;
        assert_eq!((frame.bounds.min.y, frame.bounds.max().y), (0.0, t));
        assert_eq!((frame.bounds.min.x, frame.bounds.max().x), (0.0, cab.width));
        assert_eq!(frame.bounds.max().z, cab.height);
        let fronts: Vec<_> = list.by_role(PartRole::DrawerFront).collect();
        let openings = frame_openings(&list);
        assert_eq!(openings.len(), fronts.len());
        let side = cab.standard.carcass_thickness;
        for (front, hole) in fronts.iter().zip(&openings) {
            assert_eq!((hole.min.x, hole.max().x), (side, cab.width - side));
            assert_eq!((hole.min.y, hole.max().y), (0.0, t));
            assert!(
                hole.max().z < front.bounds.max().z,
                "a rail behind the front"
            );
            assert!(hole.min.z >= frame.bounds.min.z);
        }
        // A rail between each pair of openings, covering the gap between their fronts.
        for pair in openings.windows(2) {
            assert!(pair[0].min.z > pair[1].max().z, "a rail between them");
        }
        assert!(
            list.warnings
                .iter()
                .all(|w| !w.contains("frame") && !w.contains("clash")),
            "{:?}",
            list.warnings
        );
    }

    fn sharknose_door(edge: SharknoseEdge) -> Cabinet {
        let mut cab = Cabinet::new("B01", CabinetKind::Base).sectioned();
        cab.standard.handle_place = crate::model::HandlePlace::Sharknose;
        cab.standard.sharknose_door_edge = edge;
        cab
    }

    fn grip_rails(list: &PartList) -> Vec<Aabb> {
        list.by_role(PartRole::GripRail).map(|p| p.bounds).collect()
    }

    /// The owner's picture: a dark slit under the top rail, down the door's bevel.
    #[test]
    fn a_sharknose_door_gets_a_rail_behind_its_top_edge_only() {
        let cab = sharknose_door(SharknoseEdge::Auto);
        let list = build_cabinet(&cab).unwrap();
        let door = list.part("B01.Door1").unwrap().bounds;
        let rails = grip_rails(&list);
        assert_eq!(rails.len(), 1);
        let rail = rails[0];
        let t = cab.standard.carcass_thickness;
        assert_eq!((rail.min.x, rail.max().x), (t, cab.width - t));
        assert_eq!(rail.min.y, door.max().y, "behind the door");
        assert_eq!(rail.min.z, door.max().z - cab.standard.drawer_frame_overlap);
        assert!(rail.max().z <= cab.height - t, "up to the carcass");
        assert!(
            list.warnings.iter().all(|w| !w.contains("clash")),
            "{:?}",
            list.warnings
        );
    }

    #[test]
    fn a_door_can_be_gripped_on_its_bottom_edge() {
        let list = build_cabinet(&sharknose_door(SharknoseEdge::Bottom)).unwrap();
        let door = list.part("B01.Door1").unwrap().bounds;
        let rail = grip_rails(&list)[0];
        assert_eq!(rail.max().z, door.min.z + 25.0);
        assert!(rail.size.z < 25.0, "only what the bottom panel leaves");
        assert!(list.operations.iter().any(|o| matches!(
            o,
            Operation::Chamfer { part, at, .. } if part == "B01.Door1" && at.z == door.min.z
        )));
        let top_gap = sharknose_door(SharknoseEdge::Bottom).height - door.max().z;
        assert!(top_gap < 15.0, "no finger gap at the top");
    }

    #[test]
    fn a_door_gripped_on_its_opening_side_has_a_rail_only_down_that_side() {
        let cab = sharknose_door(SharknoseEdge::Opening);
        let plain = build_cabinet(&sharknose_door(SharknoseEdge::Auto)).unwrap();
        let list = build_cabinet(&cab).unwrap();
        let door = list.part("B01.Door1").unwrap().bounds;
        let s = &cab.standard;
        assert!(
            (plain.part("B01.Door1").unwrap().bounds.size.x
                - door.size.x
                - (s.sharknose_gap - s.front_gap))
                .abs()
                < 1e-9,
            "the finger gap comes off the free edge"
        );
        let rails = grip_rails(&list);
        assert_eq!(rails.len(), 1);
        let rail = rails[0];
        assert_eq!(
            rail.max().x,
            cab.width - s.carcass_thickness,
            "at the far side"
        );
        assert_eq!(rail.min.x, door.max().x - s.drawer_frame_overlap);
        assert!(rail.size.z > door.size.z * 0.8, "down the door's height");
        assert!(
            list.by_role(PartRole::Shelf)
                .all(|p| p.bounds.min.y >= rail.max().y),
            "shelves stand behind it"
        );
        assert!(
            list.warnings.iter().all(|w| !w.contains("clash")),
            "{:?}",
            list.warnings
        );
    }

    #[test]
    fn a_door_pair_gripped_where_it_meets_gets_a_rail_each() {
        let mut cab = Cabinet::new("B01", CabinetKind::Base);
        cab.fronts = FrontLayout::DoorPair;
        cab.standard.handle_place = crate::model::HandlePlace::Sharknose;
        cab.standard.sharknose_door_edge = SharknoseEdge::Opening;
        let list = build_cabinet(&cab).unwrap();
        let rails = grip_rails(&list);
        assert_eq!(rails.len(), 2);
        assert!(
            (rails[0].max().x - rails[1].min.x).abs() < 1e-9,
            "they meet"
        );
        assert!((rails[0].max().x - cab.width / 2.0).abs() < 1e-9);
    }

    #[test]
    fn doors_with_handles_get_no_rail() {
        let list = build_cabinet(&Cabinet::new("B01", CabinetKind::Base).sectioned()).unwrap();
        assert!(grip_rails(&list).is_empty());
    }

    #[test]
    fn a_drawer_frame_works_with_half_overlay_fronts() {
        let mut cab = framed_bank();
        cab.standard.overlay = DoorOverlay::Half;
        let list = build_cabinet(&cab).unwrap();
        let frame = list.part("D01.DrawerFrame").expect("the frame").bounds;
        let t = cab.standard.drawer_frame_thickness;
        assert_eq!((frame.min.y, frame.max().y), (0.0, t));
        let side = list.part("D01.LeftSide").unwrap().bounds;
        assert_eq!(side.min.y, t, "the carcass moves back");
        assert_eq!(frame_openings(&list).len(), 3);
    }

    #[test]
    fn an_inset_drawer_frame_is_rails_between_the_sides_behind_the_fronts() {
        let mut cab = framed_bank();
        cab.standard.overlay = DoorOverlay::Inset;
        let list = build_cabinet(&cab).unwrap();
        let rails: Vec<_> = list.by_role(PartRole::DrawerFrame).collect();
        assert_eq!(rails.len(), 3, "one over each of three drawers");
        let (ft, t) = (
            cab.standard.front_thickness,
            cab.standard.drawer_frame_thickness,
        );
        let side = list.part("D01.LeftSide").unwrap().bounds;
        assert_eq!(side.min.y, 0.0, "the carcass stays");
        for rail in &rails {
            assert_eq!((rail.bounds.min.y, rail.bounds.max().y), (ft, ft + t));
            assert_eq!(rail.bounds.min.x, side.max().x, "between the sides");
        }
        let front = list.part("D01.DrawerFront1").unwrap().bounds;
        assert_eq!(front.max().y, ft);
        for n in 1..=3 {
            let f = list.part(&format!("D01.DrawerFront{n}")).unwrap().bounds;
            assert!(
                rails.iter().any(|r| r.bounds.min.z < f.max().z
                    && r.bounds.min.z > f.max().z - cab.standard.drawer_frame_overlap - 1.0),
                "a rail behind drawer {n}'s top"
            );
        }
        assert!(
            list.warnings
                .iter()
                .all(|w| !w.contains("clash") && !w.contains("frame")),
            "{:?}",
            list.warnings
        );
    }

    /// The owner's inset test: hinges and runner ends stood proud of the fronts' faces.
    #[test]
    fn inset_fronts_keep_blums_models_behind_their_back_face() {
        for (kind, name) in [
            (CabinetKind::Base, "B01.Door1"),
            (CabinetKind::DrawerBank, "D01.DrawerFront1"),
        ] {
            let mut cab = Cabinet::new(&name[..3], kind);
            cab.standard.overlay = DoorOverlay::Inset;
            let list = build_cabinet(&cab).unwrap();
            let back = list.part(name).unwrap().bounds.max().y;
            let places = crate::cad::cad_placements(&list);
            let blum: Vec<_> = places
                .iter()
                .filter(|p| p.frame == crate::cad::Frame::Blum)
                .collect();
            assert!(!blum.is_empty());
            assert!(blum.iter().all(|p| p.origin.y == back), "{name}");
        }
    }

    #[test]
    fn inset_sharknose_fronts_leave_their_finger_gap_at_the_top() {
        let mut cab = framed_bank();
        cab.standard.overlay = DoorOverlay::Inset;
        cab.standard.handle_place = crate::model::HandlePlace::Sharknose;
        let list = build_cabinet(&cab).unwrap();
        let top_rail = list
            .by_role(PartRole::DrawerFrame)
            .map(|p| p.bounds.max().z)
            .fold(0.0, f64::max);
        let top = list.part("D01.DrawerFront1").unwrap().bounds.max().z;
        assert_eq!(top_rail - top, cab.standard.sharknose_gap);
    }

    /// The owner's picture: over a sharknose's finger gap, the inside showed past the rail, down
    /// the bevel.
    #[test]
    fn a_drawer_frames_rail_comes_down_behind_the_front_past_a_sharknose_bevel() {
        let mut cab = framed_bank();
        cab.standard.handle_place = crate::model::HandlePlace::Sharknose;
        let list = build_cabinet(&cab).unwrap();
        let overlap = cab.standard.drawer_frame_overlap;
        let fronts: Vec<_> = list.by_role(PartRole::DrawerFront).collect();
        for (front, hole) in fronts.iter().zip(&frame_openings(&list)) {
            assert!((front.bounds.max().z - overlap - hole.max().z).abs() < 1e-9);
            let bevel = list
                .operations
                .iter()
                .find_map(|o| match o {
                    Operation::Chamfer { part, bounds, .. } if *part == front.name => Some(*bounds),
                    _ => None,
                })
                .expect("a bevel");
            assert!(hole.max().z <= bevel.min.z, "the rail covers the bevel");
        }
        assert!(list.warnings.iter().all(|w| !w.contains("rails")));
        // Too short a rail says so.
        cab.standard.drawer_frame_overlap = 5.0;
        let short = build_cabinet(&cab).unwrap();
        assert!(
            short
                .warnings
                .iter()
                .any(|w| w.contains("rails stop above"))
        );
    }

    #[test]
    fn a_framed_drawers_box_stays_under_the_rail() {
        for overlap in [0.0, 25.0, 40.0] {
            let mut cab = framed_bank();
            cab.standard.drawer_frame_overlap = overlap;
            let list = build_cabinet(&cab).unwrap();
            for (n, hole) in frame_openings(&list).iter().enumerate() {
                let back = list.part(&format!("D01.DrawerBack{}", n + 1)).unwrap();
                assert!(back.bounds.max().z <= hole.max().z, "drawer {}", n + 1);
            }
        }
    }

    #[test]
    fn a_framed_cabinets_plinth_stays_in_line_with_its_neighbours() {
        let framed = framed_bank();
        let mut plain = framed.clone();
        plain.standard.drawer_frame = DrawerFrame::None;
        let y = |c: &Cabinet| {
            build_cabinet(c)
                .unwrap()
                .part("D01.Plinth")
                .unwrap()
                .bounds
                .min
                .y
        };
        assert_eq!(y(&framed), y(&plain));
    }

    /// The owner's call: the carcass makes room, so nothing around the cabinet moves.
    #[test]
    fn a_drawer_frame_moves_the_carcass_back_and_leaves_the_fronts_and_back_alone() {
        let framed = framed_bank();
        let mut plain = framed.clone();
        plain.standard.drawer_frame = DrawerFrame::None;
        let (with, without) = (
            build_cabinet(&framed).unwrap(),
            build_cabinet(&plain).unwrap(),
        );
        let t = framed.standard.drawer_frame_thickness;
        let (a, b) = (
            with.part("D01.DrawerFront1").unwrap(),
            without.part("D01.DrawerFront1").unwrap(),
        );
        assert_eq!(a.bounds.min.y, b.bounds.min.y, "the fronts stay");
        assert_eq!(a.bounds.max().y, b.bounds.max().y);
        let (a, b) = (
            with.part("D01.LeftSide").unwrap(),
            without.part("D01.LeftSide").unwrap(),
        );
        assert_eq!(a.bounds.min.y, b.bounds.min.y + t, "the sides move back");
        assert_eq!(a.bounds.max().y, b.bounds.max().y, "to the same back");
        let bottom = |l: &PartList| l.part("D01.Bottom").unwrap().bounds.min.y;
        assert_eq!(
            bottom(&with),
            bottom(&without) + t,
            "and the bottom with them"
        );
    }

    #[test]
    fn framed_drawers_run_through_their_openings_and_keep_their_hardware_on_the_front() {
        let mut framed = framed_bank();
        framed.standard.opening = Opening::ServoDrive;
        let list = build_cabinet(&framed).unwrap();
        let base = list.part("D01.DrawerBottom1").unwrap();
        assert_eq!(base.bounds.min.y, 0.0, "the box starts at the front's back");
        let hole = frame_openings(&list)[0];
        assert!(hole.min.z <= base.bounds.min.z && base.bounds.max().z <= hole.max().z);
        let front = list.part("D01.DrawerFront1").unwrap();
        let places = crate::cad::cad_placements(&list);
        let bumper = places
            .iter()
            .find(|p| p.name.contains("ServoBumper"))
            .unwrap();
        assert_eq!(bumper.origin.y, front.bounds.max().y);
        let nl = crate::standins::nominal_length(&list, &front.name).unwrap();
        let unit = places
            .iter()
            .find(|p| p.name == "D01.Servo1")
            .expect("the drive unit");
        assert_eq!(unit.origin.y, nl + 19.0);
        let back = list.part("D01.LeftSide").unwrap().bounds.max().y;
        assert!(unit.origin.y < back, "inside the carcass");
    }

    #[test]
    fn a_drawer_frame_is_left_off_a_cabinet_with_doors_and_says_so() {
        let mut cab = Cabinet::new("B01", CabinetKind::Base).sectioned();
        cab.standard.drawer_frame = DrawerFrame::Board;
        let list = build_cabinet(&cab).unwrap();
        assert!(list.part("B01.DrawerFrame").is_none());
        assert!(list.warnings.iter().any(|w| w.contains("drawer frame")));
        let door = list.by_role(PartRole::Door).next().unwrap();
        assert_eq!(door.bounds.max().y, 0.0, "the doors stay on the carcass");
    }

    #[test]
    fn an_open_section_keeps_its_opening_in_the_frame() {
        let mut cab = framed_bank();
        let FrontLayout::Sections { columns } = &mut cab.fronts else {
            panic!("sections");
        };
        let mut open = columns[0].rows[0].clone();
        open.front = SectionFront::Open;
        columns[0].rows.push(open);
        let list = build_cabinet(&cab).unwrap();
        let drawers = list.by_role(PartRole::DrawerFront).count();
        let openings = frame_openings(&list);
        assert_eq!(openings.len(), drawers + 1);
        // The open section's opening is its clear height, from the drawers up to the top.
        let tallest = openings.iter().map(|o| o.max().z).fold(0.0, f64::max);
        assert!(
            tallest
                > list
                    .by_role(PartRole::DrawerFront)
                    .map(|f| f.bounds.max().z)
                    .fold(0.0, f64::max)
        );
    }

    #[test]
    fn a_sharknose_angle_sets_how_far_the_bevel_runs_down() {
        use crate::model::SharknoseAngle;
        let down = |angle| {
            let mut cab = Cabinet::new("B01", CabinetKind::Base);
            cab.standard.handle_place = HandlePlace::Sharknose;
            cab.standard.sharknose_angle = angle;
            let list = build_cabinet(&cab).unwrap();
            list.operations
                .iter()
                .find_map(|o| match o {
                    Operation::Chamfer { bounds, .. } => Some(bounds.size.z),
                    _ => None,
                })
                .unwrap()
        };
        assert!((down(SharknoseAngle::Bevel45) - SHARKNOSE_INTO).abs() < 1e-9);
        assert!(down(SharknoseAngle::Bevel30) > down(SharknoseAngle::Bevel45));
        assert!(down(SharknoseAngle::Bevel60) < down(SharknoseAngle::Bevel45));
    }

    #[test]
    fn a_thin_sharknose_front_keeps_a_nose_at_its_face() {
        let mut cab = Cabinet::new("B01", CabinetKind::Base);
        cab.standard.handle_place = HandlePlace::Sharknose;
        cab.standard.front_thickness = 12.0;
        let list = build_cabinet(&cab).unwrap();
        let into = list.operations.iter().find_map(|o| match o {
            Operation::Chamfer { bounds, .. } => Some(bounds.size.y),
            _ => None,
        });
        assert_eq!(into, Some(12.0 - SHARKNOSE_NOSE));
    }

    /// Without room above it a reverse bevel can't be gripped: 3 mm stops fingers.
    #[test]
    fn sharknose_fronts_leave_a_finger_gap_above_each_front() {
        let mut bank = Cabinet::new("D01", CabinetKind::DrawerBank).sectioned();
        bank.standard.handle_place = HandlePlace::Sharknose;
        bank.standard.sharknose_gap = 18.0;
        let list = build_cabinet(&bank).unwrap();
        let mut fronts: Vec<_> = list.by_role(PartRole::DrawerFront).collect();
        fronts.sort_by(|a, b| a.bounds.min.z.total_cmp(&b.bounds.min.z));
        for pair in fronts.windows(2) {
            let gap = pair[1].bounds.min.z - pair[0].bounds.max().z;
            assert!((gap - 18.0).abs() < 1e-6, "{gap}");
        }
        let top = fronts.last().unwrap().bounds.max().z;
        assert!(
            (bank.height - top - 18.0).abs() < 1e-6,
            "under the benchtop"
        );
        // Handles keep the usual gaps.
        bank.standard.handle_place = HandlePlace::Corner;
        let list = build_cabinet(&bank).unwrap();
        let mut fronts: Vec<_> = list.by_role(PartRole::DrawerFront).collect();
        fronts.sort_by(|a, b| a.bounds.min.z.total_cmp(&b.bounds.min.z));
        let gap = fronts[1].bounds.min.z - fronts[0].bounds.max().z;
        assert!((gap - bank.standard.front_gap).abs() < 1e-6);
    }

    #[test]
    fn a_framed_sharknose_front_warns_about_sharknose_not_finger_pulls() {
        let mut cab = Cabinet::new("B01", CabinetKind::Base);
        cab.standard.handle_place = HandlePlace::Sharknose;
        cab.standard.door_style = DoorStyle::Shaker;
        let list = build_cabinet(&cab).unwrap();
        assert!(
            list.warnings
                .iter()
                .any(|w| w.starts_with("sharknose bevels need slab fronts")),
            "{:?}",
            list.warnings
        );
    }

    /// WF-3: a sharknose front is bevelled at the back of its top edge (a wall door's bottom),
    /// across its width, and gets no handle.
    #[test]
    fn sharknose_fronts_are_bevelled_at_the_back_of_their_gripping_edge() {
        for kind in [
            CabinetKind::Base,
            CabinetKind::Wall,
            CabinetKind::DrawerBank,
        ] {
            let mut cab = Cabinet::new("X01", kind);
            cab.standard.handle_place = HandlePlace::Sharknose;
            let list = build_cabinet(&cab).unwrap();
            assert!(!list.hardware.iter().any(|h| h.item.starts_with("handle.")));
            let fronts: Vec<_> = list
                .parts
                .iter()
                .filter(|p| matches!(p.role, PartRole::Door | PartRole::DrawerFront))
                .collect();
            assert!(!fronts.is_empty());
            for f in fronts {
                let (at, b) = list
                    .operations
                    .iter()
                    .find_map(|o| match o {
                        Operation::Chamfer { part, at, bounds } if *part == f.name => {
                            Some((*at, *bounds))
                        }
                        _ => None,
                    })
                    .unwrap_or_else(|| panic!("{}", f.name));
                let fb = f.bounds;
                assert_eq!((b.min.x, b.max().x), (fb.min.x, fb.max().x), "{}", f.name);
                // At the back face (the cabinet side of the front).
                assert_eq!(at.y, fb.max().y);
                let edge = if kind == CabinetKind::Wall {
                    fb.min.z
                } else {
                    fb.max().z
                };
                assert_eq!(at.z, edge, "{}", f.name);
                assert_eq!(b.size.y, SHARKNOSE_INTO);
                assert!(
                    (b.size.z - SHARKNOSE_INTO * 3f64.sqrt()).abs() < 1e-9,
                    "30° from the face"
                );
            }
        }
    }

    /// WARD-1: the hat shelf, rails, lower shelf and drawer heights come from the standard; a
    /// lower shelf crowding the upper rail falls back to long hanging with a note.
    #[test]
    fn wardrobe_sizes_follow_the_standard() {
        let mut cab = Cabinet::new("R01", CabinetKind::Wardrobe);
        cab.wardrobe = Hanging::Double;
        cab.inner_drawers = 2;
        cab.standard.wardrobe_hat_shelf_drop = 400.0;
        cab.standard.wardrobe_rail_drop = 70.0;
        cab.standard.wardrobe_double_hang_shelf = 1100.0;
        cab.standard.wardrobe_drawer_pitch = 150.0;
        let list = build_cabinet(&cab).unwrap();
        let t = cab.standard.carcass_thickness;
        assert_eq!(
            list.part("R01.Shelf1").unwrap().bounds.max().z,
            2100.0 - 400.0
        );
        assert_eq!(list.part("R01.Shelf2").unwrap().bounds.min.z, 1100.0);
        let rails: Vec<f64> = list
            .hardware
            .iter()
            .filter(|h| h.item == "wardrobe-rail.oval")
            .map(|h| h.at.z)
            .collect();
        assert_eq!(rails, [2100.0 - 400.0 - t - 70.0, 1100.0 - 70.0]);
        let drawers_top = list
            .by_role(PartRole::DrawerBack)
            .map(|p| p.bounds.max().z)
            .fold(f64::MIN, f64::max);
        assert!(drawers_top < list.part("R01.Shelf3").unwrap().bounds.min.z);
        // The top drawer starts one 150 mm drawer up instead of 200; its box is the system's.
        let mut tall = cab.clone();
        tall.standard.wardrobe_drawer_pitch = 200.0;
        let tall_top = build_cabinet(&tall)
            .unwrap()
            .by_role(PartRole::DrawerBack)
            .map(|p| p.bounds.max().z)
            .fold(f64::MIN, f64::max);
        assert!(
            (tall_top - drawers_top - 50.0).abs() < 1e-9,
            "{tall_top} {drawers_top}"
        );

        cab.standard.wardrobe_double_hang_shelf = 1500.0;
        let list = build_cabinet(&cab).unwrap();
        assert!(list.warnings.iter().any(|w| w.contains("upper rail")));
        assert_eq!(
            list.hardware
                .iter()
                .filter(|h| h.item == "wardrobe-rail.oval")
                .count(),
            1
        );
    }

    /// P2-3b: inner drawers at a wardrobe's foot under a fixed shelf; double hanging keeps its
    /// lower rail while there is room.
    #[test]
    fn a_wardrobe_takes_inner_drawers_at_its_foot() {
        let mut cab = Cabinet::new("R01", CabinetKind::Wardrobe);
        cab.inner_drawers = 2;
        cab.wardrobe = Hanging::Double;
        let list = build_cabinet(&cab).unwrap();
        assert!(
            !list
                .warnings
                .iter()
                .any(|w| w.contains("clash") || w.contains("no room")),
            "{:?}",
            list.warnings
        );
        assert_eq!(list.by_role(PartRole::DrawerBottom).count(), 2);
        assert_eq!(list.by_role(PartRole::Shelf).count(), 3);
        let top_of_drawers = list
            .by_role(PartRole::DrawerBack)
            .map(|p| p.bounds.max().z)
            .fold(f64::MIN, f64::max);
        let shelf = list.part("R01.Shelf3").unwrap();
        assert!(shelf.bounds.min.z > top_of_drawers);
        // Doors in front of inner drawers hang on 0-protrusion hinges.
        assert!(
            list.hardware
                .iter()
                .any(|h| h.item == "blum.clip-top-blumotion.155.overlay")
        );
        cab.inner_drawers = 4;
        let list = build_cabinet(&cab).unwrap();
        assert!(list.warnings.iter().any(|w| w.contains("no room")));
    }

    fn oven_tower() -> Cabinet {
        let mut cab = Cabinet::new("T01", CabinetKind::Tall);
        cab.adjustable_shelves = 0;
        cab.fronts = FrontLayout::Stack {
            sections: vec![
                StackSection {
                    height: 0.0,
                    kind: SectionKind::Drawers { count: 2 },
                },
                StackSection {
                    height: 600.0,
                    kind: SectionKind::Niche,
                },
                StackSection {
                    height: 0.0,
                    kind: SectionKind::Door {
                        hinge_side: HingeSide::Left,
                    },
                },
            ],
        };
        cab
    }

    fn section(height: f64, front: SectionFront, inside: SectionInside) -> Section {
        Section {
            height,
            front,
            inside,
            shelf_above: true,
            opening: None,
        }
    }

    /// SEC-1: a 900 base, drawers on the left and a door on the right, split by a divider.
    fn sectioned_base() -> Cabinet {
        let mut cab = base();
        cab.width = 900.0;
        cab.adjustable_shelves = 0;
        cab.fronts = FrontLayout::Sections {
            columns: vec![
                SectionColumn {
                    width: 450.0,
                    rows: vec![section(0.0, SectionFront::drawers(3), SectionInside::Empty)],
                },
                SectionColumn {
                    width: 0.0,
                    rows: vec![section(
                        0.0,
                        SectionFront::Door {
                            hinge_side: HingeSide::Left,
                        },
                        SectionInside::shelves(1),
                    )],
                },
            ],
        };
        cab
    }

    fn quiet(list: &PartList) {
        assert!(
            !list
                .warnings
                .iter()
                .any(|w| w.contains("clash") || w.contains("need") || w.contains("left out")),
            "{:?}",
            list.warnings
        );
    }

    /// SEC-3: the app only edits sections, so every old layout must convert to the same cabinet:
    /// the same parts, machining, hardware and notes, whichever order they were made in.
    #[test]
    fn every_old_layout_builds_the_same_cabinet_as_its_sections() {
        use crate::model::{LiftKind, SectionKind};
        let sorted = |list: &PartList| {
            let v = serde_json::to_value(list).unwrap();
            let mut out = Vec::new();
            for key in ["parts", "operations", "hardware", "warnings"] {
                let mut rows: Vec<String> = v[key]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|r| r.to_string())
                    .collect();
                rows.sort();
                out.push(rows);
            }
            out
        };
        let hinge_side = HingeSide::Right;
        let layouts = [
            FrontLayout::Open,
            FrontLayout::Door { hinge_side },
            FrontLayout::DoorPair,
            FrontLayout::Drawers {
                ratios: vec![1.0, 1.0, 1.0],
            },
            FrontLayout::Drawers {
                ratios: vec![1.0, 2.0],
            },
            FrontLayout::Lift {
                kind: LiftKind::StayLift,
                handle_weight: 0.4,
            },
            FrontLayout::BiFoldLift { handle_weight: 0.4 },
            FrontLayout::Stack {
                sections: vec![
                    StackSection {
                        height: 0.0,
                        kind: SectionKind::Drawers { count: 2 },
                    },
                    StackSection {
                        height: 600.0,
                        kind: SectionKind::Niche,
                    },
                    StackSection {
                        height: 0.0,
                        kind: SectionKind::Door { hinge_side },
                    },
                ],
            },
        ];
        let mut compared = 0;
        for kind in CabinetKind::ALL {
            for layout in &layouts {
                let lifting = matches!(
                    layout,
                    FrontLayout::Lift { .. } | FrontLayout::BiFoldLift { .. }
                );
                for shelves in [0, 2] {
                    let mut old = Cabinet::new("C01", kind);
                    old.fronts = layout.clone();
                    // A front's shelves: drawers and stacks change what was behind them.
                    let room = !matches!(
                        layout,
                        FrontLayout::Drawers { .. } | FrontLayout::Stack { .. }
                    );
                    old.adjustable_shelves = if room { shelves } else { 0 };
                    if lifting && kind != CabinetKind::Wall {
                        continue;
                    }
                    let new = old.clone().sectioned();
                    let (Ok(a), Ok(b)) = (build_cabinet(&old), build_cabinet(&new)) else {
                        continue;
                    };
                    let (a, b) = (sorted(&a), sorted(&b));
                    for (x, what) in a.iter().zip(&b).zip(["parts", "ops", "hardware", "notes"]) {
                        let (x, y) = x;
                        let only_a: Vec<_> = x.iter().filter(|r| !y.contains(r)).collect();
                        let only_b: Vec<_> = y.iter().filter(|r| !x.contains(r)).collect();
                        assert!(
                            only_a.is_empty() && only_b.is_empty(),
                            "{kind:?} {layout:?} shelves {shelves}: {what}\n only old: {only_a:?}\n only new: {only_b:?}"
                        );
                    }
                    compared += 1;
                }
            }
        }
        assert!(compared > 40, "{compared}");
    }

    /// The owner's test: picking drawers left an adjustable shelf behind them. Sections have none.
    #[test]
    fn a_drawer_cabinet_converted_to_sections_has_no_shelf_behind_its_drawers() {
        let mut cab = base();
        cab.fronts = FrontLayout::Drawers {
            ratios: vec![1.0, 1.0],
        };
        cab.adjustable_shelves = 1;
        let list = build_cabinet(&cab.sectioned()).unwrap();
        assert_eq!(list.by_role(PartRole::Shelf).count(), 0);
        assert_eq!(list.by_role(PartRole::DrawerFront).count(), 2);
    }

    /// As in Blum's configurator: splitting the fronts implies no shelf; the owner adds one.
    #[test]
    fn a_split_between_sections_makes_a_fixed_shelf_only_when_asked() {
        let rows = |shelf_above| {
            let mut cab = base();
            cab.adjustable_shelves = 0;
            cab.fronts = FrontLayout::Sections {
                columns: vec![SectionColumn {
                    width: 0.0,
                    rows: vec![
                        Section {
                            shelf_above,
                            ..section(0.0, SectionFront::drawers(1), SectionInside::Empty)
                        },
                        section(0.0, SectionFront::drawers(1), SectionInside::Empty),
                    ],
                }],
            };
            build_cabinet(&cab).unwrap()
        };
        assert_eq!(rows(false).by_role(PartRole::Shelf).count(), 0);
        assert_eq!(rows(true).by_role(PartRole::Shelf).count(), 1);
        // Both drawers get a box either way, and more room without the shelf.
        let box_height = |list: &PartList| {
            list.by_role(PartRole::DrawerBack)
                .map(|p| p.bounds.size.z)
                .fold(0.0, f64::max)
        };
        assert_eq!(rows(false).by_role(PartRole::DrawerBottom).count(), 2);
        assert!(box_height(&rows(false)) >= box_height(&rows(true)));
    }

    /// Blum's session: one drawer SERVO-DRIVE beside drawers with handles. The driven one has no
    /// handle and a shorter box, for the drive behind it; the others are untouched.
    #[test]
    fn one_section_can_be_servo_drive_beside_sections_with_handles() {
        let mut cab = base();
        cab.adjustable_shelves = 0;
        cab.fronts = FrontLayout::Sections {
            columns: vec![SectionColumn {
                width: 0.0,
                rows: vec![
                    Section {
                        opening: Some(Opening::ServoDrive),
                        shelf_above: false,
                        ..section(0.0, SectionFront::drawers(1), SectionInside::Empty)
                    },
                    Section {
                        shelf_above: false,
                        ..section(0.0, SectionFront::drawers(1), SectionInside::Empty)
                    },
                ],
            }],
        };
        let list = build_cabinet(&cab).unwrap();
        assert!(
            !list.warnings.iter().any(|w| w.contains("clash")),
            "{:?}",
            list.warnings
        );
        let handled = |n: usize| !handle_holes(&list, &format!("B01.DrawerFront{n}")).is_empty();
        assert!(!handled(1), "the driven drawer has no handle");
        assert!(handled(2));
        assert_eq!(
            list.hardware
                .iter()
                .filter(|h| h.item == "blum.servo-drive.drive-unit")
                .map(|h| h.quantity)
                .sum::<u32>(),
            1
        );
        let length = |n: usize| {
            list.part(&format!("B01.DrawerBottom{n}"))
                .unwrap()
                .bounds
                .size
                .y
        };
        assert!(length(1) < length(2), "{} {}", length(1), length(2));
    }

    /// A section's own drawer system beside the job's.
    /// Blum's 2026-10-27 session: three 287.33 fronts, a fixed shelf over the bottom one, all
    /// driven: 2 brackets, each clear height − 10, each with its own attachment, cable and node.
    /// (Blum's shelf sits 7.5 lower than ours, so its cuts are 256.83 and 545.16: blum-checks.md.)
    #[test]
    fn each_compartment_of_driven_drawers_has_its_own_bracket() {
        let mut cab = Cabinet::new("B01", CabinetKind::Base);
        cab.width = 600.0;
        cab.depth = 580.0;
        cab.height = 870.0;
        cab.standard.toe_kick = ToeKick::None;
        cab.standard.top_joint = TopJoint::Rails;
        cab.standard.bottom_joint = BottomJoint::UnderSides;
        cab.standard.back = BackStyle::Applied;
        cab.standard.back_thickness = 16.0;
        cab.standard.front_gap = 3.0;
        cab.standard.top_gap = 2.0;
        cab.standard.bottom_gap = 0.0;
        cab.standard.opening = Opening::ServoDrive;
        let row = |shelf_above: bool| Section {
            height: 0.0,
            front: SectionFront::drawers(1),
            inside: SectionInside::Empty,
            shelf_above,
            opening: None,
        };
        cab.fronts = FrontLayout::Sections {
            columns: vec![SectionColumn {
                width: 0.0,
                rows: vec![row(true), row(false), row(false)],
            }],
        };
        let list = build_cabinet(&cab).unwrap();
        let count = |id: &str| {
            list.hardware
                .iter()
                .filter(|h| h.item == id)
                .map(|h| h.quantity)
                .sum::<u32>()
        };
        assert_eq!(count("blum.servo-drive.drive-unit"), 3);
        assert_eq!(count("blum.servo-drive.bracket-profile.670"), 2);
        assert_eq!(count("blum.servo-drive.bracket-attachment"), 2);
        assert_eq!(count("blum.servo-drive.distribution-cable"), 2);
        assert_eq!(count("blum.servo-drive.connecting-node"), 2);
        assert_eq!(count("blum.servo-drive.distance-bumper"), 6);
        assert_eq!(count("blum.servo-drive.power-supply.au"), 1);
        let cuts: Vec<&str> = list
            .hardware
            .iter()
            .filter(|h| h.item == "blum.servo-drive.bracket-profile.670")
            .map(|h| h.description.as_str())
            .collect();
        assert!(cuts[0].ends_with("cut to 264.3 mm"), "{cuts:?}");
        assert!(cuts[1].ends_with("cut to 537.7 mm"), "{cuts:?}");
        // The whole inside is shared out, less the shelf.
        let compartments = crate::standins::servo_compartments(&list);
        let clear: Vec<f64> = compartments.iter().map(|c| c.ceiling - c.floor).collect();
        assert!(
            (clear[0] + clear[1] + 16.0 - 838.0).abs() < 1e-6,
            "{clear:?}"
        );
        // One upright drawn per compartment, each from its own floor: cut short of Blum's model,
        // so as stand-ins.
        let uprights: Vec<_> = crate::standins::stand_ins(&list)
            .into_iter()
            .filter(|s| s.name.contains("ServoProfile"))
            .collect();
        assert_eq!(uprights.len(), 2);
        assert_eq!(uprights[0].name, "B01.ServoProfile");
        assert_eq!(uprights[1].name, "B01.ServoProfile2");
        let crate::standins::Shape::Box(upper) = uprights[1].shapes[0] else {
            panic!("a box");
        };
        assert!((upper.min.z - compartments[1].floor).abs() < 1e-9);
    }

    #[test]
    fn only_driven_drawers_and_their_compartment_get_servo_drive() {
        let mut cab = Cabinet::new("B01", CabinetKind::Base);
        let row = |shelf_above: bool, opening: Option<Opening>| Section {
            height: 0.0,
            front: SectionFront::drawers(1),
            inside: SectionInside::Empty,
            shelf_above,
            opening,
        };
        cab.fronts = FrontLayout::Sections {
            columns: vec![SectionColumn {
                width: 0.0,
                rows: vec![row(true, None), row(false, Some(Opening::ServoDrive))],
            }],
        };
        let list = build_cabinet(&cab).unwrap();
        let on_front2 = list
            .hardware
            .iter()
            .find(|h| h.item == "blum.servo-drive.drive-unit")
            .unwrap();
        let top_front = list
            .by_role(PartRole::DrawerFront)
            .max_by(|a, b| a.bounds.min.z.total_cmp(&b.bounds.min.z))
            .unwrap();
        assert_eq!(on_front2.part, top_front.name);
        let shelf = list.by_role(PartRole::Shelf).next().unwrap();
        let drawn = crate::cad::cad_placements(&list);
        let units: Vec<_> = drawn
            .iter()
            .filter(|c| c.item == "blum.servo-drive.drive-unit")
            .collect();
        assert_eq!(units.len(), 1);
        assert!(units[0].origin.z > shelf.bounds.max().z);
        let profile = crate::standins::stand_ins(&list)
            .into_iter()
            .find(|s| s.name.contains("ServoProfile"))
            .unwrap();
        let crate::standins::Shape::Box(bar) = profile.shapes[0] else {
            panic!("a box");
        };
        assert!(
            (bar.min.z - shelf.bounds.max().z).abs() < 1e-9,
            "stands on the fixed shelf"
        );
    }

    #[test]
    fn an_adjustable_shelf_in_the_way_of_the_bracket_asks_for_a_fixed_one() {
        let mut cab = Cabinet::new("B01", CabinetKind::Base);
        cab.standard.opening = Opening::ServoDrive;
        let rows = |shelf_above: bool| {
            vec![
                Section {
                    height: 0.0,
                    front: SectionFront::drawers(1),
                    inside: SectionInside::Empty,
                    shelf_above,
                    opening: None,
                },
                Section {
                    height: 0.0,
                    front: SectionFront::Open,
                    inside: SectionInside::shelves(1),
                    shelf_above: false,
                    opening: None,
                },
            ]
        };
        let warned = |shelf_above: bool| {
            let mut c = cab.clone();
            c.fronts = FrontLayout::Sections {
                columns: vec![SectionColumn {
                    width: 0.0,
                    rows: rows(shelf_above),
                }],
            };
            build_cabinet(&c)
                .unwrap()
                .warnings
                .iter()
                .any(|w| w.contains("SERVO-DRIVE bracket"))
        };
        assert!(warned(false));
        assert!(!warned(true));
    }

    #[test]
    fn a_section_can_use_another_drawer_system() {
        let mut cab = base();
        cab.adjustable_shelves = 0;
        cab.fronts = FrontLayout::Sections {
            columns: vec![SectionColumn {
                width: 0.0,
                rows: vec![section(
                    0.0,
                    SectionFront::Drawers {
                        count: 2,
                        ratios: Vec::new(),
                        system: Some(DrawerSystem::Merivobox),
                        box_height: None,
                        nominal_length: None,
                    },
                    SectionInside::Empty,
                )],
            }],
        };
        let list = build_cabinet(&cab).unwrap();
        assert!(list.hardware.iter().any(|h| h.item.contains("merivobox")));
        assert!(!list.hardware.iter().any(|h| h.item.contains("legrabox")));
    }

    /// SEC-2's pin holes: adjustable shelves in a column rest on holes in the side and the divider,
    /// not run through from the divider's other face.
    #[test]
    fn adjustable_shelves_in_a_column_get_pin_holes_in_its_panels() {
        let list = build_cabinet(&sectioned_base()).unwrap();
        let divider = list.by_role(PartRole::Divider).next().unwrap().name.clone();
        let pins = |part: &str| bores(&list, part, BorePurpose::ShelfPin);
        assert!(!pins(&divider).is_empty());
        assert!(!pins("B01.RightSide").is_empty());
        assert!(pins("B01.LeftSide").is_empty(), "drawers on that side");
        let t = cab_thickness(&list);
        for o in &list.operations {
            if let Operation::Bore {
                part,
                depth,
                purpose: BorePurpose::ShelfPin,
                ..
            } = o
                && *part == divider
            {
                assert!(depth * 2.0 < t, "{depth}");
            }
        }
    }

    fn cab_thickness(list: &PartList) -> f64 {
        list.part("B01.LeftSide").unwrap().bounds.size.x
    }

    /// AVENTOS is sized from the cabinet: a lift only across its top, and one of them.
    #[test]
    fn a_lift_section_goes_across_the_top_of_the_cabinet_or_says_why_not() {
        use crate::model::LiftKind;
        let lift = SectionFront::Lift {
            kind: LiftKind::StayLift,
            handle_weight: 0.4,
        };
        let mut cab = Cabinet::new("W01", CabinetKind::Wall);
        cab.adjustable_shelves = 0;
        let build = |cab: &mut Cabinet, rows: Vec<Section>| {
            cab.fronts = FrontLayout::Sections {
                columns: vec![SectionColumn { width: 0.0, rows }],
            };
            build_cabinet(cab).unwrap()
        };
        let top = build(
            &mut cab,
            vec![
                section(0.0, SectionFront::Open, SectionInside::Empty),
                section(300.0, lift.clone(), SectionInside::Empty),
            ],
        );
        assert_eq!(top.by_role(PartRole::LiftFront).count(), 1);
        let under = build(
            &mut cab,
            vec![
                section(300.0, lift.clone(), SectionInside::Empty),
                section(0.0, SectionFront::Open, SectionInside::Empty),
            ],
        );
        assert_eq!(under.by_role(PartRole::LiftFront).count(), 0);
        assert!(
            under
                .warnings
                .iter()
                .any(|w| w.contains("lift goes across"))
        );
    }

    /// SEC-1: a divider stands between the columns, in the gap between their fronts.
    #[test]
    fn sections_put_a_divider_between_columns() {
        let list = build_cabinet(&sectioned_base()).unwrap();
        quiet(&list);
        let dividers: Vec<_> = list.by_role(PartRole::Divider).collect();
        assert_eq!(dividers.len(), 1);
        let d = dividers[0].bounds;
        let door = list.part("B01.Door1").unwrap().bounds;
        let drawer = list
            .by_role(PartRole::DrawerFront)
            .map(|p| p.bounds.max().x)
            .fold(f64::MIN, f64::max);
        assert!(drawer < d.center().x && d.center().x < door.min.x);
        assert!(((drawer + door.min.x) / 2.0 - d.center().x).abs() < 1e-6);
        // Jointed to the bottom like a side.
        assert!(list.operations.iter().any(|o| match o {
            Operation::BiscuitSlot { parts, .. } => parts.contains(&dividers[0].name),
            Operation::Bore { part, .. } => *part == dividers[0].name,
            _ => false,
        }));
    }

    /// SEC-1: drawer boxes fit the bay between the left side and the divider, not the carcass.
    #[test]
    fn section_drawers_fit_their_bay() {
        let list = build_cabinet(&sectioned_base()).unwrap();
        let left = list.part("B01.LeftSide").unwrap().bounds.max().x;
        let divider = list.by_role(PartRole::Divider).next().unwrap().bounds.min.x;
        let bottoms: Vec<_> = list.by_role(PartRole::DrawerBottom).collect();
        assert_eq!(bottoms.len(), 3);
        for b in bottoms {
            assert!(
                b.bounds.min.x > left && b.bounds.max().x < divider,
                "{}",
                b.name
            );
        }
    }

    /// SEC-1: a door hinged on a divider shares it with its neighbour: half overlay, and the
    /// plate holes go in the divider.
    #[test]
    fn a_door_on_a_divider_gets_the_half_overlay_hinge() {
        let list = build_cabinet(&sectioned_base()).unwrap();
        let hinge = list
            .hardware
            .iter()
            .find(|h| h.item.starts_with("blum.clip-top"))
            .unwrap();
        assert_eq!(
            crate::hardware::item(&hinge.item).unwrap().article,
            "71B3650"
        );
        let divider = &list.by_role(PartRole::Divider).next().unwrap().name;
        assert!(list.operations.iter().any(|o| matches!(
            o,
            Operation::Bore { part, purpose: BorePurpose::HingePlate, .. } if part == divider
        )));
        // The supplier models sit on the divider's face too.
        let face = list
            .by_role(PartRole::Divider)
            .next()
            .unwrap()
            .bounds
            .max()
            .x;
        let plates: Vec<_> = crate::cad::cad_placements(&list)
            .into_iter()
            .filter(|c| c.name.contains("Hinge") || c.name.contains("Plate"))
            .collect();
        assert!(!plates.is_empty());
        assert!(plates.iter().all(|c| (c.origin.x - face).abs() < 1e-6));
    }

    /// SEC-1: a wardrobe column: drawers at the bottom, a hanging rail, shelves on top, with a
    /// fixed shelf between each row.
    #[test]
    fn a_wardrobe_column_stacks_drawers_hanging_and_shelves() {
        let mut cab = Cabinet::new("W01", CabinetKind::Tall);
        cab.adjustable_shelves = 0;
        cab.fronts = FrontLayout::Sections {
            columns: vec![SectionColumn {
                width: 0.0,
                rows: vec![
                    section(500.0, SectionFront::drawers(2), SectionInside::Empty),
                    section(0.0, SectionFront::Open, SectionInside::Hanging),
                    section(400.0, SectionFront::DoorPair, SectionInside::shelves(1)),
                ],
            }],
        };
        let list = build_cabinet(&cab).unwrap();
        quiet(&list);
        assert_eq!(list.by_role(PartRole::Divider).count(), 0);
        assert_eq!(list.by_role(PartRole::DrawerFront).count(), 2);
        assert_eq!(list.by_role(PartRole::Door).count(), 2);
        // Two fixed shelves between the rows and one adjustable on pins.
        assert_eq!(list.by_role(PartRole::Shelf).count(), 3);
        assert!(list.hardware.iter().any(|h| h.item == "wardrobe-rail.oval"));
    }

    /// SEC-1: inner drawers behind a section's door, which takes Blum's 0-protrusion hinge.
    #[test]
    fn inner_drawers_in_a_section_change_its_door_hinge() {
        let mut cab = sectioned_base();
        if let FrontLayout::Sections { columns } = &mut cab.fronts {
            columns[1].rows[0].inside = SectionInside::InnerDrawers { count: 2 };
        }
        let list = build_cabinet(&cab).unwrap();
        quiet(&list);
        assert_eq!(list.by_role(PartRole::DrawerBottom).count(), 5);
        assert!(list.hardware.iter().any(|h| h.item == INNER_DRAWER_HINGE));
    }

    /// STYLE-4: an oven tower: two drawers, a 600 mm clear niche between fixed shelves, a door
    /// above; nothing clashes.
    #[test]
    fn an_oven_tower_stacks_drawers_a_niche_and_a_door() {
        let list = build_cabinet(&oven_tower()).unwrap();
        assert!(
            !list
                .warnings
                .iter()
                .any(|w| w.contains("clash") || w.contains("need")),
            "{:?}",
            list.warnings
        );
        assert_eq!(list.by_role(PartRole::DrawerFront).count(), 2);
        assert_eq!(list.by_role(PartRole::DrawerBottom).count(), 2);
        assert_eq!(list.by_role(PartRole::Door).count(), 1);
        let (s1, s2) = (
            list.part("T01.Shelf1").unwrap(),
            list.part("T01.Shelf2").unwrap(),
        );
        assert!((s2.bounds.min.z - s1.bounds.max().z - 600.0).abs() < 1e-6);
        // The door starts a gap above the niche's top shelf's front edge line, and the drawers
        // end under the niche.
        let door = list.part("T01.Door1").unwrap().bounds;
        let top_drawer = list
            .by_role(PartRole::DrawerFront)
            .map(|p| p.bounds.max().z)
            .fold(f64::MIN, f64::max);
        assert!(top_drawer < s1.bounds.max().z);
        assert!(door.min.z > s2.bounds.min.z - 1e-6);
        assert!(door.max().z <= 2100.0);
    }

    /// STYLE-1: continuous grain: the oven tower's door and drawer fronts are one column, top
    /// first, all grained upright; with it off, drawer fronts' grain runs across.
    #[test]
    fn stacked_fronts_are_grain_matched_top_first() {
        let mut cab = oven_tower();
        let free = build_cabinet(&cab).unwrap();
        assert!(free.grain_matched.is_empty());
        assert_eq!(
            free.part("T01.DrawerFront1").unwrap().grain_axis,
            Some(Axis::X)
        );
        cab.standard.front_grain = FrontGrain::Matched;
        let list = build_cabinet(&cab).unwrap();
        assert_eq!(
            list.grain_matched,
            vec![vec![
                "T01.Door1".to_string(),
                "T01.DrawerFront1".into(),
                "T01.DrawerFront2".into(),
            ]]
        );
        assert!(
            list.parts
                .iter()
                .filter(|p| matches!(p.role, PartRole::Door | PartRole::DrawerFront))
                .all(|p| p.grain_axis == Some(Axis::Z))
        );
        // A pair of doors side by side are two columns of one: nothing to match.
        let mut pair = Cabinet::new("B01", CabinetKind::Base);
        pair.width = 900.0;
        pair.standard.front_grain = FrontGrain::Matched;
        assert!(build_cabinet(&pair).unwrap().grain_matched.is_empty());
        // Framed fronts are cut piece by piece.
        cab.standard.door_style = DoorStyle::Shaker;
        let list = build_cabinet(&cab).unwrap();
        assert!(list.grain_matched.is_empty());
        assert!(list.warnings.iter().any(|w| w.contains("continuous grain")));
    }

    #[test]
    fn a_stack_too_tall_for_its_cabinet_is_refused() {
        let mut cab = oven_tower();
        if let FrontLayout::Stack { sections } = &mut cab.fronts {
            sections[1].height = 2500.0;
        }
        let list = build_cabinet(&cab).unwrap();
        assert!(list.warnings.iter().any(|w| w.contains("more height")));
    }

    fn grooves_on<'a>(list: &'a PartList, prefix: &str) -> Vec<(&'a str, Aabb)> {
        list.operations
            .iter()
            .filter_map(|o| match o {
                Operation::Groove { part, bounds } if part.starts_with(prefix) => {
                    Some((part.as_str(), *bounds))
                }
                _ => None,
            })
            .collect()
    }

    /// STYLE-1: a finger pull is routed along the back of the top edge (wall fronts: the
    /// bottom); no handle, no holes; shaker fronts cannot take one.
    #[test]
    fn finger_pulls_are_routed_along_the_gripping_edge() {
        let mut base = Cabinet::new("B01", CabinetKind::Base);
        base.standard.handle_place = HandlePlace::FingerPull;
        let list = build_cabinet(&base).unwrap();
        let door = list.part("B01.Door1").unwrap().bounds;
        let g = grooves_on(&list, "B01.Door1");
        assert_eq!(g.len(), 1);
        assert_eq!(g[0].1.max().z, door.max().z);
        assert_eq!(g[0].1.max().y, door.max().y, "the back face");
        assert_eq!(g[0].1.size.x, door.size.x);
        assert!(handle_holes(&list, "B01.Door1").is_empty());
        assert!(!list.hardware.iter().any(|h| h.item.starts_with("handle.")));
        let mut wall = Cabinet::new("W01", CabinetKind::Wall);
        wall.standard.handle_place = HandlePlace::FingerPull;
        let list = build_cabinet(&wall).unwrap();
        let door = list.part("W01.Door1").unwrap().bounds;
        assert_eq!(grooves_on(&list, "W01.Door1")[0].1.min.z, door.min.z);
        base.standard.door_style = DoorStyle::Shaker;
        let list = build_cabinet(&base).unwrap();
        assert!(
            !list.warnings.iter().any(|w| w.contains("clash")),
            "{:?}",
            list.warnings
        );
        assert!(
            list.warnings
                .iter()
                .any(|w| w.contains("finger pulls need slab"))
        );
        assert!(
            grooves_on(&list, "B01.Door1")
                .iter()
                .all(|(_, b)| b.size.z != 25.0)
        );
    }

    /// A job saved when cladding was its own choice opens as seen ends, with its board and
    /// thickness, so neither the fronts' alignment nor the screws depend on how the side was set.
    #[test]
    fn a_side_clad_in_an_old_job_behaves_as_a_seen_end() {
        let mut old = Cabinet::new("B01", CabinetKind::Base);
        old.cladding.left = true;
        old.cladding.thickness = 12.0;
        old.cladding.board = "lining".into();
        let list = build_cabinet(&old).unwrap();
        let board = list.part("B01.SeenEndLeft").unwrap();
        assert_eq!((board.bounds.min.x, board.bounds.max().x), (-12.0, 0.0));
        assert_eq!(board.board, "lining");
        assert!(!list.operations.iter().any(|o| matches!(
            o,
            Operation::Bore { part, purpose: BorePurpose::Screw, .. } if part == "B01.LeftSide"
        )));

        let mut new = Cabinet::new("B01", CabinetKind::Base);
        new.exposed.left = true;
        let list = build_cabinet(&new).unwrap();
        let board = list.part("B01.SeenEndLeft").unwrap();
        assert_eq!(
            (board.bounds.min.x, board.board.as_str()),
            (
                -new.standard.front_thickness,
                new.standard.front_board.as_str()
            )
        );
    }

    #[test]
    fn a_back_board_does_not_change_the_thickness_of_the_seen_ends() {
        let mut cab = Cabinet::new("B01", CabinetKind::Base);
        cab.exposed.left = true;
        cab.cladding.back = true;
        let list = build_cabinet(&cab).unwrap();
        let end = list.part("B01.SeenEndLeft").unwrap().bounds;
        let back = list.part("B01.SeenBack").unwrap().bounds;
        assert_eq!(end.min.x, -cab.standard.front_thickness);
        assert_eq!(back.max().y - back.min.y, cab.cladding.thickness);
        assert_eq!(end.max().y, back.max().y, "the end covers the back's edge");
    }

    /// SURR-2: an island cabinet clad on its back and ends: the ends run from the door faces
    /// over the back cladding's edges, floor to top, flutes upright; nothing overlaps the carcass.
    #[test]
    fn cladding_stands_proud_of_the_clad_faces() {
        let mut cab = Cabinet::new("B01", CabinetKind::Base);
        cab.cladding = crate::model::Cladding {
            left: true,
            right: true,
            back: true,
            ..Default::default()
        };
        let list = build_cabinet(&cab).unwrap();
        let get = |n: &str| list.part(n).unwrap_or_else(|| panic!("{n}")).bounds;
        let (l, r, b) = (
            get("B01.SeenEndLeft"),
            get("B01.SeenEndRight"),
            get("B01.SeenBack"),
        );
        assert_eq!((l.max().x, r.min.x, b.min.y), (0.0, 600.0, 580.0));
        assert_eq!(
            l.max().y,
            b.max().y,
            "the end covers the back cladding's edge"
        );
        assert_eq!((l.min.z, l.max().z), (0.0, 870.0));
        assert!(
            list.parts
                .iter()
                .filter(|p| p.role == PartRole::Cladding)
                .all(|p| p.grain_axis == Some(Axis::Z))
        );
        let carcass: Vec<_> = list
            .parts
            .iter()
            .filter(|p| p.role != PartRole::Cladding)
            .collect();
        for c in list.parts.iter().filter(|p| p.role == PartRole::Cladding) {
            for p in &carcass {
                assert!(
                    !c.bounds.overlaps(&p.bounds),
                    "{} overlaps {}",
                    c.name,
                    p.name
                );
            }
        }
    }

    /// SURR-4: a canopy box: sides, top and bottom jointed, a fixed front, the insert's cut-out
    /// through the bottom; clad ends work as on any cabinet.
    #[test]
    fn a_rangehood_canopy_houses_its_insert() {
        let mut cab = Cabinet::new("H01", CabinetKind::Rangehood);
        cab.cladding.left = true;
        cab.cladding.right = true;
        let list = build_cabinet(&cab).unwrap();
        assert!(
            !list.warnings.iter().any(|w| w.contains("clash")),
            "{:?}",
            list.warnings
        );
        for n in [
            "H01.LeftSide",
            "H01.RightSide",
            "H01.Top",
            "H01.Bottom",
            "H01.CanopyFront",
            "H01.SeenEndLeft",
        ] {
            assert!(list.part(n).is_some(), "{n}");
        }
        assert!(list.part("H01.Back").is_none());
        let cut = list
            .operations
            .iter()
            .find_map(|o| match o {
                Operation::Groove { part, bounds } if part == "H01.Bottom" => Some(*bounds),
                _ => None,
            })
            .unwrap();
        assert_eq!(
            (cut.size.x, cut.size.y, cut.center().x),
            (520.0, 260.0, 450.0)
        );
        assert!(
            list.operations
                .iter()
                .any(|o| matches!(o, Operation::BiscuitSlot { .. }))
        );
        cab.insert.width = 900.0;
        assert!(
            build_cabinet(&cab)
                .unwrap()
                .warnings
                .iter()
                .any(|w| w.contains("does not fit"))
        );
    }

    /// BLUM-16: AMBIA-LINE: the top drawer a cutlery set, the others frames.
    #[test]
    fn ambia_line_puts_cutlery_in_the_top_drawer_and_frames_below() {
        let mut cab = Cabinet::new("D01", CabinetKind::DrawerBank);
        cab.drawer_inserts = true;
        let list = build_cabinet(&cab).unwrap();
        assert!(
            !list.warnings.iter().any(|w| w.contains("AMBIA")),
            "{:?}",
            list.warnings
        );
        let on = |part: &str, prefix: &str| {
            list.hardware
                .iter()
                .any(|h| h.part == part && h.item.starts_with(prefix))
        };
        assert!(on("D01.DrawerFront1", "blum.ambia-line.cutlery-insert."));
        assert!(!on("D01.DrawerFront2", "blum.ambia-line.cutlery-insert."));
        assert!(on("D01.DrawerFront2", "blum.ambia-line.frame-"));
        cab.standard.insert_design = InsertDesign::Wood;
        let list = build_cabinet(&cab).unwrap();
        assert!(
            list.hardware
                .iter()
                .filter(|h| h.item.starts_with("blum.ambia-line."))
                .all(|h| h.item.starts_with("blum.ambia-line.wood."))
        );
        cab.standard.drawer_system = DrawerSystem::Tandem;
        let list = build_cabinet(&cab).unwrap();
        assert!(
            list.warnings
                .iter()
                .any(|w| w.contains("drawer inserts go in"))
        );
    }

    /// STYLE-1: an arched top: one arch across a pair of wall doors, through both ends 80 under
    /// the top; one so high it would cut the hinge cups is refused.
    #[test]
    fn an_arched_top_spans_the_top_row_of_doors() {
        let mut cab = Cabinet::new("W01", CabinetKind::Wall);
        cab.width = 900.0;
        cab.fronts = FrontLayout::DoorPair;
        cab.arched_top = 80.0;
        let list = build_cabinet(&cab).unwrap();
        assert!(
            !list.warnings.iter().any(|w| w.contains("arch")),
            "{:?}",
            list.warnings
        );
        let arches: Vec<_> = list
            .operations
            .iter()
            .filter_map(|o| match o {
                Operation::Arch {
                    part,
                    centre,
                    radius,
                } => Some((part.clone(), *centre, *radius)),
                _ => None,
            })
            .collect();
        assert_eq!(arches.len(), 2);
        let doors: Vec<&Part> = list.by_role(PartRole::Door).collect();
        let x0 = doors
            .iter()
            .map(|d| d.bounds.min.x)
            .fold(f64::MAX, f64::min);
        let top = doors[0].bounds.max().z;
        let (_, c, r) = arches[0];
        assert_eq!((c.x, c.z), (arches[1].1.x, arches[1].1.z));
        // Through the top at the middle and 80 under it at the outer edge.
        assert!((c.z + r - top).abs() < 1e-6);
        let at_edge = c.z + (r * r - (x0 - c.x).powi(2)).sqrt();
        assert!((top - at_edge - 80.0).abs() < 1e-6);
        // The top hinges keep Blum's distance from where the arch meets their edge.
        let cups: Vec<f64> = list
            .operations
            .iter()
            .filter_map(|o| match o {
                Operation::Bore {
                    at,
                    purpose: BorePurpose::HingeCup,
                    ..
                } => Some(at.z),
                _ => None,
            })
            .collect();
        let highest = cups.iter().copied().fold(f64::MIN, f64::max);
        assert!((top - 80.0 - 80.0 - highest).abs() < 1e-6, "{cups:?}");
        // An arch too high for the doors stays square.
        cab.arched_top = 400.0;
        let list = build_cabinet(&cab).unwrap();
        assert!(list.warnings.iter().any(|w| w.contains("too high")));
        assert!(
            !list
                .operations
                .iter()
                .any(|o| matches!(o, Operation::Arch { .. }))
        );
    }

    /// BLUM-16: CABLOXX: a 720 high drawer bank takes the 800 bar set, a unit and bracket per
    /// drawer; a 900 wide one a second bar and the synchronisation.
    #[test]
    fn each_drawer_column_gets_its_own_locking_bar() {
        let mut cab = Cabinet::new("D01", CabinetKind::DrawerBank);
        cab.width = 1000.0;
        let row = || Section {
            height: 0.0,
            front: SectionFront::drawers(3),
            inside: SectionInside::Empty,
            shelf_above: false,
            opening: None,
        };
        cab.fronts = FrontLayout::Sections {
            columns: vec![
                SectionColumn {
                    width: 0.0,
                    rows: vec![row()],
                },
                SectionColumn {
                    width: 0.0,
                    rows: vec![row()],
                },
            ],
        };
        cab.lock = true;
        let list = build_cabinet(&cab).unwrap();
        let bars: Vec<_> = list
            .hardware
            .iter()
            .filter(|h| h.item.starts_with("blum.cabloxx.bar-set."))
            .collect();
        assert_eq!(bars.len(), 2, "one for each column");
        assert_eq!(bars.iter().map(|b| b.quantity).sum::<u32>(), 2);
        assert!(bars[0].at.x < 500.0 && bars[1].at.x > 500.0);
        let units: u32 = list
            .hardware
            .iter()
            .filter(|h| h.item.starts_with("blum.cabloxx.locking-unit."))
            .map(|h| h.quantity)
            .sum();
        assert_eq!(units, 6);
        assert_eq!(
            list.notes.iter().filter(|n| n.contains("CABLOXX")).count(),
            1
        );
    }

    #[test]
    fn five_drawers_in_each_of_two_columns_still_lock() {
        let mut cab = Cabinet::new("D01", CabinetKind::DrawerBank);
        cab.width = 1000.0;
        cab.height = 1200.0;
        let row = || Section {
            height: 0.0,
            front: SectionFront::drawers(5),
            inside: SectionInside::Empty,
            shelf_above: false,
            opening: None,
        };
        cab.fronts = FrontLayout::Sections {
            columns: vec![
                SectionColumn {
                    width: 0.0,
                    rows: vec![row()],
                },
                SectionColumn {
                    width: 0.0,
                    rows: vec![row()],
                },
            ],
        };
        cab.lock = true;
        let list = build_cabinet(&cab).unwrap();
        assert!(list.warnings.iter().all(|w| !w.contains("up to five")));
    }

    #[test]
    fn cabloxx_locks_every_drawer_on_one_bar() {
        let mut cab = Cabinet::new("D01", CabinetKind::DrawerBank);
        cab.lock = true;
        let list = build_cabinet(&cab).unwrap();
        let n = list.by_role(PartRole::DrawerFront).count() as u32;
        let qty = |id: &str| {
            list.hardware
                .iter()
                .filter(|h| h.item == id)
                .map(|h| h.quantity)
                .sum::<u32>()
        };
        assert_eq!(qty("blum.cabloxx.bar-set.800"), 1);
        assert_eq!(qty("blum.cabloxx.locking-unit.right"), n);
        assert_eq!(qty("blum.cabloxx.front-bracket"), n);
        assert_eq!(qty("blum.cabloxx.sync-set"), 0);
        assert!(list.notes.iter().any(|n| n.contains("cylinder")));
        cab.width = 900.0;
        let list = build_cabinet(&cab).unwrap();
        let has = |id: &str| list.hardware.iter().any(|h| h.item == id);
        assert!(has("blum.cabloxx.sync-set") && has("blum.cabloxx.locking-unit.left"));
        let base = Cabinet::new("B01", CabinetKind::Base);
        let mut b = base.clone();
        b.lock = true;
        assert!(
            build_cabinet(&b)
                .unwrap()
                .warnings
                .iter()
                .any(|w| w.contains("needs drawers"))
        );
    }

    /// BLUM-16: ORGA-LINE in TANDEMBOX drawers: a container set on top, utensil dividers below.
    #[test]
    fn orga_line_puts_containers_in_the_top_tandembox_drawer() {
        let mut cab = Cabinet::new("D01", CabinetKind::DrawerBank);
        cab.drawer_inserts = true;
        cab.standard.drawer_system = DrawerSystem::Tandembox;
        let list = build_cabinet(&cab).unwrap();
        assert!(
            !list.warnings.iter().any(|w| w.contains("ORGA")),
            "{:?}",
            list.warnings
        );
        let on = |part: &str, item: &str| {
            list.hardware
                .iter()
                .any(|h| h.part == part && h.item.starts_with(item))
        };
        // A 600 cabinet: full width sets.
        assert!(on("D01.DrawerFront1", "blum.orga-line.ve60."));
        assert!(on("D01.DrawerFront2", "blum.orga-line.vu60."));
    }

    fn handle_holes(list: &PartList, part: &str) -> Vec<Vec3> {
        list.operations
            .iter()
            .filter_map(|o| match o {
                Operation::Bore {
                    part: p,
                    at,
                    purpose: BorePurpose::HandleScrew,
                    ..
                } if p == part => Some(*at),
                _ => None,
            })
            .collect()
    }

    /// HAF-2: a base door's handle stands by its free edge near the top, a wall door's near
    /// the bottom, 128 mm centres by default, holes through the door.
    #[test]
    fn door_handles_stand_by_the_free_edge_top_on_base_bottom_on_wall() {
        let base = build_cabinet(&Cabinet::new("B01", CabinetKind::Base)).unwrap();
        let door = base.part("B01.Door1").unwrap().bounds;
        // A left-hinged door: the handle by its right edge.
        let h = handle_holes(&base, "B01.Door1");
        assert_eq!(h.len(), 2);
        for at in &h {
            assert!((at.x - (door.max().x - HANDLE_EDGE_INSET)).abs() < 1e-9);
            assert!((at.y - door.max().y).abs() < 1e-9, "from the back face");
        }
        let top = h.iter().map(|a| a.z).fold(f64::MIN, f64::max);
        assert!((top - (door.max().z - HANDLE_END_INSET)).abs() < 1e-9);
        assert!((top - h.iter().map(|a| a.z).fold(f64::MAX, f64::min) - 128.0).abs() < 1e-9);
        let wall = build_cabinet(&Cabinet::new("W01", CabinetKind::Wall)).unwrap();
        let wd = wall.part("W01.Door1").unwrap().bounds;
        let low = handle_holes(&wall, "W01.Door1")
            .iter()
            .map(|a| a.z)
            .fold(f64::MAX, f64::min);
        assert!((low - (wd.min.z + HANDLE_END_INSET)).abs() < 1e-9);
        assert!(base.hardware.iter().any(|h| h.item == "handle.cc128"));
    }

    #[test]
    fn a_door_pair_gets_handles_at_the_meeting_edges_and_drawers_across_the_middle() {
        let mut cab = Cabinet::new("B01", CabinetKind::Base);
        cab.width = 900.0;
        cab.fronts = FrontLayout::DoorPair;
        let list = build_cabinet(&cab).unwrap();
        let (l, r) = (
            handle_holes(&list, "B01.Door1")[0].x,
            handle_holes(&list, "B01.Door2")[0].x,
        );
        assert!(l > 400.0 && r < 500.0 && l < r, "{l} {r}");
        let bank = build_cabinet(&Cabinet::new("D01", CabinetKind::DrawerBank)).unwrap();
        for n in 1..=3 {
            let name = format!("D01.DrawerFront{n}");
            let f = bank.part(&name).unwrap().bounds;
            let h = handle_holes(&bank, &name);
            assert_eq!(h.len(), 2);
            assert!(((h[0].x + h[1].x) / 2.0 - f.center().x).abs() < 1e-9);
            assert!((h[0].z - h[1].z).abs() < 1e-9);
        }
    }

    /// ABI-2: the chosen catalog row sets the bore centres, the hardware id and the model article.
    /// ABI names a pull by its length: the 85 mm Camille's screws are 64 apart.
    /// ABI-4: the three timber pulls whose IFC has no mesh, at the centres on ABI's drawings, not
    /// their named lengths.
    #[test]
    fn hegman_pulls_read_from_fbx_are_bored_at_abis_centres() {
        for (id, length, centres, projection) in [
            ("handle.abi.hegmanclassic-130.ash", 130.0, 50.0, 20.0),
            ("handle.abi.hegmanclassic-250.ash", 250.0, 154.0, 20.0),
            ("handle.abi.hegmanhalfmoon-90.ash", 90.0, 22.0, 25.0),
        ] {
            let row = crate::hardware::item(id).unwrap();
            assert_eq!(row.param("length"), Ok(length), "{id}");
            assert_eq!(row.param("centres"), Ok(centres), "{id}");
            assert_eq!(row.param("projection"), Ok(projection), "{id}");
            let xs: Vec<f64> = row.drilling.iter().map(|h| h.x).collect();
            assert_eq!(xs, vec![-centres / 2.0, centres / 2.0], "{id}");
            assert!(!row.source.verified);
        }
        // The half moon's screw line is 10 from its straight edge.
        let moon = crate::hardware::item("handle.abi.hegmanhalfmoon-90.ash").unwrap();
        assert_eq!(moon.param("across_high"), Ok(10.0));
    }

    #[test]
    fn an_abi_handle_bores_its_own_centres_and_places_that_finish() {
        use crate::model::HandleRange;
        let mut cab = Cabinet::new("B01", CabinetKind::Base);
        cab.standard.handle_range = HandleRange::Abi;
        cab.standard.handle_style = "handle.abi.camille-85.brushed-brass".into();
        let list = build_cabinet(&cab).unwrap();
        let h = handle_holes(&list, "B01.Door1");
        assert_eq!(h.len(), 2);
        let span = (h[0].z - h[1].z).abs();
        assert!((span - 64.0).abs() < 1e-6, "{span}");
        let row = list
            .hardware
            .iter()
            .find(|h| h.item.starts_with("handle."))
            .unwrap();
        assert_eq!(row.item, "handle.abi.camille-85.brushed-brass");
        assert!(row.description.contains("SKU"), "{}", row.description);
        let placed = crate::cad::cad_placements(&list);
        let model = placed.iter().find(|p| p.name.contains("Handle")).unwrap();
        assert_eq!(model.article, "ABI-CAMILLE-85");
    }

    #[test]
    fn an_abi_knob_takes_one_hole_and_a_pull_that_does_not_fit_is_left_off() {
        use crate::model::HandleRange;
        let mut cab = Cabinet::new("B01", CabinetKind::Base);
        cab.standard.handle_range = HandleRange::Abi;
        cab.standard.handle_style = "handle.abi.elsa-28.brushed-brass".into();
        let list = build_cabinet(&cab).unwrap();
        assert_eq!(handle_holes(&list, "B01.Door1").len(), 1);
        assert!(
            list.hardware
                .iter()
                .any(|h| h.item == "handle.abi.elsa-28.brushed-brass")
        );
        // A 500 mm pull does not fit a 300 wide drawer. The generic handle must not be substituted.
        let mut bank = Cabinet::new("D01", CabinetKind::DrawerBank);
        bank.width = 300.0;
        bank.standard.handle_range = HandleRange::Abi;
        bank.standard.handle_style = "handle.abi.tezra-500.brushed-brass".into();
        let list = build_cabinet(&bank).unwrap();
        assert!(handle_holes(&list, "D01.DrawerFront1").is_empty());
        assert!(!list.hardware.iter().any(|h| h.item.starts_with("handle.")));
        assert!(
            list.warnings.iter().any(|w| w.contains("does not fit")),
            "{:?}",
            list.warnings
        );
    }

    /// ABI-3: a pull's name is its length, not its screws; several have more than two screws
    /// and T pulls have one. From ABI's spec drawings.
    #[test]
    fn abi_pulls_bore_the_screw_pattern_abi_draws_not_their_name() {
        use crate::model::HandleRange;
        for (style, apart) in [
            ("handle.abi.tezra-220.brushed-brass", vec![-96.0, 96.0]),
            ("handle.abi.pika-138.brushed-brass", vec![-64.0, 64.0]),
            (
                "handle.abi.pedra-220.beige-travertine",
                vec![-87.0, 0.0, 87.0],
            ),
            ("handle.abi.tezra-50.brushed-brass", vec![0.0]),
        ] {
            let mut bank = Cabinet::new("D01", CabinetKind::DrawerBank);
            bank.standard.handle_range = HandleRange::Abi;
            bank.standard.handle_style = style.into();
            let list = build_cabinet(&bank).unwrap();
            let front = list.part("D01.DrawerFront1").unwrap().bounds;
            let xs: Vec<f64> = handle_holes(&list, "D01.DrawerFront1")
                .iter()
                .map(|h| h.x - front.center().x)
                .collect();
            assert_eq!(xs.len(), apart.len(), "{style}");
            for (x, want) in xs.iter().zip(&apart) {
                assert!((x - want).abs() < 1e-6, "{style}: {xs:?}");
            }
        }
    }

    /// ABI-3: a pull whose body would overhang the front is left off, though its screws fit.
    #[test]
    fn an_abi_pull_longer_than_the_front_is_left_off() {
        use crate::model::HandleRange;
        let mut bank = Cabinet::new("D01", CabinetKind::DrawerBank);
        // Tezra 220: screws 192 apart, 220 long, on a front 210 wide.
        bank.width = 212.0;
        bank.standard.handle_range = HandleRange::Abi;
        bank.standard.handle_style = "handle.abi.tezra-220.brushed-brass".into();
        let list = build_cabinet(&bank).unwrap();
        assert!(handle_holes(&list, "D01.DrawerFront1").is_empty());
    }

    /// Owner's test: Pedra Curve left out of doors because its 79 mm overhang ran past the top.
    /// Every ABI model, on every sort of front: its whole body, placed through the same transform
    /// the views and SolidCraft use, stays on the front it is bored in, and the model stands out
    /// of the front's face. A pull is only left off where it truly cannot fit, and says so.
    #[test]
    fn every_abi_handle_stays_on_every_front_it_is_bored_in() {
        use crate::model::HandleRange;
        let mut seen = std::collections::BTreeSet::new();
        let styles: Vec<_> = crate::hardware::items()
            .iter()
            .filter(|h| h.id.starts_with("handle.abi."))
            // Those over the edge are held to their own rules below.
            .filter(|h| !h.parameters.contains_key("edge_mount"))
            .filter(|h| seen.insert(h.article.clone()))
            .collect();
        assert!(styles.len() >= 30);
        for style in styles {
            let p = |k: &str| style.parameters.get(k).copied().unwrap_or(0.0);
            for kind in [
                CabinetKind::Base,
                CabinetKind::DrawerBank,
                CabinetKind::Wall,
                CabinetKind::Tall,
                CabinetKind::Sink,
            ] {
                let mut cab = Cabinet::new("C01", kind);
                cab.standard.handle_range = HandleRange::Abi;
                cab.standard.handle_style = style.id.clone();
                let list = build_cabinet(&cab).unwrap();
                let who = format!("{} on {kind:?}", style.id);
                assert!(
                    !list.warnings.iter().any(|w| w.contains("clash")),
                    "{who}: {:?}",
                    list.warnings
                );
                let placed: Vec<_> = crate::cad::cad_placements(&list)
                    .into_iter()
                    .filter(|c| c.item == style.id)
                    .collect();
                let bored = list
                    .operations
                    .iter()
                    .filter(|o| {
                        matches!(
                            o,
                            Operation::Bore {
                                purpose: BorePurpose::HandleScrew,
                                ..
                            }
                        )
                    })
                    .count();
                assert_eq!(bored % style.drilling.len(), 0, "{who}");
                assert_eq!(placed.len(), bored / style.drilling.len(), "{who}");
                let doors: usize = list.by_role(PartRole::Door).count()
                    + list.by_role(PartRole::DrawerFront).count();
                if placed.len() < doors {
                    assert!(
                        list.warnings.iter().any(|w| w.contains("does not fit")),
                        "{who}"
                    );
                }
                for c in placed {
                    let m = crate::cad::transform(&c, false, 0.0);
                    let at = |x: f64, y: f64, z: f64| {
                        Vec3::new(
                            m[0] * x + m[1] * y + m[2] * z + m[3],
                            m[4] * x + m[5] * y + m[6] * z + m[7],
                            m[8] * x + m[9] * y + m[10] * z + m[11],
                        )
                    };
                    // The front whose face the model's origin is on.
                    let o = at(0.0, 0.0, 0.0);
                    let front = list
                        .parts
                        .iter()
                        .filter(|f| matches!(f.role, PartRole::Door | PartRole::DrawerFront))
                        .find(|f| {
                            let b = f.bounds;
                            o.x >= b.min.x
                                && o.x <= b.max().x
                                && o.z >= b.min.z
                                && o.z <= b.max().z
                                && (o.y - b.min.y).abs() < 1e-6
                        })
                        .unwrap_or_else(|| panic!("{who}: model not on a front's face: {o:?}"));
                    let b = front.bounds;
                    let half = p("length") / 2.0;
                    for x in [-half, half] {
                        for z in [p("across_low"), p("across_high")] {
                            let q = at(x, 0.0, z);
                            assert!(
                                q.x >= b.min.x - 1e-6
                                    && q.x <= b.max().x + 1e-6
                                    && q.z >= b.min.z - 1e-6
                                    && q.z <= b.max().z + 1e-6,
                                "{who}: {q:?} leaves {} {b:?}",
                                front.name
                            );
                        }
                    }
                    // The model's -Y is off the door's face, out of the cabinet.
                    assert!(
                        at(0.0, -p("projection"), 0.0).y < o.y,
                        "{who}: into the front"
                    );
                }
            }
        }
    }

    /// Pairs of Pedra Curve meet at the doors' gap: each one's straight edge at its free edge.
    #[test]
    fn a_one_straight_edged_pull_keeps_that_edge_to_the_doors_free_edge() {
        use crate::model::HandleRange;
        let mut cab = Cabinet::new("S01", CabinetKind::Sink);
        cab.standard.handle_range = HandleRange::Abi;
        cab.standard.handle_style = "handle.abi.pedracurve-320.beige-travertine".into();
        let list = build_cabinet(&cab).unwrap();
        let placed: Vec<_> = crate::cad::cad_placements(&list)
            .into_iter()
            .filter(|c| c.item.starts_with("handle."))
            .collect();
        assert_eq!(placed.len(), 2, "{:?}", list.warnings);
        let mut pulls: Vec<_> = placed
            .iter()
            .map(|c| crate::cad::transform(c, false, 0.0))
            .collect();
        pulls.sort_by(|a, b| a[3].total_cmp(&b[3]));
        // The left door's model +Z runs towards +X, the right door's towards -X.
        assert!(pulls[0][2] > 0.5, "{:?}", pulls[0]);
        assert!(pulls[1][2] < -0.5, "{:?}", pulls[1]);
    }

    #[test]
    fn an_unknown_abi_style_falls_back_to_the_generic_handle() {
        use crate::model::HandleRange;
        let mut cab = Cabinet::new("B01", CabinetKind::Base);
        cab.standard.handle_range = HandleRange::Abi;
        cab.standard.handle_style = "handle.abi.no-such".into();
        let list = build_cabinet(&cab).unwrap();
        assert!(list.hardware.iter().any(|h| h.item == "handle.cc128"));
        assert!(list.warnings.iter().any(|w| w.contains("no-such")));
    }

    #[test]
    fn a_saved_standard_without_a_handle_range_stays_generic() {
        let mut v = serde_json::to_value(crate::model::ConstructionStandard::default()).unwrap();
        let obj = v.as_object_mut().unwrap();
        obj.remove("handle_range");
        obj.remove("handle_style");
        let s: crate::model::ConstructionStandard = serde_json::from_value(v).unwrap();
        assert_eq!(s.handle_range, crate::model::HandleRange::Generic);
        assert!(s.handle_style.is_empty());
        let back = serde_json::to_value(&s).unwrap();
        assert!(
            back.get("handle_style").is_none(),
            "an empty style is not stored"
        );
    }

    #[test]
    fn knobs_take_one_hole_edge_pulls_none_and_handleless_fronts_no_handle() {
        let mut cab = Cabinet::new("B01", CabinetKind::Base);
        cab.standard.handle_size = crate::model::HandleSize::Knob;
        let list = build_cabinet(&cab).unwrap();
        assert_eq!(handle_holes(&list, "B01.Door1").len(), 1);
        cab.standard.handle_place = HandlePlace::EdgePull;
        let list = build_cabinet(&cab).unwrap();
        assert!(handle_holes(&list, "B01.Door1").is_empty());
        assert!(list.hardware.iter().any(|h| h.item == "handle.edge-pull"));
        cab.standard.opening = Opening::TipOn;
        let list = build_cabinet(&cab).unwrap();
        assert!(!list.hardware.iter().any(|h| h.item.starts_with("handle.")));
    }

    /// The L-corner's bi-fold pair: one handle, on the second door's free edge.
    #[test]
    fn an_l_corner_has_one_handle_on_the_folding_door() {
        let list = build_cabinet(&Cabinet::new("C01", CabinetKind::CornerBase)).unwrap();
        assert!(handle_holes(&list, "C01.Door1").is_empty());
        let h = handle_holes(&list, "C01.Door2");
        assert_eq!(h.len(), 2);
        let d2 = list.part("C01.Door2").unwrap().bounds;
        assert!(
            (h[0].y - (d2.min.y + HANDLE_EDGE_INSET)).abs() < 1e-9,
            "{h:?}"
        );
        assert!((h[0].x - d2.min.x).abs() < 1e-9, "from its back face");
    }

    /// HAF-3: four supports per adjustable shelf, none for fixed ones.
    #[test]
    fn adjustable_shelves_get_four_supports_and_fixed_ones_none() {
        let mut cab = Cabinet::new("T01", CabinetKind::Tall);
        cab.fixed_shelves = vec![2];
        let list = build_cabinet(&cab).unwrap();
        let supports: Vec<_> = list
            .hardware
            .iter()
            .filter(|h| h.item == "shelf-support.5mm")
            .collect();
        assert_eq!(supports.len(), 3);
        assert!(supports.iter().all(|h| h.quantity == 4));
        assert!(!supports.iter().any(|h| h.part == "T01.Shelf2"));
    }

    /// HAF-5: each light gets its strip and a Ø10 cable hole through the panel between it and
    /// the void behind the back.
    #[test]
    fn lights_bore_a_cable_hole_through_the_panel_toward_the_back_void() {
        use crate::model::{Light, LightZone, Voltage};
        let light = |zone| Light {
            zone,
            voltage: Voltage::V24,
            watts_per_metre: 10.0,
            profile: String::new(),
        };
        let cases = [
            (
                CabinetKind::Wall,
                TopJoint::Rails,
                LightZone::UnderCabinet,
                "W01.Bottom",
                false,
            ),
            (
                CabinetKind::Wall,
                TopJoint::Rails,
                LightZone::InCabinet,
                "W01.BackRail",
                true,
            ),
            (
                CabinetKind::Wall,
                TopJoint::BetweenSides,
                LightZone::InCabinet,
                "W01.Top",
                true,
            ),
            (
                CabinetKind::Base,
                TopJoint::Rails,
                LightZone::Plinth,
                "W01.Bottom",
                false,
            ),
        ];
        for (kind, top, zone, panel, up) in cases {
            let mut cab = Cabinet::new("W01", kind);
            cab.standard.top_joint = top;
            cab.lights = vec![light(zone)];
            let list = build_cabinet(&cab).unwrap();
            assert!(
                list.warnings.iter().all(|w| w.contains("shelf-pin")),
                "{zone:?}: {:?}",
                list.warnings
            );
            let holes: Vec<_> = list
                .operations
                .iter()
                .filter_map(|o| match o {
                    Operation::Bore {
                        part,
                        direction,
                        diameter,
                        depth,
                        purpose: BorePurpose::CableHole,
                        ..
                    } => Some((part.as_str(), direction.positive, *diameter, *depth)),
                    _ => None,
                })
                .collect();
            assert_eq!(holes, [(panel, up, 10.0, 16.0)], "{zone:?}");
            assert!(list.hardware.iter().any(|h| h.item == "lighting.strip.24v"));
        }
    }

    #[test]
    fn a_light_in_a_zone_the_cabinet_lacks_is_left_out_with_a_warning() {
        use crate::model::{Light, LightZone, Voltage};
        let mut cab = Cabinet::new("B01", CabinetKind::Base);
        cab.lights = vec![Light {
            zone: LightZone::UnderCabinet,
            voltage: Voltage::V12,
            watts_per_metre: 4.8,
            profile: String::new(),
        }];
        let list = build_cabinet(&cab).unwrap();
        assert!(list.warnings.iter().any(|w| w.contains("does not fit")));
        assert!(
            !list
                .hardware
                .iter()
                .any(|h| h.item.starts_with("lighting."))
        );
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

    /// Blum E-SERVICES check 1 (docs/blum-checks.md): a 600 × 850 × 580 standard cabinet,
    /// 16 mm throughout, a lay-on back and one door hinged left on EXPANDO hinges.
    fn blum_check_1() -> Cabinet {
        let mut cab = base();
        cab.height = 850.0;
        let s = &mut cab.standard;
        s.toe_kick = ToeKick::None;
        s.top_joint = crate::model::TopJoint::OverSides;
        s.bottom_joint = crate::model::BottomJoint::UnderSides;
        s.back = crate::model::BackStyle::Applied;
        s.back_thickness = 16.0;
        s.front_thickness = 16.0;
        s.front_gap = 3.0;
        s.top_gap = 2.0;
        s.bottom_gap = 0.0;
        s.hinge_boss = crate::model::HingeBoss::Expando;
        cab
    }

    fn bores(list: &PartList, part: &str, purpose: BorePurpose) -> Vec<Vec3> {
        list.operations
            .iter()
            .filter_map(|o| match o {
                Operation::Bore {
                    part: p,
                    at,
                    purpose: q,
                    ..
                } if p == part && *q == purpose => Some(*at),
                _ => None,
            })
            .collect()
    }

    /// Blum check 1: the door takes Blum's side, top and bottom gaps.
    #[test]
    fn overlay_doors_keep_blums_side_top_and_bottom_gaps() {
        let list = build_cabinet(&blum_check_1()).unwrap();
        let door = list.part("B01.Door1").unwrap().bounds;
        assert!((door.size.x - 597.0).abs() < 1e-9 && (door.min.x - 1.5).abs() < 1e-9);
        assert!((door.size.z - 848.0).abs() < 1e-9 && door.min.z.abs() < 1e-9);
        let mut cab = blum_check_1();
        cab.standard.bottom_gap = 2.0;
        let raised = build_cabinet(&cab).unwrap();
        let door = raised.part("B01.Door1").unwrap().bounds;
        assert!((door.min.z - 2.0).abs() < 1e-9 && (door.size.z - 846.0).abs() < 1e-9);
    }

    /// Blum check 1: cups 21 mm in (TB 3.5) and the end hinges 80 mm from the door's ends, the
    /// plates centred on each hinge's height.
    #[test]
    fn end_hinges_sit_80_from_the_door_ends_as_blum_places_them() {
        let list = build_cabinet(&blum_check_1()).unwrap();
        let door = list.part("B01.Door1").unwrap().bounds;
        let cups = bores(&list, "B01.Door1", BorePurpose::HingeCup);
        assert!(cups.iter().all(|c| (c.x - door.min.x - 21.0).abs() < 1e-9));
        let zs: Vec<f64> = cups.iter().map(|c| c.z - door.min.z).collect();
        assert!((zs.first().unwrap() - 80.0).abs() < 1e-9, "{zs:?}");
        assert!((zs.last().unwrap() - 768.0).abs() < 1e-9, "{zs:?}");
        let plates = bores(&list, "B01.LeftSide", BorePurpose::HingePlate);
        for c in &cups {
            let mut near: Vec<f64> = plates
                .iter()
                .filter(|p| (p.z - c.z).abs() < 20.0)
                .map(|p| p.z - c.z)
                .collect();
            near.sort_by(f64::total_cmp);
            assert_eq!(near, vec![-16.0, 16.0]);
        }
    }

    /// Blum check 1: EXPANDO hinges and plates, with two Ø8 boss holes 45 apart, 9.5 behind
    /// each cup, on whichever hand the door hinges.
    #[test]
    fn expando_hinges_drill_two_boss_holes_behind_the_cup() {
        for hand in [HingeSide::Left, HingeSide::Right] {
            let mut cab = blum_check_1();
            cab.fronts = FrontLayout::Door { hinge_side: hand };
            let list = build_cabinet(&cab).unwrap();
            let article = |prefix: &str| {
                let h = list
                    .hardware
                    .iter()
                    .find(|h| h.item.starts_with(prefix))
                    .unwrap();
                crate::hardware::item(&h.item).unwrap().article.clone()
            };
            assert_eq!(article("blum.clip-top-blumotion"), "71B358E");
            assert_eq!(article("blum.mounting-plate"), "174H7100E");
            let cups = bores(&list, "B01.Door1", BorePurpose::HingeCup);
            let bosses = bores(&list, "B01.Door1", BorePurpose::HingeBoss);
            assert_eq!(bosses.len(), 2 * cups.len());
            let away = if hand == HingeSide::Left { 1.0 } else { -1.0 };
            for c in &cups {
                let mut dz: Vec<f64> = bosses
                    .iter()
                    .filter(|b| (b.z - c.z).abs() < 30.0)
                    .inspect(|b| assert!((b.x - c.x - away * 9.5).abs() < 1e-9, "{hand:?}"))
                    .map(|b| b.z - c.z)
                    .collect();
                dz.sort_by(f64::total_cmp);
                assert_eq!(dz, vec![-22.5, 22.5]);
            }
        }
    }

    /// INSERTA comes for every application; EXPANDO only overlay, so a half-overlay door falls
    /// back to screw-on hinges and says so.
    #[test]
    fn inserta_fits_every_application_and_expando_falls_back_on_dual_doors() {
        let mut cab = blum_check_1();
        cab.standard.overlay = DoorOverlay::Half;
        cab.standard.hinge_boss = crate::model::HingeBoss::Inserta;
        let list = build_cabinet(&cab).unwrap();
        assert!(
            list.hardware
                .iter()
                .any(|h| h.item == "blum.clip-top-blumotion.110.dual.inserta")
        );
        assert!(
            list.hardware
                .iter()
                .any(|h| h.item == "blum.mounting-plate.cruciform-37-32.inserta.0")
        );
        assert!(!bores(&list, "B01.Door1", BorePurpose::HingeBoss).is_empty());
        cab.standard.hinge_boss = crate::model::HingeBoss::Expando;
        let list = build_cabinet(&cab).unwrap();
        assert!(
            list.hardware
                .iter()
                .any(|h| h.item == "blum.clip-top-blumotion.110.dual")
        );
        assert!(list.warnings.iter().any(|w| w.contains("no expando hinge")));
        assert!(bores(&list, "B01.Door1", BorePurpose::HingeBoss).is_empty());
    }

    /// Blum check 1: a cover cap on every 110° hinge's arm, the one for its application.
    #[test]
    fn every_110_hinge_gets_its_arm_cover_cap() {
        for (overlay, cap, article) in [
            (
                DoorOverlay::Full,
                "blum.hinge-cover-cap.overlay",
                "70.1503.BP",
            ),
            (
                DoorOverlay::Inset,
                "blum.hinge-cover-cap.dual-inset",
                "70.1663.BP",
            ),
        ] {
            let mut cab = blum_check_1();
            cab.standard.overlay = overlay;
            let list = build_cabinet(&cab).unwrap();
            let hinges = list
                .hardware
                .iter()
                .filter(|h| h.item.starts_with("blum.clip-top-blumotion.110."))
                .count();
            let caps = list.hardware.iter().filter(|h| h.item == cap).count();
            assert!(hinges > 0 && caps == hinges, "{overlay:?}");
            assert_eq!(crate::hardware::item(cap).unwrap().article, article);
        }
    }

    /// Blum check 1: an adjustable shelf rests on its pins, a pin's radius above the holes.
    #[test]
    fn adjustable_shelves_rest_on_top_of_their_pins() {
        let list = build_cabinet(&blum_check_1()).unwrap();
        let shelf = list.part("B01.Shelf1").unwrap().bounds;
        let pins = bores(&list, "B01.LeftSide", BorePurpose::ShelfPin);
        let under = pins
            .iter()
            .map(|p| p.z)
            .filter(|z| *z < shelf.min.z)
            .fold(f64::MIN, f64::max);
        assert!((shelf.min.z - under - SHELF_PIN_DIAMETER / 2.0).abs() < 1e-9);
    }

    fn lit(profile: &str) -> PartList {
        let mut cab = Cabinet::new("W01", CabinetKind::Wall);
        cab.lights = vec![crate::model::Light {
            zone: crate::model::LightZone::UnderCabinet,
            voltage: crate::model::Voltage::V24,
            watts_per_metre: 9.6,
            profile: profile.into(),
        }];
        build_cabinet(&cab).unwrap()
    }

    #[test]
    fn a_recessed_light_profile_routes_its_groove_in_the_panels_underside() {
        let list = lit("hafele.loox5.profile.1103");
        let bottom = list.part("W01.Bottom").unwrap().bounds;
        let groove = list
            .operations
            .iter()
            .find_map(|o| match o {
                // The bottom has a groove for the back as well.
                Operation::Groove { part, bounds }
                    if part == "W01.Bottom" && (bounds.size.y - 13.9).abs() < 1e-6 =>
                {
                    Some(*bounds)
                }
                _ => None,
            })
            .expect("a groove");
        assert_eq!(groove.min.z, bottom.min.z);
        assert_eq!(groove.size.z, 12.0);
        assert!((groove.size.y - 13.9).abs() < 1e-6);
        assert!(
            list.hardware
                .iter()
                .any(|h| h.item == "hafele.loox5.profile.1103")
        );
    }

    #[test]
    fn a_surface_light_profile_cuts_nothing() {
        let list = lit("hafele.loox5.profile.2190");
        let grooves = |l: &PartList| {
            l.operations
                .iter()
                .filter(|o| matches!(o, Operation::Groove { part, .. } if part == "W01.Bottom"))
                .count()
        };
        assert_eq!(grooves(&list), grooves(&lit("")));
        assert!(
            list.hardware
                .iter()
                .any(|h| h.item == "hafele.loox5.profile.2190")
        );
    }

    /// The owner's report: a knock-in plate's tips sat on the bottom panel with no holes, holding
    /// the plate 11 mm below it. The tips go into holes and the plate's face meets the panel.
    #[test]
    fn leg_plates_fix_into_holes_on_the_panel_for_every_width_and_fixing() {
        for fixing in [LegFixing::KnockIn, LegFixing::ScrewOn] {
            for width in [300.0, 450.0, 600.0, 900.0, 1200.0] {
                let mut cab = Cabinet::new("B01", CabinetKind::Base);
                cab.width = width;
                cab.standard.toe_kick = ToeKick::LegsAndPlinth;
                cab.standard.leg_fixing = fixing;
                let list = build_cabinet(&cab).unwrap();
                let legs = list
                    .hardware
                    .iter()
                    .filter(|h| h.item.starts_with("hafele.axilo.set."))
                    .count();
                assert!(legs >= 4);
                let bottom = list.part("B01.Bottom").unwrap().bounds;
                let holes = list
                    .operations
                    .iter()
                    .filter(|o| {
                        matches!(o, Operation::Bore { part, at, purpose: BorePurpose::LegPlate, .. }
                        if part == "B01.Bottom" && at.z == bottom.min.z)
                    })
                    .count();
                assert_eq!(
                    holes,
                    legs * 4,
                    "{fixing:?} at {width}: {:?}",
                    list.warnings
                );
                assert!(
                    !list.warnings.iter().any(|w| w.contains("plate fixings")),
                    "{:?}",
                    list.warnings
                );
            }
        }
    }

    #[test]
    fn a_recessed_profile_has_a_hole_for_its_cable_at_the_grooves_end() {
        let list = lit("hafele.loox5.profile.1103");
        let bottom = list.part("W01.Bottom").unwrap().bounds;
        let y = bottom.min.y + LIGHT_PROFILE_SETBACK;
        assert!(list.operations.iter().any(|o| matches!(o,
            Operation::Bore { part, at, purpose: BorePurpose::CableHole, .. }
                if part == "W01.Bottom" && (at.y - y).abs() < 1e-6)));
    }

    fn zoned(kind: CabinetKind, zone: crate::model::LightZone, profile: &str) -> PartList {
        let mut cab = Cabinet::new("R01", kind);
        if zone == crate::model::LightZone::UnderFixedShelf {
            cab.adjustable_shelves = 2;
            cab.fixed_shelves = vec![2];
        }
        cab.lights = vec![crate::model::Light {
            zone,
            voltage: crate::model::Voltage::V24,
            watts_per_metre: 9.6,
            profile: profile.into(),
        }];
        build_cabinet(&cab).unwrap()
    }

    #[test]
    fn a_light_can_run_up_both_sides_at_the_rear_of_a_wardrobe_with_a_channel_for_its_cable() {
        let list = zoned(
            CabinetKind::Wardrobe,
            crate::model::LightZone::SideRear,
            "hafele.loox5.profile.1103",
        );
        let grooves: Vec<&String> = list
            .operations
            .iter()
            .filter_map(|o| match o {
                Operation::Groove { part, bounds } if (bounds.size.y - 13.9).abs() < 1e-6 => {
                    Some(part)
                }
                _ => None,
            })
            .collect();
        assert_eq!(grooves.len(), 2, "{:?}", list.warnings);
        assert!(grooves.iter().any(|p| p.ends_with("LeftSide")));
        assert!(grooves.iter().any(|p| p.ends_with("RightSide")));
        assert_eq!(
            list.hardware
                .iter()
                .filter(|h| h.item == "hafele.loox5.profile.1103")
                .count(),
            2
        );
        assert!(
            !list.warnings.iter().any(|w| w.contains("clash")),
            "{:?}",
            list.warnings
        );
    }

    #[test]
    fn a_light_can_go_under_a_fixed_shelf_50_from_the_rear() {
        let list = zoned(
            CabinetKind::Tall,
            crate::model::LightZone::UnderFixedShelf,
            "hafele.loox5.profile.2190",
        );
        let row = list
            .hardware
            .iter()
            .find(|h| h.item == "hafele.loox5.profile.2190")
            .expect("a profile");
        let shelf = list.part(&row.part).unwrap();
        assert_eq!(shelf.role, PartRole::Shelf);
        assert!(
            (shelf.bounds.max().y - row.at.y - 50.0).abs() < 5.0,
            "{}",
            row.at.y
        );
    }

    #[test]
    fn grain_direction_turns_every_front() {
        let axes = |g: FrontGrain, kind| {
            let mut cab = Cabinet::new("C1", kind);
            cab.standard.front_grain = g;
            let list = build_cabinet(&cab).unwrap();
            list.parts
                .iter()
                .filter(|p| matches!(p.role, PartRole::Door | PartRole::DrawerFront))
                .map(|p| p.grain_axis)
                .collect::<Vec<_>>()
        };
        for kind in [CabinetKind::Base, CabinetKind::DrawerBank] {
            assert!(
                axes(FrontGrain::Upright, kind)
                    .iter()
                    .all(|a| *a == Some(Axis::Z))
            );
            assert!(
                axes(FrontGrain::Across, kind)
                    .iter()
                    .all(|a| *a == Some(Axis::X))
            );
        }
    }

    #[test]
    fn a_finger_pulls_notch_and_gap_can_be_set() {
        let mut cab = Cabinet::new("D1", CabinetKind::DrawerBank);
        cab.standard.handle_place = crate::model::HandlePlace::FingerPull;
        let plain = build_cabinet(&cab).unwrap();
        cab.standard.finger_pull_height = 40.0;
        cab.standard.finger_pull_depth = 12.0;
        cab.standard.finger_pull_gap = 15.0;
        let list = build_cabinet(&cab).unwrap();
        assert!(list.operations.iter().any(|o| matches!(o,
            Operation::Groove { bounds, .. }
                if (bounds.size.z - 40.0).abs() < 1e-6 && (bounds.size.y - 12.0).abs() < 1e-6)));
        let gap = |l: &PartList| {
            let mut z: Vec<Aabb> = l.by_role(PartRole::DrawerFront).map(|p| p.bounds).collect();
            z.sort_by(|a, b| a.min.z.total_cmp(&b.min.z));
            z[1].min.z - z[0].max().z
        };
        assert!(gap(&list) > gap(&plain));
        assert_eq!(gap(&list), 15.0);
    }

    #[test]
    fn a_shaker_panel_takes_its_own_board_when_chosen() {
        let mut cab = Cabinet::new("B01", CabinetKind::Base);
        cab.standard.door_style = DoorStyle::Shaker;
        cab.standard.front_board = "frame".into();
        let panel = |c: &Cabinet| {
            build_cabinet(c)
                .unwrap()
                .by_role(PartRole::FramePanel)
                .next()
                .unwrap()
                .board
                .clone()
        };
        assert_eq!(panel(&cab), "");
        cab.standard.shaker_panel_board = "panel".into();
        assert_eq!(panel(&cab), "panel");
    }

    #[test]
    fn a_shaker_rail_shows_only_between_its_stiles() {
        let mut cab = Cabinet::new("B01", CabinetKind::Base);
        cab.standard.door_style = DoorStyle::Shaker;
        let list = build_cabinet(&cab).unwrap();
        for rail in list.by_role(PartRole::FrameRail) {
            let seen = list.visible_bounds(rail);
            assert!(
                seen.size.x < rail.bounds.size.x,
                "the tenons are in the grooves"
            );
            for stile in list.by_role(PartRole::FrameStile) {
                assert!(
                    !seen.overlaps(&stile.bounds),
                    "{} in {}",
                    rail.name,
                    stile.name
                );
            }
        }
    }
}
