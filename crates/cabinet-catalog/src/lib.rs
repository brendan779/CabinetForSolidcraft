//! Catalog data: boards (Polytec, Laminex, ...), edge tape and hardware (Blum, Häfele).
//!
//! Data lives in JSON files under `catalog/` at the repository root, one file per brand and
//! product family, so overnight agents can add brands without touching code. Every record carries
//! a [`Source`]: where the fact came from, when, and whether a person checked it.

use std::collections::{BTreeMap, HashSet};
use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{path}: {source}")]
    Io {
        path: String,
        source: std::io::Error,
    },
    #[error("{path}: {source}")]
    Json {
        path: String,
        source: serde_json::Error,
    },
    #[error("invalid catalog: {0}")]
    Invalid(String),
}

/// Where a record's facts came from.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Source {
    /// Document or page URL.
    pub url: String,
    /// The document's own date or version, as printed (for example "2026-10-23").
    pub document_date: String,
    /// When the record was entered (ISO date).
    pub retrieved: String,
    /// True only once a person has checked the record against the source.
    #[serde(default)]
    pub verified: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Substrate {
    /// Particleboard (chipboard), E1 / E0.
    Particleboard,
    MoistureResistantParticleboard,
    Mdf,
    MoistureResistantMdf,
    Plywood,
    Hmr,
    Compact,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BoardKind {
    /// Low pressure melamine on a substrate.
    Melamine,
    /// High pressure laminate sheet (not a board).
    Laminate,
    /// Laminate pressed onto a substrate.
    LaminatedBoard,
    Veneer,
    Thermolaminate,
    /// Profiled or decorative wall lining.
    Lining,
    /// Raw substrate for painting.
    Raw,
}

/// A sheet size in mm: `length` is the long side, along the decor's grain when it has one.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SheetSize {
    pub length: f64,
    pub width: f64,
}

/// One stocked board: a decor in one finish, thickness and substrate, with its sheet sizes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Board {
    /// Stable id: `polytec.ravine.prime-oak.16.mr-mdf`.
    pub id: String,
    pub brand: String,
    /// The product range (Decorative, Commercial, Wardrobe...).
    pub range: String,
    /// Decor or colour name as the brand prints it ("Prime Oak").
    pub decor: String,
    /// Finish or texture as the brand prints it ("Woodmatt", "Ravine", "Smooth").
    pub finish: String,
    pub kind: BoardKind,
    pub substrate: Substrate,
    pub thickness: f64,
    pub sheets: Vec<SheetSize>,
    /// True when the decor has a direction (woodgrains), so parts must follow the sheet length.
    #[serde(default)]
    pub grained: bool,
    /// Decor on both faces.
    #[serde(default = "yes")]
    pub double_sided: bool,
    /// Emission class as printed (E0, E1, E-Zero).
    #[serde(default)]
    pub emission: String,
    /// Edge tape ids that match this decor.
    #[serde(default)]
    pub edge_tapes: Vec<String>,
    /// Approximate decor colour for previews, `#rrggbb`. Not a colour match.
    #[serde(default)]
    pub preview_colour: String,
    /// The brand's item code per sheet size, keyed `"2400x1200"`.
    #[serde(default)]
    pub item_codes: BTreeMap<String, String>,
    pub source: Source,
}

fn yes() -> bool {
    true
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TapeMaterial {
    Abs,
    Pvc,
    Pp,
    Melamine,
    Veneer,
    Acrylic,
    Aluminium,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EdgeTape {
    /// Stable id: `polytec.abs.prime-oak-woodmatt.1x21`.
    pub id: String,
    pub brand: String,
    /// The brand's article or product code, when it has one.
    #[serde(default)]
    pub code: String,
    pub decor: String,
    pub finish: String,
    pub material: TapeMaterial,
    pub thickness: f64,
    pub width: f64,
    /// Roll lengths in metres.
    pub rolls: Vec<f64>,
    /// Pre-glued (hot melt) or un-glued.
    #[serde(default)]
    pub pre_glued: bool,
    /// Laser / zero-joint functional layer.
    #[serde(default)]
    pub laser: bool,
    pub source: Source,
}

/// A brand's file: `catalog/boards/<brand>.json` and `catalog/edge-tape/<brand>.json` hold one
/// each.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BrandFile {
    pub brand: String,
    #[serde(default)]
    pub website: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub boards: Vec<Board>,
    #[serde(default)]
    pub edge_tapes: Vec<EdgeTape>,
}

/// True for a finite size above zero (NaN is not positive).
fn positive(v: f64) -> bool {
    v.is_finite() && v > 0.0
}

/// Everything loaded from a catalog folder.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Catalog {
    pub boards: Vec<Board>,
    pub edge_tapes: Vec<EdgeTape>,
}

