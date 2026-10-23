//! Checks against a real SolidCraft engine through its CLI. Skipped unless `SOLIDCRAFT_CLI` names
//! a `solidcraft-cli` binary:
//!
//! `SOLIDCRAFT_CLI=/Applications/SolidCraft.app/Contents/MacOS/solidcraft-cli cargo test -p solidcraft-bridge --test headless`

use std::path::{Path, PathBuf};
use std::process::Command;

use cabinet_core::{Cabinet, CabinetKind, build_cabinet};
use serde_json::{Value, json};
use solidcraft_bridge::{Detail, emit_cabinet, find_existing, replace_cabinet};

fn cli() -> Option<PathBuf> {
    std::env::var_os("SOLIDCRAFT_CLI").map(PathBuf::from)
}

fn run(cli: &Path, name: &str, commands: &[Value], args: &[&str]) -> Value {
    let dir = std::env::temp_dir().join("cabinet-headless");
    std::fs::create_dir_all(&dir).unwrap();
    let script = dir.join(format!("{name}.json"));
    let out = dir.join(format!("{name}.out.json"));
    std::fs::write(&script, json!({ "commands": commands }).to_string()).unwrap();
    let (verb, flags) = args.split_first().unwrap();
    let status = Command::new(cli)
        .arg(verb)
        .arg(&script)
        .args(flags)
        .arg("--json-out")
        .arg(&out)
        .status()
        .unwrap();
    assert!(status.success(), "{name}: solidcraft-cli failed");
    serde_json::from_slice(&std::fs::read(&out).unwrap()).unwrap()
}

/// Body name → bounding box (min then max) to 1/1000 mm, from `solidcraft-cli eval`.
fn bounds(cli: &Path, name: &str, commands: &[Value]) -> Vec<(String, Vec<i64>)> {
    let r = run(cli, name, commands, &["eval"]);
    let um = |v: &Value| {
        v.as_array()
            .unwrap()
            .iter()
            .map(|x| (x.as_f64().unwrap() * 1000.0).round() as i64)
            .collect::<Vec<_>>()
    };
    let mut v: Vec<(String, Vec<i64>)> = r["bodies"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| {
            let mut corners = um(&b["bbox"]["min"]);
            corners.extend(um(&b["bbox"]["max"]));
            (b["name"].as_str().unwrap().to_string(), corners)
        })
        .collect();
    v.sort_by(|a, b| a.0.cmp(&b.0));
    v
}

#[test]
fn resending_a_moved_cabinet_keeps_it_where_the_user_moved_it() {
    let Some(cli) = cli() else {
        eprintln!("SOLIDCRAFT_CLI not set: skipped");
        return;
    };
    let cab = Cabinet::new("B01", CabinetKind::Base);
    let list = build_cabinet(&cab).unwrap();
    // The user turns the cabinet a quarter and moves it along the wall.
    let mut built = emit_cabinet(&list, Detail::Design, None);
    built.push(json!({ "command": "occurrence.move", "params": {
        "occurrence": "B01:1", "space": "world", "axis": [0, 0, 1], "angle": "90 deg",
        "origin": [0, 0, 0] } }));
    built.push(json!({ "command": "occurrence.move", "params": {
        "occurrence": "B01:1", "space": "world", "translate": [1200, 40, 0] } }));
    let before = bounds(&cli, "moved", &built);

    let mut listed = built.clone();
    listed.push(json!({ "command": "component.list", "params": { "compact": true } }));
    let results = run(&cli, "listed", &listed, &["run", "--results"]);
    let listing = results.as_array().unwrap().last().unwrap();
    let existing = find_existing(listing, "B01").expect("the cabinet is in the design");
    assert!(existing.moved.is_some());

    let mut resent = built.clone();
    resent.extend(replace_cabinet(
        &list,
        Detail::Design,
        None,
        Some(&existing),
    ));
    let after = bounds(&cli, "resent", &resent);
    assert_eq!(before.len(), list.parts.len());
    assert_eq!(after, before);
}

#[test]
fn three_drawn_boxes_become_three_cabinets_in_place() {
    let Some(cli) = cli() else {
        eprintln!("SOLIDCRAFT_CLI not set: skipped");
        return;
    };
    use cabinet_core::part::PartRole;
    use solidcraft_bridge::boxes::cuboid;
    // A base and a tall cabinet on the floor and a wall cabinet above the base.
    let drawn = [
        ("Box1", [0.0, 0.0, 0.0], [600.0, 580.0, 870.0]),
        ("Box2", [600.0, 0.0, 0.0], [600.0, 580.0, 2100.0]),
        ("Box3", [0.0, 250.0, 1450.0], [600.0, 330.0, 720.0]),
    ];
    let mut script: Vec<Value> = drawn
        .iter()
        .map(|(name, min, size)| {
            json!({ "command": "solid.box", "params": {
                "length": size[0], "width": size[1], "height": size[2], "corner": min,
                "operation": "new", "body_name": name, "name": name } })
        })
        .collect();
    for (name, ..) in &drawn {
        script.push(json!({ "command": "model.faces", "params": { "body": name } }));
    }
    let results = run(&cli, "boxes", &script, &["run", "--results"]);
    let results = results.as_array().unwrap();
    let mut built = Vec::new();
    let mut kinds = Vec::new();
    for (i, (_, min, size)) in drawn.iter().enumerate() {
        let b = cuboid(&results[drawn.len() + i]).unwrap();
        assert_eq!(b.min.to_array(), *min);
        assert_eq!(b.size.to_array(), *size);
        let cab = Cabinet::from_box(format!("C{i:02}"), b);
        kinds.push(cab.kind);
        let list = build_cabinet(&cab).unwrap();
        built.extend(emit_cabinet(&list, Detail::Design, None));
        // Every carcass part lies inside the drawn box; fronts hang in front of it.
        for p in &list.parts {
            if matches!(p.role, PartRole::Door | PartRole::DrawerFront) {
                continue;
            }
            for k in 0..3 {
                let (lo, hi) = (p.bounds.min.to_array()[k], p.bounds.max().to_array()[k]);
                assert!(
                    lo >= min[k] - 1e-6 && hi <= min[k] + size[k] + 1e-6,
                    "{}",
                    p.name
                );
            }
        }
    }
    assert_eq!(
        kinds,
        [CabinetKind::Base, CabinetKind::Tall, CabinetKind::Wall]
    );
    let after = bounds(&cli, "cabinets", &built);
    assert!(after.len() > 20);
}
