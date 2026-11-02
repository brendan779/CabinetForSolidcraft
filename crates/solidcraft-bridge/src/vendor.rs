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
    /// The hardware item it is, for what the file does not carry (a finish's colour).
    #[serde(default)]
    pub item: String,
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

fn step_files(dir: &Path, out: &mut Vec<PathBuf>, stls: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if p.is_dir() {
            if name != MESH_DIR && name != "catalogue" && !name.starts_with('.') {
                step_files(&p, out, stls);
            }
        } else {
            match p
                .extension()
                .and_then(|x| x.to_str())
                .map(|x| x.to_ascii_lowercase())
            {
                Some(x) if x == "step" || x == "stp" => out.push(p),
                // A supplier model may already be a mesh (the ABI handle import): it is
                // indexed as it lies, in the frame the importer oriented it to.
                Some(x) if x == "stl" => stls.push(p),
                _ => {}
            }
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
    let mut stls = Vec::new();
    step_files(vendor_dir, &mut steps, &mut stls);
    steps.sort();
    stls.sort();
    let mut models = Vec::new();
    let mut converted = 0;
    // Mesh models need no conversion: index them as they lie.
    for stl in &stls {
        let stem = stl
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_string();
        if let Some(m) = old.iter().find(|m| m.stl == *stl) {
            models.push(m.clone());
            continue;
        }
        let mesh = load_stl(stl)?;
        let (min, max) = mesh.bounds();
        models.push(VendorModel {
            article: article_of(&stem),
            step: stl.clone(),
            stl: stl.clone(),
            min,
            max,
        });
    }
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
    add_mirrors(&mesh_dir, &mut models)?;
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
        let mut files: Vec<&VendorModel> = Vec::new();
        // Blum repeats a model under several ids: one to a shape, or they draw on top of each
        // other. A left and a right have different boxes.
        for m in models.iter().filter(|m| m.article == p.article) {
            let same =
                |a: [f64; 3], b: [f64; 3]| a.iter().zip(b).all(|(x, y)| (x - y).abs() < 0.05);
            if !files
                .iter()
                .any(|f| same(f.min, m.min) && same(f.max, m.max))
            {
                files.push(m);
            }
        }
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
                item: p.item.clone(),
                article: p.article.clone(),
                step: m.step.clone(),
                stl: m.stl.clone(),
                transform: transform(p, m.interior_positive(), BLUM_FRONT_PLANE),
            });
        }
    }
    out
}

/// Blum draws an AVENTOS mechanism or lever arm once, for one side. The other side's is its mirror
/// image, which only a mesh can be: a move can't mirror a STEP file.
fn is_one_handed(article: &str) -> bool {
    ["22K2", "20K2", "22F2", "22F3"]
        .iter()
        .any(|a| article.starts_with(a))
}

/// A mirrored mesh, as the other hand, for each one-handed model that has none yet.
fn add_mirrors(mesh_dir: &Path, models: &mut Vec<VendorModel>) -> Result<()> {
    let sources: Vec<VendorModel> = models
        .iter()
        .filter(|m| is_one_handed(&m.article) && !is_mirror(&m.stl))
        .cloned()
        .collect();
    for m in sources {
        let stem = m
            .stl
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        let stl = mesh_dir.join(format!("{stem}{MIRROR}.stl"));
        if models.iter().any(|o| o.stl == stl) && stl.exists() {
            continue;
        }
        models.retain(|o| o.stl != stl);
        let data = std::fs::read(&m.stl)?;
        std::fs::write(&stl, mirrored_stl(&data))?;
        models.push(VendorModel {
            article: m.article.clone(),
            step: stl.clone(),
            stl,
            min: [-m.max[0], m.min[1], m.min[2]],
            max: [-m.min[0], m.max[1], m.max[2]],
        });
    }
    Ok(())
}

const MIRROR: &str = "__mirror";

fn is_mirror(stl: &Path) -> bool {
    stl.file_stem()
        .and_then(|s| s.to_str())
        .is_some_and(|s| s.ends_with(MIRROR))
}

