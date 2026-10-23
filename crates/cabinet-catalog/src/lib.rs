//! Catalog data: boards (Polytec, Laminex, ...), edge tape and hardware (Blum, Häfele).
//!
//! Data lives in JSON files under `catalog/` at the repository root, one file per brand and
//! product family, so overnight agents can add brands without touching code. Every record carries
//! a [`Source`]: where the fact came from, when, and whether a person checked it.

use std::collections::{BTreeMap, HashSet};
use std::path::Path;

use serde::{Deserialize, Serialize};

pub mod hardware;
pub use hardware::{DrillHole, HardwareFile, HardwareItem, HardwareKind};

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
    /// As the source states it; absent when it does not (Laminex's board tables give a matching
    /// tape's code, thickness and roll, not its material or width).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub material: Option<TapeMaterial>,
    pub thickness: f64,
    /// Width in mm, absent when the source does not print it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<f64>,
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

/// Every `*.json` in `dir`, in name order, parsed as `T`. A missing folder holds nothing.
fn read_dir<T: serde::de::DeserializeOwned>(dir: &Path) -> Result<Vec<T>, Error> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Ok(Vec::new());
    };
    let mut paths: Vec<_> = entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    paths.sort();
    paths
        .into_iter()
        .map(|path| {
            let shown = path.display().to_string();
            let text = std::fs::read_to_string(&path).map_err(|source| Error::Io {
                path: shown.clone(),
                source,
            })?;
            serde_json::from_str(&text).map_err(|source| Error::Json {
                path: shown,
                source,
            })
        })
        .collect()
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
    pub hardware: Vec<HardwareItem>,
}

impl Catalog {
    /// Loads every `*.json` under `root/boards`, `root/edge-tape` and `root/hardware` and checks
    /// the result.
    pub fn load(root: &Path) -> Result<Self, Error> {
        let mut cat = Catalog::default();
        for sub in ["boards", "edge-tape"] {
            for file in read_dir::<BrandFile>(&root.join(sub))? {
                cat.boards.extend(file.boards);
                cat.edge_tapes.extend(file.edge_tapes);
            }
        }
        for file in read_dir::<HardwareFile>(&root.join("hardware"))? {
            cat.hardware.extend(file.items);
        }
        cat.validate()?;
        Ok(cat)
    }

    pub fn hardware_item(&self, id: &str) -> Option<&HardwareItem> {
        self.hardware.iter().find(|h| h.id == id)
    }

