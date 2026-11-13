# XCabinet engine service

This fork of CabinetForSolidcraft is the engine behind XCabinet (codeX). The cabinet core is
unchanged; `crates/engine-api` puts it behind HTTP so XCabinet's web platform and factory app can
use it. The Mac app and SolidCraft are not needed.

## Run it

```bash
cargo run -p engine-api -- demo out/          # sample kitchen → out/ (PDFs, CSVs, DXFs)
cargo run -p engine-api -- serve              # HTTP on :8080 (PORT to change)
docker build -t xcabinet-engine . && docker run -p 127.0.0.1:8080:8080 -e ENGINE_TOKEN=... xcabinet-engine
cargo run -p engine-api -- health             # exit 0 when the local service answers (Docker HEALTHCHECK)
```

| Setting | Meaning |
|---|---|
| `PORT` | Port to listen on (default 8080) |
| `BIND` | Address to listen on (default 0.0.0.0, needed inside a container) |
| `CATALOG_DIR` | Board catalog (default `catalog`). Boards it doesn't know nest on a 2800 × 2070 mm sheet |
| `ENGINE_TOKEN` | When set, `/v1` routes need `Authorization: Bearer <token>`. Always set it on a server |

The service has no users of its own: keep it on a private network that only XCabinet's API can
reach, and set `ENGINE_TOKEN`.

## Routes

| Route | What |
|---|---|
| `GET /health` | `ok` |
| `GET /v1/sample` | The sample kitchen as a run, to post back or edit |
| `POST /v1/build` | Body: a run (JSON). Returns counts, clashes, builder warnings, cut list, hardware list, sheets, part lists |
| `POST /v1/build/files` | Body: a run. Returns a zip: `cutlist.csv`, `hardware_bom.csv`, `cut_sheets.pdf`, `labels.pdf`, `assembly.pdf`, `nesting.pdf`, `dxf/<part>.dxf`, and `elevation.pdf` when the run has cabinets to draw |

Query options on both build routes: `job`, `tape` (edge tape mm, default 1), `kerf`, `trim`,
`router` (`true` nests for a CNC router).

Every refusal is JSON, `{"error": "…"}`, saying what is wrong:

| Status | When |
|---|---|
| `400` | The body isn't UTF-8 text |
| `401` | `ENGINE_TOKEN` is set and the request doesn't carry it |
| `413` | Body over 2 MB |
| `422` | Not a run; a number not finite or beyond ±20,000 mm; a cabinet's width, depth or height not in (0, 5000] mm; over 100 items, 12 drawers, 20 shelves in a cabinet or niche, or 100 entries in any list; a tile under 50 mm (give mosaic as one sheet-sized tile) or grout over 20 mm; a run that makes over 2,000 parts; an id that isn't 1–32 letters, digits, `.`, `_`, `-`, or one used twice; `tape` not 0–5, `kerf` not 0–10, `trim` not 0–50, a job name over 120 characters |
| `503` | No build slot free within 20 s (one build per core runs at once), or a build over 60 s |

## Deploy

`.github/workflows/engine-service.yml` tests the core crates and the service on every push and
pull request. Pushes to `main` and `staging` publish `ghcr.io/codex-sourcexgroup/xcabinet-engine`
tagged with the branch and the commit. Staging pulls the `staging` tag.

## Before a real cut

Supplier numbers in `catalog/` are not yet checked against Blum and Häfele documents (see the
upstream README). Check them, and cut a test part on the factory's machine, before production.