/// A binary STL reflected in the X = 0 plane, its triangles rewound so they still face out.
fn mirrored_stl(data: &[u8]) -> Vec<u8> {
    let mut out = data.to_vec();
    let flip = |b: &mut [u8]| {
        let v = -f32::from_le_bytes([b[0], b[1], b[2], b[3]]);
        b[..4].copy_from_slice(&v.to_le_bytes());
    };
    let mut at = 84;
    while at + 50 <= out.len() {
        flip(&mut out[at..at + 4]);
        for corner in 1..4 {
            let p = at + corner * 12;
            flip(&mut out[p..p + 4]);
        }
        // Swap the second and third corners.
        let (a, b) = (at + 24, at + 36);
        for i in 0..12 {
            out.swap(a + i, b + i);
        }
        at += 50;
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
    fn a_model_repeated_under_several_ids_is_drawn_once() {
        let list = build_cabinet(&Cabinet::new("B01", CabinetKind::Base)).unwrap();
        let place = cad_placements(&list);
        let hinge = place.iter().find(|p| p.name.contains("Hinge")).unwrap();
        let (mut a, mut b) = (
            model(&hinge.article, 0.0, 50.0),
            model(&hinge.article, 0.0, 50.0),
        );
        a.step = "a.step".into();
        b.step = "b.step".into();
        let found = resolve(std::slice::from_ref(hinge), &[a, b]);
        assert_eq!(found.len(), 1);
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

    /// ABI-1: a supplier model that is already a mesh (the ABI handle import) is indexed
    /// as it lies, named by its article, without SolidCraft's converter.
    #[test]
    fn a_native_stl_model_is_indexed_as_it_lies() {
        let dir = std::env::temp_dir().join(format!("abi-stl-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("ABI-TEST-96.stl");
        let mut b = vec![0u8; 80];
        b.extend(1u32.to_le_bytes());
        for v in [
            0.0f32, 0.0, 1.0, -48.0, 0.0, 0.0, 48.0, 0.0, 0.0, 0.0, -20.0, 0.0,
        ] {
            b.extend(v.to_le_bytes());
        }
        b.extend([0u8, 0]);
        std::fs::write(&path, b).unwrap();
        let (models, converted) = prepare(&dir, Path::new("/nonexistent/solidcraft-cli")).unwrap();
        assert_eq!(converted, 0, "a mesh needs no conversion");
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].article, "ABI-TEST-96");
        assert_eq!(models[0].step, path.canonicalize().unwrap());
        assert_eq!(models[0].min, [-48.0, -20.0, 0.0]);
        assert_eq!(models[0].max, [48.0, 0.0, 0.0]);
        // And it resolves like any other model.
        let placement = CadPlacement {
            name: "B01.Handle1".into(),
            item: "handle.abi.test-96".into(),
            article: "ABI-TEST-96".into(),
            side: Side::Left,
            origin: Vec3::default(),
            frame: Frame::Oriented {
                x: Vec3::new(1.0, 0.0, 0.0),
                y: Vec3::new(0.0, 1.0, 0.0),
            },
        };
        let found = resolve(&[placement], &models);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].stl, path.canonicalize().unwrap());
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn a_mirrored_stl_reflects_in_x_and_keeps_its_faces_out() {
        let mut stl = vec![0u8; 84];
        stl[80] = 1;
        let tri: [f32; 12] = [0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 2.0, 0.0, 0.0, 1.0, 1.0, 0.0];
        for v in tri {
            stl.extend(v.to_le_bytes());
        }
        stl.extend([0, 0]);
        let out = mirrored_stl(&stl);
        let f = |i: usize| f32::from_le_bytes(out[84 + i * 4..84 + i * 4 + 4].try_into().unwrap());
        assert_eq!((f(3), f(4)), (-1.0, 0.0), "first corner");
        assert_eq!((f(6), f(7)), (-1.0, 1.0), "the third corner comes second");
        assert_eq!((f(9), f(10)), (-2.0, 0.0));
        assert!(is_one_handed("22K2510") && !is_one_handed("175H3100"));
    }

    /// Each side gets the hand that reaches into the cabinet from it: the mirror on the left.
    #[test]
    fn a_one_handed_model_and_its_mirror_each_go_on_their_own_side() {
        use cabinet_core::cad::Side;
        let model = |stl: &str, min: f64, max: f64| VendorModel {
            article: "22F2510".into(),
            step: PathBuf::from(stl),
            stl: PathBuf::from(stl),
            min: [min, -59.0, -220.0],
            max: [max, 43.3, 36.8],
        };
        let models = [
            model("a.stl", -29.5, 4.5),
            model("a__mirror.stl", -4.5, 29.5),
        ];
        let place = |side, x: f64| CadPlacement {
            name: "W01.LiftMechanism1".into(),
            item: "blum.aventos.hf-top.mechanism.22f2510".into(),
            article: "22F2510".into(),
            side,
            origin: cabinet_core::geom::Vec3::new(x, 0.0, 500.0),
            frame: Frame::Blum,
        };
        for (side, x, file) in [
            (Side::Left, 16.0, "a__mirror.stl"),
            (Side::Right, 584.0, "a.stl"),
        ] {
            let found = resolve(&[place(side, x)], &models);
            assert_eq!(found.len(), 1);
            assert_eq!(found[0].stl, PathBuf::from(file));
            // Its X runs one way, so the far end of the model lands inside the cabinet.
            let inside = found[0].transform[0] * if side == Side::Left { 29.5 } else { -29.5 }
                + found[0].transform[3];
            assert!(inside > 16.0 && inside < 584.0, "{inside}");
        }
    }
}
