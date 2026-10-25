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
    /// Named two-way tables the supplier prints (minimum gap by boring distance × door
    /// thickness...).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub tables: BTreeMap<String, Table>,
    pub source: Source,
}

/// A supplier's two-way table: `values[row][col]`, `None` where the document prints no number
/// (a dash, or "trial recommended").
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Table {
    /// What the rows are (`boring_distance`), and the columns (`front_thickness`).
    pub rows_name: String,
    pub cols_name: String,
    pub rows: Vec<f64>,
    pub cols: Vec<f64>,
    pub values: Vec<Vec<Option<f64>>>,
    #[serde(default)]
    pub note: String,
}

impl Table {
    /// Between printed headings, the larger of the surrounding values: the safe reading of a
    /// minimum (a gap). `None` outside the table or where a surrounding cell prints no number.
    pub fn upper(&self, row: f64, col: f64) -> Option<f64> {
        self.around(row, col)?.into_iter().reduce(f64::max)
    }

    /// Between printed headings, the smaller of the surrounding values: the safe reading of a
    /// maximum (a door thickness).
    pub fn lower(&self, row: f64, col: f64) -> Option<f64> {
        self.around(row, col)?.into_iter().reduce(f64::min)
    }

    /// The cells at the headings either side of `row` and `col` (one heading when it matches).
    fn around(&self, row: f64, col: f64) -> Option<Vec<f64>> {
        fn either_side(heads: &[f64], v: f64) -> Option<Vec<usize>> {
            if let Some(i) = heads.iter().position(|h| (h - v).abs() < 1e-9) {
                return Some(vec![i]);
            }
            let i = heads.iter().position(|h| *h > v)?;
            (i > 0).then(|| vec![i - 1, i])
        }
        let (rs, cs) = (either_side(&self.rows, row)?, either_side(&self.cols, col)?);
        let mut out = Vec::new();
        for r in &rs {
            for c in &cs {
                out.push(self.values.get(*r)?.get(*c).copied().flatten()?);
            }
        }
        Some(out)
    }

    fn problems(&self) -> Option<String> {
        let sorted = |v: &[f64]| v.windows(2).all(|w| w[0] < w[1]);
        let shaped = self.values.len() == self.rows.len()
            && self.values.iter().all(|r| r.len() == self.cols.len());
        (!(shaped && sorted(&self.rows) && sorted(&self.cols)))
            .then(|| "headings must rise and the values fit them".to_string())
    }
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

    /// A named table, or an error naming the item and table.
    pub fn table(&self, name: &str) -> Result<&Table, String> {
        self.tables
            .get(name)
            .ok_or_else(|| format!("{} has no table {name}", self.id))
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
        for (name, t) in &self.tables {
            if let Some(p) = t.problems() {
                out.push(format!("{}: table {name}: {p}", self.id));
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
