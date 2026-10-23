//! The macOS app's view of the Rust core. Cabinets cross the boundary as JSON (the same JSON
//! stored in SolidCraft designs), results as small typed records the SwiftUI views draw directly.

use cabinet_core::cutlist::{cut_list, to_csv};
use cabinet_core::{Cabinet, CabinetKind, build_cabinet};
use solidcraft_bridge::{Client, Detail, emit_cabinet};

uniffi::setup_scaffolding!();

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum CoreError {
    #[error("{message}")]
    Invalid { message: String },
    #[error("{message}")]
    SolidCraft { message: String },
}

fn invalid(e: impl std::fmt::Display) -> CoreError {
    CoreError::Invalid {
        message: e.to_string(),
    }
}

fn parse(json: &str) -> Result<Cabinet, CoreError> {
    serde_json::from_str(json).map_err(invalid)
}

/// One panel as a box, for the app's 3D preview.
#[derive(uniffi::Record)]
pub struct PanelBox {
    pub name: String,
    pub role: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub width: f64,
    pub depth: f64,
    pub height: f64,
    pub is_front: bool,
}

#[derive(uniffi::Record)]
pub struct BuildSummary {
    pub panels: Vec<PanelBox>,
    pub operations: u32,
    pub biscuits: u32,
    pub screws: u32,
    pub hardware: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(uniffi::Record)]
pub struct ChoiceRecord {
    pub id: String,
    pub label: String,
    pub description: String,
}

#[derive(uniffi::Record)]
pub struct OptionGroupRecord {
    pub field: String,
    pub title: String,
    pub choices: Vec<ChoiceRecord>,
}

#[derive(uniffi::Record)]
pub struct BoardRecord {
    pub id: String,
    pub brand: String,
    pub range: String,
    pub decor: String,
    pub finish: String,
    pub thickness: f64,
    pub substrate: String,
    pub sheets: Vec<String>,
    pub preview_colour: String,
    pub grained: bool,
    pub verified: bool,
}

#[uniffi::export]
pub fn core_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Cabinet kinds as ids (`base`, `wall`, ...).
#[uniffi::export]
pub fn cabinet_kinds() -> Vec<String> {
    CabinetKind::ALL
        .iter()
        .map(|k| {
            serde_json::to_value(k)
                .ok()
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_default()
        })
        .collect()
}

/// A new cabinet of `kind` as JSON, with the kind's usual size and the default standard.
#[uniffi::export]
pub fn new_cabinet(id: String, kind: String) -> Result<String, CoreError> {
    let kind: CabinetKind =
        serde_json::from_value(serde_json::Value::String(kind)).map_err(invalid)?;
    serde_json::to_string(&Cabinet::new(id, kind)).map_err(invalid)
}

/// Sets one field of the cabinet's construction standard to a picker choice id.
#[uniffi::export]
pub fn set_standard_option(
    cabinet_json: String,
    field: String,
    choice: String,
) -> Result<String, CoreError> {
    let mut v: serde_json::Value = serde_json::from_str(&cabinet_json).map_err(invalid)?;
    v["standard"][&field] = serde_json::Value::String(choice);
    let cab: Cabinet = serde_json::from_value(v).map_err(invalid)?;
    serde_json::to_string(&cab).map_err(invalid)
}

#[uniffi::export]
pub fn build(cabinet_json: String) -> Result<BuildSummary, CoreError> {
    use cabinet_core::part::{BorePurpose, Operation, PartRole};
    let cab = parse(&cabinet_json)?;
    let list = build_cabinet(&cab).map_err(invalid)?;
    let panels = list
        .parts
        .iter()
        .map(|p| PanelBox {
            name: p.name.clone(),
            role: p.role.label().to_string(),
            x: p.bounds.min.x,
            y: p.bounds.min.y,
            z: p.bounds.min.z,
            width: p.bounds.size.x,
            depth: p.bounds.size.y,
            height: p.bounds.size.z,
            is_front: matches!(
                p.role,
                PartRole::Door | PartRole::DrawerFront | PartRole::Plinth
            ),
        })
        .collect();
    let biscuits = list
        .operations
        .iter()
        .filter(|o| matches!(o, Operation::BiscuitSlot { .. }))
        .count() as u32;
    let screws = list
        .operations
        .iter()
        .filter(|o| {
            matches!(
                o,
                Operation::Bore {
                    purpose: BorePurpose::Screw,
                    ..
                }
            )
        })
        .count() as u32;
    Ok(BuildSummary {
        panels,
        operations: list.operations.len() as u32,
        biscuits,
        screws,
        hardware: list
            .hardware
            .iter()
            .map(|h| format!("{} — {}", h.item, h.description))
            .collect(),
        warnings: list.warnings,
    })
}

#[uniffi::export]
pub fn option_groups() -> Vec<OptionGroupRecord> {
    cabinet_core::options::option_groups()
        .into_iter()
        .map(|g| OptionGroupRecord {
            field: g.field,
            title: g.title,
            choices: g
                .choices
                .into_iter()
                .map(|c| ChoiceRecord {
                    id: c.id,
                    label: c.label,
                    description: c.description,
                })
                .collect(),
        })
        .collect()
}

