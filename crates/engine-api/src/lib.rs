//! XCabinet engine service. Takes a [`Run`] of cabinets and returns what production needs: the
//! part lists, a cut list, the hardware list, a nesting, a collision check, and as files the PDFs
//! (cut sheets, labels, assembly, elevation) and one DXF per part face.
//!
//! The HTTP routes live here so tests drive them without a socket; `main.rs` only binds a port.

use std::io::Write;
use std::sync::Arc;

use axum::Router;
use axum::extract::{DefaultBodyLimit, Query, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use cabinet_catalog::Catalog;
use cabinet_core::model::{FrontLayout, HingeSide};
use cabinet_core::nesting::{Nesting, Settings, Stock, nest};
use cabinet_core::{
    Cabinet, CabinetKind, Part, PartList, Run, RunItem, bom, build_run, collisions, cutlist,
};
use serde::{Deserialize, Serialize};

/// Sheet size for boards the catalog doesn't know, in mm: the common European and Middle East
/// melamine sheet (the Mac app's 2400 × 1200 is too short for a tall cabinet's sides).
const GENERIC_SHEET: (f64, f64) = (2800.0, 2070.0);
/// Largest request body: a whole kitchen is a few tens of kB.
const BODY_LIMIT: usize = 2 * 1024 * 1024;

#[derive(Default)]
pub struct AppState {
    /// Boards and their sheet sizes. None: every board nests on the generic sheet.
    pub catalog: Option<Catalog>,
}

/// Build options from the query string.
#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct Options {
    /// Job name printed on sheets and labels.
    pub job: String,
    /// Edge tape thickness taken off cut sizes (mm).
    pub tape: f64,
    pub kerf: f64,
    pub trim: f64,
    /// Nest for a CNC router instead of a panel saw.
    pub router: bool,
}

impl Default for Options {
    fn default() -> Self {
        let s = Settings::default();
        Options {
            job: "XCabinet job".into(),
            tape: 1.0,
            kerf: s.kerf,
            trim: s.trim,
            router: false,
        }
    }
}

/// Everything the engine works out for a run, before any file is drawn.
pub struct Built {
    pub lists: Vec<PartList>,
    pub cut: cutlist::CutList,
    pub bom: Vec<bom::BomRow>,
    pub nesting: Nesting,
    pub settings: Settings,
    pub clashes: Vec<String>,
}

pub fn board_name(p: &Part) -> String {
    if p.board.is_empty() {
        "Generic".to_string()
    } else {
        p.board.clone()
    }
}

pub fn build(run: &Run, opts: &Options, catalog: Option<&Catalog>) -> Result<Built, String> {
    let lists = build_run(run).map_err(|e| e.to_string())?;
    let clashes = lists
        .iter()
        .flat_map(collisions::check)
        .map(|c| c.to_string())
        .collect();
    let cut = cutlist::cut_list(&lists, "Generic", opts.tape);
    let bom = bom::hardware_bom(&lists);
    let settings = Settings {
        kerf: opts.kerf,
        trim: opts.trim,
        router: opts.router,
        ..Settings::default()
    };
    let nesting = nest(
        &cut.rows,
        |board, _| match catalog.and_then(|c| c.board(board)) {
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
        &settings,
    );
    Ok(Built {
        lists,
        cut,
        bom,
        nesting,
        settings,
        clashes,
    })
}

#[derive(Serialize)]
pub struct SheetSummary {
    pub board: String,
    pub thickness: f64,
    pub length: f64,
    pub width: f64,
    pub parts: usize,
}

#[derive(Serialize)]
pub struct Summary<'a> {
    pub cabinets: usize,
    pub parts: usize,
    pub operations: usize,
    pub clashes: &'a [String],
    /// Problems the builder worked around, as `cabinet: what`.
    pub warnings: Vec<String>,
    pub cut_list: &'a [cutlist::CutRow],
    pub hardware: &'a [bom::BomRow],
    pub sheets: Vec<SheetSummary>,
    pub unplaced: &'a [String],
    pub part_lists: &'a [PartList],
}