    /// Unique ids, positive sizes, edge tape references that resolve, and a source on every
    /// board and tape: the document's URL and date, and when it was read.
    pub fn validate(&self) -> Result<(), Error> {
        let mut problems = Vec::new();
        let sourced = |id: &str, s: &Source, problems: &mut Vec<String>| {
            for (field, v) in [
                ("url", &s.url),
                ("document_date", &s.document_date),
                ("retrieved", &s.retrieved),
            ] {
                if v.trim().is_empty() {
                    problems.push(format!("{id}: source has no {field}"));
                }
            }
        };
        for b in &self.boards {
            sourced(&b.id, &b.source, &mut problems);
        }
        for t in &self.edge_tapes {
            sourced(&t.id, &t.source, &mut problems);
        }
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
            if !(positive(t.thickness) && t.width.is_none_or(positive)) {
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
        let mut hw_ids = HashSet::new();
        for h in &self.hardware {
            if !hw_ids.insert(h.id.as_str()) {
                problems.push(format!("duplicate hardware id {}", h.id));
            }
            problems.extend(h.problems());
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

    /// Edge tapes for `board` known to be at least as wide as the board is thick.
    pub fn tapes_for(&self, board: &Board) -> Vec<&EdgeTape> {
        self.edge_tapes
            .iter()
            .filter(|t| {
                board.edge_tapes.contains(&t.id) && t.width.is_some_and(|w| w >= board.thickness)
            })
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

    fn hinge_json() -> serde_json::Value {
        serde_json::json!({
            "brand": "Example",
            "items": [{
                "id": "example.hinge.110.full",
                "brand": "Example",
                "article": "H-110-F",
                "family": "Example hinge",
                "variant": "110°, full overlay",
                "kind": "hinge",
                "parameters": { "cup_diameter": 35.0, "cup_depth": 13.0 },
                "drilling": [{ "x": 0.0, "y": 21.5, "diameter": 35.0, "depth": 13.0, "note": "cup" }],
                "source": { "url": "https://example.invalid/td.pdf", "document_date": "2026",
                            "retrieved": "2026-10-23", "verified": false }
            }]
        })
    }

    fn temp_catalog(name: &str, hardware: &serde_json::Value) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!("cabinet-catalog-{name}"));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("hardware")).unwrap();
        std::fs::write(root.join("hardware/example.json"), hardware.to_string()).unwrap();
        root
    }

    #[test]
    fn hardware_items_load_with_named_parameters_and_drilling() {
        let cat = Catalog::load(&temp_catalog("ok", &hinge_json())).unwrap();
        let h = cat.hardware_item("example.hinge.110.full").unwrap();
        assert_eq!(h.kind, HardwareKind::Hinge);
        assert_eq!(h.param("cup_diameter"), Ok(35.0));
        assert!(
            h.param("boring_distance")
                .unwrap_err()
                .contains("boring_distance")
        );
        assert_eq!(h.drilling[0].note, "cup");
        assert!(!h.source.verified);
    }

    #[test]
    fn hardware_without_an_article_or_with_a_bad_hole_is_refused() {
        let mut j = hinge_json();
        j["items"][0]["article"] = "".into();
        j["items"][0]["drilling"][0]["diameter"] = 0.0.into();
        let err = Catalog::load(&temp_catalog("bad", &j))
            .unwrap_err()
            .to_string();
        assert!(err.contains("no article number"), "{err}");
        assert!(err.contains("not a real hole"), "{err}");
    }

    #[test]
    fn duplicate_hardware_ids_are_refused() {
        let mut j = hinge_json();
        let item = j["items"][0].clone();
        j["items"].as_array_mut().unwrap().push(item);
        let err = Catalog::load(&temp_catalog("dup", &j))
            .unwrap_err()
            .to_string();
        assert!(err.contains("duplicate hardware id"), "{err}");
    }

    #[test]
    fn a_board_without_a_source_date_is_refused() {
        let dir = std::env::temp_dir().join("cabinet-catalog-unsourced");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("boards")).unwrap();
        let file = serde_json::json!({ "brand": "Test", "boards": [{
            "id": "test.white.16", "brand": "Test", "range": "R", "decor": "White", "finish": "Matt",
            "kind": "melamine", "substrate": "mdf", "thickness": 16.0,
            "sheets": [{ "length": 2400.0, "width": 1200.0 }],
            "source": { "url": "https://example.com/guide.pdf", "document_date": "", "retrieved": "2026-10-24" }
        }] });
        std::fs::write(dir.join("boards/test.json"), file.to_string()).unwrap();
        let err = Catalog::load(&dir).unwrap_err().to_string();
        assert!(
            err.contains("test.white.16: source has no document_date"),
            "{err}"
        );
    }

    #[test]
    fn the_repository_catalog_loads_and_validates() {
        let cat = Catalog::load(&repo_catalog()).unwrap();
        assert!(!cat.boards.is_empty());
        // A board whose tapes all give a width has one wide enough for it. (Laminex's tables give
        // no tape widths.)
        for b in &cat.boards {
            let widths_known = b.edge_tapes.iter().all(|id| {
                cat.edge_tapes
                    .iter()
                    .any(|t| &t.id == id && t.width.is_some())
            });
            assert!(
                !cat.tapes_for(b).is_empty() || b.edge_tapes.is_empty() || !widths_known,
                "{}",
                b.id
            );
        }
    }
}
