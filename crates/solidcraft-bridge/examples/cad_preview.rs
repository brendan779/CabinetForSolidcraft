//! Builds a door base and a drawer bank with the supplier's own hardware models, as a SolidCraft
//! script, for checking where the models land.
//!
//! `cargo run -p solidcraft-bridge --example cad_preview -- VENDOR_DIR SOLIDCRAFT_CLI OUT.json`

use std::path::Path;

use cabinet_core::cad::cad_placements;
use cabinet_core::geom::Vec3;
use cabinet_core::model::DrawerSystem;
use cabinet_core::{Cabinet, CabinetKind, build_cabinet};
use serde_json::json;
use solidcraft_bridge::{Detail, emit_cabinet, vendor};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<String> = std::env::args().collect();
    let (dir, cli, out) = (Path::new(&a[1]), Path::new(&a[2]), &a[3]);
    let (models, converted) = vendor::prepare(dir, cli)?;
    println!("{} models ({converted} converted)", models.len());
    let mut base = Cabinet::new("B01", CabinetKind::Base);
    base.standard.drawer_system = DrawerSystem::Movento;
    let mut bank = Cabinet::new("D01", CabinetKind::DrawerBank);
    bank.standard.drawer_system = DrawerSystem::Movento;
    bank.origin = Vec3::new(650.0, 0.0, 0.0);
    let mut cmds = Vec::new();
    for c in [&base, &bank] {
        let list = build_cabinet(c)?;
        cmds.extend(emit_cabinet(&list, Detail::Design, None));
        let found = vendor::resolve(&cad_placements(&list), &models);
        println!("{}: {} supplier models", c.id, found.len());
        cmds.push(json!({"command": "component.activate", "params": {"component": c.id}}));
        for i in found {
            cmds.push(json!({"command": "file.insert_step", "params": {
                "path": i.step, "name": i.name, "matrix": i.transform }}));
        }
        cmds.push(json!({"command": "component.activate", "params": {"component": "root"}}));
    }
    std::fs::write(
        out,
        serde_json::to_string_pretty(&json!({ "commands": cmds }))?,
    )?;
    Ok(())
}
