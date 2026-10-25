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
    // SolidCraft 0.22 grounds a design's first part; a user ungrounds it to move it.
    built.push(json!({ "command": "occurrence.ground", "params": {
        "occurrence": "B01:1", "grounded": false } }));
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

#[test]
fn cabinets_built_into_a_design_read_back_as_they_were_specified() {
    let Some(cli) = cli() else {
        eprintln!("SOLIDCRAFT_CLI not set: skipped");
        return;
    };
    use solidcraft_bridge::design::{component_names, spec_from_metadata};
    let mut wall = Cabinet::new("W01", CabinetKind::Wall);
    wall.origin = cabinet_core::geom::Vec3::new(0.0, 250.0, 1450.0);
    let cabs = [Cabinet::new("B01", CabinetKind::Base), wall];
    let mut script = Vec::new();
    for c in &cabs {
        let list = build_cabinet(c).unwrap();
        let spec = serde_json::to_value(c).unwrap();
        script.extend(emit_cabinet(&list, Detail::Design, Some(&spec)));
    }
    // A component that is not a cabinet.
    script.push(
        json!({ "command": "component.create", "params": { "name": "Fridge", "activate": false } }),
    );
    script.push(json!({ "command": "component.list", "params": { "compact": true } }));
    let listing = run(&cli, "readback-list", &script, &["run", "--results"]);
    let names = component_names(listing.as_array().unwrap().last().unwrap());
    assert_eq!(names, ["B01", "W01", "Fridge"]);
    for name in &names {
        script.push(json!({ "command": "component.metadata", "params": { "component": name } }));
    }
    let results = run(&cli, "readback", &script, &["run", "--results"]);
    let results = results.as_array().unwrap();
    let read: Vec<Cabinet> = results[results.len() - names.len()..]
        .iter()
        .filter_map(spec_from_metadata)
        .map(|v| serde_json::from_value(v).unwrap())
        .collect();
    assert_eq!(read, cabs);
}

#[test]
fn an_integrated_toe_kick_notch_is_cut_out_of_both_sides() {
    let Some(cli) = cli() else {
        eprintln!("SOLIDCRAFT_CLI not set: skipped");
        return;
    };
    use cabinet_core::part::Operation;
    let mut cab = Cabinet::new("B01", CabinetKind::Base);
    cab.standard.toe_kick = cabinet_core::model::ToeKick::IntegratedNotch;
    let mut list = build_cabinet(&cab).unwrap();
    // Only the notches (grooves at the front, from the floor), so the volumes are exact.
    list.operations.retain(|o| {
        matches!(o, Operation::Groove { bounds, .. } if bounds.min.z == 0.0 && bounds.min.y == 0.0)
    });
    assert_eq!(list.operations.len(), 2);
    let r = run(
        &cli,
        "notch",
        &emit_cabinet(&list, Detail::Manufacturing, None),
        &["eval"],
    );
    let s = &cab.standard;
    let notch = s.carcass_thickness * s.toe_kick_setback * s.toe_kick_height;
    for side in ["B01.LeftSide", "B01.RightSide"] {
        let part = list.part(side).unwrap();
        let body = r["bodies"]
            .as_array()
            .unwrap()
            .iter()
            .find(|b| b["name"] == side)
            .unwrap();
        let got = body["volume_mm3"].as_f64().unwrap();
        let expected = part.bounds.volume() - notch;
        assert!(
            (got - expected).abs() < 1e-6 * expected,
            "{side}: {got} vs {expected}"
        );
        // The box still reaches the floor at the back: the notch is at the front only.
        assert_eq!(body["bbox"]["min"][2], 0.0, "{side}");
    }
}

#[test]
fn biscuit_slots_remove_the_volume_of_a_plunged_100_mm_cutter() {
    let Some(cli) = cli() else {
        eprintln!("SOLIDCRAFT_CLI not set: skipped");
        return;
    };
    use cabinet_core::part::Operation;
    let cab = Cabinet::new("B01", CabinetKind::Base);
    let mut list = build_cabinet(&cab).unwrap();
    list.operations
        .retain(|o| matches!(o, Operation::BiscuitSlot { .. }));
    let slots = list.operations.len();
    assert!(slots > 4);
    let r = run(
        &cli,
        "biscuits",
        &emit_cabinet(&list, Detail::Exact, None),
        &["eval"],
    );
    let removed: f64 = list
        .parts
        .iter()
        .map(|p| {
            let body = r["bodies"]
                .as_array()
                .unwrap()
                .iter()
                .find(|b| b["name"] == p.name.as_str())
                .unwrap();
            p.bounds.volume() - body["volume_mm3"].as_f64().unwrap()
        })
        .sum();
    // Each slot is a circular segment of the 100 mm cutter in each panel, as thick as a biscuit.
    let b = cab.standard.biscuit;
    let (radius, d) = (50.0_f64, b.slot_depth());
    let segment = radius * radius * ((radius - d) / radius).acos()
        - (radius - d) * (2.0 * radius * d - d * d).sqrt();
    let expected = slots as f64 * 2.0 * segment * b.thickness();
    assert!(
        (removed - expected).abs() < 0.01 * expected,
        "removed {removed:.0} mm³, expected {expected:.0} mm³"
    );
}