/// The current choice of `field` in the cabinet's standard.
#[uniffi::export]
pub fn standard_option(cabinet_json: String, field: String) -> Result<String, CoreError> {
    let v: serde_json::Value = serde_json::from_str(&cabinet_json).map_err(invalid)?;
    Ok(v["standard"][&field]
        .as_str()
        .unwrap_or_default()
        .to_string())
}

#[uniffi::export]
pub fn cut_list_csv(cabinets_json: Vec<String>) -> Result<String, CoreError> {
    let mut lists = Vec::new();
    let mut tape = 1.0;
    for j in &cabinets_json {
        let cab = parse(j)?;
        tape = cab.standard.edge_band_thickness;
        lists.push(build_cabinet(&cab).map_err(invalid)?);
    }
    Ok(to_csv(&cut_list(&lists, "Generic", tape)))
}

#[derive(uniffi::Record)]
pub struct CutRowRecord {
    pub cabinet: String,
    pub part: String,
    pub description: String,
    pub board: String,
    pub thickness: f64,
    pub length: f64,
    pub width: f64,
    pub cut_length: f64,
    pub cut_width: f64,
    /// Banded edges as text, for example "L1 L2 W1".
    pub edges: String,
    pub edge_metres: f64,
}

#[uniffi::export]
pub fn cut_list_rows(cabinets_json: Vec<String>) -> Result<Vec<CutRowRecord>, CoreError> {
    let mut lists = Vec::new();
    let mut tape = 1.0;
    for j in &cabinets_json {
        let cab = parse(j)?;
        tape = cab.standard.edge_band_thickness;
        lists.push(build_cabinet(&cab).map_err(invalid)?);
    }
    Ok(cut_list(&lists, "Generic", tape)
        .rows
        .into_iter()
        .map(|r| {
            let names = ["L1", "L2", "W1", "W2"];
            let edges = names
                .iter()
                .zip(r.edges)
                .filter(|(_, on)| *on)
                .map(|(n, _)| *n)
                .collect::<Vec<_>>()
                .join(" ");
            CutRowRecord {
                edge_metres: r.edge_metres(),
                cabinet: r.cabinet,
                part: r.part,
                description: r.description,
                board: r.board,
                thickness: r.thickness,
                length: r.length,
                width: r.width,
                cut_length: r.cut_length,
                cut_width: r.cut_width,
                edges,
            }
        })
        .collect())
}

#[uniffi::export]
pub fn cut_sheet_svg(
    cabinet_json: String,
    part_name: String,
    job: String,
) -> Result<String, CoreError> {
    let cab = parse(&cabinet_json)?;
    let list = build_cabinet(&cab).map_err(invalid)?;
    let part = list
        .part(&part_name)
        .ok_or_else(|| invalid(format!("no part {part_name}")))?;
    let board = if part.board.is_empty() {
        "Generic".to_string()
    } else {
        part.board.clone()
    };
    Ok(cabinet_sheets::part_sheet(&list, part, &board, &job))
}

/// Whether a SolidCraft app answers on `port`.
#[uniffi::export]
pub fn solidcraft_reachable(port: u16) -> bool {
    Client::connect(port)
        .and_then(|mut c| c.request("ui.inspect", serde_json::json!({})))
        .is_ok()
}

/// Builds the cabinets in the running SolidCraft. Blocking: call it off the main thread.
/// Cabinets already in the design (same component name) are replaced.
#[uniffi::export]
pub fn send_to_solidcraft(
    cabinets_json: Vec<String>,
    manufacturing: bool,
    port: u16,
) -> Result<String, CoreError> {
    let sc = |e: solidcraft_bridge::Error| CoreError::SolidCraft {
        message: e.to_string(),
    };
    let mut client = Client::connect(port).map_err(sc)?;
    let detail = if manufacturing {
        Detail::Manufacturing
    } else {
        Detail::Design
    };
    let mut report = Vec::new();
    for j in &cabinets_json {
        let cab = parse(j)?;
        let list = build_cabinet(&cab).map_err(invalid)?;
        // Replace an earlier build of this cabinet; a missing component is fine.
        let _ = client.execute(
            "component.delete",
            serde_json::json!({ "component": cab.id }),
        );
        let spec = serde_json::to_value(&cab).map_err(invalid)?;
        let cmds = emit_cabinet(&list, detail, Some(&spec));
        let r = client.batch(&cmds).map_err(sc)?;
        report.push(format!(
            "{}: {} commands in {:.1} s",
            cab.id,
            cmds.len(),
            r["total_ms"].as_f64().unwrap_or(0.0) / 1000.0
        ));
    }
    Ok(report.join("\n"))
}

#[uniffi::export]
pub fn load_boards(catalog_dir: String) -> Result<Vec<BoardRecord>, CoreError> {
    let cat =
        cabinet_catalog::Catalog::load(std::path::Path::new(&catalog_dir)).map_err(invalid)?;
    Ok(cat
        .boards
        .into_iter()
        .map(|b| BoardRecord {
            sheets: b
                .sheets
                .iter()
                .map(|s| format!("{:.0} × {:.0}", s.length, s.width))
                .collect(),
            substrate: serde_json::to_value(b.substrate)
                .ok()
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_default(),
            verified: b.source.verified,
            id: b.id,
            brand: b.brand,
            range: b.range,
            decor: b.decor,
            finish: b.finish,
            thickness: b.thickness,
            preview_colour: b.preview_colour,
            grained: b.grained,
        })
        .collect())
}
