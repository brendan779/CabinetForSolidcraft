//! Writes a cut sheet SVG for every part of a sample base cabinet, and all of them as one PDF.
//!
//! `cargo run -p cabinet-sheets --example sheets -- OUT_DIR`

use cabinet_core::{Cabinet, CabinetKind, build_cabinet};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/sheets".into());
    std::fs::create_dir_all(&out)?;
    let list = build_cabinet(&Cabinet::new("B01", CabinetKind::Base))?;
    for p in &list.parts {
        let svg = cabinet_sheets::part_sheet(
            &list,
            p,
            "Polytec Classic White Texture 16 MR MDF",
            "Sample job",
        );
        std::fs::write(format!("{out}/{}.svg", p.name), svg)?;
    }
    let pdf = cabinet_sheets::pdf::job_pdf(
        std::slice::from_ref(&list),
        |_| "Polytec Classic White Texture 16 MR MDF".into(),
        "Sample job",
    );
    std::fs::write(format!("{out}/sample.pdf"), pdf)?;
    println!("{} sheets and sample.pdf in {out}", list.parts.len());
    Ok(())
}
