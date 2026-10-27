//! The macOS app's view of the Rust core. Cabinets cross the boundary as JSON (the same JSON
//! stored in SolidCraft designs), results as small typed records the SwiftUI views draw directly.

use cabinet_core::cad::cad_placements;
use cabinet_core::cutlist::{cut_list, to_csv};
use cabinet_core::model::BoardSlot;
use cabinet_core::project::Piece;
use cabinet_core::{
    Cabinet, CabinetKind, ConstructionStandard, PartList, Project, Run, RunItem, build_cabinet,
    build_run, laid_out,
};
use solidcraft_bridge::boxes::{cuboid, selected_bodies};
use solidcraft_bridge::{Client, Detail, find_existing, hardware_inserts, replace_cabinet, vendor};
use std::path::Path;

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
    /// A door's hinged edge, `left` or `right`; empty for other parts. A shaker door's frame
    /// members and panel all carry their door's.
    pub hinged: String,
    /// The hinge line (x, y at the front face) the door swings about, when `hinged` is set.
    pub hinge_x: f64,
    pub hinge_y: f64,
    /// Catalog board id, or empty for the standard's generic board.
    pub board: String,
    /// The axis the decor's grain runs along in the cabinet frame (`x`, `y`, `z`), or empty.
    pub grain: String,
    /// A toe-kick notch cut through the front-bottom corner (a side running to the floor): how
    /// deep from the front and how high from the bottom. Zero without one.
    pub notch_depth: f64,
    pub notch_height: f64,
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
    /// `melamine`, `lining`, `raw`... as the catalog names it.
    pub kind: String,
    /// A lining's routed profile: its shape (`batten`, `scallop`...), module and groove (mm);
    /// empty and 0 for other boards.
    pub profile_shape: String,
    pub profile_module: f64,
    pub profile_groove: f64,
}

/// Handles as the viewport draws them (mm): a bar this square, standing this far off the front,
/// running this far past its end holes.
const HANDLE_BAR: f64 = 12.0;
const HANDLE_STANDOFF: f64 = 30.0;
const HANDLE_END: f64 = 15.0;

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
        Some("filler" | "end_panel" | "benchtop" | "floating_shelf" | "moulding") => {
            serde_json::from_value(v).map_err(invalid)
        }
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
        RunItem::Benchtop(b) => &b.id,
        RunItem::FloatingShelf(f) => &f.id,
        RunItem::Moulding(m) => &m.id,
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
    // Lift systems weigh fronts by their core (Blum's reference densities).
    if slot == BoardSlot::Front {
        use cabinet_catalog::Substrate;
        match board.substrate {
            Substrate::Mdf | Substrate::MoistureResistantMdf => {
                cab.standard.front_core = cabinet_core::model::FrontCore::Mdf
            }
            Substrate::Particleboard | Substrate::MoistureResistantParticleboard => {
                cab.standard.front_core = cabinet_core::model::FrontCore::Chipboard
            }
            _ => {}
        }
    }
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
    /// The job's sink and cooktop templates (a JSON array of `Appliance`); our starting library
    /// when the file has none.
    pub appliances_json: String,
}

/// The job file: `standard_json` once, each cabinet with only the fields it overrides.
#[uniffi::export]
pub fn project_json(
    name: String,
    standard_json: String,
    items_json: Vec<String>,
    placed: Vec<String>,
    appliances_json: String,
) -> Result<String, CoreError> {
    let appliances: Vec<cabinet_core::run::Appliance> = if appliances_json.trim().is_empty() {
        Vec::new()
    } else {
        serde_json::from_str(&appliances_json).map_err(invalid)?
    };
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
    p.appliances = appliances;
    serde_json::to_string_pretty(&p).map_err(invalid)
}

/// Reads a job file written by [`project_json`].
#[uniffi::export]
pub fn open_project(json: String) -> Result<ProjectRecord, CoreError> {
    let p: Project = serde_json::from_str(&json).map_err(invalid)?;
    let default_appliances = cabinet_core::run::Appliance::defaults();
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
        appliances_json: serde_json::to_string(if p.appliances.is_empty() {
            &default_appliances
        } else {
            &p.appliances
        })
        .map_err(invalid)?,
    })
}

/// The benchtop materials in the catalog, one per family, for the material picker (BENCH-7):
/// id (`natural-stone`), name, and the thicknesses it has rules for.
#[uniffi::export]
pub fn benchtop_materials() -> Vec<ChoiceRecord> {
    let mut out: Vec<ChoiceRecord> = Vec::new();
    for i in cabinet_core::hardware::items()
        .iter()
        .filter(|i| i.id.starts_with("benchtop-material."))
    {
        let family = i.id.split('.').nth(1).unwrap_or_default().to_string();
        match out.iter_mut().find(|c| c.id == family) {
            Some(c) => c.description = format!("{}, {}", c.description, i.variant),
            None => out.push(ChoiceRecord {
                id: family,
                label: i.family.clone(),
                description: i.variant.clone(),
            }),
        }
    }
    out
}

/// One of a benchtop's material rules as it applies (BENCH-7): the catalog's value, or the
/// benchtop's own where it changed it.
#[derive(uniffi::Record)]
pub struct MaterialRuleRecord {
    /// The parameter (`unsupported_overhang`).
    pub field: String,
    pub label: String,
    pub value: f64,
    /// The material's own value, which an override equal to it gives back.
    pub material_value: f64,
    pub overridden: bool,
}

