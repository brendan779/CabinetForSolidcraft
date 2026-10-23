//! Checks against a running SolidCraft app. Skipped unless `SOLIDCRAFT_LIVE_PORT` names its control
//! port, and then only run when the open design is untitled and empty (docs/OVERNIGHT.md), so it
//! never touches anyone's work:
//!
//! `SOLIDCRAFT_LIVE_PORT=7878 cargo test -p cabinet-ffi --test live -- --test-threads 1`

use cabinet_ffi::{
    JobItems, cabinets_from_selection, cabinets_in_design, hide_bodies, new_cabinet, render_png,
    send_to_solidcraft,
};
use serde_json::json;
use solidcraft_bridge::Client;

fn empty_design() -> Option<u16> {
    let port: u16 = std::env::var("SOLIDCRAFT_LIVE_PORT").ok()?.parse().ok()?;
    let doc = Client::connect(port)
        .ok()?
        .request("document.inspect", serde_json::json!({}))
        .ok()?;
    let empty = doc["path"].is_null() && doc["bodies"].as_array().is_none_or(|b| b.is_empty());
    if !empty {
        eprintln!("the open design is not untitled and empty: skipped");
    }
    empty.then_some(port)
}

#[test]
fn a_job_builds_in_solidcraft_and_renders_to_a_png() {
    let Some(port) = empty_design() else {
        eprintln!("SOLIDCRAFT_LIVE_PORT not set or design in use: skipped");
        return;
    };
    let items = vec![
        new_cabinet("B01".into(), "base".into()).unwrap(),
        r#"{"type":"filler","id":"F01","width":50}"#.to_string(),
        new_cabinet("D01".into(), "drawer_bank".into()).unwrap(),
    ];
    let job = || JobItems {
        items_json: items.clone(),
        placed: Vec::new(),
    };
    let report = send_to_solidcraft(job(), "design".into(), port).unwrap();
    assert_eq!(report.lines().count(), 3, "{report}");
    let path = std::env::temp_dir().join("cabinet-live-render.png");
    let _ = std::fs::remove_file(&path);
    render_png(job(), port, path.to_string_lossy().into(), 800, 500).unwrap();
    let png = std::fs::read(&path).unwrap();
    assert!(png.starts_with(b"\x89PNG"));
    clear(port, &["B01", "F01", "D01"], &[]);
}

/// Deletes what a test built, so the design is empty again for the next.
fn clear(port: u16, components: &[&str], bodies: &[&str]) {
    let mut c = Client::connect(port).unwrap();
    for name in components {
        c.execute("component.delete", json!({ "component": name }))
            .unwrap();
    }
    if !bodies.is_empty() {
        let items: Vec<_> = bodies
            .iter()
            .map(|b| json!({ "type": "body", "name": b }))
            .collect();
        c.execute("selection.delete", json!({ "items": items }))
            .unwrap();
    }
}

#[test]
fn boxes_selected_in_solidcraft_become_cabinets_that_load_back() {
    let Some(port) = empty_design() else {
        eprintln!("SOLIDCRAFT_LIVE_PORT not set or design in use: skipped");
        return;
    };
    let mut c = Client::connect(port).unwrap();
    // A base, a tall and a wall cabinet's envelope, drawn as plain boxes.
    let boxes = [
        ("Box1", [600.0, 580.0, 870.0], [0.0, 0.0, 0.0]),
        ("Box2", [600.0, 580.0, 2250.0], [700.0, 0.0, 0.0]),
        ("Box3", [800.0, 330.0, 720.0], [1400.0, 0.0, 1400.0]),
    ];
    for (name, [l, w, h], corner) in boxes {
        c.execute(
            "solid.box",
            json!({ "length": l, "width": w, "height": h, "corner": corner,
                    "operation": "new", "body_name": name }),
        )
        .unwrap();
    }
    let items: Vec<_> = boxes
        .iter()
        .map(|(n, ..)| json!({ "type": "body", "name": n }))
        .collect();
    c.execute("select.set", json!({ "items": items })).unwrap();

    let made = cabinets_from_selection(port, Vec::new()).unwrap();
    assert!(made.skipped.is_empty(), "{:?}", made.skipped);
    let kinds: Vec<String> = made
        .cabinets
        .iter()
        .map(|d| serde_json::from_str::<serde_json::Value>(&d.json).unwrap()["kind"].to_string())
        .collect();
    assert_eq!(kinds, [r#""base""#, r#""tall""#, r#""wall""#]);
    let ids: Vec<String> = made.cabinets.iter().map(|d| d.id.clone()).collect();
    send_to_solidcraft(
        JobItems {
            items_json: made.cabinets.iter().map(|d| d.json.clone()).collect(),
            placed: ids.clone(),
        },
        "design".into(),
        port,
    )
    .unwrap();
    hide_bodies(
        port,
        made.cabinets.iter().map(|d| d.source.clone()).collect(),
    )
    .unwrap();

    // Loading the design back gives the same cabinets, where they were drawn.
    let loaded = cabinets_in_design(port).unwrap();
    let mut back: Vec<serde_json::Value> = loaded
        .iter()
        .map(|j| serde_json::from_str(j).unwrap())
        .collect();
    back.sort_by_key(|v| v["id"].to_string());
    let mut sent: Vec<serde_json::Value> = made
        .cabinets
        .iter()
        .map(|d| serde_json::from_str(&d.json).unwrap())
        .collect();
    sent.sort_by_key(|v| v["id"].to_string());
    assert_eq!(back, sent);
    assert_eq!(
        back[2]["origin"]["z"], 1400.0,
        "the wall cabinet stays up the wall"
    );

    let ids: Vec<&str> = ids.iter().map(String::as_str).collect();
    clear(port, &ids, &["Box1", "Box2", "Box3"]);
}
