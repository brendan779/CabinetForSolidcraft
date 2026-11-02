//! Converts new supplier STEP files to meshes, like Settings › Folders › Prepare Now.
//!
//! `cargo run -p solidcraft-bridge --example prepare_cad -- vendor-cad`

use std::path::Path;

use solidcraft_bridge::vendor::prepare;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "vendor-cad".into());
    let cli = "/Applications/SolidCraft.app/Contents/MacOS/solidcraft-cli";
    let (models, converted) = prepare(Path::new(&dir), Path::new(cli))?;
    println!("{} models, {converted} converted", models.len());
    Ok(())
}
