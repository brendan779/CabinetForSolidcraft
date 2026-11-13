# XCabinet engine service

This fork of CabinetForSolidcraft is the engine behind XCabinet (codeX). The cabinet core is
unchanged; `crates/engine-api` puts it behind HTTP so XCabinet's web platform and factory app can
use it. The Mac app and SolidCraft are not needed.

## Run it

```bash
cargo run -p engine-api -- demo out/          # sample kitchen → out/ (PDFs, CSVs, DXFs)
cargo run -p engine-api -- serve              # HTTP on :8080 (PORT to change)
docker build -t xcabinet-engine . && docker run -p 8080:8080 xcabinet-engine
```

`CATALOG_DIR` points at the board catalog (default `catalog`). Boards it doesn't know nest on a
2800 × 2070 mm sheet.

## Routes

| Route | What |
|---|---|
| `GET /health` | `ok` |
| `GET /v1/sample` | The sample kitchen as a run, to post back or edit |
| `POST /v1/build` | Body: a run (JSON). Returns counts, clashes, builder warnings, cut list, hardware list, sheets, part lists |
| `POST /v1/build/files` | Body: a run. Returns a zip: `cutlist.csv`, `hardware_bom.csv`, `cut_sheets.pdf`, `labels.pdf`, `assembly.pdf`, `nesting.pdf`, `elevation.pdf`, `dxf/<part>-<face>.dxf` |

Query options on both build routes: `job`, `tape` (edge tape mm, default 1), `kerf`, `trim`,
`router` (`true` nests for a CNC router).

A body that isn't a run gets `422` with `{"error": "not a valid run: …"}`.

## Deploy

`.github/workflows/engine-service.yml` tests the core crates and the service on every push and
pull request. Pushes to `main` and `staging` publish `ghcr.io/codex-sourcexgroup/xcabinet-engine`
tagged with the branch and the commit. Staging pulls the `staging` tag.

## Before a real cut

Supplier numbers in `catalog/` are not yet checked against Blum and Häfele documents (see the
upstream README). Check them, and cut a test part on the factory's machine, before production.
