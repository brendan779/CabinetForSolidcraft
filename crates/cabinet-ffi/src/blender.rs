//! What Blender needs to make a job's scene: a script for `solidcraft-cli` that builds the job, and
//! a manifest that says what each body it makes is, by the body's name.

use super::*;
use cabinet_core::part::{PartList, PartRole};
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashMap};

fn axis(a: cabinet_core::geom::Axis) -> String {
    format!("{a:?}").to_lowercase()
}

/// One part's board: a shaker panel with none of its own shows its frame's.
fn board_of<'a>(list: &'a PartList, p: &'a cabinet_core::Part) -> &'a str {
    if p.board.is_empty() && p.role == PartRole::FramePanel {
        let base = cabinet_core::part::front_base(&p.name);
        return list
            .by_role(PartRole::FrameStile)
            .find(|s| cabinet_core::part::front_base(&s.name) == base)
            .map_or("", |s| s.board.as_str());
    }
    &p.board
}

/// The manifest: every part with its board and grain, what each board looks like, and the room.
fn manifest(
    lists: &[(PartList, Option<Cabinet>)],
    walls: Option<&Walls>,
    catalog_dir: &Path,
    colours: &HashMap<String, String>,
) -> Result<Value, CoreError> {
    let cat = cabinet_catalog::Catalog::load(catalog_dir).map_err(invalid)?;
    let read = |file: &str| -> Option<Value> {
        let text = std::fs::read_to_string(catalog_dir.join("textures").join(file)).ok()?;
        serde_json::from_str(&text).ok()
    };
    let finishes = read("finishes.json").unwrap_or(Value::Null);
    let mut parts = Vec::new();
    let mut boards = BTreeMap::new();
    let mut cabinets = Vec::new();
    for (list, cab) in lists {
        for p in &list.parts {
            let board = board_of(list, p);
            let (lo, hi) = (p.bounds.min, p.bounds.max());
            parts.push(json!({
                "name": p.name,
                "cabinet": list.cabinet,
                "role": p.role.label(),
                "front": p.role.is_front(),
                "board": board,
                "grain": p.grain_axis.map(axis),
                "thickness": axis(p.thickness_axis),
                "min": [lo.x, lo.y, lo.z],
                "max": [hi.x, hi.y, hi.z],
            }));
            if !board.is_empty() && !boards.contains_key(board) {
                let found = cat.board(board);
                let decor = found.map_or("", |b| b.decor.as_str());
                let finish = found.map_or("", |b| b.finish.as_str());
                // Each brand's images, by decor, from `scripts/fetch_board_images.py`.
                let brand =
                    found.map_or(String::new(), |b| b.brand.to_lowercase().replace(' ', "-"));
                let images = read(&format!("{brand}.json")).unwrap_or(Value::Null);
                let image = &images["images"][decor];
                // A picture is of a sheet at the longest size its decor comes in: a lining strip,
                // or a shorter sheet of the same decor, is a piece of that.
                let sheet = found.and_then(|f| {
                    cat.boards
                        .iter()
                        .filter(|b| b.brand == f.brand && b.decor == f.decor)
                        .flat_map(|b| &b.sheets)
                        .max_by(|x, y| x.length.total_cmp(&y.length))
                });
                boards.insert(
                    board.to_string(),
                    json!({
                        "decor": decor,
                        "finish": finish,
                        "grained": found.is_some_and(|b| b.grained),
                        "colour": colours.get(board),
                        "look": finishes["finishes"].get(finish).unwrap_or(&finishes["default"]),
                        "image": image["name"],
                        "normal": image["normal"],
                        "url": image["url"],
                        // What the image covers: a whole sheet, unless it says less.
                        "sheet": image.get("covers").cloned().unwrap_or_else(|| {
                            let s = sheet.map_or([2400.0, 1200.0], |s| [s.length, s.width]);
                            json!(s)
                        }),
                    }),
                );
            }
        }
        // Without the toe kick: the run's one board belongs to its first cabinet, and spans the rest.
        let bounds = list
            .parts
            .iter()
            .filter(|p| p.role != PartRole::Plinth)
            .map(|p| p.bounds)
            .reduce(|a, b| {
                let (al, ah, bl, bh) = (a.min, a.max(), b.min, b.max());
                let lo =
                    cabinet_core::geom::Vec3::new(al.x.min(bl.x), al.y.min(bl.y), al.z.min(bl.z));
                let hi =
                    cabinet_core::geom::Vec3::new(ah.x.max(bh.x), ah.y.max(bh.y), ah.z.max(bh.z));
                cabinet_core::geom::Aabb::new(
                    lo,
                    cabinet_core::geom::Vec3::new(hi.x - lo.x, hi.y - lo.y, hi.z - lo.z),
                )
            });
        if let (Some(b), Some(c)) = (bounds, cab) {
            let (lo, hi) = (b.min, b.max());
            cabinets.push(json!({
                "id": list.cabinet,
                "kind": serde_json::to_value(c.kind).map_err(invalid)?,
                "min": [lo.x, lo.y, lo.z],
                "max": [hi.x, hi.y, hi.z],
            }));
        }
    }
    Ok(json!({
        "parts": parts,
        "boards": boards,
        "cabinets": cabinets,
        "walls": walls,
        "default_look": finishes["default"],
    }))
}

