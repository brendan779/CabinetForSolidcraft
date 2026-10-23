//! Measures what each kind of machining command costs in SolidCraft (PERF-2).
//!
//! `cargo run -q -p solidcraft-bridge --example perf_probe -- [--cli PATH] [--count N]`
//!
//! For each kind, it builds one 16 × 560 × 720 mm panel (a base cabinet side) and cuts N features
//! into it, one command each, then runs the script through `solidcraft-cli run --timings
//! --profile`. It prints a Markdown table for docs/perf.md: the cost of the first, the middle and
//! the last cut, so the growth as the panel gains features shows.

use std::process::Command;

use serde_json::{Value, json};

const PANEL: [f64; 3] = [16.0, 560.0, 720.0];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let arg = |name: &str| {
        args.iter()
            .position(|a| a == name)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };
    let cli = arg("--cli")
        .unwrap_or_else(|| "/Applications/SolidCraft.app/Contents/MacOS/solidcraft-cli".into());
    let count: usize = arg("--count").and_then(|c| c.parse().ok()).unwrap_or(40);
    let dir = std::env::temp_dir().join("cabinet-perf-probe");
    std::fs::create_dir_all(&dir)?;

    println!("| command | features | first ms | middle ms | last ms | total ms | boolean % |");
    println!("|---|---:|---:|---:|---:|---:|---:|");
    for (kind, commands) in [
        ("solid.box (new panel)", boxes(count)),
        ("solid.box cut", box_cuts(count)),
        ("solid.cylinder cut", cylinder_cuts(count)),
        ("sketch + one extrude cut", sketch_cut(count)),
    ] {
        let path = dir.join(format!("{}.json", kind.split_whitespace().next().unwrap()));
        std::fs::write(
            &path,
            serde_json::to_string(&json!({ "commands": commands }))?,
        )?;
        let out = Command::new(&cli)
            .arg("run")
            .arg(&path)
            .args(["--timings", "--profile", "--quiet"])
            .output()?;
        if !out.status.success() {
            return Err(format!("{kind}: {}", String::from_utf8_lossy(&out.stderr)).into());
        }
        let r: Value = serde_json::from_slice(&out.stdout)?;
        let steps = r["timings"].as_array().ok_or("no timings")?;
        // The measured cuts follow the panel; for the box row every step is measured.
        let cuts: Vec<&Value> = if kind.starts_with("solid.box (new") {
            steps.iter().collect()
        } else {
            steps.iter().skip(1).collect()
        };
        let ms = |s: &Value| s["ms"].as_f64().unwrap_or(0.0);
        let total: f64 = cuts.iter().map(|s| ms(s)).sum();
        let boolean: f64 = cuts
            .iter()
            .map(|s| s["profile"]["boolean_ms"].as_f64().unwrap_or(0.0))
            .sum();
        // A sketch cut is many cheap sketch steps and one extrude: first/middle/last of the
        // whole sequence would say little, so report the extrude for all three.
        let (first, middle, last) = if kind.starts_with("sketch") {
            let e = cuts
                .iter()
                .find(|s| s["command"] == "solid.extrude")
                .map(|s| ms(s))
                .unwrap_or(0.0);
            (e, e, e)
        } else {
            (
                ms(cuts[0]),
                ms(cuts[cuts.len() / 2]),
                ms(cuts[cuts.len() - 1]),
            )
        };
        println!(
            "| {kind} | {count} | {first:.1} | {middle:.1} | {last:.1} | {total:.1} | {:.0} |",
            100.0 * boolean / total.max(1e-9)
        );
    }
    Ok(())
}

fn panel(name: &str, x: f64) -> Value {
    json!({ "command": "solid.box", "params": {
        "length": PANEL[0], "width": PANEL[1], "height": PANEL[2],
        "corner": [x, 0.0, 0.0], "operation": "new", "body_name": name, "name": name,
    }})
}

/// Points on a 32 mm grid on the panel face, 37 mm in from the edges, row by row.
fn grid(count: usize) -> impl Iterator<Item = (f64, f64)> {
    (0..count).map(|i| (37.0 + 32.0 * (i % 15) as f64, 37.0 + 32.0 * (i / 15) as f64))
}

fn boxes(count: usize) -> Vec<Value> {
    (0..count)
        .map(|i| panel(&format!("P{i}"), 20.0 * i as f64))
        .collect()
}

fn box_cuts(count: usize) -> Vec<Value> {
    let mut v = vec![panel("P", 0.0)];
    // Biscuit-slot sized: 4 × 56 × 12 mm into the inner face.
    v.extend(grid(count).map(|(y, z)| {
        json!({ "command": "solid.box", "params": {
            "length": 4.0, "width": 20.0, "height": 12.0, "corner": [12.0, y - 10.0, z - 6.0],
            "operation": "cut", "targets": ["P"],
        }})
    }));
    v
}

fn cylinder_cuts(count: usize) -> Vec<Value> {
    let mut v = vec![panel("P", 0.0)];
    // Shelf-pin sized: Ø5 × 10 mm into the inner face.
    v.extend(grid(count).map(|(y, z)| {
        json!({ "command": "solid.cylinder", "params": {
            "diameter": 5.0, "height": 10.0, "base": [6.0, y, z], "axis": "X",
            "operation": "cut", "targets": ["P"],
        }})
    }));
    v
}

fn sketch_cut(count: usize) -> Vec<Value> {
    let mut v = vec![
        panel("P", 0.0),
        json!({ "command": "sketch.create", "params": {
            "plane": { "origin": [PANEL[0], 0.0, 0.0], "x_dir": [0, 1, 0], "y_dir": [0, 0, 1] },
            "name": "Holes",
        }}),
    ];
    v.extend(grid(count).map(|(y, z)| {
        json!({ "command": "sketch.circle.center", "params": { "center": [y, z], "diameter": 5.0 } })
    }));
    v.push(json!({ "command": "sketch.finish", "params": {} }));
    v.push(json!({ "command": "solid.extrude", "params": {
        "sketch": "Holes", "profiles": "all", "distance": 10.0, "direction": "negative",
        "operation": "cut", "targets": ["P"],
    }}));
    v
}
