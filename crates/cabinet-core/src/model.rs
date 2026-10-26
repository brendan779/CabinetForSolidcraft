//! The cabinet model: what the user chooses. Every enum here is a visual picker in the app.

use serde::{Deserialize, Serialize};

use crate::geom::{Aabb, Vec3};

/// A drawn box at least this tall (mm) is a tall cabinet: base cabinets stop near 900, pantries
/// and oven towers start near 1800.
const TALL_FROM_HEIGHT: f64 = 1500.0;
/// A drawn box whose underside is at least this high (mm) hangs on the wall: wall cabinets sit
/// about 1350-1450 off the floor, above a 900 benchtop and splashback.
const WALL_FROM_ELEVATION: f64 = 1000.0;

/// A cabinet: an envelope (the box the user drew), a kind and a construction standard.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Cabinet {
    /// Short stable id, used in part names (`B01`).
    pub id: String,
    pub kind: CabinetKind,
    /// Outside width (X), depth (Y) and height (Z) of the envelope, carcass + toe kick, without
    /// doors.
    pub width: f64,
    pub depth: f64,
    pub height: f64,
    /// Where the envelope's minimum corner sits in the design (mm).
    #[serde(default)]
    pub origin: Vec3,
    #[serde(default)]
    pub standard: ConstructionStandard,
    #[serde(default)]
    pub fronts: FrontLayout,
    /// Shelves inside the main opening (the name is historical: fixed ones count too).
    #[serde(default)]
    pub adjustable_shelves: u32,
    /// Which shelves, numbered from 1 at the bottom, are fixed: full width, biscuited and screwed
    /// between the sides like a bottom. The rest sit on shelf pins.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fixed_shelves: Vec<u32>,
    /// Blum inner drawers behind the doors, stacked from the bottom of the opening; they take
    /// the shelves' place.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub inner_drawers: u32,
    /// Blum inner dividing sets in the steel-sided drawers: AMBIA-LINE cutlery insert and frames
    /// (LEGRABOX, MERIVOBOX), ORGA-LINE container and utensil sets (TANDEMBOX).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub drawer_inserts: bool,
    /// LED strips: under the cabinet, inside it, or in its plinth.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lights: Vec<Light>,
    /// The blind part of a blind corner cabinet (ignored by other kinds).
    #[serde(default)]
    pub blind: Blind,
    /// Sink cabinet options (ignored by other kinds).
    #[serde(default)]
    pub sink: Sink,
    /// The rangehood insert a canopy houses (ignored by other kinds).
    #[serde(default)]
    pub insert: Insert,
    /// How a wardrobe hangs clothes (ignored by other kinds).
    #[serde(default)]
    pub wardrobe: Hanging,
    /// Lining boards (a Surround profile) on the faces seen from the room: an island's back
    /// and ends.
    #[serde(default, skip_serializing_if = "Cladding::is_none")]
    pub cladding: Cladding,
    /// The second leg of an L-shaped corner cabinet (ignored by other kinds).
    #[serde(default)]
    pub corner: Corner,
    /// Sides seen from the room (the end of a run without an end panel): no screws through them.
    #[serde(default, skip_serializing_if = "Exposed::is_none")]
    pub exposed: Exposed,
}

/// An LED strip light on a cabinet (see `lighting`).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Light {
    pub zone: LightZone,
    pub voltage: Voltage,
    /// The strip's power per metre, from its data sheet.
    pub watts_per_metre: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LightZone {
    /// Under a wall cabinet, lighting the bench.
    UnderCabinet,
    /// Under the top, inside the cabinet.
    InCabinet,
    /// In the plinth, lighting the floor.
    Plinth,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Voltage {
    #[serde(rename = "12v")]
    V12,
    #[serde(rename = "24v")]
    V24,
}

impl Voltage {
    pub fn label(self) -> &'static str {
        match self {
            Voltage::V12 => "12 V",
            Voltage::V24 => "24 V",
        }
    }
}