/// Writes `build.json` for `solidcraft-cli run` and `job.json` for `blender/build_scene.py`.
#[uniffi::export]
pub fn write_blender_job(
    job: JobItems,
    catalog_dir: String,
    vendor_dir: String,
    colours: HashMap<String, String>,
    folder: String,
) -> Result<(), CoreError> {
    let folder = Path::new(&folder);
    std::fs::create_dir_all(folder).map_err(invalid)?;
    let walls = job.items_json.iter().find_map(|j| match parse_item(j) {
        Ok(RunItem::Walls(w)) => Some(w),
        _ => None,
    });
    let lists = job_lists(&job)?;
    let index = vendor::load_index(Path::new(&vendor_dir));
    let mut commands = Vec::new();
    for (list, _) in &lists {
        // Only what shows: the render has no use for bores or biscuit slots.
        let models = index
            .as_ref()
            .map(|i| models_for(Detail::Design, &cad_placements(list), i))
            .unwrap_or_default();
        let real: Vec<String> = models.iter().map(|m| m.name.clone()).collect();
        commands.extend(solidcraft_bridge::emit::emit_cabinet_with(
            list,
            Detail::Design,
            None,
            &real,
        ));
        commands.extend(hardware_inserts(&list.cabinet, &models));
    }
    let write = |name: &str, v: &Value| {
        std::fs::write(
            folder.join(name),
            serde_json::to_string_pretty(v).map_err(invalid)?,
        )
        .map_err(invalid)
    };
    write("build.json", &json!({ "commands": commands }))?;
    write(
        "job.json",
        &manifest(&lists, walls.as_ref(), Path::new(&catalog_dir), &colours)?,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catalog() -> String {
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../catalog").to_string()
    }

    fn job(front_board: &str) -> JobItems {
        let mut base = Cabinet::new("B01", cabinet_core::CabinetKind::Base);
        base.standard.front_board = front_board.into();
        JobItems {
            items_json: vec![serde_json::to_string(&base).unwrap()],
            placed: Vec::new(),
        }
    }

    fn written(job: JobItems) -> (Value, Value) {
        let dir =
            std::env::temp_dir().join(format!("blender-job-{:?}", std::thread::current().id()));
        write_blender_job(
            job,
            catalog(),
            String::new(),
            HashMap::new(),
            dir.to_string_lossy().into(),
        )
        .unwrap();
        let read =
            |f: &str| serde_json::from_str(&std::fs::read_to_string(dir.join(f)).unwrap()).unwrap();
        let got = (read("job.json"), read("build.json"));
        std::fs::remove_dir_all(dir).unwrap();
        got
    }

    #[test]
    fn every_part_in_the_manifest_is_a_built_body_with_its_board_and_grain() {
        let board = "polytec.commercial.angora-oak-woodmatt.18.mr-mdf";
        let (m, build) = written(job(board));
        let made: Vec<&str> = build["commands"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|c| c["params"]["body_name"].as_str())
            .collect();
        for part in m["parts"].as_array().unwrap() {
            let name = part["name"].as_str().unwrap();
            assert!(made.contains(&name), "{name} is not built");
        }
        let door = m["parts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["name"] == "B01.Door1")
            .unwrap();
        assert_eq!(door["board"], board);
        assert_eq!(door["grain"], "z");
        assert_eq!(m["boards"][board]["decor"], "Angora Oak");
    }

    #[test]
    fn a_cabinet_keeps_its_own_width_though_it_holds_the_runs_toe_kick() {
        let mut items = Vec::new();
        for id in ["B01", "B02"] {
            let mut c = Cabinet::new(id, cabinet_core::CabinetKind::Base);
            c.standard.toe_kick = cabinet_core::model::ToeKick::LegsAndPlinth;
            items.push(serde_json::to_string(&c).unwrap());
        }
        let (m, _) = written(JobItems {
            items_json: items,
            placed: Vec::new(),
        });
        assert_eq!(m["cabinets"][0]["max"][0], 600.0);
        assert_eq!(m["cabinets"][1]["min"][0], 600.0);
    }

    #[test]
    fn a_board_takes_its_finishes_look_and_an_unknown_finish_the_default() {
        let (m, _) = written(job("polytec.commercial.angora-oak-woodmatt.18.mr-mdf"));
        let board = &m["boards"]["polytec.commercial.angora-oak-woodmatt.18.mr-mdf"];
        assert_eq!(board["look"]["bump"], 0.45);
        assert_eq!(board["sheet"][0], 3600.0);
    }

    #[test]
    fn a_board_takes_its_brands_image_and_says_what_it_covers() {
        let (m, _) = written(job("polytec.commercial.angora-oak-woodmatt.18.mr-mdf"));
        let board = &m["boards"]["polytec.commercial.angora-oak-woodmatt.18.mr-mdf"];
        assert_eq!(board["image"], "angora-oak");
        assert!(board["url"].as_str().unwrap().ends_with("/angora-oak.jpg"));
    }

    #[test]
    fn a_lining_strip_takes_the_scale_of_its_decors_full_sheet() {
        let (m, _) = written(job("polytec.commercial.angora-oak-woodmatt.18.mr-mdf"));
        let cat = cabinet_catalog::Catalog::load(Path::new(&catalog())).unwrap();
        let longest = cat
            .boards
            .iter()
            .filter(|b| b.brand == "Polytec" && b.decor == "Angora Oak")
            .flat_map(|b| &b.sheets)
            .map(|s| s.length)
            .fold(0.0, f64::max);
        let board = &m["boards"]["polytec.commercial.angora-oak-woodmatt.18.mr-mdf"];
        assert_eq!(board["sheet"][0].as_f64().unwrap(), longest);
    }

    #[test]
    fn a_shaker_panel_with_no_board_of_its_own_takes_its_frames() {
        let mut base = Cabinet::new("B01", cabinet_core::CabinetKind::Base);
        base.standard.door_style = cabinet_core::model::DoorStyle::Shaker;
        base.standard.front_board = "polytec.commercial.angora-oak-woodmatt.18.mr-mdf".into();
        let (m, _) = written(JobItems {
            items_json: vec![serde_json::to_string(&base).unwrap()],
            placed: Vec::new(),
        });
        let panel = m["parts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["role"] == "Frame panel")
            .unwrap();
        assert_eq!(
            panel["board"],
            "polytec.commercial.angora-oak-woodmatt.18.mr-mdf"
        );
    }
}