impl Built {
    pub fn summary(&self) -> Summary<'_> {
        Summary {
            cabinets: self.lists.len(),
            parts: self.lists.iter().map(|l| l.parts.len()).sum(),
            operations: self.lists.iter().map(|l| l.operations.len()).sum(),
            clashes: &self.clashes,
            warnings: self
                .lists
                .iter()
                .flat_map(|l| {
                    l.warnings
                        .iter()
                        .map(move |w| format!("{}: {w}", l.cabinet))
                })
                .collect(),
            cut_list: &self.cut.rows,
            hardware: &self.bom,
            sheets: self
                .nesting
                .sheets
                .iter()
                .map(|s| SheetSummary {
                    board: s.board.clone(),
                    thickness: s.thickness,
                    length: s.stock.length,
                    width: s.stock.width,
                    parts: s.placements.len(),
                })
                .collect(),
            unplaced: &self.nesting.unplaced,
            part_lists: &self.lists,
        }
    }

    /// Every output as `(path, bytes)`, the paths as they go in the zip.
    pub fn files(&self, job: &str) -> Vec<(String, Vec<u8>)> {
        let mut out = vec![
            (
                "cutlist.csv".to_string(),
                cutlist::to_csv(&self.cut).into_bytes(),
            ),
            (
                "hardware_bom.csv".to_string(),
                bom::bom_csv(&self.bom, |_| None).into_bytes(),
            ),
            (
                "cut_sheets.pdf".to_string(),
                cabinet_sheets::pdf::job_pdf(&self.lists, board_name, job),
            ),
            (
                "labels.pdf".to_string(),
                cabinet_sheets::labels::labels_pdf(&self.lists, board_name, job),
            ),
            (
                "assembly.pdf".to_string(),
                cabinet_sheets::assembly::assembly_pdf(&self.lists, job),
            ),
            (
                "nesting.pdf".to_string(),
                cabinet_sheets::nesting::nesting_pdf(&self.nesting, job, &self.settings),
            ),
        ];
        if let Some(e) = cabinet_sheets::elevation::elevation_pdf(&self.lists, None, job) {
            out.push(("elevation.pdf".to_string(), e));
        }
        for d in cabinet_sheets::dxf::job_dxfs(&self.lists) {
            out.push((format!("dxf/{}.dxf", d.name), d.contents.into_bytes()));
        }
        out
    }
}

pub fn zip(files: &[(String, Vec<u8>)]) -> std::io::Result<Vec<u8>> {
    let mut w = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    for (name, bytes) in files {
        w.start_file(name.as_str(), opts)?;
        w.write_all(bytes)?;
    }
    Ok(w.finish()?.into_inner())
}

/// The sample kitchen: one straight run of five cabinets, for trying the service.
pub fn sample_run() -> Run {
    fn cab(id: &str, kind: CabinetKind, width: f64, fronts: FrontLayout) -> RunItem {
        let mut c = Cabinet::new(id, kind);
        c.width = width;
        c.fronts = fronts;
        RunItem::Cabinet(Box::new(c))
    }
    Run {
        origin: Default::default(),
        items: vec![
            cab(
                "B01",
                CabinetKind::Base,
                450.0,
                FrontLayout::Door {
                    hinge_side: HingeSide::Left,
                },
            ),
            cab(
                "B02",
                CabinetKind::DrawerBank,
                600.0,
                FrontLayout::Drawers {
                    ratios: vec![1.0, 1.5, 1.5],
                },
            ),
            cab("B03", CabinetKind::Sink, 800.0, FrontLayout::DoorPair),
            cab(
                "B04",
                CabinetKind::Base,
                600.0,
                FrontLayout::Door {
                    hinge_side: HingeSide::Right,
                },
            ),
            cab("T01", CabinetKind::Tall, 600.0, FrontLayout::DoorPair),
        ],
    }
}

fn error(status: StatusCode, msg: impl Into<String>) -> Response {
    (
        status,
        axum::Json(serde_json::json!({ "error": msg.into() })),
    )
        .into_response()
}

/// Reads a run from a JSON body, saying what is wrong in plain words when it isn't one.
fn parse_run(body: &str) -> Result<Run, String> {
    serde_json::from_str(body).map_err(|e| format!("not a valid run: {e}"))
}

async fn health() -> &'static str {
    "ok"
}

async fn sample() -> axum::Json<Run> {
    axum::Json(sample_run())
}

async fn build_json(
    State(st): State<Arc<AppState>>,
    Query(opts): Query<Options>,
    body: String,
) -> Response {
    let run = match parse_run(&body) {
        Ok(r) => r,
        Err(e) => return error(StatusCode::UNPROCESSABLE_ENTITY, e),
    };
    match build(&run, &opts, st.catalog.as_ref()) {
        Ok(b) => axum::Json(b.summary()).into_response(),
        Err(e) => error(StatusCode::UNPROCESSABLE_ENTITY, e),
    }
}

async fn build_files(
    State(st): State<Arc<AppState>>,
    Query(opts): Query<Options>,
    body: String,
) -> Response {
    let run = match parse_run(&body) {
        Ok(r) => r,
        Err(e) => return error(StatusCode::UNPROCESSABLE_ENTITY, e),
    };
    // Drawing every PDF is CPU work: keep it off the async workers.
    let res = tokio::task::spawn_blocking(move || {
        let b = build(&run, &opts, st.catalog.as_ref())?;
        zip(&b.files(&opts.job)).map_err(|e| e.to_string())
    })
    .await;
    match res {
        Ok(Ok(bytes)) => (
            [
                (header::CONTENT_TYPE, "application/zip"),
                (
                    header::CONTENT_DISPOSITION,
                    "attachment; filename=\"xcabinet-job.zip\"",
                ),
            ],
            bytes,
        )
            .into_response(),
        Ok(Err(e)) => error(StatusCode::UNPROCESSABLE_ENTITY, e),
        Err(e) => error(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("build failed: {e}"),
        ),
    }
}

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/v1/sample", get(sample))
        .route("/v1/build", post(build_json))
        .route("/v1/build/files", post(build_files))
        .layer(DefaultBodyLimit::max(BODY_LIMIT))
        .with_state(state)
}
