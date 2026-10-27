//! Supplier CAD on this machine (BLUM-9): the STEP files in the supplier CAD folder, turned into
//! STL meshes once with SolidCraft's command-line tool, and matched to [`cad_placements`] by
//! article number.
//!
//! Files are named by article (`71B3550.step`, `750.5501S_L.step`,
//! `770K5502S__42112505.step`). Which hand a file is for is read from its geometry: Blum draws
//! every part with the cabinet side's inner face at X = 0, so a part whose bulk lies at +X is for
//! a left side. An article with files on both sides is two-handed; otherwise its files are drawn
//! on either side, turned (hinges and plates).
//!
//! [`cad_placements`]: cabinet_core::cad::cad_placements

use std::path::{Path, PathBuf};
use std::process::Command;

use cabinet_core::cad::{BLUM_FRONT_PLANE, CadPlacement, Frame, transform};
use serde::{Deserialize, Serialize};

use crate::{Error, Result};

/// The folder (inside the supplier CAD folder) holding the converted meshes and their index.
pub const MESH_DIR: &str = "mesh";
const INDEX: &str = "index.json";

/// One converted model.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VendorModel {
    pub article: String,
    pub step: PathBuf,
    pub stl: PathBuf,
    /// Bounding box in the model's own frame, mm.
    pub min: [f64; 3],
    pub max: [f64; 3],
}

impl VendorModel {
    /// Whether the part lies on the +X side of Blum's frame (a left-hand part).
    pub fn interior_positive(&self) -> bool {
        self.min[0] + self.max[0] > 0.0
    }
}

/// A model to draw: which mesh and where.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VendorInstance {
    pub name: String,
    pub article: String,
    pub step: PathBuf,
    pub stl: PathBuf,
    /// Model mm → cabinet frame mm, row-major 3 × 4.
    pub transform: [f64; 12],
}

/// The article a file is for: its name up to `__`, `_L`/`_R`, or a Blum download's `_<id>`.
pub fn article_of(stem: &str) -> String {
    if let Some((a, _)) = stem.split_once("__") {
        return a.to_string();
    }
    for hand in ["_L", "_R"] {
        if let Some(a) = stem.strip_suffix(hand) {
            return a.to_string();
        }
    }
    match stem.rsplit_once('_') {
        Some((a, id)) if !id.is_empty() && id.chars().all(|c| c.is_ascii_digit()) => a.to_string(),
        _ => stem.to_string(),
    }
}

fn step_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if p.is_dir() {
            if name != MESH_DIR && name != "catalogue" && !name.starts_with('.') {
                step_files(&p, out);
            }
        } else if p
            .extension()
            .and_then(|x| x.to_str())
            .is_some_and(|x| x.eq_ignore_ascii_case("step") || x.eq_ignore_ascii_case("stp"))
        {
            out.push(p);
        }
    }
}

/// Converts every STEP file under `vendor_dir` that has no up-to-date mesh, with `cli`
/// (`solidcraft-cli`), and writes the index. Returns the index and how many files it converted.
pub fn prepare(vendor_dir: &Path, cli: &Path) -> Result<(Vec<VendorModel>, usize)> {
    // The index matches files by path and the app opens the meshes from it, so keep it absolute
    // whichever way the folder was named.
    let vendor_dir = &std::fs::canonicalize(vendor_dir)?;
    let mesh_dir = vendor_dir.join(MESH_DIR);
    std::fs::create_dir_all(&mesh_dir)?;
    let old: Vec<VendorModel> = load_index(vendor_dir).unwrap_or_default();
    let mut steps = Vec::new();
    step_files(vendor_dir, &mut steps);
    steps.sort();
    let mut models = Vec::new();
    let mut converted = 0;
    for step in steps {
        let stem = step
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_string();
        let stl = mesh_dir.join(format!("{stem}.stl"));
        let fresh = |p: &Path| {
            let (Ok(a), Ok(b)) = (std::fs::metadata(p), std::fs::metadata(&step)) else {
                return false;
            };
            matches!((a.modified(), b.modified()), (Ok(a), Ok(b)) if a >= b)
        };
        if let Some(m) = old.iter().find(|m| m.step == step).filter(|_| fresh(&stl)) {
            models.push(m.clone());
            continue;
        }
        let out = Command::new(cli)
            .args(["run", "--in"])
            .arg(&step)
            .arg("--out")
            .arg(&stl)
            .arg("--quiet")
            .output()?;
        if !out.status.success() || !stl.exists() {
            return Err(Error::Remote(format!(
                "converting {}: {}",
                step.display(),
                String::from_utf8_lossy(&out.stderr).trim()
            )));
        }
        let mesh = load_stl(&stl)?;
        let (min, max) = mesh.bounds();
        models.push(VendorModel {
            article: article_of(&stem),
            step: step.clone(),
            stl,
            min,
            max,
        });
        converted += 1;
    }
    std::fs::write(mesh_dir.join(INDEX), serde_json::to_string_pretty(&models)?)?;
    Ok((models, converted))
}

/// The index [`prepare`] wrote, or none yet.
pub fn load_index(vendor_dir: &Path) -> Option<Vec<VendorModel>> {
    let text = std::fs::read_to_string(vendor_dir.join(MESH_DIR).join(INDEX)).ok()?;
    serde_json::from_str(&text).ok()
}

