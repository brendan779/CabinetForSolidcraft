# Backlog

The master list of everything left to build. **This file is the plan of record.** Overnight agents
pick work from it, and people add to it. Read [OVERNIGHT.md](OVERNIGHT.md) before working from it.

## How to use this file

- Each item has an **id**, a **status**, a **size** (S ≤ 2 h, M ≤ 1 day, L = split it first), and
  **Done when** criteria. Work is finished only when those criteria are met and `scripts/check.sh`
  is green.
- Status: `todo` · `doing (agent/date)` · `blocked (why)` · `done (commit)` · `needs-human (why)`.
- Pick the **first `todo` item in the highest-priority section whose dependencies are done**. Items
  marked 👤 need the owner (credentials, supplier files, judgement calls). Agents skip them.
- New work you discover: add it under the right section with a new id. Don't renumber existing
  ids.
- When an item is too big, split it into new ids and mark the parent `split → A, B`.

Priorities: **P0** blocks most other work · **P1** MVP · **P2** commercial parity · **P3** later.

---

## P0 — Foundations

| id | status | size | task | done when |
|---|---|---|---|---|
| INF-1 | done (initial) | M | Cargo workspace, crates, SwiftUI app shell, build-core.sh, check.sh | `scripts/check.sh` green |
| INF-2 | done (e4e904d) | S | GitHub Actions CI on `macos-latest`: `scripts/check.sh` | CI badge green on main |
| INF-3 | done (night/2026-10-23) | S | Move the Swift app to Swift 6 language mode with strict concurrency | builds with `SWIFT_VERSION = 6`, no warnings |
| INF-4 | done (night/2026-10-23) | M | Snapshot tests for SVG cut sheets (insta crate) and a golden part list per cabinet kind | `cargo insta test` passes; goldens committed |
| INF-5 | done (night/2026-10-23) | S | `cargo deny` licence/advisory check in check.sh | added and green |
| PERF-1 | done (night/2026-10-23) | M | **Batch machining per panel.** Emit all bores on one face of a panel as one sketch (circles) + one extrude cut instead of one `solid.cylinder` cut each. Tall cabinet at Manufacturing detail currently takes ~121 s for 293 commands | Tall cabinet at Manufacturing detail builds in < 10 s against SolidCraft 0.20; timing recorded in docs/perf.md |
| PERF-2 | done (night/2026-10-23) | S | Measure and record per-command cost (`engine.batch` `profile: true`) for box, cut box, cylinder cut and sketch cut on 16 mm panels | table in docs/perf.md |
| SC-1 | done (night/2026-10-23) | M | Replace-in-place: when re-sending a cabinet, delete its component and rebuild in one batch (currently two requests) and keep its placement if the user moved it in SolidCraft | resend after a move keeps the new position |
| SC-2 | needs-human (built in night/2026-10-23 and tested headless; needs a try in the running app: select 3 boxes, SolidCraft menu › Make Cabinets from Selected Boxes) | M | **Box → cabinet.** Read selected bodies from SolidCraft (`document.inspect` selection + `model.faces`), recognise axis-aligned cuboids, turn each into a cabinet at that size and position, hide the source box | selecting 3 boxes in SolidCraft and pressing "Make cabinets" in the app creates 3 cabinets in place |
| SC-3 | needs-human (built in night/2026-10-23 and tested headless; needs a try in the running app: SolidCraft menu › Load Cabinets from Design) | S | Read cabinets back from a design: list components carrying `io.github.brendan779.cabinetforsolidcraft` metadata and load their spec into the app | open a design built earlier and the app shows its cabinets |
| SC-4 | needs-human (built in night/2026-10-23; needs a try with SolidCraft installed: quit it, then SolidCraft menu › Open SolidCraft or the status bar link) | S | Detect SolidCraft not running / wrong port and offer "Open SolidCraft with control port" (launch `/Applications/SolidCraft.app` with `--control 7878`) | button launches it and the status dot turns green |

