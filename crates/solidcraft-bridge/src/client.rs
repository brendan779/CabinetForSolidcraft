//! A blocking client for SolidCraft's control protocol.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::time::Duration;

use serde_json::{Value, json};

use crate::{Error, Result};

pub const DEFAULT_PORT: u16 = 7878;

pub struct Client {
    reader: BufReader<TcpStream>,
    writer: TcpStream,
    next_id: u64,
    token: Option<String>,
}

/// Where SolidCraft writes the token for `port` on this platform.
pub fn token_path(port: u16) -> PathBuf {
    if let Ok(dir) = std::env::var("SOLIDCRAFT_CONTROL_DIR") {
        return PathBuf::from(dir).join(format!("{port}.token"));
    }
    let home = std::env::var("HOME").unwrap_or_default();
    let base = if cfg!(target_os = "macos") {
        PathBuf::from(home).join("Library/Application Support/SolidCraft")
    } else if cfg!(windows) {
        PathBuf::from(std::env::var("LOCALAPPDATA").unwrap_or_default()).join("SolidCraft")
    } else {
        std::env::var("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(home).join(".local/share"))
            .join("solidcraft")
    };
    base.join("control").join(format!("{port}.token"))
}

impl Client {
    /// Connects to the app on `port`, reading the token from `SOLIDCRAFT_CONTROL_TOKEN` or the
    /// app's token file.
    pub fn connect(port: u16) -> Result<Self> {
        let token = match std::env::var("SOLIDCRAFT_CONTROL_TOKEN") {
            Ok(t) => t,
            Err(_) => {
                let path = token_path(port);
                std::fs::read_to_string(&path)
                    .map_err(|source| Error::Token {
                        path: path.display().to_string(),
                        source,
                    })?
                    .trim()
                    .to_string()
            }
        };
        let stream = TcpStream::connect(("127.0.0.1", port))
            .map_err(|source| Error::Connect { port, source })?;
        // engine.batch may run for up to an hour; the app answers when done.
        stream.set_read_timeout(Some(Duration::from_secs(3700)))?;
        let writer = stream.try_clone()?;
        Ok(Self {
            reader: BufReader::new(stream),
            writer,
            next_id: 1,
            token: Some(token),
        })
    }

    /// Sends one request and waits for its reply's `result`.
    pub fn request(&mut self, method: &str, params: Value) -> Result<Value> {
        let id = self.next_id;
        self.next_id += 1;
        let mut req = json!({ "id": id, "method": method, "params": params });
        if let Some(t) = self.token.take() {
            req["token"] = Value::String(t);
        }
        let mut line = serde_json::to_string(&req)?;
        line.push('\n');
        self.writer.write_all(line.as_bytes())?;
        self.writer.flush()?;
        let mut reply = String::new();
        if self.reader.read_line(&mut reply)? == 0 {
            return Err(Error::Remote("the app closed the connection".into()));
        }
        let v: Value = serde_json::from_str(&reply)?;
        if v.get("ok").and_then(Value::as_bool) == Some(true) {
            Ok(v.get("result").cloned().unwrap_or(Value::Null))
        } else {
            let msg = v
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("unknown error");
            Err(Error::Remote(msg.to_string()))
        }
    }

    /// Runs one engine command.
    pub fn execute(&mut self, command: &str, params: Value) -> Result<Value> {
        self.request(
            "engine.execute",
            json!({ "command": command, "params": params }),
        )
    }

    /// Runs commands as one all-or-nothing batch. A failed step is reported as an error naming it.
    pub fn batch(&mut self, commands: &[Value]) -> Result<Value> {
        let r = self.request(
            "engine.batch",
            json!({ "commands": commands, "timings": true }),
        )?;
        if let Some(f) = r.get("failed") {
            return Err(Error::Remote(format!("batch step failed: {f}")));
        }
        Ok(r)
    }

    pub fn inspect(&mut self) -> Result<Value> {
        self.request("document.inspect", json!({ "measure": true }))
    }
}
