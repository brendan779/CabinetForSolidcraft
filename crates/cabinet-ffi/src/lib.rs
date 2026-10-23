//! The macOS app's view of the Rust core. Cabinets cross the boundary as JSON (the same JSON
//! stored in SolidCraft designs), results as small typed records the SwiftUI views draw directly.

use cabinet_core::cutlist::{cut_list, to_csv};
use cabinet_core::model::BoardSlot;
use cabinet_core::project::Piece;
use cabinet_core::{
    Cabinet, CabinetKind, ConstructionStandard, PartList, Project, Run, RunItem, build_cabinet,
    build_run, laid_out,
};
use solidcraft_bridge::boxes::{cuboid, selected_bodies};
use solidcraft_bridge::{Client, Detail, find_existing, replace_cabinet};

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
    /// A door's hinged edge, `left` or `right`; empty for other parts.
    pub hinged: String,
    /// Catalog board id, or empty for the standard's generic board.
    pub board: String,
    /// The axis the decor's grain runs along in the cabinet frame (`x`, `y`, `z`), or empty.
    pub grain: String,
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

/// A job's items in the sidebar's order: cabinets (cabinet JSON) and the fillers and end panels
/// between them (run item JSON, `type` `filler` or `end_panel`). Cabinets named in `placed` keep
/// their own position; everything else is one run along X from the origin.
#[derive(uniffi::Record)]
pub struct JobItems {
    pub items_json: Vec<String>,
    pub placed: Vec<String>,
}

/// A cabinet, or a filler or end panel.
fn parse_item(json: &str) -> Result<RunItem, CoreError> {
    let v: serde_json::Value = serde_json::from_str(json).map_err(invalid)?;
    match v.get("type").and_then(|t| t.as_str()) {
        Some("filler" | "end_panel") => serde_json::from_value(v).map_err(invalid),
        _ => Ok(RunItem::Cabinet(Box::new(
            serde_json::from_value(v).map_err(invalid)?,
        ))),
    }
}

/// The id a run item's parts are named after.
fn item_id(item: &RunItem) -> &str {
    match item {
        RunItem::Cabinet(c) => &c.id,
        RunItem::Filler(f) => &f.id,
        RunItem::EndPanel(e) => &e.id,
    }
}

/// Every item built where the job puts it, in order, each with the cabinet it was built from (none
/// for fillers and end panels).
fn job_lists(job: &JobItems) -> Result<Vec<(PartList, Option<Cabinet>)>, CoreError> {
    let items = job
        .items_json
        .iter()
        .map(|j| parse_item(j))
        .collect::<Result<Vec<_>, _>>()?;
    let in_run: Vec<bool> = items
        .iter()
        .map(|i| !job.placed.iter().any(|p| p == item_id(i)))
        .collect();
    let run = laid_out(&Run {
        origin: Default::default(),
        items: items
            .iter()
            .zip(&in_run)
            .filter(|(_, r)| **r)
            .map(|(i, _)| i.clone())
            .collect(),
    });
    let mut built = if run.items.is_empty() {
        Vec::new()
    } else {
        build_run(&run).map_err(invalid)?
    }
    .into_iter()
    .zip(run.items);
    items
        .into_iter()
        .zip(in_run)
        .map(|(item, r)| {
            let (list, item) = if r {
                built.next().expect("one list per run item")
            } else {
                let RunItem::Cabinet(c) = &item else {
                    return Err(invalid("only cabinets can be placed"));
                };
                (build_cabinet(c).map_err(invalid)?, item)
            };
            let cab = match item {
                RunItem::Cabinet(c) => Some(*c),
                _ => None,
            };
            Ok((list, cab))
        })
        .collect()
}

/// One item's build, or why it does not build.
#[derive(uniffi::Record)]
pub struct ItemBuild {
    pub summary: Option<BuildSummary>,
    pub error: Option<String>,
}