fn is_zero(n: &u32) -> bool {
    *n == 0
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Exposed {
    pub left: bool,
    pub right: bool,
}

impl Exposed {
    pub fn is_none(&self) -> bool {
        !self.left && !self.right
    }
}

/// A sink cabinet is a base cabinet without the back rail, where the bowl and plumbing go.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Sink {
    /// A cut-out low in the back for the waste and water pipes.
    pub back_cutout: bool,
    /// A waterproof liner on the bottom panel.
    pub liner: bool,
}

impl Default for Sink {
    fn default() -> Self {
        Self {
            back_cutout: true,
            liner: true,
        }
    }
}

/// A rangehood insert's cut-out in the canopy's bottom: from the insert maker's drawing.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Insert {
    pub width: f64,
    pub depth: f64,
}

impl Default for Insert {
    /// Our placeholder until the insert is chosen: a common 520 × 260 cut-out.
    fn default() -> Self {
        Self {
            width: 520.0,
            depth: 260.0,
        }
    }
}

/// Which faces are clad, and in what.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Cladding {
    pub left: bool,
    pub right: bool,
    pub back: bool,
    /// Catalog lining board id, or empty for generic.
    pub board: String,
    pub thickness: f64,
}

impl Default for Cladding {
    /// Nothing clad; 12 mm, a common Surround thickness, when something is.
    fn default() -> Self {
        Self {
            left: false,
            right: false,
            back: false,
            board: String::new(),
            thickness: 12.0,
        }
    }
}

impl Cladding {
    pub fn is_none(&self) -> bool {
        !(self.left || self.right || self.back)
    }
}

/// A wardrobe's interior: a fixed hat shelf near the top with a rail under it, and for double
/// hanging a second fixed shelf and rail halfway down.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Hanging {
    /// Long hanging: coats and dresses.
    #[default]
    Long,
    /// Two rails, one above the other: shirts and trousers.
    Double,
}

/// A blind corner cabinet's front: from its blind side, a fixed blind panel and a filler cover
/// `width` (where the other run of cabinets meets it); one door covers the rest.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Blind {
    pub side: HingeSide,
    /// Front width covered by the blind panel and filler, from the blind side edge.
    pub width: f64,
    /// Width of the filler between the blind panel and the door.
    pub filler: f64,
}

impl Default for Blind {
    fn default() -> Self {
        // Covers a 580 mm deep run meeting it, plus the filler that lets the door clear that
        // run's handles.
        Self {
            side: HingeSide::Left,
            width: 630.0,
            filler: 50.0,
        }
    }
}

/// An L-shaped corner base or corner wall: its main leg runs `width` along the back wall like any
/// straight cabinet (front at Y = 0); the second leg runs `width` here along the side wall on `side`, coming
/// forward (to negative Y), both `depth` deep. The legs share one interior behind a bi-fold door
/// pair. Its envelope therefore reaches `width − depth` in front of the cabinet's origin.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Corner {
    pub side: HingeSide,
    /// Outside length of the second leg, from the back wall.
    pub width: f64,
}

impl Default for Corner {
    fn default() -> Self {
        Self {
            side: HingeSide::Left,
            width: 900.0,
        }
    }
}

impl Corner {
    /// Square by default: a corner wall's second leg is as long as its usual width.
    pub fn for_kind(kind: CabinetKind) -> Self {
        match kind {
            CabinetKind::CornerWall => Self {
                width: kind.default_size().0,
                ..Self::default()
            },
            _ => Self::default(),
        }
    }
}

impl Cabinet {
    /// A cabinet of `kind` with the kind's usual size and the default standard.
    pub fn new(id: impl Into<String>, kind: CabinetKind) -> Self {
        let (w, d, h) = kind.default_size();
        Self {
            id: id.into(),
            kind,
            width: w,
            depth: d,
            height: h,
            origin: Vec3::default(),
            standard: ConstructionStandard::default(),
            fronts: kind.default_fronts_for(w),
            adjustable_shelves: kind.default_shelves(),
            fixed_shelves: Vec::new(),
            inner_drawers: 0,
            drawer_inserts: false,
            lights: Vec::new(),
            blind: Blind::default(),
            sink: Sink::default(),
            wardrobe: Hanging::default(),
            insert: Insert::default(),
            cladding: Cladding::default(),
            corner: Corner::for_kind(kind),
            exposed: Exposed::default(),
        }
    }

