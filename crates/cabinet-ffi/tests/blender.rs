//! Checks of the scene script inside Blender. Skipped unless `BLENDER` names its binary:
//!
//! `BLENDER=/Applications/Blender.app/Contents/MacOS/Blender cargo test -p cabinet-ffi --test blender`

use std::process::Command;

#[test]
fn the_scene_script_passes_its_checks_in_blender() {
    let Some(blender) = std::env::var_os("BLENDER") else {
        eprintln!("BLENDER not set: skipped");
        return;
    };
    let script = concat!(env!("CARGO_MANIFEST_DIR"), "/../../blender/test_scene.py");
    let out = Command::new(blender)
        .args([
            "-b",
            "--factory-startup",
            "--python-exit-code",
            "1",
            "-P",
            script,
        ])
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success() && text.contains("scene checks passed"),
        "{text}\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
}