/// Builds every item of the job. An item that does not build on its own reports its error and
/// stays out of the run, so the rest still lay out.
#[uniffi::export]
pub fn build_job(job: JobItems) -> Vec<ItemBuild> {
    let errors: Vec<Option<String>> = job
        .items_json
        .iter()
        .map(|j| match parse_item(j) {
            Ok(RunItem::Cabinet(c)) => build_cabinet(&c).err().map(|e| e.to_string()),
            Ok(_) => None,
            Err(e) => Some(e.to_string()),
        })
        .collect();
    let good = JobItems {
        items_json: job
            .items_json
            .iter()
            .zip(&errors)
            .filter(|(_, e)| e.is_none())
            .map(|(j, _)| j.clone())
            .collect(),
        placed: job.placed.clone(),
    };
    let mut lists = match job_lists(&good) {
        Ok(l) => l.into_iter().map(|(l, _)| Ok(l)).collect::<Vec<_>>(),
        Err(e) => vec![Err(e.to_string()); good.items_json.len()],
    }
    .into_iter();
    errors
        .into_iter()
        .map(|e| match e {
            Some(error) => ItemBuild {
                summary: None,
                error: Some(error),
            },
            None => match lists.next().expect("one list per good item") {
                Ok(list) => ItemBuild {
                    summary: Some(summary(list)),
                    error: None,
                },
                Err(error) => ItemBuild {
                    summary: None,
                    error: Some(error),
                },
            },
        })
        .collect()
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

/// Uses catalog board `board_id` for the cabinet's `slot` (carcass, front, back); the slot's
/// thickness becomes the board's.
#[uniffi::export]
pub fn assign_board(
    cabinet_json: String,
    slot: String,
    board_id: String,
    catalog_dir: String,
) -> Result<String, CoreError> {
    let mut cab = parse(&cabinet_json)?;
    let slot: BoardSlot =
        serde_json::from_value(serde_json::Value::String(slot)).map_err(invalid)?;
    let cat =
        cabinet_catalog::Catalog::load(std::path::Path::new(&catalog_dir)).map_err(invalid)?;
    let board = cat
        .board(&board_id)
        .ok_or_else(|| invalid(format!("no board {board_id} in the catalog")))?;
    cab.standard.set_board(slot, &board.id, board.thickness);
    serde_json::to_string(&cab).map_err(invalid)
}

/// A job as the app holds it: the project standard and every cabinet with its standard resolved.
#[derive(uniffi::Record)]
pub struct ProjectRecord {
    pub name: String,
    pub standard_json: String,
    /// Cabinets (cabinet JSON) and fillers and end panels (run item JSON), in the job's order.
    pub items_json: Vec<String>,
    /// Ids of cabinets kept where they were drawn in SolidCraft.
    pub placed: Vec<String>,
}

/// The job file: `standard_json` once, each cabinet with only the fields it overrides.
#[uniffi::export]
pub fn project_json(
    name: String,
    standard_json: String,
    items_json: Vec<String>,
    placed: Vec<String>,
) -> Result<String, CoreError> {
    let standard: ConstructionStandard = serde_json::from_str(&standard_json).map_err(invalid)?;
    let mut cabinets = Vec::new();
    let mut pieces = Vec::new();
    for (index, j) in items_json.iter().enumerate() {
        match parse_item(j)? {
            RunItem::Cabinet(c) => cabinets.push(*c),
            item => pieces.push(Piece { index, item }),
        }
    }
    let mut p = Project::new(name, standard, &cabinets).map_err(invalid)?;
    p.placed = placed;
    p.pieces = pieces;
    serde_json::to_string_pretty(&p).map_err(invalid)
}

/// Reads a job file written by [`project_json`].
#[uniffi::export]
pub fn open_project(json: String) -> Result<ProjectRecord, CoreError> {
    let p: Project = serde_json::from_str(&json).map_err(invalid)?;
    let mut items_json: Vec<String> = p
        .resolved()
        .map_err(invalid)?
        .iter()
        .map(|c| serde_json::to_string(c).map_err(invalid))
        .collect::<Result<_, _>>()?;
    for piece in &p.pieces {
        let at = piece.index.min(items_json.len());
        items_json.insert(at, serde_json::to_string(&piece.item).map_err(invalid)?);
    }
    Ok(ProjectRecord {
        name: p.name,
        standard_json: serde_json::to_string(&p.standard).map_err(invalid)?,
        items_json,
        placed: p.placed,
    })
}

#[uniffi::export]
pub fn build(cabinet_json: String) -> Result<BuildSummary, CoreError> {
    let cab = parse(&cabinet_json)?;
    Ok(summary(build_cabinet(&cab).map_err(invalid)?))
}

fn summary(list: PartList) -> BuildSummary {
    use cabinet_core::part::{BorePurpose, Operation};
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
            is_front: p.role.is_front(),
            hinged: list
                .hardware
                .iter()
                .find(|h| h.part == p.name && h.item.starts_with("blum.clip-top"))
                .map(|h| {
                    if h.at.x < p.bounds.center().x {
                        "left"
                    } else {
                        "right"
                    }
                })
                .unwrap_or_default()
                .to_string(),
            board: p.board.clone(),
            grain: p
                .grain_axis
                .map(|a| format!("{a:?}").to_lowercase())
                .unwrap_or_default(),
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
    BuildSummary {
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
    }
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
pub fn cut_list_csv(job: JobItems) -> Result<String, CoreError> {
    let (lists, tape) = cut_lists(&job)?;
    Ok(to_csv(&cut_list(&lists, "Generic", tape)))
}

/// Every item's part list, and the edge tape thickness of the job's cabinets.
fn cut_lists(job: &JobItems) -> Result<(Vec<PartList>, f64), CoreError> {
    let mut tape = 1.0;
    let lists = job_lists(job)?
        .into_iter()
        .map(|(l, c)| {
            if let Some(c) = c {
                tape = c.standard.edge_band_thickness;
            }
            l
        })
        .collect();
    Ok((lists, tape))
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
pub fn cut_list_rows(job: JobItems) -> Result<Vec<CutRowRecord>, CoreError> {
    let (lists, tape) = cut_lists(&job)?;
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
pub fn cut_sheet_svg(items: JobItems, part_name: String, job: String) -> Result<String, CoreError> {
    let list = job_lists(&items)?
        .into_iter()
        .map(|(l, _)| l)
        .find(|l| l.part(&part_name).is_some())
        .ok_or_else(|| invalid(format!("no part {part_name}")))?;
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

/// Every part's cut sheet in one PDF, after a cover and an index.
#[uniffi::export]
pub fn cut_sheets_pdf(items: JobItems, job: String) -> Result<Vec<u8>, CoreError> {
    let lists: Vec<PartList> = job_lists(&items)?.into_iter().map(|(l, _)| l).collect();
    Ok(cabinet_sheets::pdf::job_pdf(
        &lists,
        |p| {
            if p.board.is_empty() {
                "Generic".to_string()
            } else {
                p.board.clone()
            }
        },
        &job,
    ))
}

/// One line of the hardware bill.
#[derive(uniffi::Record)]
pub struct BomRecord {
    pub item: String,
    pub brand: String,
    /// The supplier's article number from the hardware catalog, or "not catalogued".
    pub article: String,
    pub description: String,
    pub quantity: u32,
    pub cabinets: String,
}

fn bom_records(items: &JobItems, catalog_dir: &str) -> Result<Vec<BomRecord>, CoreError> {
    use cabinet_core::bom::{NOT_CATALOGUED, hardware_bom};
    let lists: Vec<PartList> = job_lists(items)?.into_iter().map(|(l, _)| l).collect();
    let cat = cabinet_catalog::Catalog::load(std::path::Path::new(catalog_dir)).map_err(invalid)?;
    Ok(hardware_bom(&lists)
        .into_iter()
        .map(|r| {
            let found = cat.hardware_item(&r.item);
            BomRecord {
                brand: found.map(|h| h.brand.clone()).unwrap_or_default(),
                article: found
                    .map(|h| h.article.clone())
                    .unwrap_or_else(|| NOT_CATALOGUED.into()),
                item: r.item,
                description: r.description,
                quantity: r.quantity,
                cabinets: r.cabinets.join(" "),
            }
        })
        .collect())
}

/// The job's hardware, one row per item, with article numbers from the catalog at `catalog_dir`.
#[uniffi::export]
pub fn hardware_bom(items: JobItems, catalog_dir: String) -> Result<Vec<BomRecord>, CoreError> {
    bom_records(&items, &catalog_dir)
}

#[uniffi::export]
pub fn hardware_bom_csv(items: JobItems, catalog_dir: String) -> Result<String, CoreError> {
    let lists: Vec<PartList> = job_lists(&items)?.into_iter().map(|(l, _)| l).collect();
    let cat =
        cabinet_catalog::Catalog::load(std::path::Path::new(&catalog_dir)).map_err(invalid)?;
    Ok(cabinet_core::bom::bom_csv(
        &cabinet_core::bom::hardware_bom(&lists),
        |id| {
            cat.hardware_item(id)
                .map(|h| (h.brand.clone(), h.article.clone()))
        },
    ))
}

#[uniffi::export]
pub fn hardware_bom_pdf(
    items: JobItems,
    catalog_dir: String,
    job: String,
) -> Result<Vec<u8>, CoreError> {
    let rows: Vec<Vec<String>> = bom_records(&items, &catalog_dir)?
        .into_iter()
        .map(|r| {
            vec![
                format!("{}#", r.quantity),
                r.description,
                r.brand,
                r.article,
                r.item,
                r.cabinets,
            ]
        })
        .collect();
    Ok(cabinet_sheets::pdf::table_pdf(
        "Hardware",
        &job,
        &[
            "Qty#",
            "Description",
            "Brand",
            "Article",
            "Item",
            "Cabinets",
        ],
        &[14.0, 92.0, 22.0, 30.0, 70.0, 39.0],
        &rows,
    ))
}

/// Whether a SolidCraft app answers on `port`.
#[uniffi::export]
pub fn solidcraft_reachable(port: u16) -> bool {
    Client::connect(port)
        .and_then(|mut c| c.request("ui.inspect", serde_json::json!({})))
        .is_ok()
}

/// Builds the cabinets in the running SolidCraft. Blocking: call it off the main thread.
/// Cabinets already in the design (same component name) are replaced in place.
#[uniffi::export]
pub fn send_to_solidcraft(job: JobItems, detail: String, port: u16) -> Result<String, CoreError> {
    let sc = |e: solidcraft_bridge::Error| CoreError::SolidCraft {
        message: e.to_string(),
    };
    let detail = match detail.as_str() {
        "design" => Detail::Design,
        "manufacturing" => Detail::Manufacturing,
        "exact" => Detail::Exact,
        other => return Err(invalid(format!("unknown detail level {other}"))),
    };
    let mut client = Client::connect(port).map_err(sc)?;
    let mut report = Vec::new();
    for (list, cab) in job_lists(&job)? {
        // One batch replaces an earlier build of this item and keeps it where the user moved it
        // in SolidCraft. Only cabinets carry a spec to load back (SC-3).
        let existing = find_existing(&client.components().map_err(sc)?, &list.cabinet);
        let spec = cab
            .map(|c| serde_json::to_value(&c).map_err(invalid))
            .transpose()?;
        let cmds = replace_cabinet(&list, detail, spec.as_ref(), existing.as_ref());
        let r = client.batch(&cmds).map_err(sc)?;
        report.push(format!(
            "{}: {} commands in {:.1} s",
            list.cabinet,
            cmds.len(),
            r["total_ms"].as_f64().unwrap_or(0.0) / 1000.0
        ));
    }
    Ok(report.join("\n"))
}

/// A cabinet made from a box drawn in SolidCraft.
#[derive(uniffi::Record)]
pub struct DrawnCabinet {
    pub id: String,
    pub json: String,
    /// The box body it was made from.
    pub source: String,
}

#[derive(uniffi::Record)]
pub struct BoxImport {
    pub cabinets: Vec<DrawnCabinet>,
    /// Selected bodies that were not made into cabinets, with the reason.
    pub skipped: Vec<String>,
}

/// Turns each box selected in SolidCraft into a cabinet of its size and position. Ids follow the
/// kind's letter and skip `taken_ids`. Blocking: call it off the main thread.
#[uniffi::export]
pub fn cabinets_from_selection(port: u16, taken_ids: Vec<String>) -> Result<BoxImport, CoreError> {
    let sc = |e: solidcraft_bridge::Error| CoreError::SolidCraft {
        message: e.to_string(),
    };
    let mut client = Client::connect(port).map_err(sc)?;
    let bodies = selected_bodies(&client.inspect().map_err(sc)?);
    let mut taken = taken_ids;
    let mut out = BoxImport {
        cabinets: Vec::new(),
        skipped: Vec::new(),
    };
    for body in bodies {
        let bounds = match cuboid(&client.faces(&body).map_err(sc)?) {
            Ok(b) => b,
            Err(why) => {
                out.skipped.push(format!("{body} {why}"));
                continue;
            }
        };
        let kind = CabinetKind::for_box(&bounds);
        let id = next_id(kind, &taken);
        let cab = Cabinet::from_box(id.clone(), bounds);
        if let Err(e) = build_cabinet(&cab) {
            out.skipped.push(format!("{body}: {e}"));
            continue;
        }
        taken.push(id.clone());
        out.cabinets.push(DrawnCabinet {
            id,
            json: serde_json::to_string(&cab).map_err(invalid)?,
            source: body,
        });
    }
    Ok(out)
}

/// `B01`, `B02`... for base cabinets, the first one not taken. Same scheme as the app's Add menu.
fn next_id(kind: CabinetKind, taken: &[String]) -> String {
    let letter = serde_json::to_value(kind)
        .ok()
        .and_then(|v| v.as_str().and_then(|s| s.chars().next()))
        .unwrap_or('C')
        .to_ascii_uppercase();
    (1..)
        .map(|n| format!("{letter}{n:02}"))
        .find(|id| !taken.contains(id))
        .unwrap()
}

/// The cabinets built into the design open in SolidCraft, as cabinet JSON. Components whose stored
/// spec this version cannot read are left out. Blocking.
#[uniffi::export]
pub fn cabinets_in_design(port: u16) -> Result<Vec<String>, CoreError> {
    let sc = |e: solidcraft_bridge::Error| CoreError::SolidCraft {
        message: e.to_string(),
    };
    let mut client = Client::connect(port).map_err(sc)?;
    Ok(solidcraft_bridge::design::cabinets_in_design(&mut client)
        .map_err(sc)?
        .into_iter()
        .filter_map(|v| serde_json::from_value::<Cabinet>(v).ok())
        .filter_map(|c| serde_json::to_string(&c).ok())
        .collect())
}

/// Hides bodies in SolidCraft (the boxes cabinets were made from). Blocking.
#[uniffi::export]
pub fn hide_bodies(port: u16, bodies: Vec<String>) -> Result<(), CoreError> {
    let sc = |e: solidcraft_bridge::Error| CoreError::SolidCraft {
        message: e.to_string(),
    };
    let mut client = Client::connect(port).map_err(sc)?;
    client
        .execute(
            "browser.visibility",
            serde_json::json!({ "visible": false, "items": bodies }),
        )
        .map_err(sc)?;
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    fn catalog() -> String {
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../catalog").to_string()
    }

    fn door_thickness(json: &str) -> f64 {
        let list = build_cabinet(&parse(json).unwrap()).unwrap();
        list.part("B01.Door1").unwrap().thickness()
    }

    #[test]
    fn changing_the_front_board_changes_the_door_thickness() {
        let json = new_cabinet("B01".into(), "base".into()).unwrap();
        let sixteen = assign_board(
            json.clone(),
            "front".into(),
            "polytec.decorative.classic-white-matt.16.mr-mdf".into(),
            catalog(),
        )
        .unwrap();
        assert_eq!(door_thickness(&sixteen), 16.0);
        let eighteen = assign_board(
            sixteen,
            "front".into(),
            "polytec.decorative.classic-white-matt.18.mr-mdf".into(),
            catalog(),
        )
        .unwrap();
        assert_eq!(door_thickness(&eighteen), 18.0);
        let cab = parse(&eighteen).unwrap();
        assert_eq!(
            cab.standard.front_board,
            "polytec.decorative.classic-white-matt.18.mr-mdf"
        );
    }

    #[test]
    fn an_unknown_board_is_refused() {
        let json = new_cabinet("B01".into(), "base".into()).unwrap();
        assert!(assign_board(json, "front".into(), "nope".into(), catalog()).is_err());
    }

    #[test]
    fn drawn_cabinets_take_the_next_free_id_of_their_kind() {
        let taken = vec!["B01".to_string(), "B02".to_string(), "T01".to_string()];
        assert_eq!(next_id(CabinetKind::Base, &taken), "B03");
        assert_eq!(next_id(CabinetKind::Tall, &taken), "T02");
        assert_eq!(next_id(CabinetKind::Wall, &taken), "W01");
    }

    #[test]
    fn a_job_file_round_trips_its_cabinets_standard_and_placed_ids() {
        let base = new_cabinet("B01".into(), "base".into()).unwrap();
        let wall = set_standard_option(
            new_cabinet("W01".into(), "wall".into()).unwrap(),
            "back".into(),
            "applied".into(),
        )
        .unwrap();
        let standard = serde_json::to_string(&parse(&base).unwrap().standard).unwrap();
        let file = project_json(
            "Kitchen".into(),
            standard.clone(),
            vec![base.clone(), wall.clone()],
            vec!["W01".into()],
        )
        .unwrap();
        let v: serde_json::Value = serde_json::from_str(&file).unwrap();
        assert_eq!(
            v["cabinets"][1]["overrides"],
            serde_json::json!({ "back": "applied" })
        );
        let p = open_project(file).unwrap();
        assert_eq!(p.name, "Kitchen");
        assert_eq!(p.standard_json, standard);
        assert_eq!(p.placed, vec!["W01".to_string()]);
        let cabs: Vec<Cabinet> = p.items_json.iter().map(|c| parse(c).unwrap()).collect();
        assert_eq!(cabs, vec![parse(&base).unwrap(), parse(&wall).unwrap()]);
    }

    fn job(items: &[String]) -> JobItems {
        JobItems {
            items_json: items.to_vec(),
            placed: Vec::new(),
        }
    }

    #[test]
    fn a_job_with_an_end_panel_each_side_lays_out_and_lists_every_piece() {
        let end = |id: &str| format!(r#"{{"type":"end_panel","id":"{id}","scribe":20}}"#);
        let items = vec![
            end("E01"),
            new_cabinet("B01".into(), "base".into()).unwrap(),
            r#"{"type":"filler","id":"F01","width":50}"#.to_string(),
            new_cabinet("B02".into(), "base".into()).unwrap(),
            end("E02"),
        ];
        let built = build_job(job(&items));
        assert_eq!(built.len(), 5);
        assert!(built.iter().all(|b| b.error.is_none()));
        let left_x = |i: usize| {
            built[i]
                .summary
                .as_ref()
                .unwrap()
                .panels
                .iter()
                .map(|p| p.x)
                .fold(f64::MAX, f64::min)
        };
        let ft = parse(&items[1]).unwrap().standard.front_thickness;
        assert_eq!(left_x(1), ft, "B01 after the end panel");
        assert_eq!(left_x(3), ft + 600.0 + 50.0, "B02 after the filler");
        let rows = cut_list_rows(job(&items)).unwrap();
        for part in ["E01.EndPanel", "F01.Filler", "E02.EndPanel"] {
            assert!(
                rows.iter().any(|r| r.part == part),
                "{part} in the cut list"
            );
        }
        assert!(cut_sheet_svg(job(&items), "F01.Filler".into(), "Job".into()).is_ok());
        // The job file keeps the pieces where they were.
        let standard = serde_json::to_string(&parse(&items[1]).unwrap().standard).unwrap();
        let file = project_json("Kitchen".into(), standard, items.clone(), Vec::new()).unwrap();
        let back = open_project(file).unwrap();
        let ids: Vec<String> = back
            .items_json
            .iter()
            .map(|j| item_id(&parse_item(j).unwrap()).to_string())
            .collect();
        assert_eq!(ids, ["E01", "B01", "F01", "B02", "E02"]);
    }

    #[test]
    fn a_cabinet_that_does_not_build_reports_its_error_and_the_rest_still_build() {
        let mut bad: serde_json::Value =
            serde_json::from_str(&new_cabinet("B02".into(), "base".into()).unwrap()).unwrap();
        bad["width"] = serde_json::json!(10.0);
        let items = vec![
            new_cabinet("B01".into(), "base".into()).unwrap(),
            bad.to_string(),
        ];
        let built = build_job(job(&items));
        assert!(built[0].summary.is_some());
        assert!(built[1].error.is_some());
    }

    #[test]
    fn the_sample_run_has_a_hardware_bill_as_csv_and_pdf() {
        let items: Vec<String> = [("B01", "base"), ("D01", "drawer_bank"), ("T01", "tall")]
            .iter()
            .map(|(id, kind)| new_cabinet(id.to_string(), kind.to_string()).unwrap())
            .collect();
        let rows = hardware_bom(job(&items), catalog()).unwrap();
        assert!(
            rows.iter()
                .any(|r| r.item.starts_with("blum.clip-top") && r.cabinets.contains("T01"))
        );
        let csv = hardware_bom_csv(job(&items), catalog()).unwrap();
        assert_eq!(csv.lines().count(), rows.len() + 1);
        let pdf = hardware_bom_pdf(job(&items), catalog(), "Sample".into()).unwrap();
        assert!(pdf.starts_with(b"%PDF"));
    }
}