## P1 — MVP: cabinets, joinery, Blum basics, cut list, cut sheets

### Core model

| id | status | size | task | done when |
|---|---|---|---|---|
| CORE-1 | done (night/2026-10-23) | M | Integrated toe-kick notch: verify the groove op cuts the notch in SolidCraft and in sheets | screenshot + sheet show the notch |
| CORE-2 | done (night/2026-10-23) | M | Fixed shelves (biscuited + screwed like a bottom) as an option per shelf | test: fixed shelf has joints both sides |
| CORE-3 | done (night/2026-10-23) | M | Wall cabinet: hanging rail / wall-hanger hardware option and back recess | wall kind builds with hanger hardware placements |
| CORE-4 | split → CORE-4a, CORE-4b, CORE-4c | L | Corner cabinets: blind corner base, L-corner base, corner wall | split into CORE-4a/b/c |
| CORE-4a | done (night/2026-10-23) | M | Blind corner base: a base carcass with a fixed blind panel on the blind side (width = run-in − door), a filler/stile against the neighbouring run, door on the open side; `blind_side` and `blind_width` in the model | a 1000 mm blind corner builds with blind panel + filler + one door; test on the open width |
| CORE-4b | needs-human (which door: bi-fold pair on two fronts, or one diagonal door? For bi-fold, which hinge system (e.g. Blum CLIP top 170° + bi-fold connector)? OK to model L-shaped bottom/top/shelves as one rectangular blank with a through notch, as the toe-kick notch is?) | M | L-shaped corner base (diagonal or bi-fold door): two carcass legs sharing a corner, parts not axis-aligned to one front; needs a second front direction in the part model | an L-corner 900 × 900 builds; cut list has every panel once |
| CORE-4c | blocked (CORE-4b) | M | Corner wall cabinet (diagonal front or L): as CORE-4b at wall height with hangers (CORE-3) | builds with two hangers per leg |
| CORE-5 | done (night/2026-10-23) | M | Sink cabinet (no bottom rail, back cut-out option, waterproof base liner option) | builds; liner appears in BOM |
| CORE-6 | todo | M | Fillers, end panels and scribes as parts of a run | a run with an end panel each side builds |
| CORE-7 | todo | M | Runs: shared construction standard per project, per-cabinet overrides only | project JSON stores one standard + overrides |
| CORE-8 | todo | M | Drawer boxes for wood drawers on runners (sides, front, back, bottom) from Blum MOVENTO/TANDEM rules: inside width = opening − 42, length = NL − 10 (verify against Blum TD) | parts + cut list for a 3-drawer bank |
| CORE-9 | todo | S | Material per part: carcass board, front board, back board from the catalog; thickness follows the board | changing the front board changes door thickness |
| CORE-10 | todo | M | Half-overlay and inset hinge geometry (cranked hinges, plate offsets) | test per overlay type |
| CORE-11 | todo | S | Unit display: metric with optional imperial display in the app | toggle in preferences |
| CORE-12 | todo | M | Collision checker as a module: every bore/slot vs every other op and vs panel edges, with warnings | test suite over all kinds × standards with zero clashes |

### Joinery