    /// A cabinet filling a box the user drew in the design: the box is the envelope, its front is
    /// the box's −Y face, and its size picks the kind (see [`CabinetKind::for_box`]).
    pub fn from_box(id: impl Into<String>, bounds: Aabb) -> Self {
        let kind = CabinetKind::for_box(&bounds);
        let mut cab = Self::new(id, kind);
        cab.width = bounds.size.x;
        cab.depth = bounds.size.y;
        cab.height = bounds.size.z;
        cab.origin = bounds.min;
        cab.fronts = kind.default_fronts_for(cab.width);
        cab
    }

    pub fn has_toe_kick(&self) -> bool {
        matches!(
            self.kind,
            CabinetKind::Base
                | CabinetKind::Tall
                | CabinetKind::DrawerBank
                | CabinetKind::BlindCorner
                | CabinetKind::CornerBase
                | CabinetKind::Sink
                | CabinetKind::Wardrobe
        ) && self.standard.toe_kick != ToeKick::None
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CabinetKind {
    Base,
    Wall,
    Tall,
    DrawerBank,
    OpenShelf,
    /// A base cabinet in an inside corner: part of its front is blind behind the other run.
    BlindCorner,
    /// A base cabinet for a sink: no back rail, a cut-out for pipes and a waterproof liner.
    Sink,
    /// An L-shaped base cabinet in an inside corner, both legs one interior, with bi-fold doors.
    CornerBase,
    /// The wall-height L-corner: the corner base's legs and bi-fold doors, hung on hangers.
    CornerWall,
    /// A full-height wardrobe with a hat shelf and hanging rails.
    Wardrobe,
    /// A canopy box between wall cabinets housing a rangehood insert.
    Rangehood,
}

impl CabinetKind {
    pub const ALL: [CabinetKind; 11] = [
        CabinetKind::Base,
        CabinetKind::Wall,
        CabinetKind::Tall,
        CabinetKind::DrawerBank,
        CabinetKind::OpenShelf,
        CabinetKind::BlindCorner,
        CabinetKind::Sink,
        CabinetKind::CornerBase,
        CabinetKind::CornerWall,
        CabinetKind::Wardrobe,
        CabinetKind::Rangehood,
    ];

    /// The kind a drawn box most likely is: as tall as a pantry → tall, hanging off the floor →
    /// wall, otherwise base. The user changes it in the picker.
    pub fn for_box(bounds: &Aabb) -> CabinetKind {
        if bounds.size.z >= TALL_FROM_HEIGHT {
            CabinetKind::Tall
        } else if bounds.min.z >= WALL_FROM_ELEVATION {
            CabinetKind::Wall
        } else {
            CabinetKind::Base
        }
    }

    /// Typical Australian kitchen sizes (width, depth, height), mm.
    pub fn default_size(self) -> (f64, f64, f64) {
        match self {
            CabinetKind::Base => (600.0, 580.0, 870.0),
            CabinetKind::DrawerBank => (600.0, 580.0, 870.0),
            CabinetKind::Wall => (600.0, 330.0, 720.0),
            CabinetKind::Tall => (600.0, 580.0, 2100.0),
            CabinetKind::OpenShelf => (600.0, 300.0, 720.0),
            CabinetKind::BlindCorner => (1000.0, 580.0, 870.0),
            CabinetKind::Sink => (900.0, 580.0, 870.0),
            CabinetKind::CornerBase => (900.0, 580.0, 870.0),
            CabinetKind::CornerWall => (600.0, 330.0, 720.0),
            CabinetKind::Wardrobe => (900.0, 600.0, 2100.0),
            CabinetKind::Rangehood => (900.0, 330.0, 720.0),
        }
    }

    pub fn default_fronts_for(self, width: f64) -> FrontLayout {
        match self {
            CabinetKind::OpenShelf => FrontLayout::Open,
            // Its bi-fold pair, whatever the layout says.
            CabinetKind::CornerBase | CabinetKind::CornerWall => FrontLayout::DoorPair,
            // One door on the open part, hinged away from the corner.
            CabinetKind::BlindCorner => FrontLayout::Door {
                hinge_side: HingeSide::Right,
            },
            CabinetKind::DrawerBank => FrontLayout::Drawers {
                ratios: vec![1.0, 1.0, 1.0],
            },
            _ if width > 600.0 => FrontLayout::DoorPair,
            _ => FrontLayout::Door {
                hinge_side: HingeSide::Left,
            },
        }
    }

    fn default_shelves(self) -> u32 {
        match self {
            CabinetKind::Base
            | CabinetKind::Wall
            | CabinetKind::OpenShelf
            | CabinetKind::BlindCorner
            | CabinetKind::CornerBase
            | CabinetKind::CornerWall => 1,
            // The hat shelf and rails are its own; a canopy holds an extractor.
            CabinetKind::Wardrobe | CabinetKind::Rangehood => 0,
            // The bowl and pipes take the space.
            CabinetKind::Sink => 0,
            CabinetKind::Tall => 4,
            CabinetKind::DrawerBank => 0,
        }
    }
}

/// How the front of the main opening is filled.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum FrontLayout {
    Open,
    Door {
        hinge_side: HingeSide,
    },
    DoorPair,
    /// Drawer fronts top to bottom, sized in proportion to `ratios`.
    Drawers {
        ratios: Vec<f64>,
    },
    /// One front that lifts on Blum AVENTOS: HK top, HS top or HL top.
    Lift {
        #[serde(default)]
        kind: LiftKind,
        /// Weight of the handle on the front (kg), for choosing the mechanism.
        #[serde(default = "default_handle_weight")]
        handle_weight: f64,
    },
    /// Two equal fronts that fold up together on Blum AVENTOS HF top.
    BiFoldLift {
        /// Weight of the handle on the fronts (kg), for the power factor.
        #[serde(default = "default_handle_weight")]
        handle_weight: f64,
    },
    /// Sections one above the other, bottom first, each with a fixed shelf between it and the
    /// next: drawers, a door or an open niche for an appliance (an oven tower, a fridge housing).
    Stack {
        sections: Vec<StackSection>,
    },
}

/// One section of a stacked front.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StackSection {
    /// For drawers and doors, the height of the section's fronts; for a niche, its clear
    /// opening between the shelves. 0 takes what the others leave.
    #[serde(default)]
    pub height: f64,
    #[serde(flatten)]
    pub kind: SectionKind,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum SectionKind {
    Drawers {
        count: u32,
    },
    Door {
        hinge_side: HingeSide,
    },
    DoorPair,
    /// No front: an oven, microwave or fridge goes in.
    Niche,
}

/// The owner's typical handle: 400 g (decided 2026-10-26).
pub const DEFAULT_HANDLE_WEIGHT: f64 = 0.4;

fn default_handle_weight() -> f64 {
    DEFAULT_HANDLE_WEIGHT
}

impl Default for FrontLayout {
    fn default() -> Self {
        FrontLayout::Door {
            hinge_side: HingeSide::Left,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HingeSide {
    Left,
    Right,
}

/// A shop's way of building cabinets, applied to every cabinet unless overridden.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ConstructionStandard {
    pub name: String,
    pub carcass: CarcassStyle,
    /// Carcass board thickness.
    pub carcass_thickness: f64,
    /// Catalog id of the carcass board (see cabinet-catalog), or empty for generic.
    pub carcass_board: String,
    pub front_thickness: f64,
    pub front_board: String,
    /// Catalog id of the back board, or empty for generic. Its thickness is `back_thickness`.
    pub back_board: String,
    pub top_joint: TopJoint,
    pub bottom_joint: BottomJoint,
    pub back: BackStyle,
    pub back_thickness: f64,
    pub toe_kick: ToeKick,
    pub toe_kick_height: f64,
    /// How far the plinth sits back from the carcass front.
    pub toe_kick_setback: f64,
    pub overlay: DoorOverlay,
    /// Gap between a front and its neighbour or the envelope edge.
    pub front_gap: f64,
    pub fixings: FixingRecipe,
    pub biscuit: BiscuitSize,
    /// The DOMINO tenon when the fixings are DOMINO.
    pub domino: DominoSize,
    /// The dowel, and where dowels go, when the fixings are dowels.
    pub dowel: DowelSize,
    pub dowel_spacing: DowelSpacing,
    pub fixing_density: Density,
    /// Width of the front and back rails when the top is rails.
    pub rail_width: f64,
    /// Front and back clearance of adjustable shelves.
    pub shelf_front_setback: f64,
    pub edge_band_thickness: f64,
    /// How wall cabinets hang.
    pub wall_hanging: WallHanging,
    /// How doors and drawers are opened.
    pub opening: Opening,
    /// The handle's hole centres, or a knob, when the fronts have handles.
    pub handle_size: HandleSize,
    /// Where handles go on the fronts.
    pub handle_place: HandlePlace,
    /// The handles' finish.
    pub handle_finish: HandleFinish,
    /// Slab fronts, or shaker (five-piece) frames with a panel.
    pub door_style: DoorStyle,
    /// What the drawers are made of and run on.
    pub drawer_system: DrawerSystem,
    /// Whether deep LEGRABOX drawers may be high fronted pull-outs (C, F).
    pub drawer_sides: DrawerSides,
    /// Whether a cabinet's stacked fronts are cut in sequence so their grain runs on.
    pub front_grain: FrontGrain,
    /// AMBIA-LINE drawer inserts in steel or wood design.
    pub insert_design: InsertDesign,
    /// Board thickness of wooden drawer boxes (sides, base, front and back).
    pub drawer_board_thickness: f64,
    /// What the fronts are made of, for their weight on lift systems.
    pub front_core: FrontCore,
}

/// Which boards of a cabinet a catalog board is assigned to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BoardSlot {
    Carcass,
    Front,
    Back,
}

impl ConstructionStandard {
    /// Uses catalog board `id` for `slot`. The thickness follows the board: a cabinet is built
    /// from the boards it is made of.
    pub fn set_board(&mut self, slot: BoardSlot, id: impl Into<String>, thickness: f64) {
        let (board, t) = match slot {
            BoardSlot::Carcass => (&mut self.carcass_board, &mut self.carcass_thickness),
            BoardSlot::Front => (&mut self.front_board, &mut self.front_thickness),
            BoardSlot::Back => (&mut self.back_board, &mut self.back_thickness),
        };
        *board = id.into();
        *t = thickness;
    }
}

impl Default for ConstructionStandard {
    fn default() -> Self {
        Self {
            name: "Frameless 16 mm, biscuit + screw".into(),
            carcass: CarcassStyle::Frameless,
            carcass_thickness: 16.0,
            carcass_board: String::new(),
            front_thickness: 18.0,
            front_board: String::new(),
            back_board: String::new(),
            top_joint: TopJoint::Rails,
            bottom_joint: BottomJoint::BetweenSides,
            back: BackStyle::InsetGroove,
            back_thickness: 6.0,
            toe_kick: ToeKick::LegsAndPlinth,
            toe_kick_height: 150.0,
            toe_kick_setback: 50.0,
            overlay: DoorOverlay::Full,
            front_gap: 2.0,
            fixings: FixingRecipe::BiscuitAndScrew,
            biscuit: BiscuitSize::No20,
            domino: DominoSize::D5x30,
            dowel: DowelSize::D8x35,
            dowel_spacing: DowelSpacing::System32,
            fixing_density: Density::Normal,
            rail_width: 100.0,
            shelf_front_setback: 2.0,
            edge_band_thickness: 1.0,
            wall_hanging: WallHanging::Hangers,
            opening: Opening::Handles,
            handle_size: HandleSize::Cc128,
            handle_place: HandlePlace::Corner,
            handle_finish: HandleFinish::Stainless,
            door_style: DoorStyle::Slab,
            drawer_system: DrawerSystem::Legrabox,
            drawer_sides: DrawerSides::Standard,
            front_grain: FrontGrain::Free,
            insert_design: InsertDesign::Steel,
            drawer_board_thickness: 16.0,
            front_core: FrontCore::Mdf,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CarcassStyle {
    Frameless,
    FaceFrame,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TopJoint {
    /// Full top panel fitted between the sides.
    BetweenSides,
    /// Full top panel over the sides; sides are shorter.
    OverSides,
    /// Front and back rails between the sides (base cabinets under a benchtop).
    Rails,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BottomJoint {
    BetweenSides,
    UnderSides,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackStyle {
    /// Thin back in grooves in sides, top and bottom, set in from the rear.
    InsetGroove,
    /// Back screwed flush onto the rear of the carcass.
    Applied,
    /// Full-thickness back between sides, top and bottom.
    Structural,
    None,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Opening {
    /// Handles or knobs (HAF-2 places them).
    Handles,
    /// Handleless: press the front and a Blum TIP-ON unit pushes it open.
    TipOn,
    /// Handleless, electric: Blum SERVO-DRIVE opens drawers at a touch. Hinged doors cannot
    /// take it and get TIP-ON.
    ServoDrive,
}

/// A handle's hole centres (the common sizes, as handle makers list them), or a knob.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HandleSize {
    Knob,
    Cc96,
    #[default]
    Cc128,
    Cc160,
    Cc192,
    Cc224,
    Cc256,
    Cc320,
    Cc448,
}

impl HandleSize {
    /// Hole centres (mm); `None` for a knob's single hole.
    pub fn centres(self) -> Option<f64> {
        match self {
            HandleSize::Knob => None,
            HandleSize::Cc96 => Some(96.0),
            HandleSize::Cc128 => Some(128.0),
            HandleSize::Cc160 => Some(160.0),
            HandleSize::Cc192 => Some(192.0),
            HandleSize::Cc224 => Some(224.0),
            HandleSize::Cc256 => Some(256.0),
            HandleSize::Cc320 => Some(320.0),
            HandleSize::Cc448 => Some(448.0),
        }
    }
}

/// How fronts are made.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DoorStyle {
    /// One board.
    #[default]
    Slab,
    /// Two stiles and two rails, stub-tenoned, with a thin panel in their grooves. Fronts too
    /// small for a frame stay slab.
    Shaker,
    /// The shaker frame rebated at the back for glass held by beads (doors and lift fronts;
    /// drawer fronts get a shaker panel).
    GlassFrame,
}

/// A handle's finish.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HandleFinish {
    #[default]
    Stainless,
    Black,
    BrushedBrass,
    BrushedNickel,
}

impl HandleFinish {
    pub fn label(self) -> &'static str {
        match self {
            HandleFinish::Stainless => "stainless steel",
            HandleFinish::Black => "matt black",
            HandleFinish::BrushedBrass => "brushed brass",
            HandleFinish::BrushedNickel => "brushed nickel",
        }
    }
}

/// Where a front's handle goes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HandlePlace {
    /// Doors: upright, by the free edge near the top (base), bottom (wall) or at hand height
    /// (tall). Drawers: across the middle of the front, near the top of deep ones.
    #[default]
    Corner,
    /// Doors: upright, by the free edge, halfway up. Drawers: across the middle of the front.
    Centre,
    /// An edge pull on the top edge (bottom edge of wall doors): screwed from behind, no holes
    /// through the front.
    EdgePull,
    /// Handleless: a finger grip routed along the back of the top edge (bottom edge of wall
    /// fronts).
    FingerPull,
}

/// How a single lifting front moves.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LiftKind {
    /// Swings up about its top edge and stays (AVENTOS HK top).
    #[default]
    StayLift,
    /// Rises and tips over the cabinet top (AVENTOS HS top), for large fronts.
    UpAndOver,
    /// Rises parallel to itself (AVENTOS HL top), for small fronts.
    LiftUp,
    /// A small stay lift (AVENTOS HK-S), for small wall cabinets and over the fridge.
    SmallStayLift,
    /// A stay lift on hinges along the top (AVENTOS HK-XS), for the smallest fronts.
    HingedStayLift,
    /// A stay lift set into pockets in the cabinet sides (AVENTOS HKi, fully integrated).
    IntegratedStayLift,
}

/// A front's core board, for its weight (Blum's reference densities).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FrontCore {
    Mdf,
    Chipboard,
}

/// Drawer boxes and their runners.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DrawerSystem {
    /// Blum LEGRABOX: steel sides, a cut base and back; N, M or K by the opening's height.
    Legrabox,
    /// Blum MERIVOBOX, as LEGRABOX.
    Merivobox,
    /// Blum TANDEMBOX antaro, as LEGRABOX.
    Tandembox,
    /// A wooden box on Blum MOVENTO runners (40 kg, 60/70 kg for the longest).
    Movento,
    /// A wooden box on Blum TANDEM 560H runners (30 kg).
    Tandem,
}

/// The look of Blum AMBIA-LINE inserts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InsertDesign {
    #[default]
    Steel,
    /// Wood décor sides on steel (oak or walnut, with a matching steel colour).
    Wood,
}

