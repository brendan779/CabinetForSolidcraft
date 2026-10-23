//! The bridge to SolidCraft.
//!
//! [`Client`] speaks SolidCraft's control protocol (JSON lines on `127.0.0.1:PORT`, token auth,
//! see SolidCraft's `docs/control-protocol.md`). [`emit`] turns a part list into the engine
//! commands that build it. Both are deliberately thin: when SolidCraft's WASM plugin runtime ships,
//! the same command lists go through `engine.run` instead of the socket.

pub mod client;
pub mod emit;

pub use client::Client;
pub use emit::{Detail, Existing, emit_cabinet, find_existing, replace_cabinet};

/// The design metadata key this tool stores cabinet specs under (reverse-DNS, as SolidCraft
/// requires for other tools' data).
pub const METADATA_KEY: &str = "io.github.brendan779.cabinetforsolidcraft";

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("cannot reach SolidCraft on port {port}: {source}. Start it with `--control {port}`.")]
    Connect { port: u16, source: std::io::Error },
    #[error("cannot read the control token at {path}: {source}")]
    Token {
        path: String,
        source: std::io::Error,
    },
    #[error("connection: {0}")]
    Io(#[from] std::io::Error),
    #[error("bad reply: {0}")]
    Json(#[from] serde_json::Error),
    #[error("SolidCraft: {0}")]
    Remote(String),
}

pub type Result<T> = std::result::Result<T, Error>;