#[test]
fn every_slot_holds_a_biscuit_body_of_the_biscuits_size() {
    let Some(cli) = cli() else {
        eprintln!("SOLIDCRAFT_CLI not set: skipped");
        return;
    };
    use cabinet_core::part::Operation;
    let cab = Cabinet::new("B01", CabinetKind::Base);
    let list = build_cabinet(&cab).unwrap();
    let slots = list
        .operations
        .iter()
        .filter(|o| matches!(o, Operation::BiscuitSlot { .. }))
        .count();
    let r = run(
        &cli,
        "biscuit-bodies",
        &emit_cabinet(&list, Detail::Manufacturing, None),
        &["eval"],
    );
    let biscuits: Vec<&Value> = r["bodies"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|b| b["name"].as_str().unwrap().contains(".Biscuit"))
        .collect();
    assert_eq!(biscuits.len(), slots);
    // A lens of two circular arcs: chord = the biscuit's length, bulge = half its width.
    let b = cab.standard.biscuit;
    let (half, h) = (b.length() / 2.0, b.width() / 2.0);
    let r_arc = (half * half + h * h) / (2.0 * h);
    let segment = r_arc * r_arc * (half / r_arc).asin() - (r_arc - h) * half;
    let expected = 2.0 * segment * b.thickness();
    for body in biscuits {
        let v = body["volume_mm3"].as_f64().unwrap();
        assert!((v - expected).abs() < 0.01 * expected, "{v} vs {expected}");
    }
}

#[test]
fn hinges_and_runners_show_as_one_stand_in_body_each_where_the_core_puts_them() {
    let Some(cli) = cli() else {
        eprintln!("SOLIDCRAFT_CLI not set: skipped");
        return;
    };
    use cabinet_core::standins::stand_ins;
    for (id, kind) in [("B01", CabinetKind::Base), ("D01", CabinetKind::DrawerBank)] {
        let list = build_cabinet(&Cabinet::new(id, kind)).unwrap();
        let expected = stand_ins(&list);
        assert!(!expected.is_empty(), "{id}");
        let built = bounds(
            &cli,
            &format!("standins-{id}"),
            &emit_cabinet(&list, Detail::Manufacturing, None),
        );
        let um = |v: f64| (v * 1000.0).round() as i64;
        for s in &expected {
            let b = s.bounds();
            let (lo, hi) = (b.min, b.max());
            let want = vec![um(lo.x), um(lo.y), um(lo.z), um(hi.x), um(hi.y), um(hi.z)];
            let got = &built
                .iter()
                .find(|(n, _)| *n == s.name)
                .unwrap_or_else(|| panic!("{} is not a body", s.name))
                .1;
            // The cup's tessellated round sides may fall a few micrometres short of the box.
            for (g, w) in got.iter().zip(&want) {
                assert!((g - w).abs() <= 5, "{}: {got:?} vs {want:?}", s.name);
            }
        }
        let names = |prefix: &str| built.iter().filter(|(n, _)| n.contains(prefix)).count();
        assert_eq!(names(".Hinge") + names(".Runner"), expected.len(), "{id}");
    }
}

#[test]
fn the_new_cabinets_build_every_part_where_the_core_puts_it() {
    let Some(cli) = cli() else {
        eprintln!("SOLIDCRAFT_CLI not set: skipped");
        return;
    };
    use cabinet_core::model::{DrawerSystem, FrontLayout, HingeSide};
    let corner = Cabinet::new("C01", CabinetKind::CornerBase);
    let mut right = Cabinet::new("C02", CabinetKind::CornerBase);
    right.corner.side = HingeSide::Right;
    let mut lift = Cabinet::new("W01", CabinetKind::Wall);
    lift.fronts = FrontLayout::BiFoldLift { handle_weight: 0.4 };
    let mut wood = Cabinet::new("D01", CabinetKind::DrawerBank);
    wood.standard.drawer_system = DrawerSystem::Tandem;
    let legrabox = Cabinet::new("D02", CabinetKind::DrawerBank);
    let mut tandembox = Cabinet::new("D03", CabinetKind::DrawerBank);
    tandembox.standard.drawer_system = DrawerSystem::Tandembox;
    for cab in [corner, right, lift, wood, legrabox, tandembox] {
        let list = build_cabinet(&cab).unwrap();
        let built = bounds(
            &cli,
            &format!("new-{}", cab.id),
            &emit_cabinet(&list, Detail::Manufacturing, None),
        );
        let um = |v: f64| (v * 1000.0).round() as i64;
        for p in &list.parts {
            let (lo, hi) = (p.bounds.min, p.bounds.max());
            let want = vec![um(lo.x), um(lo.y), um(lo.z), um(hi.x), um(hi.y), um(hi.z)];
            let got = &built
                .iter()
                .find(|(n, _)| *n == p.name)
                .unwrap_or_else(|| panic!("{} is not a body", p.name))
                .1;
            for (g, w) in got.iter().zip(&want) {
                assert!((g - w).abs() <= 5, "{}: {got:?} vs {want:?}", p.name);
            }
        }
    }
}
