//! Checks against a real SolidCraft engine through its CLI. Skipped unless `SOLIDCRAFT_CLI` names
//! a `solidcraft-cli` binary:
//!
//! `SOLIDCRAFT_CLI=/Applications/SolidCraft.app/Contents/MacOS/solidcraft-cli cargo test -p solidcraft-bridge --test headless`

use std::path::{Path, PathBuf};
use std::process::Command;

use cabinet_core::geom::Vec3;
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
        &[],
    ));
    let after = bounds(&cli, "resent", &resent);
    // Every panel, plus the legs it stands on.
    assert!(before.len() >= list.parts.len());
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

/// Every lining profile the catalog has is cut for real into a seen end, on its outer face, and takes
/// the wood its section says: measured sections and those worked out from printed figures alike.
#[test]
fn every_lining_profile_is_routed_into_its_board() {
    let Some(cli) = cli() else {
        eprintln!("SOLIDCRAFT_CLI not set: skipped");
        return;
    };
    let cat = cabinet_catalog::Catalog::load(Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../catalog"
    )))
    .unwrap();
    assert!(cat.profiles.len() > 20);
    for profile in &cat.profiles {
        let mut cab = Cabinet::new("B01", CabinetKind::Base);
        cab.cladding.left = true;
        // A board thick enough for any profile's depth.
        cab.cladding.thickness = 18.0;
        cab.cladding.board = "lining".into();
        let list = build_cabinet(&cab).unwrap();
        let end = list.part("B01.SeenEndLeft").unwrap().clone();
        let section = solidcraft_bridge::LiningSection {
            module: profile.module,
            points: profile.section(),
        };
        let mut cmds = emit_cabinet(&list, Detail::Design, None);
        cmds.extend(solidcraft_bridge::emit::lining_cut(&end, &section, 300.0));
        let r = run(&cli, &format!("lining-{}", profile.id), &cmds, &["eval"]);
        let body = r["bodies"]
            .as_array()
            .unwrap()
            .iter()
            .find(|b| b["name"] == "B01.SeenEndLeft")
            .unwrap_or_else(|| panic!("{}: no body", profile.id));
        let got = body["volume_mm3"].as_f64().unwrap();
        // The section's area per repeat, over the board's width, along its height.
        let area: f64 = section
            .points
            .windows(2)
            .map(|w| (w[1][0] - w[0][0]) * (w[0][1] + w[1][1]) / 2.0)
            .sum();
        let width = end.bounds.size.y;
        let repeats = (width / section.module).floor();
        let least = area * repeats * end.bounds.size.z;
        let most = area * (repeats + 1.0) * end.bounds.size.z;
        let taken = end.bounds.volume() - got;
        assert!(
            taken >= least * 0.97 && taken <= most * 1.03,
            "{}: took {taken:.0} mm3, its section says {least:.0} to {most:.0}",
            profile.id
        );
        // The outer face is what was cut: the board's inner face is untouched, so the box holds.
        assert!(
            (body["bbox"]["max"][0].as_f64().unwrap() - end.bounds.max().x).abs() < 1e-6,
            "{}",
            profile.id
        );
    }
}

