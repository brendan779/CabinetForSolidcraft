//! Builds a sample run of cabinets in the running SolidCraft app.
//!
//! `cargo run -p solidcraft-bridge --example emit_base -- [--manufacturing | --exact] [--port 7878]`
//!
//! With `--script FILE` it writes a SolidCraft script instead of connecting, for headless runs:
//! `solidcraft-cli run FILE --out run.step`.

use cabinet_core::geom::Vec3;
use cabinet_core::{Cabinet, CabinetKind, build_cabinet};
use solidcraft_bridge::{Client, Detail, emit_cabinet};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let detail = if args.iter().any(|a| a == "--exact") {
        Detail::Exact
    } else if args.iter().any(|a| a == "--manufacturing") {
        Detail::Manufacturing
    } else {
        Detail::Design
    };
    let port = args
        .iter()
        .position(|a| a == "--port")
        .and_then(|i| args.get(i + 1))
        .and_then(|p| p.parse().ok())
        .unwrap_or(solidcraft_bridge::client::DEFAULT_PORT);

    let mut run = vec![
        Cabinet::new("B01", CabinetKind::Base),
        Cabinet::new("D01", CabinetKind::DrawerBank),
        Cabinet::new("T01", CabinetKind::Tall),
    ];
    let mut x = 0.0;
    for c in &mut run {
        c.origin = Vec3::new(x, 0.0, 0.0);
        x += c.width;
    }
    if let Some(path) = args
        .iter()
        .position(|a| a == "--script")
        .and_then(|i| args.get(i + 1))
    {
        let mut commands = Vec::new();
        for c in &run {
            let list = build_cabinet(c)?;
            commands.extend(emit_cabinet(&list, detail, Some(&serde_json::to_value(c)?)));
        }
        std::fs::write(
            path,
            serde_json::to_string_pretty(&serde_json::json!({ "commands": commands }))?,
        )?;
        println!("wrote {} commands to {path}", commands.len());
        return Ok(());
    }
    let mut client = Client::connect(port)?;
    for c in &run {
        let list = build_cabinet(c)?;
        let spec = serde_json::to_value(c)?;
        let cmds = emit_cabinet(&list, detail, Some(&spec));
        let r = client.batch(&cmds)?;
        println!(
            "{}: {} commands, {:.0} ms, {} warnings",
            c.id,
            cmds.len(),
            r["total_ms"].as_f64().unwrap_or(0.0),
            list.warnings.len()
        );
        for w in &list.warnings {
            println!("  warning: {w}");
        }
    }
    Ok(())
}