| id | status | size | task | done when |
|---|---|---|---|---|
| JOIN-1 | done (initial) | M | Biscuits (#0/#10/#20) + screws between, density presets, short joints screwed | tests in joinery.rs |
| JOIN-2 | todo | M | Real biscuit slot shape (arc ends from the 100 mm cutter) instead of a box | slot volume matches analytic value ±1 % |
| JOIN-3 | todo | S | Biscuit bodies as solids (lens shape) in Manufacturing detail, coloured | visible in SolidCraft |
| JOIN-4 | todo | M | Cam-lock (Häfele Minifix 15) bores: Ø15 cam, Ø8/Ø5 bolt, from verified Häfele data | cut sheet shows cam bores |
| JOIN-5 | todo | S | Screw countersinks and cover caps; "hide screws on visible end panels" rule | end panels have no through screws |
| JOIN-6 | todo | M | Lamello Clamex P-14 / Tenso P-14 P-System slots | option in picker; slots on sheets |
| JOIN-7 | todo | S | Back panel fixing (screws at 150 mm centres for applied backs; none for grooved) | ops present per back style |

### Hardware: Blum

👤 = needs the Blum files the owner will download into `vendor-cad/blum/`.

| id | status | size | task | done when |
|---|---|---|---|---|
| BLUM-1 | todo | M | Hardware catalog schema (`catalog/hardware/*.json`): article, family, variant, parameters, drilling pattern, source, verified | schema + loader + tests |
| BLUM-2 | needs-human (Blum TD sheets) | M | CLIP top BLUMOTION 110° data: cup Ø35, depth, boring distance range, plate drilling (cruciform/wing, 0/3/9 mm), overlay tables | rows verified against the Blum document; build.rs constants replaced by catalog data |
| BLUM-3 | todo | M | Hinge selection rule: overlay + door thickness → hinge + plate article | unit test table |
| BLUM-4 | todo | M | LEGRABOX: nominal lengths, side heights (N/M/K/C/F), front fixing drilling, bottom/back cut sizes | drawer bank with LEGRABOX gives bottom and back parts in cut list |
| BLUM-5 | todo | M | MERIVOBOX and TANDEMBOX as alternatives | picker shows 3 box systems |
| BLUM-6 | todo | M | MOVENTO/TANDEM runners for wood drawers (see CORE-8) | — |
| BLUM-7 | todo | L | AVENTOS lift systems (HK top, HK-S, HK-XS, HF, HS, HL): power factor = f(front height, weight) → article | split |
| BLUM-8 | todo | S | TIP-ON / SERVO-DRIVE options for handleless fronts | option + BOM |
| BLUM-9 | 👤 needs-human | M | Import the owner's Blum STEP files from `vendor-cad/blum/` as linked components (`component.insert_design` after STEP → .solidcraft conversion) in place of stand-ins | a hinge renders with the real Blum model |
| BLUM-10 | todo | M | Parametric stand-in models (hinge cup + arm, plate, runner, box side) emitted as bodies | visible in Manufacturing detail |

### Hardware: Häfele and lighting

| id | status | size | task | done when |
|---|---|---|---|---|
| HAF-1 | todo | M | Adjustable legs + plinth clips (count rule by width), from Häfele data | BOM rows with article numbers |
| HAF-2 | todo | M | Handles: catalog of CC spacings; placement picker (top corner, centre, edge pull) drawn on the door | handle bores on door sheets |
| HAF-3 | todo | M | Shelf supports (5 mm pins) article + count | BOM |
| HAF-4 | needs-human (Häfele data) | L | Loox5 lighting: profiles (surface, recessed, shelf-edge), 12 V/24 V strips, drivers, distributors, switches/sensors | split |
| HAF-5 | todo | M | Lighting rules: no mixing 12/24 V; ≤ 80 % driver load; ports per distributor; cable holes Ø10 through panels added as ops | unit tests for each rule |
| HAF-6 | todo | M | Lighting placement UI: click under-cabinet / in-cabinet / plinth zones on a cabinet drawing | picker in inspector |
| HAF-7 | 👤 needs-human | M | Import the owner's Häfele STEP files (`vendor-cad/hafele/`) like BLUM-9 | — |
| HAF-8 | todo | S | Wall hanger + wall rail articles for the generic `wall-hanger.concealed` / `wall-hanger.rail` placements (CORE-3): pick a system (Häfele/Camar), its drilling and rail lengths, from the supplier's data | BOM rows with article numbers; hanger drilling ops |

### Board and edge tape data (Australia)

Sources must be the brand's own current documents. Every record keeps `source.url`,
`document_date` and `verified: false` until the owner or a check confirms it. No swatch images are
committed: they are the brands' copyright. `preview_colour` is an approximation.

| id | status | size | task | done when |
|---|---|---|---|---|
| DATA-1 | done (night/2026-10-23) | M | **Polytec parser.** `tools/parse_polytec.py`: download the Availability Guide PDF, `pdftotext -layout`, and parse every board table (decor, finish, thickness, substrate, sheet sizes, item codes) and matching edge tape codes (1×21, 1×29, roll lengths) into `catalog/boards/polytec.json` | ≥ 90 % of the guide's decorative 16/18 mm rows parsed; spot-check 20 random rows against the PDF and record them in the PR |
| DATA-2 | todo | M | Laminex: decorative board (Laminex Colour Collection, Natural Wood, Essastone is out of scope), sizes, substrates, matching edges, from laminex.com.au availability documents | `catalog/boards/laminex.json` with sources |
| DATA-3 | todo | M | Other Australian brands: Egger (AU range), Nikpol (Sonae Arauco/Egger/Fundermax distribution), Formica AU, Bestwood, Briggs Veneers, Navurban, Karaboard/Wilsonart | one file per brand |
| DATA-4 | todo | M | Edge tape suppliers: Surteco, Dölken (Doellken), Rehau, Titus Tekform, Häfele AU edging, Fastedge; brand match tables (tape code ↔ board decor) | `catalog/edge-tape/*.json` |
| DATA-5 | todo | M | Specialty boards: HMR, compact laminate, plywood (Hoop pine, birch), MR MDF raw for paint, thermolaminates, high-gloss acrylic | records with `kind` set |
| DATA-6 | todo | M | Linings: Polytec wall linings / Steccawood / Easycraft, Laminex Surround: profile, module width, length, thickness (`kind: lining`) | records + a lining picker stub |
| DATA-7 | todo | S | Catalog validation in CI: ids unique, tapes resolve, each brand file has a source | part of check.sh (already partly) |
| DATA-8 | todo | M | Board picker in the app: brand → range → decor grid with grain and finish filters; assign to carcass/fronts/backs | picking a decor updates the 3D colour and the cut list |
| DATA-9 | 👤 needs-human | S | Owner verifies a sample of rows per brand; flip `verified: true` | — |
| DATA-10 | todo | M | Polytec guide, the rest: COMMERCIAL board + edge tape (guide p14-17), Internal Carcass tape widths (the table prints "1mm Rigid" without a width), wall linings (DATA-6) | records with sources; parse_polytec.py tests |

### Outputs

| id | status | size | task | done when |
|---|---|---|---|---|
| OUT-1 | done (initial) | M | Cut list rows + CSV; per-part SVG cut sheets | tests |
| OUT-2 | todo | M | Cut sheets as multi-page PDF (one part per page plus cover and index) | PDF opens in Preview with N+2 pages |
| OUT-3 | todo | M | Both faces + edge views on sheets when a part has machining on more than one face | side sheet shows inner face and edges |
| OUT-4 | todo | M | Sheet quality: dimension chains from the reference corner for every hole row; hole table grouped by diameter/depth | reviewed by owner 👤 |
| OUT-5 | todo | L | Guillotine nesting optimiser with grain and kerf (panel saw), sheet diagrams in PDF, offcut list | split |
| OUT-6 | todo | M | Hardware BOM with supplier article numbers and quantities, CSV + PDF | BOM for the sample run |
| OUT-7 | todo | M | Part labels (62 mm label printer) with QR code of part id | PDF of labels |
| OUT-8 | todo | M | Assembly sheet per cabinet: exploded view (SolidCraft exploded view + export view) | image per cabinet |
| OUT-9 | todo | L | DXF per part for CNC with layer conventions (outline, bores by Ø/depth, grooves) | split |
| OUT-10 | todo | M | Renders: Render workspace setup (`render.settings.set`, `render.start`, `render.export`) from the app, PNG saved next to the job | "Render" button produces a PNG |

### macOS app (make it shine)

| id | status | size | task | done when |
|---|---|---|---|---|
| UI-1 | done (initial) | M | Split view: sidebar, RealityKit viewport, inspector with visual pickers, cut list table, sheet viewer, board library | app runs |
| UI-2 | todo | M | Document-based app (`.cabinetjob` files, autosave, versions, recent jobs) replacing the single Application Support file | File > New/Open/Save work |
| UI-3 | todo | M | Undo/redo for every edit (UndoManager) | ⌘Z reverts a picker change |
| UI-4 | todo | M | Viewport: selection outline/glow, hover highlight, part tooltip, exploded slider, section view, door open animation | each item visible in a screenshot |
| UI-5 | todo | M | Viewport materials from board preview colours; woodgrain procedural texture for grained decors | oak fronts look like oak |
| UI-6 | todo | S | Status dot visible in the SolidCraft toolbar menu (currently hidden by the menu label) | screenshot |
| UI-7 | todo | M | Room planner: wall lengths, run along a wall, appliances as placeholders | split |
| UI-8 | todo | M | Drag to reorder and resize cabinets in the viewport (width handles snap to presets) | drag changes width |
| UI-9 | todo | S | Keyboard: ⌘1–4 workspaces, arrows to step through cabinets, ⌫ delete | works |
| UI-10 | todo | M | Onboarding: first launch connects to SolidCraft, explains Design vs Manufacturing detail | first-run sheet |
| UI-11 | todo | S | App icon and accent colour | icon in Dock |
| UI-12 | todo | M | Preferences: SolidCraft port, default standard, units, catalog folder, vendor-cad folder | Settings scene |
| UI-13 | todo | M | Construction standard editor: save/load named standards ("My shop") as visual presets | standards menu |
| UI-14 | todo | S | Accessibility pass: VoiceOver labels on every drawn tile, Dynamic Type sizes | Accessibility Inspector clean |
| UI-15 | todo | M | UI tests (XCUITest) for add cabinet, change option, export CSV | tests pass in check.sh |

## P2 — Commercial parity

| id | status | size | task |
|---|---|---|---|
| P2-1 | todo | L | Face-frame construction (US style) |
| P2-2 | todo | L | Shaker / 5-piece doors, glass frames, aluminium frame doors (Polytec Alu frames) |
| P2-3 | todo | L | Wardrobes: hanging rails, drawers, shoe shelves, sliding doors |
| P2-4 | todo | L | Benchtops, cornices, light rails, panels with scribes |
| P2-5 | todo | L | Quoting: material + hardware + labour rates |
| P2-6 | todo | L | CNC post-processors (MPR, CIX), BXF exchange with Blum (needs Blum spec 👤) |
| P2-7 | todo | L | True-shape nesting for CNC |

## P3 — SolidCraft plugin

| id | status | size | task |
|---|---|---|---|
| PLG-1 | blocked (SolidCraft has no plugin runtime yet; check #1221/#4980 weekly) | L | Port the bridge to a `command-plugin` WASM component (same commands via `engine.run`) |
| PLG-2 | blocked (PLG-1) | L | `panel-plugin` with pins; propose an image-choice widget upstream so pickers can live in SolidCraft |
| PLG-3 | todo | S | Weekly check: has SolidCraft merged plugin runtime work? Update this row and docs/FEASIBILITY_PLAN.md |

## Log

Agents append one line per session: date, items touched, outcome.

- 2026-10-23 — initial scaffold: core, catalog, bridge, sheets, FFI, macOS app; live build in SolidCraft 0.20 verified; PERF-1 found.
- 2026-10-23 — repo published as brendan779/CabinetForSolidcraft; INF-2 CI green on macos-latest.
- 2026-10-23 (night) — PERF-2 done: per-command costs in docs/perf.md (cylinder cuts grow ~3 ms per existing feature; one sketch extrude is 11–33× faster).
- 2026-10-23 (night) — PERF-1 done: bores batched per panel face (sketch + one extrude); tall cabinet at Manufacturing detail 88.1 s → 3.9 s headless, volumes unchanged. Connected (app) timing still to confirm.
- 2026-10-23 (night) — DATA-1 done: tools/parse_polytec.py reads the DECORATIVE board tables (9-32 mm) from word positions; 459 boards, 717 tapes; 263/263 decorative 16/18 mm rows with board codes parsed and cross-checked against `pdftotext -layout`. DATA-10 added for the rest of the guide.
- 2026-10-23 (night) — INF-4 done: insta golden part list per cabinet kind and SVG snapshots of every base cabinet sheet; `cargo insta test --check` passes.
- 2026-10-23 (night) — INF-5 done: `cargo deny check` (licences, advisories, sources) in check.sh and CI; uniffi is MPL-2.0 and its basic-toml advisory is ignored with a reason.
- 2026-10-23 (night) — INF-3 done: Swift 6 language mode with complete strict concurrency, zero warnings (Xcode 26.6). App launched and quit cleanly; no screenshot (this Mac has not granted Screen Recording).
- 2026-10-23 (night) — SC-1 done: resend = one batch (delete + rebuild + re-apply the user's move read from `component.list`); verified headless by `tests/headless.rs` (bounding boxes equal after a turn + move). Not yet tried against the running app.
- 2026-10-23 (night) — SC-2 built: box recogniser (`model.faces`), `Cabinet::from_box` (kind from size/height), FFI + "Make Cabinets from Selected Boxes" in the SolidCraft menu; headless test turns 3 drawn boxes into base/tall/wall cabinets in place. Selection reading and hiding the boxes are untested: no SolidCraft app on this Mac.
- 2026-10-23 (night) — SC-3 built: `design::cabinets_in_design` reads every component's spec under our metadata key; "Load Cabinets from Design" merges them by id. Headless test round-trips two cabinets and ignores a foreign component; the app path is untested here.
- 2026-10-23 (night) — SC-4 built: when SolidCraft does not answer, the SolidCraft menu and the status bar offer Open SolidCraft (launches /Applications/SolidCraft.app with `--control 7878`, waits up to 30 s for the dot to turn green; explains when SolidCraft is missing or already open without the port). Untested: SolidCraft is not installed on this Mac.
- 2026-10-23 (night) — CORE-1 done: headless test shows SolidCraft cuts exactly the notch volume from both sides; sheets label a through groove as a notch and draw it as a cut-out. Pictures: docs/images/core1-toe-kick-notch.png, core1-side-sheet.png.
- 2026-10-23 (night) — CORE-2 done: `fixed_shelves` (per shelf number): full width, to the back, biscuited + screwed to both sides; no shelf pins within a pitch of it. Inspector shows a lock chip per shelf.
- 2026-10-23 (night) — CORE-3 done: `wall_hanging` standard option (hangers + rail / none) with a picker drawing; wall cabinets get two hangers in the top back corners (void behind the inset back) and a rail; other backs warn. Article numbers wait for HAF-8 (added).
- 2026-10-23 (night) — CORE-4 split into CORE-4a (blind corner base), CORE-4b (L-corner base), CORE-4c (corner wall).
- 2026-10-23 (night) — CORE-4a done: `blind_corner` kind (1000 wide): blind panel + filler from the blind side (`blind.width` 630, `filler` 50 by default), one door on the open width hinged away from the corner; too-wide blinds are refused. Inspector "Blind corner" section with drawn side tiles and width chips.
- 2026-10-23 (night) — CORE-4b set to needs-human with the design questions (door type, hinge system, notched blanks); CORE-4c blocked on it.
- 2026-10-23 (night) — CORE-5 done: `sink` kind (900 wide, door pair, no shelf): no back rail ("no bottom rail" read as the back top rail the bowl would hit — owner to confirm), optional pipe cut-out through the back (400 high, 50 in from the sides) and waterproof liner in the hardware list.