/// A shaker rail's tenons are real: they fill the stiles' grooves, which run out through the stile
/// ends, so no square hole shows at a joint.
#[test]
fn a_shaker_rail_keeps_its_tenons_in_the_stile_grooves() {
    let Some(cli) = cli() else {
        eprintln!("SOLIDCRAFT_CLI not set: skipped");
        return;
    };
    use cabinet_core::part::Operation;
    let mut cab = Cabinet::new("B01", CabinetKind::Base);
    cab.standard.door_style = cabinet_core::model::DoorStyle::Shaker;
    let mut list = build_cabinet(&cab).unwrap();
    // Only the grooves, so the volumes are exact.
    list.operations
        .retain(|o| matches!(o, Operation::Groove { .. }));
    let r = run(
        &cli,
        "tenons",
        &emit_cabinet(&list, Detail::Manufacturing, None),
        &["eval"],
    );
    for name in ["B01.Door1Rail1", "B01.Door1Rail2"] {
        let rail = list.part(name).unwrap();
        let groove = list
            .operations
            .iter()
            .find_map(|o| match o {
                Operation::Groove { part, bounds } if part == name => Some(bounds.volume()),
                _ => None,
            })
            .unwrap();
        let tenons: f64 = list.rail_tenons(rail).iter().map(|t| t.volume()).sum();
        let expected = list.visible_bounds(rail).volume() + tenons - groove;
        let body = r["bodies"]
            .as_array()
            .unwrap()
            .iter()
            .find(|b| b["name"] == name)
            .unwrap();
        let got = body["volume_mm3"].as_f64().unwrap();
        assert!(
            (got - expected).abs() < 1e-6 * expected,
            "{name}: {got} vs {expected}"
        );
    }
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
        assert_eq!(
            names(".Hinge")
                + names(".Plate")
                + names(".Runner")
                + names(".BoxSide")
                + names(".Leg"),
            expected.len(),
            "{id}"
        );
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
    use cabinet_core::model::{Light, LightZone, Voltage};
    let light = |zone| Light {
        zone,
        voltage: Voltage::V24,
        watts_per_metre: 10.0,
        profile: String::new(),
    };
    let mut corner_wall = Cabinet::new("C03", CabinetKind::CornerWall);
    corner_wall.lights = vec![light(LightZone::UnderCabinet), light(LightZone::InCabinet)];
    let mut inner = Cabinet::new("B01", CabinetKind::Base);
    inner.inner_drawers = 2;
    inner.lights = vec![light(LightZone::Plinth)];
    let mut high = Cabinet::new("D04", CabinetKind::DrawerBank);
    high.fronts = FrontLayout::Drawers {
        ratios: vec![1.0, 1.0],
    };
    high.standard.drawer_sides = cabinet_core::model::DrawerSides::HighFronted;
    use cabinet_core::model::{SectionKind, StackSection};
    let mut oven = Cabinet::new("T02", CabinetKind::Tall);
    oven.adjustable_shelves = 0;
    oven.fronts = FrontLayout::Stack {
        sections: vec![
            StackSection {
                height: 0.0,
                kind: SectionKind::Drawers { count: 2 },
            },
            StackSection {
                height: 600.0,
                kind: SectionKind::Niche,
            },
            StackSection {
                height: 0.0,
                kind: SectionKind::Door {
                    hinge_side: HingeSide::Left,
                },
            },
        ],
    };
    let mut canopy = Cabinet::new("H01", CabinetKind::Rangehood);
    canopy.cladding.left = true;
    let mut wardrobe = Cabinet::new("R01", CabinetKind::Wardrobe);
    wardrobe.wardrobe = cabinet_core::model::Hanging::Double;
    let mut shaker = Cabinet::new("B02", CabinetKind::Base);
    shaker.standard.door_style = cabinet_core::model::DoorStyle::Shaker;
    let mut hki = Cabinet::new("W02", CabinetKind::Wall);
    hki.height = 450.0;
    hki.fronts = FrontLayout::Lift {
        kind: cabinet_core::model::LiftKind::IntegratedStayLift,
        handle_weight: 0.4,
    };
    let mut lift = Cabinet::new("W01", CabinetKind::Wall);
    lift.fronts = FrontLayout::BiFoldLift { handle_weight: 0.4 };
    let mut wood = Cabinet::new("D01", CabinetKind::DrawerBank);
    wood.standard.drawer_system = DrawerSystem::Tandem;
    let legrabox = Cabinet::new("D02", CabinetKind::DrawerBank);
    let mut tandembox = Cabinet::new("D03", CabinetKind::DrawerBank);
    tandembox.standard.drawer_system = DrawerSystem::Tandembox;
    let mut fridge = Cabinet::new("T03", CabinetKind::Tall);
    fridge.fronts = FrontLayout::Stack {
        sections: vec![
            cabinet_core::model::StackSection {
                kind: cabinet_core::model::SectionKind::Niche,
                height: 0.0,
            },
            cabinet_core::model::StackSection {
                kind: cabinet_core::model::SectionKind::Door {
                    hinge_side: cabinet_core::model::HingeSide::Left,
                },
                height: 350.0,
            },
        ],
    };
    for cab in [
        fridge,
        corner,
        right,
        corner_wall,
        inner,
        high,
        shaker,
        wardrobe,
        oven,
        canopy,
        hki,
        lift,
        wood,
        legrabox,
        tandembox,
    ] {
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

/// BENCH-4: a benchtop's sink and cooktop cut-outs and tap hole cut through in SolidCraft: the
/// piece keeps its outline.
#[test]
fn benchtop_edge_profiles_remove_the_expected_material_at_every_detail_level() {
    let Some(cli) = cli() else { return };
    use cabinet_core::run::{Benchtop, Run, RunItem, build_run};
    for profile in ["pencil_round", "bullnose", "bevel"] {
        for detail in [Detail::Design, Detail::Manufacturing] {
            let top: Benchtop = serde_json::from_value(json!({
                "id": "BT1", "edge": {"type": profile},
                "side_overhang": if profile == "bullnose" { 20.0 } else { 0.0 },
            }))
            .unwrap();
            let run = Run {
                origin: Vec3::new(100.0, 200.0, 50.0),
                items: vec![
                    RunItem::Cabinet(Box::new(Cabinet::new("B01", CabinetKind::Base))),
                    RunItem::Benchtop(top),
                ],
            };
            let list = build_run(&run).unwrap().pop().unwrap();
            let p = &list.parts[0];
            let result = crate::run(
                &cli,
                &format!("edge-{profile}-{detail:?}"),
                &emit_cabinet(&list, detail, None),
                &["eval"],
            );
            let body = &result["bodies"][0];
            assert_eq!(body["closed"], true);
            let r: f64 = if profile == "bullnose" {
                p.thickness() / 2.0
            } else {
                2.0
            };
            let section = if profile == "bevel" {
                r * r / 2.0
            } else {
                2.0 * r * r * (1.0 - std::f64::consts::PI / 4.0)
            };
            let expected = p.bounds.volume() - p.bounds.size.x * section;
            let actual = body["volume_mm3"].as_f64().unwrap();
            // The CLI's volume estimate is coarse for small fillets; require the expected
            // material loss within 12%, rather than a tolerance that could admit a square slab.
            let removed = p.bounds.volume() - expected;
            assert!(
                (actual - expected).abs()
                    < if profile == "bevel" {
                        0.1
                    } else {
                        removed * 0.12
                    },
                "{profile} {detail:?}: {actual} vs {expected}"
            );
        }
    }
}

#[test]
fn translated_corner_benchtops_and_waterfalls_export_at_the_core_bounds() {
    let Some(cli) = cli() else { return };
    use cabinet_core::run::{Benchtop, Run, RunItem, build_run};
    for side in [
        cabinet_core::model::HingeSide::Left,
        cabinet_core::model::HingeSide::Right,
    ] {
        let mut cab = Cabinet::new("C01", CabinetKind::CornerBase);
        cab.corner.side = side;
        let top: Benchtop = serde_json::from_value(json!({
            "id": "BT1", "waterfall_left": true, "waterfall_right": true,
        }))
        .unwrap();
        let run = Run {
            origin: Vec3::new(1000.0, -2000.0, 250.0),
            items: vec![RunItem::Cabinet(Box::new(cab)), RunItem::Benchtop(top)],
        };
        let list = build_run(&run).unwrap().pop().unwrap();
        let built = bounds(
            &cli,
            &format!("bench-moved-{side:?}"),
            &emit_cabinet(&list, Detail::Design, None),
        );
        assert_eq!(list.parts.len(), built.len());
        for part in &list.parts {
            let got = &built.iter().find(|(name, _)| *name == part.name).unwrap().1;
            let lo = part.bounds.min;
            let hi = part.bounds.max();
            for (g, w) in got.iter().zip([lo.x, lo.y, lo.z, hi.x, hi.y, hi.z]) {
                assert!(
                    (*g - (w * 1000.0).round() as i64).abs() <= 5,
                    "{} {side:?}: {got:?}",
                    part.name
                );
            }
        }
    }
}

#[test]
fn a_benchtop_with_cut_outs_builds() {
    let Some(cli) = cli() else {
        eprintln!("SOLIDCRAFT_CLI not set: skipped");
        return;
    };
    use cabinet_core::run::{
        Benchtop, Cutout, CutoutKind, Niche, PowerPoint, Run, RunItem, build_run,
    };
    let cutout = |over: &str, kind, width, depth| Cutout::new(over, kind, width, depth);
    // BENCH-9: a sink template with radius corners.
    let mut sink = cutout("S01", CutoutKind::Sink, 760.0, 440.0);
    sink.corner_radius = 15.0;
    let run = Run {
        origin: Default::default(),
        items: vec![
            RunItem::Cabinet(Box::new(Cabinet::new("B01", CabinetKind::Base))),
            RunItem::Cabinet(Box::new(Cabinet::new("S01", CabinetKind::Sink))),
            RunItem::Benchtop(Benchtop {
                build_up: Default::default(),
                id: "BT1".into(),
                thickness: 33.0,
                front_overhang: 20.0,
                side_overhang: 0.0,
                back_clearance: 0.0,
                board: String::new(),
                edge: Default::default(),
                back_overhang: 0.0,
                waterfall_left: true,
                waterfall_right: false,
                splashback: 600.0,
                splashback_thickness: 20.0,
                niches: vec![Niche {
                    over: "S01".into(),
                    width: 0.0,
                    height: 400.0,
                    depth: 110.0,
                    sill: 150.0,
                    shelves: 1,
                }],
                power_points: vec![PowerPoint {
                    over: "B01".into(),
                    height: 150.0,
                    plate: cabinet_core::run::DEFAULT_POWER_POINT_PLATE.into(),
                }],
                end_radius: 60.0,
                material: "laminate".into(),
                material_rules: Default::default(),
                cutouts: vec![sink, cutout("B01", CutoutKind::Cooktop, 560.0, 490.0)],
            }),
        ],
    };
    let lists = build_run(&run).unwrap();
    let top = &lists[2];
    let built = bounds(
        &cli,
        "benchtop",
        &emit_cabinet(top, Detail::Manufacturing, None),
    );
    let p = &top.parts[0];
    let um = |v: f64| (v * 1000.0).round() as i64;
    let (lo, hi) = (p.bounds.min, p.bounds.max());
    let want = [um(lo.x), um(lo.y), um(lo.z), um(hi.x), um(hi.y), um(hi.z)];
    let got = &built.iter().find(|(n, _)| *n == p.name).unwrap().1;
    for (g, w) in got.iter().zip(want) {
        assert!((g - w).abs() <= 5, "{got:?} vs {want:?}");
    }
    // BENCH-3e: the free front corner (the left end is a waterfall) is filleted: a curved face
    // besides the tap hole's, and the sink cut-out's four rounded corners.
    let r = crate::run(
        &cli,
        "benchtop",
        &emit_cabinet(top, Detail::Manufacturing, None),
        &["eval"],
    );
    let body = r["bodies"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["name"] == p.name.as_str())
        .unwrap();
    assert_eq!(body["face_types"]["cylinder"], 6, "{body}");
}

/// STYLE-1: an arched pair of wall doors: SolidCraft cuts each door's top to the arch (a curved
/// face, less volume than the box) and keeps its bounding box below the top.
#[test]
fn an_arched_top_cuts_both_doors() {
    let Some(cli) = cli() else {
        eprintln!("SOLIDCRAFT_CLI not set: skipped");
        return;
    };
    let mut cab = Cabinet::new("W01", CabinetKind::Wall);
    cab.width = 900.0;
    cab.fronts = cabinet_core::model::FrontLayout::DoorPair;
    cab.arched_top = 80.0;
    let list = build_cabinet(&cab).unwrap();
    let r = run(
        &cli,
        "arched",
        &emit_cabinet(&list, Detail::Design, None),
        &["eval"],
    );
    for door in ["W01.Door1", "W01.Door2"] {
        let p = list.part(door).unwrap();
        let body = r["bodies"]
            .as_array()
            .unwrap()
            .iter()
            .find(|b| b["name"] == door)
            .unwrap();
        let full = p.bounds.size.x * p.bounds.size.y * p.bounds.size.z;
        let volume = body["volume_mm3"].as_f64().unwrap();
        assert!(volume < full - 1000.0, "{door}: {volume} of {full}");
        assert!(
            body["face_types"]["cylinder"].as_u64().unwrap_or(0) >= 1,
            "{body}"
        );
    }
}

/// WF-3: a sharknose door and a wall door come out of SolidCraft with their gripping edge's
/// back corner bevelled off: the 20 × 12 mm triangle less than the box.
#[test]
fn a_drawer_frame_is_cut_out_for_each_drawer() {
    let Some(cli) = cli() else {
        eprintln!("SOLIDCRAFT_CLI not set: skipped");
        return;
    };
    use cabinet_core::model::DoorOverlay;
    for overlay in [DoorOverlay::Full, DoorOverlay::Half, DoorOverlay::Inset] {
        let mut cab = Cabinet::new("D01", CabinetKind::DrawerBank).sectioned();
        cab.standard.drawer_frame = cabinet_core::model::DrawerFrame::Board;
        cab.standard.overlay = overlay;
        let list = build_cabinet(&cab).unwrap();
        let r = run(
            &cli,
            "drawer-frame",
            &emit_cabinet(&list, Detail::Design, None),
            &["eval"],
        );
        // Inset, it's rails with nothing to cut: every one comes through as one solid.
        if overlay == DoorOverlay::Inset {
            for p in list.by_role(cabinet_core::part::PartRole::DrawerFrame) {
                let body = r["bodies"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|b| b["name"] == p.name)
                    .unwrap();
                let full = p.bounds.size.x * p.bounds.size.y * p.bounds.size.z;
                assert!((full - body["volume_mm3"].as_f64().unwrap()).abs() < 50.0);
            }
            continue;
        }
        let p = list.part("D01.DrawerFrame").unwrap();
        let cut: f64 = list
            .operations
            .iter()
            .filter_map(|o| match o {
                cabinet_core::part::Operation::Cutout { part, bounds, .. } if *part == p.name => {
                    Some(bounds.size.x * bounds.size.y * bounds.size.z)
                }
                _ => None,
            })
            .sum();
        assert!(cut > 0.0);
        let body = r["bodies"]
            .as_array()
            .unwrap()
            .iter()
            .find(|b| b["name"] == p.name)
            .unwrap();
        let full = p.bounds.size.x * p.bounds.size.y * p.bounds.size.z;
        let volume = body["volume_mm3"].as_f64().unwrap();
        assert!(
            (full - cut - volume).abs() < 50.0,
            "{volume} of {full} less {cut}"
        );
    }
}

/// SolidCraft's own box round each tilted board against the maths that placed it.
#[test]
fn a_job_exports_as_one_step_file_with_every_cabinet_in_it() {
    let Some(cli) = cli() else { return };
    use cabinet_core::run::{Run, RunItem, build_run};
    let run = Run {
        origin: Default::default(),
        items: vec![
            RunItem::Cabinet(Box::new(Cabinet::new("B01", CabinetKind::Base))),
            RunItem::Cabinet(Box::new(Cabinet::new("D01", CabinetKind::DrawerBank))),
        ],
    };
    let lists = build_run(&run).unwrap();
    let step = solidcraft_bridge::step::job_step(&lists, Detail::Design, &cli).unwrap();
    let text = String::from_utf8_lossy(&step);
    assert!(
        text.starts_with("ISO-10303-21"),
        "{}",
        &text[..40.min(text.len())]
    );
    assert!(text.contains("B01.LeftSide") && text.contains("D01.LeftSide"));
}

#[test]
fn a_sloped_shoe_shelf_lands_where_the_maths_puts_it() {
    let Some(cli) = cli() else {
        eprintln!("SOLIDCRAFT_CLI not set: skipped");
        return;
    };
    use cabinet_core::model::{FrontLayout, SectionInside};
    for angle in [5.0, 15.0] {
        let mut cab = Cabinet::new("R01", CabinetKind::Wardrobe).sectioned();
        let FrontLayout::Sections { columns } = &mut cab.fronts else {
            panic!("sections");
        };
        columns[0].rows[0].inside = SectionInside::ShoeShelves {
            count: 3,
            angle,
            lip: 30.0,
        };
        let list = build_cabinet(&cab).unwrap();
        assert!(!list.tilts.is_empty());
        let r = run(
            &cli,
            "shoe-rack",
            &emit_cabinet(&list, Detail::Design, None),
            &["eval"],
        );
        let bodies = r["bodies"].as_array().unwrap();
        for (name, tilt) in &list.tilts {
            let part = list.part(name).unwrap();
            let (lo, hi) = tilt.reach(part.bounds);
            let body = bodies.iter().find(|b| b["name"] == *name).unwrap();
            let at = |key: &str, i: usize| body["bbox"][key][i].as_f64().unwrap();
            for (key, y, z) in [("min", lo.y, lo.z), ("max", hi.y, hi.z)] {
                assert!(
                    (at(key, 1) - y).abs() < 0.01,
                    "{name} {key} y: {} vs {y}",
                    at(key, 1)
                );
                assert!(
                    (at(key, 2) - z).abs() < 0.01,
                    "{name} {key} z: {} vs {z}",
                    at(key, 2)
                );
            }
        }
    }
}

#[test]
fn a_rappanas_rebate_is_cut_in_the_back_of_the_front() {
    let Some(cli) = cli() else {
        eprintln!("SOLIDCRAFT_CLI not set: skipped");
        return;
    };
    let mut cab = Cabinet::new("D01", CabinetKind::DrawerBank).sectioned();
    cab.standard.handle_range = cabinet_core::model::HandleRange::Abi;
    cab.standard.handle_style = cabinet_core::hardware::items()
        .iter()
        .find(|i| i.id.starts_with("handle.abi.rappana-100."))
        .unwrap()
        .id
        .clone();
    let list = build_cabinet(&cab).unwrap();
    let r = run(
        &cli,
        "rappana",
        &emit_cabinet(&list, Detail::Exact, None),
        &["eval"],
    );
    let p = list.part("D01.DrawerFront1").unwrap();
    let body = r["bodies"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["name"] == p.name)
        .unwrap();
    let rebate = list
        .operations
        .iter()
        .find_map(|o| match o {
            cabinet_core::part::Operation::Groove { part, bounds } if *part == p.name => {
                Some(bounds.size.x * bounds.size.y * bounds.size.z)
            }
            _ => None,
        })
        .unwrap();
    let full = p.bounds.size.x * p.bounds.size.y * p.bounds.size.z;
    let volume = body["volume_mm3"].as_f64().unwrap();
    let bored: f64 = list
        .operations
        .iter()
        .filter_map(|o| match o {
            cabinet_core::part::Operation::Bore {
                part,
                diameter,
                depth,
                ..
            } if *part == p.name => Some(std::f64::consts::PI * diameter * diameter / 4.0 * depth),
            _ => None,
        })
        .sum();
    assert!(
        (full - rebate - bored - volume).abs() < 100.0,
        "{volume} of {full} less {rebate} and {bored}"
    );
}

#[test]
fn a_sliding_wardrobe_builds_with_its_doors_overlapping() {
    let Some(cli) = cli() else {
        eprintln!("SOLIDCRAFT_CLI not set: skipped");
        return;
    };
    use cabinet_core::model::{FrontLayout, SectionFront};
    let mut cab = Cabinet::new("R01", CabinetKind::Wardrobe);
    cab.width = 1800.0;
    let mut cab = cab.sectioned();
    let FrontLayout::Sections { columns } = &mut cab.fronts else {
        panic!("sections");
    };
    columns[0].rows[0].front = SectionFront::Sliding {
        count: 2,
        overlap: 30.0,
        track: "generic.sliding-track".into(),
    };
    let list = build_cabinet(&cab).unwrap();
    let r = run(
        &cli,
        "sliding",
        &emit_cabinet(&list, Detail::Design, None),
        &["eval"],
    );
    let names: Vec<&str> = r["bodies"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|b| b["name"].as_str())
        .collect();
    for door in ["R01.Door1", "R01.Door2"] {
        assert!(names.contains(&door), "{names:?}");
    }
}

#[test]
fn a_door_is_bevelled_on_the_edge_chosen_with_a_rail_behind_it() {
    let Some(cli) = cli() else {
        eprintln!("SOLIDCRAFT_CLI not set: skipped");
        return;
    };
    use cabinet_core::model::SharknoseEdge;
    use cabinet_core::model::{FrontLayout, HingeSide};
    // The down leg is longer than the door is thick: legs the wrong way round fail outright.
    for (edge, hinge_side) in [
        (SharknoseEdge::Top, HingeSide::Left),
        (SharknoseEdge::Bottom, HingeSide::Left),
        (SharknoseEdge::Opening, HingeSide::Left),
        (SharknoseEdge::Opening, HingeSide::Right),
    ] {
        let mut cab = Cabinet::new("B01", CabinetKind::Base);
        cab.fronts = FrontLayout::Door { hinge_side };
        cab.standard.handle_place = cabinet_core::model::HandlePlace::Sharknose;
        cab.standard.sharknose_door_edge = edge;
        let list = build_cabinet(&cab).unwrap();
        let r = run(
            &cli,
            "door-edge",
            &emit_cabinet(&list, Detail::Design, None),
            &["eval"],
        );
        let bodies = r["bodies"].as_array().unwrap();
        let p = list.part("B01.Door1").unwrap();
        let body = bodies.iter().find(|b| b["name"] == "B01.Door1").unwrap();
        let (into, down) = (12.0, 12.0 * 3f64.sqrt());
        let length = if edge == SharknoseEdge::Opening {
            p.bounds.size.z
        } else {
            p.bounds.size.x
        };
        let full = p.bounds.size.x * p.bounds.size.y * p.bounds.size.z;
        let volume = body["volume_mm3"].as_f64().unwrap();
        let cut = length * into * down / 2.0;
        assert!(
            (full - cut - volume).abs() < 50.0,
            "{edge:?}: {volume} of {full} less {cut}"
        );
        assert!(
            bodies.iter().any(|b| b["name"] == "B01.GripRail1"),
            "{edge:?}"
        );
    }
}

#[test]
fn a_sharknose_bevels_the_gripping_edge() {
    let Some(cli) = cli() else {
        eprintln!("SOLIDCRAFT_CLI not set: skipped");
        return;
    };
    use cabinet_core::model::SharknoseAngle;
    for (kind, door, angle, down) in [
        (
            CabinetKind::Base,
            "B01.Door1",
            SharknoseAngle::Bevel30,
            12.0 * 3f64.sqrt(),
        ),
        (
            CabinetKind::Wall,
            "W01.Door1",
            SharknoseAngle::Bevel30,
            12.0 * 3f64.sqrt(),
        ),
        (
            CabinetKind::Base,
            "B01.Door1",
            SharknoseAngle::Bevel45,
            12.0,
        ),
    ] {
        let id = &door[..3];
        let mut cab = Cabinet::new(id, kind);
        cab.standard.handle_place = cabinet_core::model::HandlePlace::Sharknose;
        cab.standard.sharknose_angle = angle;
        let list = build_cabinet(&cab).unwrap();
        let r = run(
            &cli,
            "sharknose",
            &emit_cabinet(&list, Detail::Design, None),
            &["eval"],
        );
        let p = list.part(door).unwrap();
        let body = r["bodies"]
            .as_array()
            .unwrap()
            .iter()
            .find(|b| b["name"] == door)
            .unwrap();
        let full = p.bounds.size.x * p.bounds.size.y * p.bounds.size.z;
        let cut = p.bounds.size.x * down * 12.0 / 2.0;
        let volume = body["volume_mm3"].as_f64().unwrap();
        assert!(
            (full - cut - volume).abs() < 50.0,
            "{door} {angle:?}: {volume} vs {}",
            full - cut
        );
    }
}

/// A 10 x 20 x 30 box from the origin as a binary STL, enough to see where a mesh lands.
fn box_stl(path: &Path) {
    let p = |x: f32, y: f32, z: f32| [x, y, z];
    let c = [
        p(0., 0., 0.),
        p(10., 0., 0.),
        p(10., 20., 0.),
        p(0., 20., 0.),
        p(0., 0., 30.),
        p(10., 0., 30.),
        p(10., 20., 30.),
        p(0., 20., 30.),
    ];
    let faces = [
        [0, 2, 1],
        [0, 3, 2],
        [4, 5, 6],
        [4, 6, 7],
        [0, 1, 5],
        [0, 5, 4],
        [1, 2, 6],
        [1, 6, 5],
        [2, 3, 7],
        [2, 7, 6],
        [3, 0, 4],
        [3, 4, 7],
    ];
    let mut b = vec![0u8; 80];
    b.extend((faces.len() as u32).to_le_bytes());
    for f in faces {
        b.extend([0u8; 12]);
        for i in f {
            for v in c[i] {
                b.extend(v.to_le_bytes());
            }
        }
        b.extend([0u8; 2]);
    }
    std::fs::write(path, b).unwrap();
}

/// What the owner found: handles sitting at the origin. An engine that lands a mesh inserted by
/// `hardware_inserts` anywhere but its placement fails here. Older engines have no mesh insert.
#[test]
fn a_supplier_mesh_lands_where_its_placement_says() {
    let Some(cli) = cli() else {
        eprintln!("SOLIDCRAFT_CLI not set: skipped");
        return;
    };
    let help = Command::new(&cli).arg("commands").output().unwrap().stdout;
    if !String::from_utf8_lossy(&help).contains("file.insert_mesh") {
        eprintln!("this engine has no file.insert_mesh: skipped");
        return;
    }
    let dir = std::env::temp_dir().join("cabinet-headless");
    std::fs::create_dir_all(&dir).unwrap();
    let stl = dir.join("mesh-box.stl");
    box_stl(&stl);
    let model = solidcraft_bridge::vendor::VendorInstance {
        name: "B01.Handle1".into(),
        item: "handle.abi.tezra-220.brushed-brass".into(),
        article: "ABI-TEST".into(),
        step: stl.clone(),
        stl,
        // A quarter turn about Y, as a pull on a door stands: model X up the door.
        transform: [
            0.0, 0.0, 1.0, 100.0, 0.0, 1.0, 0.0, 200.0, -1.0, 0.0, 0.0, 300.0,
        ],
    };
    let mut commands = vec![json!({ "command": "component.create", "params": { "name": "B01" } })];
    commands.extend(solidcraft_bridge::hardware_inserts("B01", &[model]));
    let got = bounds(&cli, "mesh-lands", &commands);
    assert!(!got.is_empty(), "no body came in");
    let (_, b) = &got[0];
    // x = 100 + z, y = 200 + y, z = 300 - x.
    assert_eq!(
        b,
        &vec![100_000, 200_000, 290_000, 130_000, 220_000, 300_000],
        "{got:?}"
    );
}

/// The same, the way the app does it: a whole cabinet built, then its handle models inserted into
/// its component. Each lands on its own front's face, in the core's own placement.
#[test]
fn abi_handle_models_land_on_their_fronts_in_a_built_cabinet() {
    let Some(cli) = cli() else {
        eprintln!("SOLIDCRAFT_CLI not set: skipped");
        return;
    };
    let help = Command::new(&cli).arg("commands").output().unwrap().stdout;
    if !String::from_utf8_lossy(&help).contains("file.insert_mesh") {
        eprintln!("this engine has no file.insert_mesh: skipped");
        return;
    }
    let dir = std::env::temp_dir().join("cabinet-headless");
    std::fs::create_dir_all(&dir).unwrap();
    let stl = dir.join("mesh-box.stl");
    box_stl(&stl);
    let mut cab = Cabinet::new("S01", CabinetKind::Sink);
    cab.standard.handle_range = cabinet_core::model::HandleRange::Abi;
    cab.standard.handle_style = "handle.abi.tezra-220.brushed-brass".into();
    let list = build_cabinet(&cab).unwrap();
    let models: Vec<_> = cabinet_core::cad::cad_placements(&list)
        .into_iter()
        .filter(|p| p.item.starts_with("handle.abi."))
        .map(|p| solidcraft_bridge::vendor::VendorInstance {
            name: p.name.clone(),
            item: p.item.clone(),
            article: p.article.clone(),
            step: stl.clone(),
            stl: stl.clone(),
            transform: cabinet_core::cad::transform(&p, false, 0.0),
        })
        .collect();
    assert_eq!(models.len(), 2);
    let real: Vec<String> = models.iter().map(|m| m.name.clone()).collect();
    // Design is the app's default level, and the owner's handles went missing at it.
    let mut commands = solidcraft_bridge::replace_cabinet(&list, Detail::Design, None, None, &real);
    commands.extend(solidcraft_bridge::hardware_inserts("S01", &models));
    let got = bounds(&cli, "abi-in-cabinet", &commands);
    let meshes: Vec<_> = got
        .iter()
        .filter(|(n, b)| {
            !list.parts.iter().any(|p| p.name == *n) && n.contains("mesh")
                || n.contains("Handle") && b.len() == 6
        })
        .collect();
    assert_eq!(
        meshes.len(),
        2,
        "{:?}",
        got.iter().map(|g| &g.0).collect::<Vec<_>>()
    );
    for m in &models {
        let t = m.transform;
        // The test box's corner at the model's origin goes to the placement's translation.
        let want = [t[3], t[7], t[11]].map(|v| (v * 1000.0).round() as i64);
        let hit = meshes
            .iter()
            .any(|(_, b)| (0..3).all(|a| b[a] <= want[a] + 5 && want[a] <= b[a + 3] + 5));
        assert!(hit, "{}: {want:?} not on {meshes:?}", m.name);
        assert!(t[3] > 1.0 && t[11] > 100.0, "{}: at the origin", m.name);
    }
}

#[test]
fn a_tiled_splashback_builds_with_a_body_for_each_tile() {
    use cabinet_core::run::{Run, RunItem, Splashback, build_run};
    use cabinet_core::tiling::{TilePattern, TileSpec};
    let Some(cli) = cli() else {
        return;
    };
    let run_items = Run {
        origin: Vec3::new(0.0, 0.0, 0.0),
        items: vec![
            RunItem::Cabinet(Box::new(Cabinet::new("B01", CabinetKind::Base))),
            RunItem::Splashback(Splashback {
                id: "S1".into(),
                spec: TileSpec {
                    tile_width: 200.0,
                    tile_height: 100.0,
                    grout: 2.0,
                    pattern: TilePattern::Brick,
                    colour: "#F2EFE8".into(),
                    grout_colour: "#555555".into(),
                },
                thickness: 10.0,
            }),
        ],
    };
    let lists = build_run(&run_items).unwrap();
    let field = lists[1].tile_fields[0].clone();
    let commands = emit_cabinet(&lists[1], Detail::Design, None);
    let bodies = bounds(&cli, "tiled", &commands);
    let tiles = field.spec.layout(field.bounds.size.x, field.bounds.size.z);
    assert_eq!(bodies.len(), tiles.len() + 1, "one bed and every tile");
}

#[test]
fn mitred_stone_build_up_and_waterfalls_are_closed_solids_with_stock_supports() {
    let Some(cli) = cli() else { return };
    use cabinet_core::run::{Benchtop, Run, RunItem, build_run};
    for (falls, kind, side) in [
        (
            false,
            CabinetKind::Base,
            cabinet_core::model::HingeSide::Left,
        ),
        (
            true,
            CabinetKind::Base,
            cabinet_core::model::HingeSide::Left,
        ),
        (
            false,
            CabinetKind::CornerBase,
            cabinet_core::model::HingeSide::Left,
        ),
        (
            false,
            CabinetKind::CornerBase,
            cabinet_core::model::HingeSide::Right,
        ),
    ] {
        let b: Benchtop = serde_json::from_value(json!({"id":"BT1","thickness":20,"side_overhang":20,
            "waterfall_left":falls,"waterfall_right":falls,"edge":{"type":"mitred_apron","depth":60},
            "build_up":{"shadow_height":20,"edge_tape":{"id":"accent","colour":"#aa7722","thickness":1}}})).unwrap();
        let mut cabinet = Cabinet::new("B01", kind);
        cabinet.corner.side = side;
        let list = build_run(&Run {
            origin: Vec3::default(),
            items: vec![RunItem::Cabinet(Box::new(cabinet)), RunItem::Benchtop(b)],
        })
        .unwrap()
        .pop()
        .unwrap();
        for detail in [Detail::Design, Detail::Manufacturing] {
            let result = run(
                &cli,
                &format!("mitred-build-up-{falls}-{kind:?}-{side:?}-{detail:?}"),
                &emit_cabinet(&list, detail, None),
                &["eval"],
            );
            let bodies = result["bodies"].as_array().unwrap();
            assert_eq!(bodies.len(), list.parts.len());
            for body in bodies {
                let name = body["name"].as_str().unwrap();
                let p = list.part(name).unwrap();
                assert_eq!(body["closed"], true, "{name}");
                let actual = body["volume_mm3"].as_f64().unwrap();
                assert!(
                    actual > 0.0 && actual <= p.bounds.volume() + 0.1,
                    "{name}: {actual}"
                );
                if name.contains("Apron") || name.contains("Waterfall") || name.contains("Benchtop")
                {
                    assert!(actual < p.bounds.volume() - 1.0, "mitre absent: {name}");
                }
            }
        }
    }
}

#[test]
fn rear_shadow_frames_and_continuous_seen_backs_export_with_and_without_waterfalls() {
    let Some(cli) = cli() else { return };
    use cabinet_core::run::{Benchtop, Run, RunItem, build_run};
    for falls in [false, true] {
        let mut a = Cabinet::new("B01", CabinetKind::Base);
        a.width = 900.0;
        a.cladding.back = true;
        let mut b = a.clone();
        b.id = "B02".into();
        let top:Benchtop=serde_json::from_value(json!({"id":"BT1","thickness":20,"front_overhang":0,"side_overhang":0,"back_overhang":300,
            "waterfall_left":falls,"waterfall_right":falls,"edge":{"type":"mitred_apron","depth":60},
            "build_up":{"shadow_height":15,"setback":10,"strip_width":30,"board_thickness":18,
            "edge_tape":{"id":"accent","colour":"#B59A61","thickness":1}}})).unwrap();
        let lists = build_run(&Run {
            origin: Vec3::new(100.0, 200.0, 50.0),
            items: vec![
                RunItem::Cabinet(Box::new(a)),
                RunItem::Cabinet(Box::new(b)),
                RunItem::Benchtop(top),
            ],
        })
        .unwrap();
        assert_eq!(lists[0].part("B01.SeenBack").unwrap().bounds.size.x, 1800.0);
        for detail in [Detail::Design, Detail::Manufacturing] {
            let list = &lists[2];
            let result = run(
                &cli,
                &format!("rear-frame-{falls}-{detail:?}"),
                &emit_cabinet(list, detail, None),
                &["eval"],
            );
            assert_eq!(result["bodies"].as_array().unwrap().len(), list.parts.len());
            for body in result["bodies"].as_array().unwrap() {
                assert_eq!(body["closed"], true, "{}", body["name"]);
                assert!(body["volume_mm3"].as_f64().unwrap() > 0.0);
            }
        }
    }
}

#[test]
fn three_matching_seen_backs_export_as_one_body_without_cabinet_seams() {
    let Some(cli) = cli() else { return };
    use cabinet_core::run::{Run, RunItem, build_run};
    let items = [600.0, 600.0, 500.0]
        .iter()
        .enumerate()
        .map(|(i, &width)| {
            let mut c = Cabinet::new(format!("B0{}", i + 1), CabinetKind::Base);
            c.width = width;
            c.cladding.back = true;
            c.cladding.thickness = 16.0;
            RunItem::Cabinet(Box::new(c))
        })
        .collect();
    let lists = build_run(&Run {
        origin: Vec3::default(),
        items,
    })
    .unwrap();
    let commands: Vec<_> = lists
        .iter()
        .flat_map(|l| emit_cabinet(l, Detail::Design, None))
        .collect();
    let bodies = bounds(&cli, "three-continuous-backs", &commands);
    let backs: Vec<_> = bodies
        .iter()
        .filter(|(name, _)| name.contains(".SeenBack"))
        .collect();
    assert_eq!(backs.len(), 1, "cabinet seams survived the export");
    assert_eq!(backs[0].1[3] - backs[0].1[0], 1_700_000);
}