/// How the grain of a cabinet's fronts lines up.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FrontGrain {
    /// Each front cut on its own; drawer fronts' grain runs across.
    #[default]
    Free,
    /// Fronts stacked in a column (an oven tower's doors and drawers, a drawer bank) run their
    /// grain upright and are cut one after the other from one strip of the sheet, so the grain
    /// runs on from front to front.
    Matched,
}

/// How tall a steel box's sides may be.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DrawerSides {
    /// N, M or K: a deep drawer is a K box behind a tall front.
    #[default]
    Standard,
    /// LEGRABOX pure C (177 mm) and F (241 mm) sides for deep drawers, where there is room.
    HighFronted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WallHanging {
    /// Concealed adjustable hangers in the top back corners, hooked onto a rail screwed to the
    /// wall. They sit in the void behind an inset back.
    Hangers,
    /// No hanging hardware: the installer fixes the cabinet.
    None,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToeKick {
    /// Carcass on adjustable legs, separate clip-on plinth.
    LegsAndPlinth,
    /// Carcass on a separate plinth box.
    PlinthBox,
    /// Sides run to the floor with a notch at the front.
    IntegratedNotch,
    None,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DoorOverlay {
    Full,
    Half,
    Inset,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FixingRecipe {
    /// Biscuits for alignment, a screw between each pair of biscuits.
    BiscuitAndScrew,
    BiscuitOnly,
    ScrewOnly,
    /// 7 mm confirmat screws.
    Confirmat,
    /// Fluted wooden dowels (size and spacing in the standard).
    Dowel,
    /// Cam and bolt (Häfele Minifix style) for flat-pack.
    CamLock,
    /// Festool DOMINO loose tenons, glued.
    Domino,
}

/// Fluted wooden dowels, diameter × length.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DowelSize {
    D6x30,
    D8x30,
    D8x35,
    D8x40,
    D10x40,
    D10x50,
}

impl DowelSize {
    pub fn diameter(self) -> f64 {
        match self {
            DowelSize::D6x30 => 6.0,
            DowelSize::D8x30 | DowelSize::D8x35 | DowelSize::D8x40 => 8.0,
            DowelSize::D10x40 | DowelSize::D10x50 => 10.0,
        }
    }

    pub fn length(self) -> f64 {
        match self {
            DowelSize::D6x30 | DowelSize::D8x30 => 30.0,
            DowelSize::D8x35 => 35.0,
            DowelSize::D8x40 | DowelSize::D10x40 => 40.0,
            DowelSize::D10x50 => 50.0,
        }
    }
}

/// Where dowels go along a joint.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DowelSpacing {
    /// On the 32 mm grid from the joint's front end, as a line boring machine drills them.
    System32,
    /// Evenly spread at the fixing spacing, as a hand doweling jig sets them out.
    Jig,
}

/// Festool DOMINO tenon sizes, thickness × nominal length (the width is in the catalog). The
/// first six are cut with the DF 500, the rest with the DF 700 (DOMINO XL).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DominoSize {
    D4x20,
    D5x30,
    D6x40,
    D8x40,
    D8x50,
    D10x50,
    D8x80,
    D10x80,
    D12x100,
    D14x100,
}

