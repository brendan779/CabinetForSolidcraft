# Handoff to zcode (2026-11-01)

You are continuing CabinetForSolidcraft, a Rust cabinet engine + SwiftUI macOS app
("Cabinet Studio") that builds cabinets in SolidCraft over its control socket. Read
**CLAUDE.md** first (layout, commands, conventions) and **docs/BACKLOG.md** (the plan of
record: how to pick items, statuses, the log). The previous session (OpenCode, branch
`night/2026-11-01`) finished two items and left the next ones queued.

## Where things stand

- Branch `night/2026-11-01`, two commits on top of `main` (`b4f296f`):
  1. `WF-2c` — Blum SERVO-DRIVE models (drive units, bracket profile, distance bumpers)
     placed as vendor models at the datums measured on the owner's Blum session output;
     bracket-profile rows Z10T670AA/Z10T1143B with ICH ranges; per-front bumper counts.
  2. `ABI-1` — ABI Interiors handle import (the commit you are handed): 40 handle models
     in `vendor-cad/abi/` (STL, in the app's handle frame), 192 finish rows in
     `catalog/hardware/abi-handles.json`, `vendor.rs` indexes mesh-native models and
     inserts them into SolidCraft with `file.insert_mesh`, importer
     `scripts/import_abi_handles.py` (idempotent; `--dry-run` prints what it would do).
- `scripts/check.sh` is green at the time of handoff (fmt, clippy -D warnings, tests,
  app build). Run it before committing; one commit per backlog item, `<ID>: <what>`.
- Working tree also contains uncommitted changes if the handoff happened mid-item:
  check `git status` and the BACKLOG rows marked `doing`.

## Local-only data (never commit — .gitignore already excludes vendor-cad/)

- `vendor-cad/blum/` Blum STEP files + catalogue PDF; `vendor-cad/hafele/axilo/` Häfele;
  `vendor-cad/blum-sessions/2026-10-27-drawer-base.zip` (SEC-3's source: a screen
  recording, Blum's output PDF/STEP and the parts list as STEPs — extracted copy at
  `/private/var/folders/jb/hrn_pf9j6fs1m5m1_z5wjkkc0000gn/T/opencode/blum-session/`);
  `vendor-cad/abi-bim/` the ABI BIMobject library (440 products);
  `vendor-cad/abi/` the imported handle STLs + `import-report.json`.
- The mesh index `vendor-cad/mesh/index.json` holds absolute paths into this clone and
  268 models (228 STEP-converted + 40 ABI mesh-native). Re-run
  `vendor::prepare` (the app's Settings does this) after moving the repo.

## What to do next (in order)

1. **ABI-2 then ABI-3** (BACKLOG § "Hardware: ABI Interiors handles"): make the handle
   choice in the model cover ABI sizes and families (the generic `HandleSize` enum in
   `crates/cabinet-core/src/model.rs` only has Cc96..Cc448 + Knob; ABI needs 50, 57, 60,
   85, 98, 115, 138, 150, 200, 220, 250, 320, 350, 400, 500, 800 and a family/finish
   choice), then the picker in the app like the board picker: family tiles drawn from the
   meshes (`load_mesh`/`hardware_models` already exist over UniFFI), size chips, finish
   swatches from `finish_colour_rgb` (81 rows have no colour: their variant says
   "colour to verify"), the chosen finish's SKU/price into the BOM/quote. The owner wants
   this to feel like the board picker.
   Key facts: ABI names a pull by its screw centres; the STLs are already oriented with
   holes on X at ±centres/2, the door face at Y = 0, the handle towards −Y, so
   `cad_placements`' handle branch (which builds the frame from the screw bores) works
   unchanged once the builder emits an ABI hardware row. Insertion into SolidCraft goes
   through `hardware_inserts` (mesh path → `file.insert_mesh` — done).
2. **SEC-3** is queued but the owner said to hold it for the on-platform session; do not
   start it unless the owner asks. Its sources are ready (see the extracted session
   folder above; `docs/blum-checks.md` check 2 is to be written from Blum's PDF).
3. Otherwise pick work from BACKLOG.md per its own rules (first todo in the highest
   priority section, skip 👤/needs-human).

## Gotchas learned this session

- SolidCraft's CLI cannot convert STL→STEP ("mesh bodies can only be moved, measured and
  exported"); mesh models go in with `file.insert_mesh`. STEP conversion of new supplier
  STEP files happens in `vendor::prepare` (Settings pane) with
  `/Applications/SolidCraft.app/Contents/MacOS/solidcraft-cli`.
- The owner's Blum session STEP parses with the STEP assembly walker at
  `/private/var/folders/.../opencode/step_asm.py` (occurrence transforms; flip=False is
  the correct ITEM_DEFINED_TRANSFORMATION order for OCC files). Panel occurrences in it
  are unreliable; the PDF is authoritative for panels.
- ABI's IFC exports are Revit coarse-LOD faceted meshes, sometimes un-stitched
  multi-solid (the "Adjustable" pulls) and sold-as-pair files (Hegman Half Moon/Kurve);
  `import_abi_handles.py` handles both. The adjustable pulls' feet measure inset of the
  named centres (Kingsley 250 model spans 214): the named size wins, the measured span
  is recorded in the report.
- `scripts/check.sh` takes ~4 minutes (app build included). `cargo insta review` if a
  rule change moves the golden part lists.
- macOS layout rules in CLAUDE.md are load-bearing (AppKit throws); test Release UI
  under lldb per CLAUDE.md before committing app changes.

## Owner preferences

- The UI is the product: every option is a picture, not a number field. Liquid Glass,
  `.snappy` animations, system controls.
- Supplier facts live in `catalog/` with sources and `verified: false` (only the owner
  flips verification). Never commit supplier files, catalogue PDFs or images.
- Millimetres everywhere; part names stable across rebuilds; every new rule gets a test.
