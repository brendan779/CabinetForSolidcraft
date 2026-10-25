//! Parts: what gets cut, machined and assembled.

use serde::{Deserialize, Serialize};

use crate::geom::{Aabb, Axis, Vec3};
use crate::model::BiscuitSize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PartRole {
    LeftSide,
    RightSide,
    Top,
    Bottom,
    FrontRail,
    BackRail,
    Back,
    Shelf,
    Door,
    DrawerFront,
    Plinth,
    /// The fixed front over the blind part of a blind corner cabinet.
    BlindPanel,
    /// A strip in the plane of the fronts: beside a blind panel, between cabinets or to a wall.
    Filler,
    /// A decorative panel on the end of a run.
    EndPanel,
    /// The base of a drawer box, cut to the drawer system's sizes.
    DrawerBottom,
    /// The back of a drawer box.
    DrawerBack,
    /// The sides of a wooden drawer box.
    DrawerLeftSide,
    DrawerRightSide,
    /// The front of a wooden drawer box, behind the drawer front.
    DrawerInnerFront,
    /// The upper (1) and lower (2) front of a bi-fold lift.
    LiftFront,
}

impl PartRole {
    pub fn label(self) -> &'static str {
        match self {
            PartRole::LeftSide => "Left side",
            PartRole::RightSide => "Right side",
            PartRole::Top => "Top",
            PartRole::Bottom => "Bottom",
            PartRole::FrontRail => "Front rail",
            PartRole::BackRail => "Back rail",
            PartRole::Back => "Back",
            PartRole::Shelf => "Shelf",
            PartRole::Door => "Door",
            PartRole::DrawerFront => "Drawer front",
            PartRole::Plinth => "Plinth",
            PartRole::BlindPanel => "Blind panel",
            PartRole::Filler => "Filler",
            PartRole::EndPanel => "End panel",
            PartRole::DrawerBottom => "Drawer bottom",
            PartRole::DrawerBack => "Drawer back",
            PartRole::DrawerLeftSide | PartRole::DrawerRightSide => "Drawer side",
            PartRole::DrawerInnerFront => "Drawer box front",
            PartRole::LiftFront => "Lift front",
        }
    }

    /// Used in body names: `B01.LeftSide`.
    pub fn code(self) -> &'static str {
        match self {
            PartRole::LeftSide => "LeftSide",
            PartRole::RightSide => "RightSide",
            PartRole::Top => "Top",
            PartRole::Bottom => "Bottom",
            PartRole::FrontRail => "FrontRail",
            PartRole::BackRail => "BackRail",
            PartRole::Back => "Back",
            PartRole::Shelf => "Shelf",
            PartRole::Door => "Door",
            PartRole::DrawerFront => "DrawerFront",
            PartRole::Plinth => "Plinth",
            PartRole::BlindPanel => "BlindPanel",
            PartRole::Filler => "Filler",
            PartRole::EndPanel => "EndPanel",
            PartRole::DrawerBottom => "DrawerBottom",
            PartRole::DrawerBack => "DrawerBack",
            PartRole::DrawerLeftSide => "DrawerLeftSide",
            PartRole::DrawerRightSide => "DrawerRightSide",
            PartRole::DrawerInnerFront => "DrawerInnerFront",
            PartRole::LiftFront => "LiftFront",
        }
    }

    /// Made from the front board and seen from the room.
    pub fn is_front(self) -> bool {
        matches!(
            self,
            PartRole::Door
                | PartRole::DrawerFront
                | PartRole::LiftFront
                | PartRole::Plinth
                | PartRole::BlindPanel
                | PartRole::Filler
                | PartRole::EndPanel
        )
    }
}

/// Which edges of a panel get edge tape, named by the panel's face directions in the cabinet
/// frame. For a part whose thickness is along X, the edges are its ±Y and ±Z faces.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EdgeBanding {
    pub front: bool,
    pub back: bool,
    pub top: bool,
    pub bottom: bool,
    pub left: bool,
    pub right: bool,
}

impl EdgeBanding {
    pub fn front_only() -> Self {
        Self {
            front: true,
            ..Self::default()
        }
    }

    pub fn all() -> Self {
        Self {
            front: true,
            back: true,
            top: true,
            bottom: true,
            left: true,
            right: true,
        }
    }

