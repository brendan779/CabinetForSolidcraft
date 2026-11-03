//! The Revit import script's helpers, which need Python but not Revit.

use std::process::Command;

#[test]
fn the_revit_import_helpers_pass_their_checks() {
    let script = concat!(env!("CARGO_MANIFEST_DIR"), "/../../revit/test_import.py");
    let Ok(out) = Command::new("python3").args(["-I", script]).output() else {
        eprintln!("no python3: skipped");
        return;
    };
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success() && text.contains("import checks passed"),
        "{text}\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
}
