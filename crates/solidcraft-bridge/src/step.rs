//! The job as one STEP file, for any CAD program: `solidcraft-cli` builds it headless, so SolidCraft
//! does not have to be open. Panels with their machining at the detail asked for; supplier hardware
//! models stay out, since their licence does not let us pass them on.

use std::path::Path;
use std::process::Command;

use cabinet_core::part::PartList;
use serde_json::json;

use crate::emit::{Detail, emit_cabinet};
use crate::{Error, Result};

/// The STEP file's bytes for `lists`. Blocking: a big job at Exact detail takes minutes.
pub fn job_step(lists: &[PartList], detail: Detail, cli: &Path) -> Result<Vec<u8>> {
    let commands: Vec<_> = lists
        .iter()
        .filter(|l| !l.parts.is_empty())
        .flat_map(|l| emit_cabinet(l, detail, None))
        .collect();
    let dir = std::env::temp_dir().join(format!("cabinet-step-{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    let script = dir.join("job.json");
    let out = dir.join("job.step");
    std::fs::write(&script, json!({ "commands": commands }).to_string())?;
    let done = Command::new(cli)
        .arg("run")
        .arg(&script)
        .arg("--out")
        .arg(&out)
        .arg("--quiet")
        .output();
    let result = done.map_err(Error::from).and_then(|o| {
        if o.status.success() && out.exists() {
            Ok(std::fs::read(&out)?)
        } else {
            Err(Error::Remote(format!(
                "making the STEP file: {}",
                String::from_utf8_lossy(&o.stderr).trim()
            )))
        }
    });
    let _ = std::fs::remove_dir_all(&dir);
    result
}