    /// Banded edges at the low and high end of `axis`.
    pub fn on_axis(&self, axis: Axis) -> (bool, bool) {
        match axis {
            Axis::X => (self.left, self.right),
            Axis::Y => (self.front, self.back),
            Axis::Z => (self.bottom, self.top),
        }
    }
}

/// One panel, an axis-aligned box in the cabinet frame.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Part {
    /// `B01.LeftSide`, `B01.Shelf2`. Unique in a cabinet and stable across rebuilds.
    pub name: String,
    pub role: PartRole,
    pub bounds: Aabb,
    /// The axis the board's thickness runs along.
    pub thickness_axis: Axis,
    /// The axis wood grain or decor direction runs along, when it matters.
    pub grain_axis: Option<Axis>,
    /// Catalog board id, or empty for the standard's default.
    pub board: String,
    pub edges: EdgeBanding,
}

impl Part {
    pub fn thickness(&self) -> f64 {
        self.bounds.size.get(self.thickness_axis)
    }

    /// Finished length and width: the two in-plane sizes, length along the grain (or the longer
    /// one without grain).
    pub fn finished_size(&self) -> (f64, f64) {
        let [a, b] = self.thickness_axis.others();
        let (sa, sb) = (self.bounds.size.get(a), self.bounds.size.get(b));
        match self.grain_axis {
            Some(g) if g == b => (sb, sa),
            Some(_) => (sa, sb),
            None if sb > sa => (sb, sa),
            None => (sa, sb),
        }
    }
}

/// A machining operation, positioned in the cabinet frame.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Operation {
    /// A round hole. `at` is the centre of the hole's mouth, `direction` points into the material.
    Bore {
        part: String,
        at: Vec3,
        direction: Dir,
        diameter: f64,
        depth: f64,
        purpose: BorePurpose,
    },
    /// A biscuit slot spanning two parts. `bounds` is the slot volume across the joint.
    BiscuitSlot {
        parts: [String; 2],
        bounds: Aabb,
        along: Axis,
        /// The biscuit that goes in it.
        #[serde(default = "default_biscuit")]
        size: BiscuitSize,
    },
    /// A rectangular groove or rebate.
    Groove { part: String, bounds: Aabb },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BorePurpose {
    /// A screw's clearance hole through the face panel.
    Screw,
    /// The seat for a countersunk screw head, at the mouth of its clearance hole. Recorded for
    /// sheets and CNC; not cut in SolidCraft (a cone there is cosmetic).
    Countersink,
    ScrewPilot,
    ShelfPin,
    HingeCup,
    HingePlate,
    Dowel,
    /// A drawer system's front fixing (LEGRABOX EXPANDO bracket), in the back of a drawer front.
    FrontFixing,
}

fn default_biscuit() -> BiscuitSize {
    BiscuitSize::No20
}

/// A direction along an axis.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dir {
    pub axis: Axis,
    pub positive: bool,
}

impl Dir {
    pub const fn new(axis: Axis, positive: bool) -> Self {
        Self { axis, positive }
    }

    pub fn sign(self) -> f64 {
        if self.positive { 1.0 } else { -1.0 }
    }
}

/// A hardware item placed in the cabinet (hinge, runner, leg...).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HardwarePlacement {
    /// Catalog id, for example `blum.clip-top-blumotion.110.full`.
    pub item: String,
    pub description: String,
    pub at: Vec3,
    /// The part it is fixed to.
    pub part: String,
    pub quantity: u32,
}

/// Everything [`crate::build_cabinet`] produces for one cabinet.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PartList {
    pub cabinet: String,
    pub parts: Vec<Part>,
    pub operations: Vec<Operation>,
    pub hardware: Vec<HardwarePlacement>,
    /// Problems the builder worked around (moved fixings, skipped biscuits...).
    pub warnings: Vec<String>,
}

impl PartList {
    pub fn part(&self, name: &str) -> Option<&Part> {
        self.parts.iter().find(|p| p.name == name)
    }

    pub fn by_role(&self, role: PartRole) -> impl Iterator<Item = &Part> {
        self.parts.iter().filter(move |p| p.role == role)
    }
}
