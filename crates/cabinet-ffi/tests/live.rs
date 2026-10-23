//! Checks against a running SolidCraft app. Skipped unless `SOLIDCRAFT_LIVE_PORT` names its control
//! port, and then only run when the open design is untitled and empty (docs/OVERNIGHT.md), so it
//! never touches anyone's work:
//!
//! `SOLIDCRAFT_LIVE_PORT=7878 cargo test -p cabinet-ffi --test live -- --test-threads 1`

use cabinet_ffi::{JobItems, new_cabinet, render_png, send_to_solidcraft};
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
}
