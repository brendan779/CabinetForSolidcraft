//! `xcabinet-engine serve` runs the HTTP service; `xcabinet-engine demo <dir>` writes the sample
//! kitchen's outputs to a folder, no server needed.
//!
//! Environment: `PORT` (default 8080), `CATALOG_DIR` (boards and sheet sizes; default `catalog`
//! when that folder exists).

use std::path::Path;
use std::sync::Arc;

use engine_api::{AppState, Options, build, router, sample_run};

fn load_catalog() -> Option<cabinet_catalog::Catalog> {
    let dir = std::env::var("CATALOG_DIR").unwrap_or_else(|_| "catalog".into());
    let path = Path::new(&dir);
    if !path.is_dir() {
        eprintln!("no catalog at {dir}: boards nest on the generic sheet");
        return None;
    }
    match cabinet_catalog::Catalog::load(path) {
        Ok(c) => Some(c),
        Err(e) => {
            eprintln!("catalog at {dir} did not load ({e}): boards nest on the generic sheet");
            None
        }
    }
}

fn demo(dir: &str) -> Result<(), String> {
    let opts = Options {
        job: "XCabinet sample kitchen".into(),
        ..Options::default()
    };
    let catalog = load_catalog();
    let b = build(&sample_run(), &opts, catalog.as_ref())?;
    let s = b.summary();
    println!(
        "{} cabinets, {} parts, {} operations, {} clashes, {} sheets, {} hardware lines",
        s.cabinets,
        s.parts,
        s.operations,
        s.clashes.len(),
        s.sheets.len(),
        s.hardware.len()
    );
    for (name, bytes) in b.files(&opts.job) {
        let path = Path::new(dir).join(&name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
    }
    println!("written to {dir}");
    Ok(())
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("demo") => {
            if let Err(e) = demo(args.get(2).map(String::as_str).unwrap_or("out")) {
                eprintln!("demo failed: {e}");
                std::process::exit(1);
            }
        }
        Some("serve") | None => {
            let port = std::env::var("PORT").unwrap_or_else(|_| "8080".into());
            let app = router(Arc::new(AppState {
                catalog: load_catalog(),
            }));
            let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
                .await
                .expect("bind the port");
            println!("xcabinet-engine listening on :{port}");
            axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    let _ = tokio::signal::ctrl_c().await;
                })
                .await
                .expect("serve");
        }
        Some(other) => {
            eprintln!("unknown command {other}: use `serve` or `demo <dir>`");
            std::process::exit(2);
        }
    }
}
