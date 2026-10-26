//! Cabinet domain model for CabinetForSolidcraft.
//!
//! The cabinet model is the source of truth. [`build::build_cabinet`] turns a [`model::Cabinet`]
//! into a [`part::PartList`]: every panel as an axis-aligned box in the cabinet's frame, plus the
//! machining (bores, biscuit slots, grooves) and hardware placements. Cut lists, cut sheets and the
//! SolidCraft geometry are all derived from that part list.
//!
//! Cabinet frame: X runs left to right across the front, Y runs from the front face (Y = 0) to the
//! back, Z runs up from the floor. Lengths are millimetres.

pub mod bom;
pub mod build;
pub mod cad;
pub mod collisions;
pub mod corner;
pub mod cutlist;
pub mod drawers;
pub mod geom;
pub mod hardware;
pub mod hinges;
pub mod joinery;
pub mod lift;
pub mod lighting;
pub mod model;
pub mod nesting;
pub mod options;
pub mod part;
pub mod project;
pub mod quote;
pub mod run;
pub mod standins;

pub use build::build_cabinet;
pub use model::{Cabinet, CabinetKind, ConstructionStandard};
pub use part::{Part, PartList};
pub use project::Project;
pub use run::{Run, RunItem, build_run, laid_out};

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum Error {
    #[error("cabinet is too small: {0}")]
    TooSmall(String),
    #[error("invalid value: {0}")]
    Invalid(String),
}

pub type Result<T> = std::result::Result<T, Error>;
