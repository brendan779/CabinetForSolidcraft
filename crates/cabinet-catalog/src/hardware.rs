//! Hardware catalog records: hinges, plates, runners, drawer systems, legs, hangers, connectors.
//! One file per brand and family under `catalog/hardware/`. Every number a builder uses (a cup
//! diameter, a boring distance, a runner deduction) is a named parameter here with the
//! document it came from, never a literal in code.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::Source;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HardwareKind {
    Hinge,
    HingePlate,
    Runner,
    DrawerSystem,
    LiftSystem,
    Leg,
    Handle,
    Hanger,
    Connector,
    ShelfSupport,
    Lighting,
    Other,
}

/// One hole of an item's drilling pattern, in the item's own frame: `x` along the panel edge it
/// is set out from, `y` in from that edge, both in mm from the item's reference point.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DrillHole {
    pub x: f64,
    pub y: f64,
    pub diameter: f64,
    pub depth: f64,
    /// What the hole is for, as the supplier names it ("cup", "plate screw", "dowel").
    #[serde(default)]
    pub note: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HardwareItem {
    /// Stable id the builder places: `blum.clip-top-blumotion.110.full`.
    pub id: String,
    pub brand: String,
    /// The supplier's article number, as printed.
    pub article: String,
    /// Product family ("CLIP top BLUMOTION").
    pub family: String,
    /// What distinguishes this article in its family ("110°, full overlay").
    #[serde(default)]
    pub variant: String,
    pub kind: HardwareKind,
    /// Named dimensions in mm (or degrees, kg, as the name says): `cup_diameter`,
    /// `boring_distance_min`, `runner_deduction`...
    #[serde(default)]
    pub parameters: BTreeMap<String, f64>,
    #[serde(default)]
    pub drilling: Vec<DrillHole>,
    pub source: Source,
}

impl HardwareItem {
    /// A named parameter, or an error naming the item and parameter, so a missing number stops
    /// a build instead of becoming a guess.
    pub fn param(&self, name: &str) -> Result<f64, String> {
        self.parameters
            .get(name)
            .copied()
            .ok_or_else(|| format!("{} has no parameter {name}", self.id))
    }

    pub(crate) fn problems(&self) -> Vec<String> {
        let mut out = Vec::new();
        if self.article.trim().is_empty() {
            out.push(format!("{}: no article number", self.id));
        }
        if self.source.url.trim().is_empty() {
            out.push(format!("{}: no source", self.id));
        }
        for (k, v) in &self.parameters {
            if !v.is_finite() {
                out.push(format!("{}: parameter {k} is not a number", self.id));
            }
        }
        for (i, h) in self.drilling.iter().enumerate() {
            if !(h.diameter > 0.0 && h.depth > 0.0 && h.x.is_finite() && h.y.is_finite()) {
                out.push(format!(
                    "{}: drilling hole {} is not a real hole",
                    self.id,
                    i + 1
                ));
            }
        }
        out
    }
}

/// A hardware file: `catalog/hardware/<brand>-<family>.json`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct HardwareFile {
    pub brand: String,
    #[serde(default)]
    pub website: String,
    #[serde(default)]
    pub notes: String,
    pub items: Vec<HardwareItem>,
}