/// The models to draw for `placements`: for a two-handed article the files of the placement's
/// hand, otherwise all its files. Placements without a model are left out.
pub fn resolve(placements: &[CadPlacement], models: &[VendorModel]) -> Vec<VendorInstance> {
    let mut out = Vec::new();
    for p in placements {
        let mut files: Vec<&VendorModel> =
            models.iter().filter(|m| m.article == p.article).collect();
        // An upright part is one model; the same file comes in several downloaded sets.
        if p.frame != Frame::Blum {
            files.truncate(1);
        }
        let two_handed = files.iter().any(|m| m.interior_positive())
            && files.iter().any(|m| !m.interior_positive());
        let want = p.side.interior_sign() > 0.0;
        for m in files {
            if two_handed && m.interior_positive() != want {
                continue;
            }
            out.push(VendorInstance {
                name: p.name.clone(),
                article: p.article.clone(),
                step: m.step.clone(),
                stl: m.stl.clone(),
                transform: transform(p, m.interior_positive(), BLUM_FRONT_PLANE),
            });
        }
    }
    out
}

/// A triangle mesh: three positions (and one face normal) per triangle, mm.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Mesh {
    pub positions: Vec<f32>,
    pub normals: Vec<f32>,
}

impl Mesh {
    pub fn bounds(&self) -> ([f64; 3], [f64; 3]) {
        let mut min = [f64::MAX; 3];
        let mut max = [f64::MIN; 3];
        for v in self.positions.as_chunks::<3>().0 {
            for i in 0..3 {
                min[i] = min[i].min(v[i] as f64);
                max[i] = max[i].max(v[i] as f64);
            }
        }
        (min, max)
    }
}

/// Reads a binary STL (what `solidcraft-cli` writes).
pub fn load_stl(path: &Path) -> Result<Mesh> {
    let bytes = std::fs::read(path)?;
    let bad = || Error::Remote(format!("{} is not a binary STL", path.display()));
    if bytes.len() < 84 {
        return Err(bad());
    }
    let n = u32::from_le_bytes(bytes[80..84].try_into().map_err(|_| bad())?) as usize;
    if bytes.len() < 84 + n * 50 {
        return Err(bad());
    }
    let f = |o: usize| f32::from_le_bytes([bytes[o], bytes[o + 1], bytes[o + 2], bytes[o + 3]]);
    let mut mesh = Mesh {
        positions: Vec::with_capacity(n * 9),
        normals: Vec::with_capacity(n * 9),
    };
    for t in 0..n {
        let o = 84 + t * 50;
        let normal = [f(o), f(o + 4), f(o + 8)];
        for v in 0..3 {
            let p = o + 12 + v * 12;
            mesh.positions.extend([f(p), f(p + 4), f(p + 8)]);
            mesh.normals.extend(normal);
        }
    }
    Ok(mesh)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cabinet_core::cad::{Side, apply, cad_placements};
    use cabinet_core::geom::Vec3;
    use cabinet_core::{Cabinet, CabinetKind, build_cabinet};

    #[test]
    fn articles_come_from_file_names_in_every_shape_blum_uses() {
        assert_eq!(article_of("71B3550"), "71B3550");
        assert_eq!(article_of("750.5501S_L"), "750.5501S");
        assert_eq!(article_of("770K5502S__42112505"), "770K5502S");
        assert_eq!(article_of("79T8500_46584366"), "79T8500");
        assert_eq!(article_of("70T3550.TL__47173013"), "70T3550.TL");
    }

    fn model(article: &str, x0: f64, x1: f64) -> VendorModel {
        VendorModel {
            article: article.into(),
            step: format!("{article}{x0}.step").into(),
            stl: format!("{article}{x0}.stl").into(),
            min: [x0, -10.0, -500.0],
            max: [x1, 10.0, 35.0],
        }
    }

    #[test]
    fn two_handed_runners_use_their_own_hand_and_hinges_turn_for_the_left_side() {
        let mut cab = Cabinet::new("D01", CabinetKind::DrawerBank);
        cab.standard.drawer_system = cabinet_core::model::DrawerSystem::Movento;
        let list = build_cabinet(&cab).unwrap();
        let models = [
            model("760H5500S", 0.0, 65.0),
            model("760H5500S", -65.0, 0.0),
        ];
        let found = resolve(&cad_placements(&list), &models);
        assert_eq!(found.len(), 6);
        let place = cad_placements(&list);
        for (inst, p) in found
            .iter()
            .zip(place.iter().filter(|p| p.name.contains("Runner")))
        {
            // The runner's bulk ends up inside the cabinet, beside its side.
            let mid = apply(
                &inst.transform,
                Vec3::new(if p.side == Side::Left { 30.0 } else { -30.0 }, 0.0, 0.0),
            );
            assert_eq!(
                (mid.x - p.origin.x).signum(),
                p.side.interior_sign(),
                "{}",
                inst.name
            );
        }
        let base = build_cabinet(&Cabinet::new("B01", CabinetKind::Base)).unwrap();
        let hinges = resolve(&cad_placements(&base), &[model("71B3550", -26.0, 10.0)]);
        assert_eq!(hinges.len(), 2, "one model, drawn at both hinges");
    }

    #[test]
    fn a_tiny_stl_round_trips() {
        let dir = std::env::temp_dir().join(format!("stl-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("t.stl");
        let mut b = vec![0u8; 80];
        b.extend(1u32.to_le_bytes());
        for v in [
            0.0f32, 0.0, 1.0, 0.0, 0.0, 0.0, 10.0, 0.0, 0.0, 0.0, 10.0, 0.0,
        ] {
            b.extend(v.to_le_bytes());
        }
        b.extend([0u8, 0]);
        std::fs::write(&path, b).unwrap();
        let m = load_stl(&path).unwrap();
        assert_eq!(m.positions.len(), 9);
        assert_eq!(m.bounds(), ([0.0, 0.0, 0.0], [10.0, 10.0, 0.0]));
        std::fs::remove_dir_all(dir).ok();
    }
}