/// The rules for a benchtop item (its JSON), and which of them it overrides; empty for an
/// unknown material.
#[uniffi::export]
pub fn benchtop_rules(item_json: String) -> Result<Vec<MaterialRuleRecord>, CoreError> {
    let RunItem::Benchtop(b) = parse_item(&item_json)? else {
        return Err(invalid("not a benchtop"));
    };
    let Ok(r) = b.rules() else {
        return Ok(Vec::new());
    };
    let mut plain = b.clone();
    plain.material_rules.clear();
    let Ok(m) = plain.rules() else {
        return Ok(Vec::new());
    };
    let rows = [
        (
            "unsupported_overhang",
            "Unsupported overhang",
            r.unsupported_overhang,
            m.unsupported_overhang,
        ),
        (
            "substrate_overhang",
            "With a substrate to",
            r.substrate_overhang,
            m.substrate_overhang,
        ),
        (
            "support_spacing",
            "Supports apart",
            r.support_spacing,
            m.support_spacing,
        ),
        (
            "front_clearance",
            "Cut-out from front",
            r.front_clearance,
            m.front_clearance,
        ),
        (
            "rear_clearance",
            "Cut-out from rear",
            r.rear_clearance,
            m.rear_clearance,
        ),
        (
            "tap_clearance",
            "Sink to tap hole",
            r.tap_clearance,
            m.tap_clearance,
        ),
        (
            "electrical_edge",
            "Power point from edge",
            r.electrical_edge,
            m.electrical_edge,
        ),
    ];
    Ok(rows
        .into_iter()
        .map(|(field, label, value, material_value)| MaterialRuleRecord {
            field: field.into(),
            label: label.into(),
            value,
            material_value,
            overridden: b.material_rules.contains_key(field),
        })
        .collect())
}

/// The power point plates in the catalog, for the splashback's plate picker (BENCH-8): id,
/// name, and the plate and cut-out sizes.
#[uniffi::export]
pub fn power_point_plates() -> Vec<ChoiceRecord> {
    cabinet_core::hardware::items()
        .iter()
        .filter(|i| i.param("cutout_width").is_ok() && i.param("plate_width").is_ok())
        .map(|i| {
            let p = |n: &str| i.param(n).unwrap_or_default();
            ChoiceRecord {
                id: i.id.clone(),
                label: format!("{} {}", i.family, i.article),
                description: format!(
                    "{}; plate {:.0} × {:.0}, cut-out {:.0} × {:.0}",
                    i.variant,
                    p("plate_width"),
                    p("plate_height"),
                    p("cutout_width"),
                    p("cutout_height")
                ),
            }
        })
        .collect()
}

/// Our starting sink and cooktop templates, as a JSON array of `Appliance`.
#[uniffi::export]
pub fn default_appliances_json() -> String {
    serde_json::to_string(&cabinet_core::run::Appliance::defaults()).unwrap_or_else(|_| "[]".into())
}

#[uniffi::export]
pub fn build(cabinet_json: String) -> Result<BuildSummary, CoreError> {
    let cab = parse(&cabinet_json)?;
    Ok(summary(build_cabinet(&cab).map_err(invalid)?))
}