impl DominoSize {
    /// The tenon's id in `catalog/hardware/festool-domino.json`.
    pub fn catalog_id(self) -> String {
        let code = serde_json::to_value(self)
            .ok()
            .and_then(|v| v.as_str().map(str::to_string))
            .unwrap_or_default();
        format!("festool.domino.{code}")
    }
}

/// Lamello-style biscuit sizes. Nominal length × width × thickness in mm.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BiscuitSize {
    No0,
    No10,
    No20,
}

impl BiscuitSize {
    pub fn length(self) -> f64 {
        match self {
            BiscuitSize::No0 => 47.0,
            BiscuitSize::No10 => 53.0,
            BiscuitSize::No20 => 56.0,
        }
    }

    pub fn width(self) -> f64 {
        match self {
            BiscuitSize::No0 => 15.0,
            BiscuitSize::No10 => 19.0,
            BiscuitSize::No20 => 23.0,
        }
    }

    pub fn thickness(self) -> f64 {
        4.0
    }

    /// Slot depth in each panel: half the biscuit width plus 1 mm glue room.
    pub fn slot_depth(self) -> f64 {
        self.width() / 2.0 + 1.0
    }

    /// Length of the slot at the panel face for a 100 mm cutter plunged to `slot_depth`.
    pub fn slot_length(self) -> f64 {
        let r = 50.0;
        let d = self.slot_depth();
        2.0 * (r * r - (r - d) * (r - d)).sqrt()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Density {
    Sparse,
    Normal,
    Dense,
}

impl Density {
    /// Largest centre-to-centre spacing between biscuits.
    pub fn max_spacing(self) -> f64 {
        match self {
            Density::Sparse => 250.0,
            Density::Normal => 180.0,
            Density::Dense => 120.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bounds(min: [f64; 3], size: [f64; 3]) -> Aabb {
        Aabb::new(
            Vec3::new(min[0], min[1], min[2]),
            Vec3::new(size[0], size[1], size[2]),
        )
    }

    #[test]
    fn a_box_on_the_floor_becomes_a_base_cabinet_of_its_size_where_it_is() {
        let cab = Cabinet::from_box("B01", bounds([1200.0, 30.0, 0.0], [450.0, 560.0, 870.0]));
        assert_eq!(cab.kind, CabinetKind::Base);
        assert_eq!((cab.width, cab.depth, cab.height), (450.0, 560.0, 870.0));
        assert_eq!(cab.origin, Vec3::new(1200.0, 30.0, 0.0));
        assert!(crate::build_cabinet(&cab).is_ok());
    }

    #[test]
    fn a_box_up_the_wall_becomes_a_wall_cabinet() {
        let cab = Cabinet::from_box("W01", bounds([0.0, 0.0, 1400.0], [600.0, 330.0, 720.0]));
        assert_eq!(cab.kind, CabinetKind::Wall);
    }

    #[test]
    fn a_pantry_sized_box_becomes_a_tall_cabinet() {
        let cab = Cabinet::from_box("T01", bounds([0.0, 0.0, 0.0], [600.0, 580.0, 2100.0]));
        assert_eq!(cab.kind, CabinetKind::Tall);
    }

    #[test]
    fn a_wide_box_gets_a_door_pair() {
        let cab = Cabinet::from_box("B01", bounds([0.0, 0.0, 0.0], [900.0, 580.0, 870.0]));
        assert_eq!(cab.fronts, FrontLayout::DoorPair);
    }
}
