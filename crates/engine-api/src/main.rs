//! `xcabinet-engine serve` runs the HTTP service; `xcabinet-engine demo <dir>` writes the sample
//! kitchen's outputs to a folder, no server needed.
//!
//! `xcabinet-engine health` exits 0 when the local service answers (for Docker's HEALTHCHECK).
//!
//! Environment: `PORT` (default 8080), `BIND` (default 0.0.0.0), `CATALOG_DIR` (boards and sheet
//! sizes; default `catalog` when that folder exists), `ENGINE_TOKEN` (when set, `/v1` routes need
//! `Authorization: Bearer <token>`).

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

/// Asks the local service for /health over plain TCP: no HTTP client needed in the image.
fn health() -> bool {
    use std::io::{Read, Write};
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".into());
    let Ok(mut s) = std::net::TcpStream::connect(format!("127.0.0.1:{port}")) else {
        return false;
    };
    let _ = s.set_read_timeout(Some(std::time::Duration::from_secs(2)));
    if s.write_all(b"GET /health HTTP/1.0\r\nHost: localhost\r\n\r\n")
        .is_err()
    {
        return false;
    }
    let mut reply = String::new();
    let _ = s.read_to_string(&mut reply);
    reply.starts_with("HTTP/1.0 200") || reply.starts_with("HTTP/1.1 200")
}

/// Ctrl-C, or SIGTERM from `docker stop`.
async fn shutdown() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let term = async {
        if let Ok(mut s) = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            s.recv().await;
        }
    };
    #[cfg(not(unix))]
    let term = std::future::pending::<()>();
    tokio::select! {
        _ = ctrl_c => {},
        _ = term => {},
    }
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
        Some("health") => std::process::exit(if health() { 0 } else { 1 }),
        Some("serve") | None => {
            let port = std::env::var("PORT").unwrap_or_else(|_| "8080".into());
            let bind = std::env::var("BIND").unwrap_or_else(|_| "0.0.0.0".into());
            let token = std::env::var("ENGINE_TOKEN").ok();
            if token.as_deref().unwrap_or("").is_empty() {
                eprintln!(
                    "ENGINE_TOKEN is not set: /v1 routes are open to anyone who can reach the port"
                );
            }
            let app = router(Arc::new(AppState::new(load_catalog(), token)));
            let listener = tokio::net::TcpListener::bind(format!("{bind}:{port}"))
                .await
                .expect("bind the port");
            println!("xcabinet-engine listening on :{port}");
            axum::serve(listener, app)
                .with_graceful_shutdown(shutdown())
                .await
                .expect("serve");
        }
        Some(other) => {
            eprintln!("unknown command {other}: use `serve`, `health` or `demo <dir>`");
            std::process::exit(2);
        }
    }
}
