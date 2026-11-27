//! Material packs (docs/material-packs.md): a manufacturer's decor and slab pictures, the catalog
//! records that describe them, and how each picture maps, in one zip. The whole pack is checked
//! before anything is written, so a bad or tampered file installs nothing.

use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// The manifest's `format`.
pub const FORMAT: &str = "cabinet-studio-material-pack";
/// The `format_version` this reader understands.
pub const FORMAT_VERSION: u32 = 1;
/// The most one file of a pack may hold, and the whole pack, unpacked: a zip that claims more is
/// refused before it is read.
const MAX_FILE: u64 = 400 << 20;
const MAX_PACK: u64 = 4 << 30;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Manifest {
    pub format: String,
    pub format_version: u32,
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub brand: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub source: String,
    /// Every file but the manifest, with its SHA-256.
    pub files: BTreeMap<String, String>,
}

/// A pack as installed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Installed {
    pub id: String,
    pub name: String,
    pub version: String,
    pub images: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum PackError {
    #[error("not a material pack: {0}")]
    Invalid(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

fn invalid(m: impl Into<String>) -> PackError {
    PackError::Invalid(m.into())
}

fn safe_name(s: &str) -> bool {
    !s.is_empty()
        && !s.starts_with('.')
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
}

/// Where a file of the pack may be: a picture, a textures file or a boards file. None otherwise.
fn kind(path: &str) -> Option<&'static str> {
    let (dir, name) = path.rsplit_once('/')?;
    if !safe_name(name) {
        return None;
    }
    let ext = name.rsplit_once('.')?.1.to_ascii_lowercase();
    match (dir, ext.as_str()) {
        ("images", "jpg" | "jpeg" | "png") => Some("image"),
        ("catalog/textures", "json") => Some("textures"),
        ("catalog/boards", "json") => Some("boards"),
        _ => None,
    }
}

fn sha256(data: &[u8]) -> String {
    Sha256::digest(data)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Reads and checks a pack: its manifest, that it holds exactly the files the manifest lists with
/// their checksums, only in the places a pack may put them, and every picture its textures name.
pub fn read_pack(zip: &Path) -> Result<(Manifest, BTreeMap<String, Vec<u8>>), PackError> {
    let mut archive = zip::ZipArchive::new(std::fs::File::open(zip)?)
        .map_err(|e| invalid(format!("not a zip file ({e})")))?;
    let mut manifest = None;
    let mut files = BTreeMap::new();
    let mut total = 0u64;
    for i in 0..archive.len() {
        let entry = archive
            .by_index(i)
            .map_err(|e| invalid(format!("unreadable entry ({e})")))?;
        if entry.is_dir() {
            continue;
        }
        let name = entry
            .name()
            .map_err(|e| invalid(format!("an entry's name ({e})")))?
            .to_string();
        if entry.size() > MAX_FILE {
            return Err(invalid(format!("{name} is too large")));
        }
        total += entry.size();
        if total > MAX_PACK {
            return Err(invalid("the pack is too large"));
        }
        if name != "pack.json" && kind(&name).is_none() {
            return Err(invalid(format!(
                "{name} is not a place a pack may put a file"
            )));
        }
        let mut data = Vec::with_capacity(entry.size() as usize);
        entry.take(MAX_FILE + 1).read_to_end(&mut data)?;
        if name == "pack.json" {
            manifest = Some(
                serde_json::from_slice::<Manifest>(&data)
                    .map_err(|e| invalid(format!("pack.json: {e}")))?,
            );
        } else {
            files.insert(name, data);
        }
    }
    let m = manifest.ok_or_else(|| invalid("no pack.json"))?;
    if m.format != FORMAT {
        return Err(invalid(format!("format {}", m.format)));
    }
    if m.format_version != FORMAT_VERSION {
        return Err(invalid(format!(
            "format version {} (this Cabinet Studio reads {FORMAT_VERSION})",
            m.format_version
        )));
    }
    if m.id.is_empty()
        || !m
            .id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err(invalid(format!("id {:?}", m.id)));
    }
    if m.version.is_empty() {
        return Err(invalid("no version"));
    }
    for (name, data) in &files {
        match m.files.get(name) {
            None => return Err(invalid(format!("{name} is not in pack.json"))),
            Some(sum) if *sum != sha256(data) => {
                return Err(invalid(format!("{name} does not match its checksum")));
            }
            Some(_) => {}
        }
    }
    if let Some(gone) = m.files.keys().find(|k| !files.contains_key(*k)) {
        return Err(invalid(format!("{gone} is missing")));
    }
    // Each picture a textures file names is in the pack.
    let pictures: std::collections::HashSet<&str> = files
        .keys()
        .filter_map(|k| k.strip_prefix("images/"))
        .filter_map(|n| n.rsplit_once('.').map(|(stem, _)| stem))
        .collect();
    for (name, data) in files.iter().filter(|(k, _)| kind(k) == Some("textures")) {
        let doc: serde_json::Value =
            serde_json::from_slice(data).map_err(|e| invalid(format!("{name}: {e}")))?;
        for (decor, rec) in doc["images"].as_object().into_iter().flatten() {
            for key in ["name", "normal"] {
                if let Some(p) = rec[key].as_str()
                    && !pictures.contains(p)
                {
                    return Err(invalid(format!(
                        "{name}: {decor}'s {key} {p} is not in images/"
                    )));
                }
            }
        }
    }
    for (name, data) in files.iter().filter(|(k, _)| kind(k) == Some("boards")) {
        serde_json::from_slice::<serde_json::Value>(data)
            .map_err(|e| invalid(format!("{name}: {e}")))?;
    }
    Ok((m, files))
}

/// The installed packs, by id, in `app_dir` (`~/Library/Application Support/CabinetStudio`).
pub fn installed(app_dir: &Path) -> BTreeMap<String, Installed> {
    std::fs::read_to_string(app_dir.join("packs").join("installed.json"))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

/// Writes through a temporary file, so a reader never sees half a picture.
fn write(path: &Path, data: &[u8]) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp: PathBuf = path.with_extension("partial");
    std::fs::write(&tmp, data)?;
    std::fs::rename(&tmp, path)
}

/// Checks a pack, then installs it: its pictures into `app_dir/decors`, its catalog files under
/// `app_dir/packs/<id>/catalog`, and its entry in `app_dir/packs/installed.json`.
pub fn install_pack(zip: &Path, app_dir: &Path) -> Result<Installed, PackError> {
    let (m, files) = read_pack(zip)?;
    let decors = app_dir.join("decors");
    let own = app_dir.join("packs").join(&m.id);
    let mut images = 0;
    for (name, data) in &files {
        match name.strip_prefix("images/") {
            Some(file) => {
                write(&decors.join(file), data)?;
                images += 1;
            }
            None => write(&own.join(name), data)?,
        }
    }
    write(
        &own.join("pack.json"),
        &serde_json::to_vec_pretty(&m).unwrap_or_default(),
    )?;
    let done = Installed {
        id: m.id.clone(),
        name: m.name.clone(),
        version: m.version.clone(),
        images,
    };
    let mut all = installed(app_dir);
    all.insert(m.id.clone(), done.clone());
    write(
        &app_dir.join("packs").join("installed.json"),
        &serde_json::to_vec_pretty(&all).unwrap_or_default(),
    )?;
    Ok(done)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn pack(dir: &Path, files: &[(&str, &[u8])], tamper: Option<&str>) -> PathBuf {
        let listed: BTreeMap<String, String> = files
            .iter()
            .map(|(n, d)| (n.to_string(), sha256(d)))
            .collect();
        let manifest = serde_json::json!({
            "format": FORMAT, "format_version": 1, "id": "test-brand", "name": "Test",
            "version": "2026.11.27", "files": listed,
        });
        let path = dir.join("pack.zip");
        let mut z = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
        let opts = zip::write::SimpleFileOptions::default();
        z.start_file("pack.json", opts).unwrap();
        z.write_all(manifest.to_string().as_bytes()).unwrap();
        for (n, d) in files {
            z.start_file(*n, opts).unwrap();
            z.write_all(if tamper == Some(*n) { b"changed" } else { d })
                .unwrap();
        }
        z.finish().unwrap();
        path
    }

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("packs-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    const TEXTURES: &[u8] = br#"{"images": {"Oak": {"name": "test-oak", "covers": [2400, 1200]}}}"#;

    #[test]
    fn a_pack_installs_its_pictures_and_records_itself() {
        let d = temp("ok");
        let zip = pack(
            &d,
            &[
                ("catalog/textures/test-brand.json", TEXTURES),
                ("images/test-oak.jpg", b"jpeg"),
            ],
            None,
        );
        let app = d.join("app");
        let done = install_pack(&zip, &app).unwrap();
        assert_eq!((done.images, done.version.as_str()), (1, "2026.11.27"));
        assert_eq!(
            std::fs::read(app.join("decors/test-oak.jpg")).unwrap(),
            b"jpeg"
        );
        assert!(
            app.join("packs/test-brand/catalog/textures/test-brand.json")
                .exists()
        );
        assert_eq!(installed(&app)["test-brand"].version, "2026.11.27");
    }

    #[test]
    fn a_tampered_or_incomplete_pack_installs_nothing() {
        let d = temp("bad");
        let files: &[(&str, &[u8])] = &[
            ("catalog/textures/test-brand.json", TEXTURES),
            ("images/test-oak.jpg", b"jpeg"),
        ];
        let zip = pack(&d, files, Some("images/test-oak.jpg"));
        let app = d.join("app");
        let e = install_pack(&zip, &app).unwrap_err().to_string();
        assert!(e.contains("checksum"), "{e}");
        assert!(!app.join("decors").exists());
        // A picture the textures name but the pack leaves out.
        let zip = pack(&d, &files[..1], None);
        let e = install_pack(&zip, &app).unwrap_err().to_string();
        assert!(e.contains("test-oak") && e.contains("not in images"), "{e}");
    }

    #[test]
    fn a_pack_may_write_only_where_packs_go() {
        let d = temp("escape");
        for bad in [
            "../escape.jpg",
            "images/../../x.jpg",
            "decors/x.jpg",
            "images/x.exe",
        ] {
            let zip = pack(&d, &[(bad, b"x")], None);
            assert!(install_pack(&zip, &d.join("app")).is_err(), "{bad}");
        }
    }
}