fn summary(list: PartList) -> BuildSummary {
    use cabinet_core::part::{BorePurpose, Operation};
    // A door, or a shaker door's members (`B01.Door1Stile1`...), by the door's name.
    let door_of = |p: &cabinet_core::Part| -> String {
        use cabinet_core::part::PartRole;
        match p.role {
            PartRole::FrameStile | PartRole::FrameRail | PartRole::FramePanel => p
                .name
                .trim_end_matches(char::is_numeric)
                .trim_end_matches(p.role.code())
                .to_string(),
            _ => p.name.clone(),
        }
    };
    let hinge = |door: &str| -> Option<(String, f64, f64)> {
        let members: Vec<&cabinet_core::Part> =
            list.parts.iter().filter(|q| door_of(q) == door).collect();
        let h = list.hardware.iter().find(|h| {
            h.item.starts_with("blum.clip-top") && members.iter().any(|m| m.name == h.part)
        })?;
        let lo = members
            .iter()
            .map(|m| m.bounds.min.x)
            .fold(f64::MAX, f64::min);
        let hi = members
            .iter()
            .map(|m| m.bounds.max().x)
            .fold(f64::MIN, f64::max);
        let front = members
            .iter()
            .map(|m| m.bounds.min.y)
            .fold(f64::MAX, f64::min);
        Some(if h.at.x < (lo + hi) / 2.0 {
            ("left".into(), lo, front)
        } else {
            ("right".into(), hi, front)
        })
    };
    // A toe-kick notch cut right through a part's front-bottom corner, as (back y, top z).
    let notch = |p: &cabinet_core::Part| {
        list.operations.iter().find_map(|o| match o {
            Operation::Groove { part, bounds }
                if *part == p.name
                    && bounds.size.x >= p.bounds.size.x - 1e-6
                    && (bounds.min.y - p.bounds.min.y).abs() < 1e-6
                    && (bounds.min.z - p.bounds.min.z).abs() < 1e-6
                    && bounds.max().z < p.bounds.max().z
                    && bounds.max().y < p.bounds.max().y =>
            {
                Some((bounds.max().y, bounds.max().z))
            }
            _ => None,
        })
    };
    let mut panels: Vec<PanelBox> = list
        .parts
        .iter()
        .map(|p| {
            let (hinged, hinge_x, hinge_y) = hinge(&door_of(p)).unwrap_or_default();
            let (notch_depth, notch_height) = notch(p)
                .map(|(y, z)| (y - p.bounds.min.y, z - p.bounds.min.z))
                .unwrap_or_default();
            PanelBox {
                name: p.name.clone(),
                role: p.role.label().to_string(),
                x: p.bounds.min.x,
                y: p.bounds.min.y,
                z: p.bounds.min.z,
                width: p.bounds.size.x,
                depth: p.bounds.size.y,
                height: p.bounds.size.z,
                is_front: p.role.is_front(),
                hinged,
                hinge_x,
                hinge_y,
                board: p.board.clone(),
                grain: p
                    .grain_axis
                    .map(|a| format!("{a:?}").to_lowercase())
                    .unwrap_or_default(),
                notch_depth,
                notch_height,
            }
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
    // Handles, from their screw holes: a bar standing off the front face (a knob a square),
    // swinging or sliding with their front.
    let finish = list
        .hardware
        .iter()
        .find(|h| h.item.starts_with("handle."))
        .map(|h| {
            h.description
                .rsplit(", ")
                .next()
                .unwrap_or_default()
                .to_string()
        })
        .unwrap_or_default();
    for p in &list.parts {
        let holes: Vec<_> = list
            .operations
            .iter()
            .filter_map(|o| match o {
                Operation::Bore {
                    part,
                    at,
                    direction,
                    depth,
                    purpose: BorePurpose::HandleScrew,
                    ..
                } if *part == p.name => Some((*at, *direction, *depth)),
                _ => None,
            })
            .collect();
        let Some(&(first, dir, depth)) = holes.first() else {
            continue;
        };
        let last = holes.last().unwrap().0;
        let sign = if dir.positive { 1.0 } else { -1.0 };
        // The front face, and outwards from it.
        let face = first.get(dir.axis) + sign * depth;
        let (lo, hi) = (
            cabinet_core::geom::Vec3::new(
                first.x.min(last.x),
                first.y.min(last.y),
                first.z.min(last.z),
            ),
            cabinet_core::geom::Vec3::new(
                first.x.max(last.x),
                first.y.max(last.y),
                first.z.max(last.z),
            ),
        );
        let reach = |v: f64, axis| {
            if axis == dir.axis {
                (
                    face.min(face + sign * HANDLE_STANDOFF),
                    face.max(face + sign * HANDLE_STANDOFF),
                )
            } else {
                (v - HANDLE_BAR / 2.0, v + HANDLE_BAR / 2.0)
            }
        };
        use cabinet_core::geom::Axis;
        let span = |a: Axis| {
            let (l, h) = (lo.get(a), hi.get(a));
            if a == dir.axis {
                reach(l, a)
            } else if h - l > 1.0 {
                (l - HANDLE_END, h + HANDLE_END)
            } else {
                reach(l, a)
            }
        };
        let (x, y, z) = (span(Axis::X), span(Axis::Y), span(Axis::Z));
        let front = panels.iter().find(|b| b.name == p.name);
        let (hinged, hinge_x, hinge_y) = front
            .map(|b| (b.hinged.clone(), b.hinge_x, b.hinge_y))
            .unwrap_or_default();
        let on_drawer = p.role == cabinet_core::part::PartRole::DrawerFront;
        panels.push(PanelBox {
            name: format!("{}.Handle", p.name),
            role: if on_drawer { "Drawer handle" } else { "Handle" }.into(),
            x: x.0,
            y: y.0,
            z: z.0,
            width: x.1 - x.0,
            depth: y.1 - y.0,
            height: z.1 - z.0,
            is_front: false,
            hinged,
            hinge_x,
            hinge_y,
            board: format!("handle:{finish}"),
            grain: String::new(),
            notch_depth: 0.0,
            notch_height: 0.0,
        });
    }
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

/// Whether the picker for `field` means anything for this cabinet's standard (a fixing's size
/// only with that fixing, handle pickers only with handles...).
#[uniffi::export]
pub fn option_applies(cabinet_json: String, field: String) -> bool {
    parse(&cabinet_json)
        .map(|c| cabinet_core::options::applies(&field, &c.standard))
        .unwrap_or(true)
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

/// A 62 mm label per part, each with a QR code of the part's name.
#[uniffi::export]
pub fn part_labels_pdf(items: JobItems, job: String) -> Result<Vec<u8>, CoreError> {
    let lists: Vec<PartList> = job_lists(&items)?.into_iter().map(|(l, _)| l).collect();
    Ok(cabinet_sheets::labels::labels_pdf(&lists, board_name, &job))
}

fn board_name(p: &cabinet_core::Part) -> String {
    if p.board.is_empty() {
        "Generic".to_string()
    } else {
        p.board.clone()
    }
}

/// Every part's cut sheet in one PDF, after a cover and an index.
#[uniffi::export]
pub fn cut_sheets_pdf(items: JobItems, job: String) -> Result<Vec<u8>, CoreError> {
    let lists: Vec<PartList> = job_lists(&items)?.into_iter().map(|(l, _)| l).collect();
    Ok(cabinet_sheets::pdf::job_pdf(&lists, board_name, &job))
}

/// The usual Australian sheet for parts without a catalog board (mm).
const GENERIC_SHEET: (f64, f64) = (2400.0, 1200.0);

/// The job's parts nested on stock sheets for the panel saw (or a CNC router), on whichever of each board's catalog
/// sheet sizes uses least board; parts with no board go on 2400 × 1200 sheets.
fn job_nesting(
    items: &JobItems,
    catalog_dir: &str,
    kerf: f64,
    trim: f64,
    router: bool,
    offcuts: &[cabinet_core::nesting::Offcut],
) -> Result<
    (
        cabinet_core::nesting::Nesting,
        cabinet_core::nesting::Settings,
    ),
    CoreError,
> {
    use cabinet_core::nesting::{Settings, Stock, nest_with_offcuts};
    let (lists, tape) = cut_lists(items)?;
    let cat = cabinet_catalog::Catalog::load(std::path::Path::new(catalog_dir)).map_err(invalid)?;
    let rows = cut_list(&lists, "Generic", tape).rows;
    let settings = Settings {
        kerf,
        trim,
        router,
        ..Settings::default()
    };
    let nesting = nest_with_offcuts(
        &rows,
        |board, _| match cat.board(board) {
            Some(b) => b
                .sheets
                .iter()
                .map(|s| Stock {
                    length: s.length,
                    width: s.width,
                    grained: b.grained,
                })
                .collect(),
            None => vec![Stock {
                length: GENERIC_SHEET.0,
                width: GENERIC_SHEET.1,
                grained: false,
            }],
        },
        offcuts,
        &settings,
    );
    Ok((nesting, settings))
}

/// Kept offcuts as the app stores them (a JSON array; empty text for none).
fn offcuts(json: &str) -> Result<Vec<cabinet_core::nesting::Offcut>, CoreError> {
    if json.trim().is_empty() {
        return Ok(Vec::new());
    }
    serde_json::from_str(json).map_err(invalid)
}

/// The offcuts to keep after cutting the job (as JSON): the kept ones it did not use and its
/// leftovers.
#[uniffi::export]
pub fn nesting_kept_offcuts(
    items: JobItems,
    catalog_dir: String,
    kerf: f64,
    trim: f64,
    router: bool,
    offcuts_json: String,
) -> Result<String, CoreError> {
    let stored = offcuts(&offcuts_json)?;
    let (nesting, _) = job_nesting(&items, &catalog_dir, kerf, trim, router, &stored)?;
    serde_json::to_string(&cabinet_core::nesting::kept_offcuts(&nesting, &stored)).map_err(invalid)
}

/// The nesting as a PDF: a summary with the offcuts, then a page per sheet.
#[uniffi::export]
pub fn nesting_pdf(
    items: JobItems,
    catalog_dir: String,
    job: String,
    kerf: f64,
    trim: f64,
    router: bool,
    offcuts_json: String,
) -> Result<Vec<u8>, CoreError> {
    let (nesting, settings) = job_nesting(
        &items,
        &catalog_dir,
        kerf,
        trim,
        router,
        &offcuts(&offcuts_json)?,
    )?;
    Ok(cabinet_sheets::nesting::nesting_pdf(
        &nesting, &job, &settings,
    ))
}

/// The same pages as SVG documents, for the app to show.
#[uniffi::export]
pub fn nesting_pages(
    items: JobItems,
    catalog_dir: String,
    job: String,
    kerf: f64,
    trim: f64,
    router: bool,
    offcuts_json: String,
) -> Result<Vec<String>, CoreError> {
    let (nesting, settings) = job_nesting(
        &items,
        &catalog_dir,
        kerf,
        trim,
        router,
        &offcuts(&offcuts_json)?,
    )?;
    Ok(cabinet_sheets::nesting::nesting_svgs(
        &nesting, &job, &settings,
    ))
}

/// One line of a quote.
#[derive(uniffi::Record)]
pub struct QuoteLineRecord {
    /// board, edge_tape, hardware or labour.
    pub section: String,
    /// The price list key: board or hardware id, `edge_tape` or `labour`.
    pub key: String,
    pub description: String,
    pub quantity: f64,
    pub unit: String,
    pub unit_price: Option<f64>,
    pub total: f64,
}

#[derive(uniffi::Record)]
pub struct QuoteRecord {
    pub currency: String,
    pub lines: Vec<QuoteLineRecord>,
    pub cost: f64,
    pub markup: f64,
    pub tax: f64,
    pub total: f64,
    pub missing: Vec<String>,
}

fn job_quote_core(
    items: &JobItems,
    catalog_dir: &str,
    kerf: f64,
    trim: f64,
    router: bool,
    offcuts_json: &str,
    prices_json: &str,
) -> Result<cabinet_core::quote::Quote, CoreError> {
    let prices: cabinet_core::quote::PriceList = if prices_json.trim().is_empty() {
        Default::default()
    } else {
        serde_json::from_str(prices_json).map_err(invalid)?
    };
    let (nesting, _) = job_nesting(
        items,
        catalog_dir,
        kerf,
        trim,
        router,
        &offcuts(offcuts_json)?,
    )?;
    let (lists, tape) = cut_lists(items)?;
    let cut = cut_list(&lists, "Generic", tape);
    let bom = cabinet_core::bom::hardware_bom(&lists);
    let cabinets = job_lists(items)?
        .iter()
        .filter(|(_, c)| c.is_some())
        .count();
    Ok(cabinet_core::quote::quote(
        &nesting, &cut, &bom, cabinets, &prices,
    ))
}

/// What the job costs from the shop's price list (JSON, as `cabinet_core::quote::PriceList`).
#[uniffi::export]
pub fn job_quote(
    items: JobItems,
    catalog_dir: String,
    kerf: f64,
    trim: f64,
    router: bool,
    offcuts_json: String,
    prices_json: String,
) -> Result<QuoteRecord, CoreError> {
    let q = job_quote_core(
        &items,
        &catalog_dir,
        kerf,
        trim,
        router,
        &offcuts_json,
        &prices_json,
    )?;
    Ok(QuoteRecord {
        currency: q.currency,
        lines: q
            .lines
            .into_iter()
            .map(|l| QuoteLineRecord {
                section: serde_json::to_value(l.section)
                    .ok()
                    .and_then(|v| v.as_str().map(String::from))
                    .unwrap_or_default(),
                key: l.key,
                description: l.description,
                quantity: l.quantity,
                unit: l.unit,
                unit_price: l.unit_price,
                total: l.total,
            })
            .collect(),
        cost: q.cost,
        markup: q.markup,
        tax: q.tax,
        total: q.total,
        missing: q.missing,
    })
}

/// The quote as a PDF table.
#[uniffi::export]
// Swift calls it with named arguments; a settings record would only wrap the same values.
#[allow(clippy::too_many_arguments)]
pub fn quote_pdf(
    items: JobItems,
    catalog_dir: String,
    job: String,
    kerf: f64,
    trim: f64,
    router: bool,
    offcuts_json: String,
    prices_json: String,
) -> Result<Vec<u8>, CoreError> {
    let q = job_quote_core(
        &items,
        &catalog_dir,
        kerf,
        trim,
        router,
        &offcuts_json,
        &prices_json,
    )?;
    let money = |v: f64| format!("{v:.2}#");
    let mut rows: Vec<Vec<String>> = q
        .lines
        .iter()
        .map(|l| {
            vec![
                l.description.clone(),
                format!("{}#", l.quantity),
                l.unit.clone(),
                l.unit_price.map_or("no price#".into(), money),
                money(l.total),
            ]
        })
        .collect();
    rows.push(vec![String::new(); 5]);
    for (label, v) in [
        ("Cost", q.cost),
        ("Markup", q.markup),
        ("Tax", q.tax),
        ("Total", q.total),
    ] {
        rows.push(vec![
            format!("{label} ({})", q.currency),
            "#".into(),
            String::new(),
            "#".into(),
            money(v),
        ]);
    }
    if !q.missing.is_empty() {
        rows.push(vec![String::new(); 5]);
        rows.push(vec![
            format!(
                "No price yet for {} lines: the total is short by them",
                q.missing.len()
            ),
            "#".into(),
            String::new(),
            "#".into(),
            "#".into(),
        ]);
    }
    Ok(cabinet_sheets::pdf::table_pdf(
        "Quote",
        &job,
        &["Description", "Qty#", "Unit", "Price#", "Total#"],
        &[150.0, 25.0, 20.0, 35.0, 37.0],
        &rows,
    ))
}

/// The quote as the customer sees it: one price per cabinet, filler and end panel, tax included.
#[uniffi::export]
// Swift calls it with named arguments; a settings record would only wrap the same values.
#[allow(clippy::too_many_arguments)]
pub fn customer_quote_pdf(
    items: JobItems,
    catalog_dir: String,
    job: String,
    kerf: f64,
    trim: f64,
    router: bool,
    offcuts_json: String,
    prices_json: String,
) -> Result<Vec<u8>, CoreError> {
    let prices: cabinet_core::quote::PriceList = if prices_json.trim().is_empty() {
        Default::default()
    } else {
        serde_json::from_str(prices_json.as_str()).map_err(invalid)?
    };
    let q = job_quote_core(
        &items,
        &catalog_dir,
        kerf,
        trim,
        router,
        &offcuts_json,
        &prices_json,
    )?;
    let (nesting, _) = job_nesting(
        &items,
        &catalog_dir,
        kerf,
        trim,
        router,
        &offcuts(&offcuts_json)?,
    )?;
    let built = job_lists(&items)?;
    let ids: Vec<String> = built
        .iter()
        .filter_map(|(_, c)| c.as_ref().map(|c| c.id.clone()))
        .collect();
    let lists: Vec<PartList> = built.iter().map(|(l, _)| l.clone()).collect();
    let (_, tape) = cut_lists(&items)?;
    let cut = cut_list(&lists, "Generic", tape);
    let shares = cabinet_core::quote::per_cabinet(&nesting, &cut, &lists, &ids, &prices);
    let describe = |id: &str| {
        built
            .iter()
            .find_map(|(_, c)| c.as_ref().filter(|c| c.id == id))
            .map(|c| {
                let kind = serde_json::to_value(c.kind)
                    .ok()
                    .and_then(|v| v.as_str().map(|s| s.replace('_', " ")))
                    .unwrap_or_default();
                format!(
                    "{id}  {kind}, {:.0} × {:.0} × {:.0} mm",
                    c.width, c.height, c.depth
                )
            })
            .unwrap_or_else(|| id.to_string())
    };
    let mut rows: Vec<Vec<String>> = shares
        .iter()
        .map(|c| vec![describe(&c.cabinet), format!("{:.2}#", c.total)])
        .collect();
    rows.push(vec![String::new(), String::new()]);
    rows.push(vec![
        format!("Total ({}, including {:.2} tax)", q.currency, q.tax),
        format!("{:.2}#", q.total),
    ]);
    if !q.missing.is_empty() {
        rows.push(vec![
            "Draft: some prices are still missing".into(),
            "#".into(),
        ]);
    }
    Ok(cabinet_sheets::pdf::table_pdf(
        "Quote",
        &job,
        &["Item", "Price#"],
        &[220.0, 47.0],
        &rows,
    ))
}

/// The benchtop schedule for the benchtop maker: a summary, then each piece's sheet.
#[uniffi::export]
pub fn benchtops_pdf(items: JobItems, job: String) -> Result<Vec<u8>, CoreError> {
    let lists: Vec<PartList> = job_lists(&items)?.into_iter().map(|(l, _)| l).collect();
    Ok(cabinet_sheets::benchtops::benchtops_pdf(&lists, &job))
}

/// An exploded assembly drawing per cabinet with its parts list, as a PDF.
#[uniffi::export]
pub fn assembly_pdf(items: JobItems, job: String) -> Result<Vec<u8>, CoreError> {
    let lists: Vec<PartList> = job_lists(&items)?.into_iter().map(|(l, _)| l).collect();
    Ok(cabinet_sheets::assembly::assembly_pdf(&lists, &job))
}

/// One part's DXF file for CNC (see `cabinet_sheets::dxf` for the layers).
#[derive(uniffi::Record)]
pub struct DxfRecord {
    /// The part name, `-back` for its second face; no extension.
    pub name: String,
    pub contents: String,
}

/// Every part's DXF files.
#[uniffi::export]
pub fn job_dxfs(items: JobItems) -> Result<Vec<DxfRecord>, CoreError> {
    let lists: Vec<PartList> = job_lists(&items)?.into_iter().map(|(l, _)| l).collect();
    Ok(cabinet_sheets::dxf::job_dxfs(&lists)
        .into_iter()
        .map(|f| DxfRecord {
            name: f.name,
            contents: f.contents,
        })
        .collect())
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

/// The render setup the app saves in the design: SolidCraft 0.20's built-in renderer offers the
/// `preview` mode only.
const RENDER_SETUP: &str = "Cabinet Studio";

/// A three-quarter view from the front that takes in every part of `lists`: SolidCraft's own
/// isometric angles, aimed at the middle, far enough back for the whole box.
fn render_camera(lists: &[PartList]) -> serde_json::Value {
    let mut lo = [f64::MAX; 3];
    let mut hi = [f64::MIN; 3];
    for p in lists.iter().flat_map(|l| &l.parts) {
        let (a, b) = (p.bounds.min.to_array(), p.bounds.max().to_array());
        for i in 0..3 {
            lo[i] = lo[i].min(a[i]);
            hi[i] = hi[i].max(b[i]);
        }
    }
    if lo[0] > hi[0] {
        (lo, hi) = ([0.0; 3], [600.0; 3]);
    }
    let centre: Vec<f64> = (0..3).map(|i| (lo[i] + hi[i]) / 2.0).collect();
    let diagonal = (0..3).map(|i| (hi[i] - lo[i]).powi(2)).sum::<f64>().sqrt();
    serde_json::json!({
        "target": centre,
        "yaw": -std::f64::consts::FRAC_PI_4,
        "pitch": 0.45,
        "distance": 1.35 * diagonal,
        "fov": 0.0,
        "roll": 0.0,
        "up": "z",
    })
}

/// Renders the design open in SolidCraft to a PNG at `path`, `width` × `height` pixels, framed on
/// the job's cabinets, and returns the path. Saves (or updates) a render setup named "Cabinet
/// Studio" in the design. Blocking, up to two minutes: call it off the main thread.
#[uniffi::export]
pub fn render_png(
    items: JobItems,
    port: u16,
    path: String,
    width: u32,
    height: u32,
) -> Result<String, CoreError> {
    let sc = |e: solidcraft_bridge::Error| CoreError::SolidCraft {
        message: e.to_string(),
    };
    let lists: Vec<PartList> = job_lists(&items)?.into_iter().map(|(l, _)| l).collect();
    let mut client = Client::connect(port).map_err(sc)?;
    client
        .execute(
            "render.settings.set",
            serde_json::json!({ "name": RENDER_SETUP, "set": {
                "mode": "preview", "width": width, "height": height, "camera": render_camera(&lists)
            } }),
        )
        .map_err(sc)?;
    let started = client
        .execute("render.start", serde_json::json!({ "setup": RENDER_SETUP }))
        .map_err(sc)?;
    let job = started["job"].clone();
    if job.is_null() {
        return Err(invalid(format!(
            "SolidCraft started no render job: {started}"
        )));
    }
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(120);
    loop {
        // The status names its error when the job failed; otherwise the export succeeds once the
        // image is done.
        let status = client
            .execute("render.status", serde_json::json!({ "job": job }))
            .map_err(sc)?;
        if let Some(e) = status.get("error").filter(|e| !e.is_null()) {
            return Err(CoreError::SolidCraft {
                message: format!("the render failed: {e}"),
            });
        }
        let exported = client.execute(
            "render.export",
            serde_json::json!({ "job": job, "path": path, "overwrite": true }),
        );
        if exported.is_ok() {
            return Ok(path);
        }
        if std::time::Instant::now() > deadline {
            return Err(CoreError::SolidCraft {
                message: format!("the render did not finish in two minutes: {status}"),
            });
        }
        std::thread::sleep(std::time::Duration::from_millis(400));
    }
}

/// Whether a SolidCraft app answers on `port`.
#[uniffi::export]
pub fn solidcraft_reachable(port: u16) -> bool {
    Client::connect(port)
        .and_then(|mut c| c.request("ui.inspect", serde_json::json!({})))
        .is_ok()
}

/// What [`prepare_supplier_cad`] found and did.
#[derive(uniffi::Record)]
pub struct SupplierCadReport {
    pub models: u32,
    pub converted: u32,
    /// Articles with a model, sorted.
    pub articles: Vec<String>,
}

/// Turns the STEP files under `vendor_dir` into meshes the app can draw, with SolidCraft's
/// command-line tool at `cli`. Converts only new or changed files. Blocking.
#[uniffi::export]
pub fn prepare_supplier_cad(
    vendor_dir: String,
    cli: String,
) -> Result<SupplierCadReport, CoreError> {
    let (models, converted) = vendor::prepare(Path::new(&vendor_dir), Path::new(&cli))
        .map_err(|e| invalid(e.to_string()))?;
    let mut articles: Vec<String> = models.iter().map(|m| m.article.clone()).collect();
    articles.sort();
    articles.dedup();
    Ok(SupplierCadReport {
        models: models.len() as u32,
        converted: converted as u32,
        articles,
    })
}

/// A supplier model to draw: its mesh file and where it goes, model mm → design mm, row-major
/// 3 × 4.
#[derive(uniffi::Record)]
pub struct HardwareModel {
    pub name: String,
    pub article: String,
    pub mesh: String,
    pub transform: Vec<f64>,
}

/// The supplier models for every item of the job that has one prepared under `vendor_dir`.
#[uniffi::export]
pub fn hardware_models(job: JobItems, vendor_dir: String) -> Result<Vec<HardwareModel>, CoreError> {
    let Some(index) = vendor::load_index(Path::new(&vendor_dir)) else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for (list, _) in job_lists(&job)? {
        for m in vendor::resolve(&cad_placements(&list), &index) {
            out.push(HardwareModel {
                name: m.name,
                article: m.article,
                mesh: m.stl.display().to_string(),
                transform: m.transform.to_vec(),
            });
        }
    }
    Ok(out)
}

/// A triangle mesh in model mm: three positions and three normals per triangle.
#[derive(uniffi::Record)]
pub struct MeshData {
    pub positions: Vec<f32>,
    pub normals: Vec<f32>,
}

#[uniffi::export]
pub fn load_mesh(path: String) -> Result<MeshData, CoreError> {
    let m = vendor::load_stl(Path::new(&path)).map_err(|e| invalid(e.to_string()))?;
    Ok(MeshData {
        positions: m.positions,
        normals: m.normals,
    })
}

/// One solid of a stand-in, for the viewport: a box, or a cylinder from `base` along `axis`
/// (0 = x, 1 = y, 2 = z) towards + when `positive`.
#[derive(uniffi::Record)]
pub struct StandInShape {
    pub name: String,
    pub item: String,
    pub cylinder: bool,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub width: f64,
    pub depth: f64,
    pub height: f64,
    pub axis: u8,
    pub positive: bool,
    pub diameter: f64,
    pub length: f64,
}

/// The stand-in shapes of the job's hardware (legs, and hinges and runners), except those whose
/// supplier model `hardware_models` returns for `vendor_dir`.
#[uniffi::export]
pub fn stand_in_shapes(job: JobItems, vendor_dir: String) -> Result<Vec<StandInShape>, CoreError> {
    use cabinet_core::geom::Axis;
    use cabinet_core::standins::{Shape, stand_ins};
    let index = vendor::load_index(Path::new(&vendor_dir)).unwrap_or_default();
    let mut out = Vec::new();
    for (list, _) in job_lists(&job)? {
        let real: std::collections::HashSet<String> =
            vendor::resolve(&cad_placements(&list), &index)
                .into_iter()
                .map(|m| m.name)
                .collect();
        for s in stand_ins(&list) {
            // A stand-in hinge also covers its plate; runners are named per hand.
            if real.contains(&s.name) {
                continue;
            }
            for shape in &s.shapes {
                let mut r = StandInShape {
                    name: s.name.clone(),
                    item: s.item.clone(),
                    cylinder: false,
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                    width: 0.0,
                    depth: 0.0,
                    height: 0.0,
                    axis: 2,
                    positive: true,
                    diameter: 0.0,
                    length: 0.0,
                };
                match shape {
                    Shape::Box(b) => {
                        (r.x, r.y, r.z) = (b.min.x, b.min.y, b.min.z);
                        (r.width, r.depth, r.height) = (b.size.x, b.size.y, b.size.z);
                    }
                    Shape::Cylinder {
                        base,
                        axis,
                        diameter,
                        height,
                    } => {
                        r.cylinder = true;
                        (r.x, r.y, r.z) = (base.x, base.y, base.z);
                        r.axis = match axis.axis {
                            Axis::X => 0,
                            Axis::Y => 1,
                            Axis::Z => 2,
                        };
                        r.positive = axis.positive;
                        (r.diameter, r.length) = (*diameter, *height);
                    }
                }
                out.push(r);
            }
        }
    }
    Ok(out)
}

/// Builds the cabinets in the running SolidCraft. Blocking: call it off the main thread.
/// Cabinets already in the design (same component name) are replaced in place. Beyond Design
/// detail, hardware with a prepared supplier model under `vendor_dir` goes in as that model.
#[uniffi::export]
pub fn send_to_solidcraft(
    job: JobItems,
    detail: String,
    port: u16,
    vendor_dir: String,
) -> Result<String, CoreError> {
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
    let vendor_path = Path::new(&vendor_dir);
    let index = vendor::load_index(vendor_path);
    let mut report = Vec::new();
    for (list, cab) in job_lists(&job)? {
        // One batch replaces an earlier build of this item and keeps it where the user moved it
        // in SolidCraft. Only cabinets carry a spec to load back (SC-3).
        let existing = find_existing(&client.components().map_err(sc)?, &list.cabinet);
        let spec = cab
            .map(|c| serde_json::to_value(&c).map_err(invalid))
            .transpose()?;
        let models = match (&index, detail) {
            (Some(index), Detail::Manufacturing | Detail::Exact) => {
                vendor::resolve(&cad_placements(&list), index)
            }
            _ => Vec::new(),
        };
        let real: Vec<String> = models.iter().map(|m| m.name.clone()).collect();
        let cmds = replace_cabinet(&list, detail, spec.as_ref(), existing.as_ref(), &real);
        let r = client.batch(&cmds).map_err(sc)?;
        // Supplier models load files, which a batch refuses: one request each.
        for c in hardware_inserts(&list.cabinet, &models) {
            client
                .execute_reading(
                    c["command"].as_str().unwrap_or_default(),
                    c["params"].clone(),
                    &[vendor_path],
                )
                .map_err(sc)?;
        }
        report.push(format!(
            "{}: {} commands in {:.1} s{}",
            list.cabinet,
            cmds.len(),
            r["total_ms"].as_f64().unwrap_or(0.0) / 1000.0,
            if models.is_empty() {
                String::new()
            } else {
                format!(", {} supplier models", models.len())
            }
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
    let profiles: Vec<_> = cat
        .boards
        .iter()
        .map(|b| cat.profile_of(b).cloned())
        .collect();
    Ok(cat
        .boards
        .into_iter()
        .zip(profiles)
        .map(|(b, profile)| BoardRecord {
            profile_shape: profile
                .as_ref()
                .and_then(|p| serde_json::to_value(p.shape).ok())
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_default(),
            profile_module: profile.as_ref().map_or(0.0, |p| p.module),
            profile_groove: profile
                .as_ref()
                .and_then(|p| p.groove.or(p.radius.map(|r| r.min(p.module / 2.0))))
                .unwrap_or(0.0),
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
            kind: serde_json::to_value(b.kind)
                .ok()
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_default(),
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
            String::new(),
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
        let file = project_json(
            "Kitchen".into(),
            standard,
            items.clone(),
            Vec::new(),
            String::new(),
        )
        .unwrap();
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

    #[test]
    fn kept_offcuts_read_from_the_apps_store() {
        assert!(offcuts("").unwrap().is_empty());
        let o = offcuts(
            r#"[{"board":"Generic","thickness":16.0,"length":900.0,"width":600.0,"grained":false}]"#,
        )
        .unwrap();
        assert_eq!(o[0].width, 600.0);
        assert!(offcuts("not json").is_err());
    }

    #[test]
    fn a_job_quotes_and_its_customer_quote_has_a_line_per_item() {
        let catalog = concat!(env!("CARGO_MANIFEST_DIR"), "/../../catalog").to_string();
        let items = vec![
            new_cabinet("B01".into(), "base".into()).unwrap(),
            r#"{"type":"filler","id":"F01","width":50}"#.to_string(),
            new_cabinet("W01".into(), "wall".into()).unwrap(),
        ];
        let prices = r#"{"boards":{"Generic":20.0},"labour_per_hour":70.0,"edge_tape_per_m":1.0}"#;
        let q = job_quote(
            job(&items),
            catalog.clone(),
            4.4,
            10.0,
            false,
            String::new(),
            prices.into(),
        )
        .unwrap();
        assert!(q.total > 0.0);
        assert!(
            q.lines
                .iter()
                .any(|l| l.section == "board" && l.key == "Generic")
        );
        let pdf = customer_quote_pdf(
            job(&items),
            catalog,
            "Kitchen".into(),
            4.4,
            10.0,
            false,
            String::new(),
            prices.into(),
        )
        .unwrap();
        let text = String::from_utf8_lossy(&pdf);
        for id in ["B01", "F01", "W01"] {
            assert!(text.contains(id), "{id}");
        }
    }

    #[test]
    fn a_shaker_doors_pieces_all_swing_about_its_hinge_line() {
        let mut cab = Cabinet::new("B01", cabinet_core::CabinetKind::Base);
        cab.standard.door_style = cabinet_core::model::DoorStyle::Shaker;
        let sum = summary(cabinet_core::build_cabinet(&cab).unwrap());
        let door: Vec<&PanelBox> = sum
            .panels
            .iter()
            .filter(|p| p.name.starts_with("B01.Door1"))
            .collect();
        // Five pieces and the handle.
        assert_eq!(door.len(), 6);
        let left = door.iter().map(|p| p.x).fold(f64::MAX, f64::min);
        for p in door {
            assert_eq!((p.hinged.as_str(), p.hinge_x), ("left", left), "{}", p.name);
        }
        assert!(
            sum.panels
                .iter()
                .filter(|p| p.role == "Left side")
                .all(|p| p.hinged.is_empty())
        );
    }

    #[test]
    fn notched_sides_carry_their_notch_once_each() {
        let mut cab = Cabinet::new("B01", cabinet_core::CabinetKind::Base);
        cab.standard.toe_kick = cabinet_core::model::ToeKick::IntegratedNotch;
        let sum = summary(cabinet_core::build_cabinet(&cab).unwrap());
        let sides: Vec<&PanelBox> = sum
            .panels
            .iter()
            .filter(|p| p.role.ends_with("side"))
            .collect();
        assert_eq!(sides.len(), 2);
        for p in sides {
            assert_eq!(p.z, 0.0);
            assert_eq!(p.notch_depth, cab.standard.toe_kick_setback);
            assert_eq!(p.notch_height, cab.standard.toe_kick_height);
        }
        assert!(
            sum.panels
                .iter()
                .filter(|p| !p.role.ends_with("side"))
                .all(|p| p.notch_height == 0.0)
        );
    }

    /// BENCH-9: the job keeps its appliance library; a job without one opens with ours.
    #[test]
    fn the_appliance_library_keeps_in_the_job_file() {
        let standard = serde_json::to_string(&ConstructionStandard::default()).unwrap();
        let fresh = project_json(
            "Kitchen".into(),
            standard.clone(),
            Vec::new(),
            Vec::new(),
            String::new(),
        )
        .unwrap();
        let opened = open_project(fresh).unwrap();
        assert_eq!(opened.appliances_json, default_appliances_json());
        let mut library: Vec<cabinet_core::run::Appliance> =
            serde_json::from_str(&opened.appliances_json).unwrap();
        library[0].name = "Franke 820".into();
        library[0].corner_radius = 25.0;
        let json = serde_json::to_string(&library).unwrap();
        let file = project_json(
            "Kitchen".into(),
            standard,
            Vec::new(),
            Vec::new(),
            json.clone(),
        )
        .unwrap();
        assert_eq!(open_project(file).unwrap().appliances_json, json);
    }

    #[test]
    fn a_benchtop_item_builds_over_the_run_and_keeps_in_the_job_file() {
        let top = r#"{"type":"benchtop","id":"BT01","thickness":33,"front_overhang":20}"#;
        let items = vec![
            new_cabinet("B01".into(), "base".into()).unwrap(),
            new_cabinet("B02".into(), "base".into()).unwrap(),
            top.to_string(),
        ];
        let built = build_job(job(&items));
        let panels = &built[2].summary.as_ref().unwrap().panels;
        assert_eq!(panels.len(), 1);
        assert_eq!(panels[0].role, "Benchtop");
        assert!((panels[0].width - 1200.0).abs() < 1e-9);
        let standard = serde_json::to_string(&parse(&items[0]).unwrap().standard).unwrap();
        let file = project_json(
            "Kitchen".into(),
            standard,
            items.clone(),
            Vec::new(),
            String::new(),
        )
        .unwrap();
        let back = open_project(file).unwrap();
        assert_eq!(back.items_json.len(), 3);
        assert!(back.items_json[2].contains("benchtop"));
        assert!(
            cut_list_rows(job(&items))
                .unwrap()
                .iter()
                .any(|r| r.description == "Benchtop")
        );
    }

    /// The app writes a cornice like this; it keeps in the job file.
    #[test]
    fn a_cornice_item_builds_over_the_tall_cabinets() {
        let cornice =
            r#"{"id":"CO01","type":"moulding","kind":"cornice","height":60,"returns":true}"#;
        let items = vec![
            new_cabinet("T01".into(), "tall".into()).unwrap(),
            new_cabinet("T02".into(), "tall".into()).unwrap(),
            cornice.to_string(),
        ];
        let built = build_job(job(&items));
        let panels = &built[2].summary.as_ref().unwrap().panels;
        assert_eq!(panels.len(), 3);
        assert!(panels.iter().all(|p| p.role == "Cornice"));
        let standard = serde_json::to_string(&parse(&items[0]).unwrap().standard).unwrap();
        let file = project_json(
            "Kitchen".into(),
            standard,
            items.clone(),
            Vec::new(),
            String::new(),
        )
        .unwrap();
        let back = open_project(file).unwrap();
        assert!(back.items_json[2].contains("moulding"));
    }

    /// The app writes an oven tower's stacked fronts like this.
    #[test]
    fn a_stacked_front_reads_from_the_apps_json() {
        let mut cab: serde_json::Value =
            serde_json::from_str(&new_cabinet("T01".into(), "tall".into()).unwrap()).unwrap();
        cab["adjustable_shelves"] = 0.into();
        cab["fronts"] = serde_json::json!({"type": "stack", "sections": [
            {"type": "drawers", "count": 2, "height": 0},
            {"type": "niche", "height": 600},
            {"type": "door", "hinge_side": "left", "height": 0}
        ]});
        let sum = build(cab.to_string()).unwrap();
        assert_eq!(
            sum.panels
                .iter()
                .filter(|p| p.role == "Drawer front")
                .count(),
            2
        );
        assert_eq!(sum.panels.iter().filter(|p| p.role == "Door").count(), 1);
    }

    /// STYLE-3: a door's handle is drawn standing off its front face, swinging with the door.
    #[test]
    fn handles_are_drawn_off_the_front_and_swing_with_it() {
        let mut cab = Cabinet::new("B01", cabinet_core::CabinetKind::Base);
        cab.standard.handle_finish = cabinet_core::model::HandleFinish::Black;
        let sum = summary(cabinet_core::build_cabinet(&cab).unwrap());
        let door = sum.panels.iter().find(|p| p.name == "B01.Door1").unwrap();
        let h = sum
            .panels
            .iter()
            .find(|p| p.name == "B01.Door1.Handle")
            .unwrap();
        assert_eq!(h.role, "Handle");
        assert_eq!(h.board, "handle:matt black");
        assert_eq!(
            (h.hinged.as_str(), h.hinge_x),
            (door.hinged.as_str(), door.hinge_x)
        );
        assert!((h.y + h.depth - door.y).abs() < 1e-9, "on the front face");
        assert!((h.height - (128.0 + 30.0)).abs() < 1e-9);
    }
}