impl Catalog {
    /// Loads every `*.json` under `root/boards` and `root/edge-tape` and checks the result.
    pub fn load(root: &Path) -> Result<Self, Error> {
        let mut cat = Catalog::default();
        for sub in ["boards", "edge-tape"] {
            let dir = root.join(sub);
            let Ok(entries) = std::fs::read_dir(&dir) else {
                continue;
            };
            let mut paths: Vec<_> = entries
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.extension().is_some_and(|x| x == "json"))
                .collect();
            paths.sort();
            for path in paths {
                let shown = path.display().to_string();
                let text = std::fs::read_to_string(&path).map_err(|source| Error::Io {
                    path: shown.clone(),
                    source,
                })?;
                let file: BrandFile =
                    serde_json::from_str(&text).map_err(|source| Error::Json {
                        path: shown.clone(),
                        source,
                    })?;
                cat.boards.extend(file.boards);
                cat.edge_tapes.extend(file.edge_tapes);
            }
        }
        cat.validate()?;
        Ok(cat)
    }

    /// Unique ids, positive sizes and edge tape references that resolve.
    pub fn validate(&self) -> Result<(), Error> {
        let mut problems = Vec::new();
        let mut ids = HashSet::new();
        for b in &self.boards {
            if !ids.insert(b.id.as_str()) {
                problems.push(format!("duplicate board id {}", b.id));
            }
            if !positive(b.thickness) {
                problems.push(format!("{}: thickness must be positive", b.id));
            }
            if b.sheets.is_empty() && b.kind != BoardKind::Lining {
                problems.push(format!("{}: no sheet sizes", b.id));
            }
            for s in &b.sheets {
                if !(positive(s.length) && positive(s.width)) {
                    problems.push(format!("{}: bad sheet size", b.id));
                }
            }
        }
        let mut tape_ids = HashSet::new();
        for t in &self.edge_tapes {
            if !tape_ids.insert(t.id.as_str()) {
                problems.push(format!("duplicate edge tape id {}", t.id));
            }
            if !(positive(t.thickness) && positive(t.width)) {
                problems.push(format!("{}: bad tape size", t.id));
            }
        }
        for b in &self.boards {
            for t in &b.edge_tapes {
                if !tape_ids.contains(t.as_str()) {
                    problems.push(format!("{}: unknown edge tape {t}", b.id));
                }
            }
        }
        if problems.is_empty() {
            Ok(())
        } else {
            Err(Error::Invalid(problems.join("; ")))
        }
    }

    pub fn board(&self, id: &str) -> Option<&Board> {
        self.boards.iter().find(|b| b.id == id)
    }

    /// Edge tapes for `board` that are at least as wide as the board is thick.
    pub fn tapes_for(&self, board: &Board) -> Vec<&EdgeTape> {
        self.edge_tapes
            .iter()
            .filter(|t| board.edge_tapes.contains(&t.id) && t.width >= board.thickness)
            .collect()
    }

    pub fn brands(&self) -> Vec<&str> {
        let mut v: Vec<&str> = self.boards.iter().map(|b| b.brand.as_str()).collect();
        v.sort();
        v.dedup();
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo_catalog() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../catalog")
    }

    #[test]
    fn the_repository_catalog_loads_and_validates() {
        let cat = Catalog::load(&repo_catalog()).unwrap();
        assert!(!cat.boards.is_empty());
        for b in &cat.boards {
            assert!(
                !cat.tapes_for(b).is_empty() || b.edge_tapes.is_empty(),
                "{}",
                b.id
            );
        }
    }
}
