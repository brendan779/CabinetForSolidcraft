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
| INF-3 | done (cd8b363) | S | Move the Swift app to Swift 6 language mode with strict concurrency | builds with `SWIFT_VERSION = 6`, no warnings |
| INF-4 | done (e8d2eb3) | M | Snapshot tests for SVG cut sheets (insta crate) and a golden part list per cabinet kind | `cargo insta test` passes; goldens committed |
| INF-5 | done (d4452e9) | S | `cargo deny` licence/advisory check in check.sh | added and green |
| PERF-1 | done (67aac6d) | M | **Batch machining per panel.** Emit all bores on one face of a panel as one sketch (circles) + one extrude cut instead of one `solid.cylinder` cut each. Tall cabinet at Manufacturing detail currently takes ~121 s for 293 commands | Tall cabinet at Manufacturing detail builds in < 10 s against SolidCraft 0.20; timing recorded in docs/perf.md |
| PERF-2 | done (e8ffb58) | S | Measure and record per-command cost (`engine.batch` `profile: true`) for box, cut box, cylinder cut and sketch cut on 16 mm panels | table in docs/perf.md |
| SC-1 | done (01b8daf) | M | Replace-in-place: when re-sending a cabinet, delete its component and rebuild in one batch (currently two requests) and keep its placement if the user moved it in SolidCraft | resend after a move keeps the new position |
| SC-2 | needs-human (built in 90b3953; the FFI path now passes live against a running SolidCraft 0.20: 3 selected boxes become base/tall/wall cabinets in place and the boxes hide (`cabinet-ffi/tests/live.rs`). Left to try: the menu click in Cabinet Studio) | M | **Box → cabinet.** Read selected bodies from SolidCraft (`document.inspect` selection + `model.faces`), recognise axis-aligned cuboids, turn each into a cabinet at that size and position, hide the source box | selecting 3 boxes in SolidCraft and pressing "Make cabinets" in the app creates 3 cabinets in place |
| SC-3 | needs-human (built in 3292595; live test reads the built cabinets back from a running SolidCraft identical to what was sent. Left to try: the menu click) | S | Read cabinets back from a design: list components carrying `io.github.brendan779.cabinetforsolidcraft` metadata and load their spec into the app | open a design built earlier and the app shows its cabinets |
| SC-4 | needs-human (built in bd96cff; SolidCraft 0.20 started with `--control 7878` answers on the port within 2 s on this Mac. Left to try: the menu button) | S | Detect SolidCraft not running / wrong port and offer "Open SolidCraft with control port" (launch `/Applications/SolidCraft.app` with `--control 7878`) | button launches it and the status dot turns green |

## P1 — MVP: cabinets, joinery, Blum basics, cut list, cut sheets

### Core model

| id | status | size | task | done when |
|---|---|---|---|---|
| CORE-1 | done (d94c3e3) | M | Integrated toe-kick notch: verify the groove op cuts the notch in SolidCraft and in sheets | screenshot + sheet show the notch |
| CORE-2 | done (e0b3667) | M | Fixed shelves (biscuited + screwed like a bottom) as an option per shelf | test: fixed shelf has joints both sides |
| CORE-3 | done (4c42ae7) | M | Wall cabinet: hanging rail / wall-hanger hardware option and back recess | wall kind builds with hanger hardware placements |
| CORE-4 | split → CORE-4a, CORE-4b, CORE-4c | L | Corner cabinets: blind corner base, L-corner base, corner wall | split into CORE-4a/b/c |
| CORE-4a | done (ff4fd55) | M | Blind corner base: a base carcass with a fixed blind panel on the blind side (width = run-in − door), a filler/stile against the neighbouring run, door on the open side; `blind_side` and `blind_width` in the model | a 1000 mm blind corner builds with blind panel + filler + one door; test on the open width |
| CORE-4b | done (0aaca7c) | M | L-shaped corner base (diagonal or bi-fold door): two carcass legs sharing a corner, parts not axis-aligned to one front; needs a second front direction in the part model | an L-corner 900 × 900 builds; cut list has every panel once |
| CORE-4d | done (owner 2026-10-26: keep Blum's corner bi-fold hinge 79T8500 + 155° hinges) | S | Bi-fold corner hinge article and its drilling, replacing the generic placement | hinge row verified against the supplier's document |
| CORE-4c | done (f791844) | M | Corner wall cabinet (diagonal front or L): as CORE-4b at wall height with hangers (CORE-3) | builds with two hangers per leg |
| CORE-5 | done (owner 2026-10-26 doesn't know; keep dropping the back rail, as built in 4bc5e17) | M | Sink cabinet (no bottom rail, back cut-out option, waterproof base liner option) | builds; liner appears in BOM |
| CORE-6 | done (ed3f43a) | M | Fillers, end panels and scribes as parts of a run | a run with an end panel each side builds |
| CORE-7 | done (43ead80) | M | Runs: shared construction standard per project, per-cabinet overrides only | project JSON stores one standard + overrides |
| CORE-8 | done (f910aaf) | M | Drawer boxes for wood drawers on runners (sides, front, back, bottom) from Blum MOVENTO/TANDEM rules: inside width = opening − 42, length = NL − 10 (verify against Blum TD) | parts + cut list for a 3-drawer bank |
| CORE-9 | done (6b515ee) | S | Material per part: carcass board, front board, back board from the catalog; thickness follows the board | changing the front board changes door thickness |
| CORE-10 | done (0f06768) | M | Half-overlay and inset hinge geometry (cranked hinges, plate offsets) | test per overlay type |
| CORE-11 | done (e3e3ad1) | S | Unit display: metric with optional imperial display in the app | toggle in preferences |
| CORE-12 | done (cee783c) | M | Collision checker as a module: every bore/slot vs every other op and vs panel edges, with warnings | test suite over all kinds × standards with zero clashes |

### Joinery

| id | status | size | task | done when |
|---|---|---|---|---|
| JOIN-1 | done (initial) | M | Biscuits (#0/#10/#20) + screws between, density presets, short joints screwed | tests in joinery.rs |
| JOIN-2 | done (2acc0d9; owner accepted: box slots at Manufacturing, lens slots opt-in at Exact) | M | Real biscuit slot shape (arc ends from the 100 mm cutter) instead of a box | slot volume matches analytic value ±1 % |
| JOIN-3 | done (67eaf55) | S | Biscuit bodies as solids (lens shape) in Manufacturing detail, coloured | visible in SolidCraft |
| JOIN-4 | needs-human (owner is collecting the documents; needs verified Häfele Minifix 15 drilling data in the catalog: owner-supplied, like HAF-4) | M | Cam-lock (Häfele Minifix 15) bores: Ø15 cam, Ø8/Ø5 bolt, from verified Häfele data | cut sheet shows cam bores |
| JOIN-5 | done (86c5db4) | S | Screw countersinks and cover caps; "hide screws on visible end panels" rule | end panels have no through screws |
| JOIN-6 | needs-human (owner is collecting the documents; needs Lamello P-System slot data from Lamello's documents, added to the hardware catalog after BLUM-1) | M | Lamello Clamex P-14 / Tenso P-14 P-System slots | option in picker; slots on sheets |
| JOIN-8 | done (6335b18) | M | Festool Domino loose-tenon mortises as a fixing recipe and picker choice: DF 500 (4/5/6/8/10 mm) and DF 700 (8/10/12/14 mm); mortise width/length/depth from Festool's published sizes as catalog data with a source and `verified: false`; mortises in both parts like biscuit slots, spacing and setback rules, picker thumbnail, cut sheet rendering | joint test per size; mortises on sheets |
| JOIN-9 | done (e322534) | M | Traditional fluted wooden dowels extending the 8 mm dowel recipe: 6/8/10 mm with length choices, a doweling-jig spacing pattern as well as the 32 mm grid, depth split between face and end parts, picker thumbnail | tests for spacing and depth split |
| JOIN-7 | done (d31e409) | S | Back panel fixing (screws at 150 mm centres for applied backs; none for grooved) | ops present per back style |

### Hardware: Blum

👤 = needs the Blum files the owner will download into `vendor-cad/blum/`.

| id | status | size | task | done when |
|---|---|---|---|---|
| BLUM-1 | done (56a5858) | M | Hardware catalog schema (`catalog/hardware/*.json`): article, family, variant, parameters, drilling pattern, source, verified | schema + loader + tests |
| BLUM-2 | done (8151121; data `verified: false`: owner to check the rows against pp. 76-77, 150-153) | M | CLIP top BLUMOTION 110° data: cup Ø35, depth, boring distance range, plate drilling (cruciform/wing, 0/3/9 mm), overlay tables | rows verified against the Blum document; build.rs constants replaced by catalog data |
| BLUM-3 | done (8151121) | M | Hinge selection rule: overlay + door thickness → hinge + plate article | unit test table |
| BLUM-4 | done (e66f505; drawers N/M/K; inner drawers, C/F pull-outs and the cabinet-profile side drilling are BLUM-4b) | M | LEGRABOX: nominal lengths, side heights (N/M/K/C/F), front fixing drilling, bottom/back cut sizes | drawer bank with LEGRABOX gives bottom and back parts in cut list |
| BLUM-4b | blocked (inner drawers and C/F pull-outs done in 7189be7; the cabinet-profile screw positions wait for the Blum CAD files, owner 2026-10-26) | M | LEGRABOX inner drawers (M/K with inner front piece), high fronted pull-outs C/F, and the cabinet-profile screw positions on the sides (p.248: positions along the profile are printed, their height on the side and which take system screws need reading from the drawings) | inner drawer builds; profile bores on the side sheets |
| BLUM-5 | done (79eb7d0, f86d090; METABOX not added) | M | MERIVOBOX and TANDEMBOX as alternatives | picker shows 3 box systems |
| BLUM-6 | done (f910aaf; runner side drilling is BLUM-6b) | M | MOVENTO/TANDEM runners for wood drawers (see CORE-8) | — |
| BLUM-6b | blocked (owner 2026-10-26: wait for the Blum CAD files) | S | MOVENTO/TANDEM runner screw positions on the cabinet sides (p.428, p.460: which hole of each length takes the Ø5 system screw), and TANDEM's Ø6 rear hook hole in the drawer back (p.435) | runner bores on side sheets |
| BLUM-7 | split → BLUM-7a, BLUM-7b | L | AVENTOS lift systems (HK top, HK-S, HK-XS, HF, HS, HL): power factor = f(front height, weight) → article | split |
| BLUM-7a | done (c660427 HF, c3ae68b HK, 8f77c7f HS/HL: `lift` and `bi_fold_lift` layouts; HKi, HK-S, HK-XS not modelled) | M | Lift-system model: a wall cabinet front opening upward (`opening: lift`), front height and weight from the board catalog (density × area), power factor computed per Blum's formula, and the AVENTOS family chosen by front height; picker drawing per family | test: front weight and power factor for a 600 × 360 front |
| BLUM-7b | split → HF, HK, HS, HL top done (c660427, c3ae68b, 8f77c7f); HK-S and HK-XS done (1b0e81f: parts and BOM; HK-XS's top hinges and every mechanism's side drilling are BLUM-7c); HKi done (f87997e, fully integrated) | M | AVENTOS HK top, HK-S, HK-XS, HF, HS, HL catalog rows (articles by power-factor range, drilling) | hardware rows with sources; BLUM-7a picks articles from them |
| BLUM-7c | blocked (owner 2026-10-26: drilling waits for the Blum CAD, like HF top) | M | AVENTOS drilling: mechanism side holes (HK top p.48, HK-S p.57, HK-XS p.64, HF p.24) and HK-XS's hinges along the top (cups in the front, plates under the top panel) | holes on side and front sheets |
| BLUM-8 | done (05f6929) | S | TIP-ON / SERVO-DRIVE options for handleless fronts | option + BOM |
| BLUM-9 | 👤 needs-human (until STEP files exist in vendor-cad/blum/; see BLUM-CAD-1) | M | Import the owner's Blum STEP files from `vendor-cad/blum/` as linked components (`component.insert_design` after STEP → .solidcraft conversion) in place of stand-ins | a hinge renders with the real Blum model |
| BLUM-10 | done (fbb3804) | M | Parametric stand-in models (hinge cup + arm, plate, runner, box side) emitted as bodies | visible in Manufacturing detail |

| BLUM-11 | done (716fbda) | S | Facts-only `docs/blum/webcodes.csv` (code, product, printed pages); no catalogue text beyond product names | committed |
| BLUM-12 | done (d33ead6) | M | BLUMOTION for doors (printed 176–189) and TIP-ON for doors (190–193); TIP-ON doors use unsprung hinges (finishes BLUM-8's hinge row) | option + BOM rows with part numbers |
| BLUM-13 | done (0abec81, f86d090: TIP-ON BLUMOTION for every box system and MOVENTO, TANDEM TIP-ON runners, SERVO-DRIVE) | M | SERVO-DRIVE, TIP-ON BLUMOTION and TIP-ON for box systems and runners (printed 362–408, 462–531) | options + BOM rows |
| BLUM-14 | done (0abec81; articles only: bracket screw positions and the 22 × 33 back cut-out vs our inset-back void need the owner's look, EXPANDO T not used yet) | S | Wall hanging bracket (598), cabinet connectors (599), EXPANDO T (596) for CORE-3's hangers | part numbers + drilling |
| BLUM-15 | done (0abec81) | S | General information and System 32 definitions (710–731) as the reference for our constants; record any constant in code that disagrees | list in docs/blum/ |
| BLUM-13b | done (d3e54c7) | S | SERVO-DRIVE for MOVENTO / TANDEM wooden drawers (printed 462 on) in place of the generic rows | BOM rows with part numbers |
| BLUM-16 | todo (later: pocket and interior systems, not cabinet basics; left for after the P1 cabinet systems) | M | Later: REVEGO (534–540), AMBIA-LINE / ORGA-LINE (542–569), SPACE STEP (586), CABLOXX (594) | — |
| BLUM-CAD-1 | needs-human (list committed in 7943b00: owner downloads the STEP files) | S | `docs/blum/cad-shopping-list.md`: short prioritised list of configurations the app uses (default hinges + plates, LEGRABOX N/M/K, MOVENTO/TANDEM common lengths, AVENTOS HF top): web code, configurator choices, expected part numbers, file name `vendor-cad/blum/<part-no>.step`, for the owner to download from E-SERVICES | list committed; owner downloads |

All Blum rows: source = Blum Catalogue and technical manual 2024/2025 in `vendor-cad/blum/catalogue/` (local only; PDF page = printed + 4; web codes in its webcodes.csv); records get `source.url` "Blum Catalogue and technical manual 2024/2025, p.<printed>, web code <code>", `document_date` "2024/2025", `verified: false`. Never commit catalogue text, pages or drawings. BLUM-10's stand-in dimensions should move to this data.

### Hardware: Häfele and lighting

| id | status | size | task | done when |
|---|---|---|---|---|
| HAF-1 | needs-human (Häfele data, like HAF-4: which leg and plinth clip, and Häfele's spacing rule; today every base cabinet gets 4 generic `hafele.leg.adjustable`) | M | Adjustable legs + plinth clips (count rule by width), from Häfele data | BOM rows with article numbers |
| HAF-2 | done (f844e59; handle rows are generic `handle.cc128` etc. until handle articles are chosen; insets 35/50 mm and tall handles at 1050 are our choices) | M | Handles: catalog of CC spacings; placement picker (top corner, centre, edge pull) drawn on the door | handle bores on door sheets |
| HAF-3 | needs-human (count done in a7d72a0: 4 per adjustable shelf as generic `shelf-support.5mm`; the article needs the owner's pick and the supplier's data, like HAF-4) | M | Shelf supports (5 mm pins) article + count | BOM |
| HAF-4 | needs-human (Häfele data) | L | Loox5 lighting: profiles (surface, recessed, shelf-edge), 12 V/24 V strips, drivers, distributors, switches/sensors | split |
| HAF-5 | done (95d183b) | M | Lighting rules: no mixing 12/24 V; ≤ 80 % driver load; ports per distributor; cable holes Ø10 through panels added as ops | unit tests for each rule |
| HAF-6 | needs-human (built in 299d534; not seen on screen here: switch on a light in the inspector and look) | M | Lighting placement UI: click under-cabinet / in-cabinet / plinth zones on a cabinet drawing | picker in inspector |
| HAF-7 | 👤 needs-human | M | Import the owner's Häfele STEP files (`vendor-cad/hafele/`) like BLUM-9 | — |
| HAF-8 | dropped (owner 2026-10-26: use Blum's wall hanging bracket 48N0510 and its back cut-out, as built in BLUM-14) | S | Wall hanger + wall rail articles for the generic `wall-hanger.concealed` / `wall-hanger.rail` placements (CORE-3): pick a system (Häfele/Camar), its drilling and rail lengths, from the supplier's data | BOM rows with article numbers; hanger drilling ops |

### Board and edge tape data (Australia)

Sources must be the brand's own current documents. Every record keeps `source.url`,
`document_date` and `verified: false` until the owner or a check confirms it. No swatch images are
committed: they are the brands' copyright. `preview_colour` is an approximation.

| id | status | size | task | done when |
|---|---|---|---|---|
| DATA-1 | done (e9d599e) | M | **Polytec parser.** `tools/parse_polytec.py`: download the Availability Guide PDF, `pdftotext -layout`, and parse every board table (decor, finish, thickness, substrate, sheet sizes, item codes) and matching edge tape codes (1×21, 1×29, roll lengths) into `catalog/boards/polytec.json` | ≥ 90 % of the guide's decorative 16/18 mm rows parsed; spot-check 20 random rows against the PDF and record them in the PR |
| DATA-2 | done (e4f23a7) | M | Laminex: decorative board (Laminex Colour Collection, Natural Wood, Essastone is out of scope), sizes, substrates, matching edges, from laminex.com.au availability documents | `catalog/boards/laminex.json` with sources |
| DATA-3 | split → DATA-3a, DATA-3b | M | Other Australian brands: Egger (AU range), Nikpol (Sonae Arauco/Egger/Fundermax distribution), Formica AU (decorated MDF already in `formica.json` from the Laminex guide, DATA-2; its HPL is not), Bestwood, Briggs Veneers, Navurban, Karaboard/Wilsonart | one file per brand |
| DATA-3a | done (ffacb39) | S | EGGER (AU range via ForestOne, its distributor): 18/25 mm Eurodekor, 50 mm Eurolight, 13 mm compact from the trade brochure's decor-by-product table | `catalog/boards/egger.json`, tools/parse_egger.py |
| DATA-3b | todo | M | Nikpol, Formica AU HPL, Bestwood, Briggs Veneers, Navurban, Karaboard/Wilsonart: find each brand's current public availability document first (none found for Nikpol on a first search) | one file per brand |
| DATA-4 | todo (2026-10-26b: a first search found no published cross-brand match tables; Titus Tekform's guide matches only Stefano Orlati boards. Needs the suppliers' documents, maybe from the owner) | M | Edge tape suppliers: Surteco, Dölken (Doellken), Rehau, Titus Tekform, Häfele AU edging, Fastedge; brand match tables (tape code ↔ board decor) | `catalog/edge-tape/*.json` |
| DATA-5 | split → DATA-5a, DATA-5b | M | Specialty boards: HMR, compact laminate, plywood (Hoop pine, birch), MR MDF raw for paint, thermolaminates, high-gloss acrylic | records with `kind` set |
| DATA-5a | done (cb49931) | S | Raw and white MDF and particleboard (MR MDF raw for paint, white carcass board): Trade Essentials tables of the Laminex guide | `trade-essentials.json`, `kind` raw / melamine |
| DATA-5b | todo (Laminex compact laminate and Trade Essentials plywood done, last b5092fd; HMR, thermolaminates, acrylic and birch ply need other brands' documents) | M | Plywood (Hoop pine, birch), compact laminate, HMR, thermolaminates, high-gloss acrylic, from each brand's documents (the Laminex guide's plywood and compact pages first) | records with `kind` set |
| DATA-6 | split → DATA-6a, DATA-6b | M | Linings: Polytec wall linings / Steccawood / Easycraft, Laminex Surround: profile, module width, length, thickness (`kind: lining`) | records + a lining picker stub |
| DATA-6a | done (d6e2505) | S | Laminex Surround (MDF Primed Panels) from the Laminex guide, and a Boards / Linings switch in the board picker | records + picker stub |
| DATA-6b | todo (polytec's routed Easycraft panels, guide p.28, done in DATA-10; the Ikara/Contours mouldings and timber lining boards of pp. 30-31 are left) | M | Polytec wall linings, Steccawood, Easycraft and Laminex Architectural Panels (decorated battens and VGroove: pack and module sizes need reading) | records with sources |
| DATA-7 | done (3e69102) | S | Catalog validation in CI: ids unique, tapes resolve, each brand file has a source | part of check.sh (already partly) |
| DATA-8 | needs-human (built in 670db45; pick an oak for Fronts in the inspector and check the viewport and cut list) | M | Board picker in the app: brand → range → decor grid with grain and finish filters; assign to carcass/fronts/backs | picking a decor updates the 3D colour and the cut list |
| DATA-9 | 👤 needs-human | S | Owner verifies a sample of rows per brand; flip `verified: true` | — |
| DATA-10 | done (285ce80) | M | Polytec guide, the rest: COMMERCIAL board + edge tape (guide p14-17), Internal Carcass tape widths (the table prints "1mm Rigid" without a width), wall linings (DATA-6) | records with sources; parse_polytec.py tests |

### Outputs

| id | status | size | task | done when |
|---|---|---|---|---|
| OUT-1 | done (initial) | M | Cut list rows + CSV; per-part SVG cut sheets | tests |
| OUT-2 | done (6537e37) | M | Cut sheets as multi-page PDF (one part per page plus cover and index) | PDF opens in Preview with N+2 pages |
| OUT-3 | done (d29080e) | M | Both faces + edge views on sheets when a part has machining on more than one face | side sheet shows inner face and edges |
| OUT-4 | needs-human (built in f557f9b: look at a side's cut sheet: ordinate dimensions for every hole row, hole table grouped by Ø/depth) | M | Sheet quality: dimension chains from the reference corner for every hole row; hole table grouped by diameter/depth | reviewed by owner 👤 |
| OUT-5 | split → OUT-5a, OUT-5b, OUT-5c | L | Guillotine nesting optimiser with grain and kerf (panel saw), sheet diagrams in PDF, offcut list | split |
| OUT-5a | done (382339e) | M | `cabinet_core::nesting`: guillotine free-rectangle placement (best short side, shorter-leftover split), kerf, trim, grain, offcuts | tests: inside trim, a kerf apart, grain kept |
| OUT-5b | done (f73e8d1) | M | Nesting PDF: summary + offcut list, a diagram per sheet; Cut List › Nesting… | PDF per job; docs/images/out5-nesting-sheet-4.png |
| OUT-5c | needs-human (built in 96c4edb: Cut List › Sheets with kerf/trim; not seen on screen here) | M | Nesting in the app: preview the sheets in the Cut List workspace, kerf/trim/sheet size settings (choose among the board's catalog sizes), reuse stored offcuts | sheets shown in the app |
| OUT-5d | needs-human (built in e4b4446; try Cut List › Sheets › Offcuts on screen) | M | Nesting stock choices: pick among a board's catalog sheet sizes (or the cheapest by yield), and keep offcuts from earlier jobs to nest on first | offcut store + test |
| OUT-6 | done (13b9afb) | M | Hardware BOM with supplier article numbers and quantities, CSV + PDF | BOM for the sample run |
| OUT-7 | done (dee5a94) | M | Part labels (62 mm label printer) with QR code of part id | PDF of labels |
| OUT-8 | done (3a7abbe; our own exploded isometric, not SolidCraft's `explode.create` + render: that is OUT-8b) | M | Assembly sheet per cabinet: exploded view (SolidCraft exploded view + export view) | image per cabinet |
| OUT-8b | todo | M | Assembly view from SolidCraft: `explode.create` per cabinet component and a headless render (as OUT-10) into the assembly PDF, if the owner prefers it to the drawn one | image per cabinet from SolidCraft |
| OUT-9 | done (4ac9391; layer names are ours, see crates/cabinet-sheets/src/dxf.rs: owner to try them in his CAM) | L | DXF per part for CNC with layer conventions (outline, bores by Ø/depth, grooves) | split |
| OUT-10 | done (e7eeccb) | M | Renders: Render workspace setup (`render.settings.set`, `render.start`, `render.export`) from the app, PNG saved next to the job | "Render" button produces a PNG |

### macOS app (make it shine)

| id | status | size | task | done when |
|---|---|---|---|---|
| UI-1 | done (initial) | M | Split view: sidebar, RealityKit viewport, inspector with visual pickers, cut list table, sheet viewer, board library | app runs |
| UI-2 | needs-human (built in 5a91eed; not seen on screen here: try File › New, Save, Open Recent and Browse All Versions) | M | Document-based app (`.cabinetjob` files, autosave, versions, recent jobs) replacing the single Application Support file | File > New/Open/Save work |
| UI-3 | needs-human (built in 5a91eed; not seen on screen here: try ⌘Z after a picker change) | M | Undo/redo for every edit (UndoManager) | ⌘Z reverts a picker change |
| UI-4 | needs-human (built in 63a3bff; not seen on screen here: look at the glow, hover tooltip, explode, Section and Open) | M | Viewport: selection outline/glow, hover highlight, part tooltip, exploded slider, section view, door open animation | each item visible in a screenshot |
| UI-5 | needs-human (built in 665cfe1; assign an oak decor to the fronts and look at the viewport) | M | Viewport materials from board preview colours; woodgrain procedural texture for grained decors | oak fronts look like oak |
| UI-6 | needs-human (built in 4765cd2; look at the toolbar with SolidCraft open and closed) | S | Status dot visible in the SolidCraft toolbar menu (currently hidden by the menu label) | screenshot |
| UI-7 | needs-human (design questions in the 2026-10-26b PR: room shapes, runs per wall, wall cabinets over base runs, appliances; it changes the job file, sidebar and viewport) | M | Room planner: wall lengths, run along a wall, appliances as placeholders | split |
| UI-8 | needs-human (built in 8c867b5; drag in the run strip; say whether a strip is acceptable instead of dragging the 3D cabinets) | M | Drag to reorder and resize cabinets in the viewport (width handles snap to presets) | drag changes width |
| UI-9 | needs-human (built in 9fd31d0; try the keys) | S | Keyboard: ⌘1–4 workspaces, arrows to step through cabinets, ⌫ delete | works |
| UI-10 | needs-human (built in 94108a4; not seen on screen here: launch once with defaults reset, or Help › Welcome to Cabinet Studio) | M | Onboarding: first launch connects to SolidCraft, explains Design vs Manufacturing detail | first-run sheet |
| UI-11 | done (bae6fe6) | S | App icon and accent colour | icon in Dock |
| UI-12 | needs-human (built in 983d877; open Settings (⌘,) and change each) | M | Preferences: SolidCraft port, default standard, units, catalog folder, vendor-cad folder | Settings scene |
| UI-13 | needs-human (built in 6ac857e; save a standard, start a new job, apply it) | M | Construction standard editor: save/load named standards ("My shop") as visual presets | standards menu |
| UI-14 | needs-human (built in a696d89; run Accessibility Inspector over the inspector and sidebar) | S | Accessibility pass: VoiceOver labels on every drawn tile, Dynamic Type sizes | Accessibility Inspector clean |
| UI-15 | blocked (UI tests need macOS UI automation permission; on MacAirServer an AppleEvent to System Events timed out waiting for it. Grant Accessibility/Automation to Xcode's test runner on a Mac that runs check.sh) | M | UI tests (XCUITest) for add cabinet, change option, export CSV | tests pass in check.sh |
| UI-16 | needs-human (built in 8bfa5ee; not seen on screen here: add an end panel each side and look at the viewport and cut list) | M | Run items in the app: add filler and end panel to the sidebar list (drawn tiles, width/scribe chips), lay out and send the project through `cabinet_core::build_run` (FFI) instead of the Swift layout loop | a run with an end panel each side shows in the viewport, cut list and SolidCraft |
| UI-17 | needs-human (built in dbc5e9d; not seen on screen here: change the back style for the job, then with "Only B01") | M | Project standard in the app: the construction pickers edit the project standard, with an "Only this cabinet" override switch; save through `cabinet_core::Project` (migrate the old project.json) | changing the back style for the project changes every cabinet without an override |

## P2 — Commercial parity

| id | status | size | task |
|---|---|---|---|
| P2-1 | todo | L | Face-frame construction (US style) |
| P2-2 | split → P2-2a done (0a5ea06: shaker frames; member width, panel and groove are our defaults, not yet options), P2-2b done (e88b3c7: glass frames, rebate + glass and bead rows), P2-2c todo (Polytec aluminium frame doors: needs their profile data) | L | Shaker / 5-piece doors, glass frames, aluminium frame doors (Polytec Alu frames) |
| P2-3 | split → P2-3a done (29a1648: wardrobe kind, hat shelf, long/double hanging rails), P2-3b todo (drawers and shoe shelves in a wardrobe's lower part), P2-3c needs-human (sliding doors: which track system, e.g. Häfele Slido; its data) | L | Wardrobes: hanging rails, drawers, shoe shelves, sliding doors |
| P2-4 | todo | L | Benchtops, cornices, light rails, panels with scribes |
| P2-5 | split → P2-5a done (ab3f118: core quote, PDF, Cut List › Quote with prices typed per line; not seen on screen), P2-5b built (76cdcbf: rates popover, per-cabinet customer quote; price per sheet left: per m² of sheet is the same thing for one sheet size) | L | Quoting: material + hardware + labour rates |
| P2-6 | todo | L | CNC post-processors (MPR, CIX), BXF exchange with Blum (needs Blum spec 👤) |
| P2-7 | todo | L | True-shape nesting for CNC |

## P2 — Kitchen design: benchtops, Surround linings and curved details

Stretch goal set by the owner on 2026-10-24, to start **after the cabinet systems are finished**.
The aim is kitchens like the owner's reference photos: fluted and curved islands (rounded ends
clad in vertical fluting), stone benchtops with rounded ends and overhangs, slab splashbacks
with recessed open shelves, handleless full-height fronts, clad rangehoods, and arched pantry
doors with shaker wall cabinets. The photos are not in the repo (third-party images). Describe
features in words.

| id | status | size | task | done when |
|---|---|---|---|---|
| BENCH-1 | todo | M | Benchtop model per run: thickness presets (20, 33, 40, 60, 80+ mm built-up), front/side overhangs, back and end clearances, material from catalog (laminate, compact, stone, timber) | a run gets a benchtop part that follows the run and overhangs |
| BENCH-2 | todo | M | Edge profiles as a visual picker: square, pencil round, bullnose, bevel, mitred apron (thick look) | profile shown in the 3D view and on sheets |
| BENCH-3 | todo | L | Transitions: L and U corners (mitre or butt joint, joint position picker), thickness steps, waterfall ends (mitred drop to floor), rounded/radius island ends, breakfast-bar overhang with a support rule | split into BENCH-3a..e |
| BENCH-4 | todo | M | Cut-outs: sink (undermount/top mount), cooktop, tap holes, power pop-ups, from appliance templates | cut-outs on the benchtop sheet |
| BENCH-5 | todo | M | Benchtop outputs: dimensioned template sheet per piece with joins, cut-outs and edge profiles; area and edge metres for quoting | PDF/SVG sheet |
| BENCH-6 | todo | M | Splashbacks: slab splashback panels matching the benchtop, with a niche/recess for open shelves and power points | splashback part + sheet |
| SURR-1 | todo | M | Laminex Surround profiles as catalog data (`kind: lining`) from Laminex's own current documents: profile names (Classic VJ, Scallop, Demi Round...), module width, flute/groove pitch, thickness, sheet sizes, substrate, flexible/curved variants | catalog rows with sources |
| SURR-2 | todo | M | Cladding as a cabinet finish: island backs and ends, end panels and kick panels clad in a Surround profile, flutes vertical and aligned across panels | an island back renders fluted |
| SURR-3 | todo | L | Curved ends: rounded island ends (radius picker) from curved substrate (bending ply / kerfed MDF / flexible Surround), fluting following the curve, benchtop ends to match | split |
| SURR-4 | todo | M | Clad rangehood / canopy box with rounded front corners, in a Surround profile or paint finish | rangehood part between wall cabinets |
| SURR-5 | todo | S | Flute and VJ procedural textures in the viewport (normal-mapped grooves) | looks fluted in the app |
| STYLE-1 | todo | M | Door styles for the look: shaker 5-piece, arched-top tall doors, handleless (J-pull, finger pull, push-to-open), full-height tall fronts with continuous grain | picker tiles |
| STYLE-2 | todo | M | Open shelving: floating timber shelves (concealed brackets) in niches between wall cabinets, with LED strip (ties to HAF-5) | shelf parts + hardware |
| STYLE-3 | todo | S | Handle finishes: brushed brass, black, stainless; long pulls on tall doors | rendered handle colour |
| STYLE-4 | todo | M | Appliance housings: integrated ovens (single/double stacked), integrated fridge with SERVO-DRIVE flex, rangehood insert | tall cabinet variants |

## P3 — SolidCraft plugin

| id | status | size | task |
|---|---|---|---|
| PLG-1 | blocked (SolidCraft has no plugin runtime yet; check #1221/#4980 weekly) | L | Port the bridge to a `command-plugin` WASM component (same commands via `engine.run`) |
| PLG-2 | blocked (PLG-1) | L | `panel-plugin` with pins; propose an image-choice widget upstream so pickers can live in SolidCraft |
| PLG-3 | todo | S | Weekly check: has SolidCraft merged plugin runtime work? Update this row and docs/FEASIBILITY_PLAN.md. Last check 2026-10-26: no runtime; #1221 and draft #4980 open, #4330 closed in favour of open PR #5586, latest release v0.23.0 |

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
- 2026-10-23 (night) — CORE-6 done in the core: `run::{Run, RunItem, build_run}` lays out cabinets, fillers (door plane, scribe allowance) and end panels (door face to wall + scribe, optionally to the floor) left to right. App support is UI-16 (added).
- 2026-10-23 (night) — CORE-7 done in the core: `project::Project` stores the standard once and each cabinet's differing fields as `overrides`; resolves back exactly. App side is UI-17 (added).
- 2026-10-23 (night) — CORE-9 done: `back_board` slot; front-role parts use the front board; `set_board` / FFI `assign_board` set a slot's board and its catalogue thickness (test: 16 → 18 mm front board changes the door to 18 mm). Board library cards get "Use for Carcass / Fronts / Back" and badges. CORE-8 marked blocked on BLUM-1/BLUM-6 (supplier numbers belong in catalog data).
- 2026-10-23 (night) — CORE-11 done: Settings scene (⌘,) with "Show lengths in" millimetres / inches (nearest 1/16″); sizes, chips, sidebar, run length and cut list follow it; files, CSV and SolidCraft stay in mm. Not seen on screen (no Screen Recording permission here). CORE-10 blocked on BLUM-2.
- 2026-10-23 (night) — CORE-12 done: `collisions::check` (bores as cylinders, slots/grooves as boxes; overlaps, break-outs, thin floors), reported in build warnings. The sweep over every kind × every choice found one real clash family — the rearmost biscuit of each back-to-front joint cut into the back groove — fixed by stopping those joints at the interior back (goldens: fixings moved, nothing else).
- 2026-10-23 (night) — JOIN-2 done as a new `Exact` detail level: biscuit slots as area-exact 4-chord lenses (volume within 1 % of the analytic lens in a headless test). Manufacturing keeps box slots: lens slots make the tall cabinet 12.2 s instead of 3.9 s (docs/perf.md). Owner: is that trade right?
- 2026-10-23 (night) — JOIN-3 done: at Manufacturing and Exact detail every slot gets a named lens-shaped biscuit body (`B01.Biscuit3`, beech colour), true arcs (no boolean, so no slowdown: tall still 3.8 s). Headless test: one per slot, volume within 1 %. Picture: docs/images/join3-biscuits.png. Slot operations now record their biscuit size.
- 2026-10-23 (night) — JOIN-5 done: every screw clearance hole gets a 90° countersink (Ø8 chipboard, Ø10 confirmat; in the part list for sheets/CNC, not cut in SolidCraft) and a cover cap in the hardware list; `exposed.left/right` sides get no through screws (biscuits + glue, blind dowels where biscuits do not fit); inspector "Seen ends" chips. JOIN-4 blocked on Häfele data.
- 2026-10-23 (night) — JOIN-7 done: applied backs are screwed to the sides, bottom, top/back rail at ≤150 mm centres (countersunk, capped); grooved backs get none. The clash sweep caught 35 mm pilots reaching the rear shelf-pin row, so back screws pilot 15 mm (4 × 20 through a 6 mm back). JOIN-6 blocked on Lamello data.
- 2026-10-23 (night) — BLUM-1 done: `cabinet_catalog::hardware` (HardwareItem: article, family, variant, kind, named parameters, drilling pattern, source/verified), loaded from `catalog/hardware/*.json` with validation (unique ids, article, source, real holes); format note in catalog/hardware/README.md. No Blum numbers entered: they wait for the owner's TD sheets (BLUM-2).
- 2026-10-23 (night) — BLUM-3/4/5 blocked on Blum's own tables; BLUM-7 split into 7a (lift model) and 7b (AVENTOS data, blocked).
- 2026-10-23 (night) — BLUM-8 done: `opening` standard option (handles / TIP-ON / SERVO-DRIVE) with drawn picker tiles; TIP-ON unit per door (2 over 1200 mm) and per drawer, SERVO-DRIVE unit per drawer + power supply per cabinet, doors under SERVO-DRIVE fall back to TIP-ON with a warning. Generic item ids until Blum articles are catalogued. Owner: TIP-ON doors assume no-spring hinges; the hinge row does not change yet.
- 2026-10-24 (night) — BACKLOG rows from the first night now carry commit SHAs instead of the branch name.
- 2026-10-24 (night) — BLUM-10 done: `cabinet_core::standins` draws a hinge (cup in its bore, arm, plate on its two holes) and per drawer a runner + box side on each inside face, from the parts and machining around them (proportions drawn, not Blum data). Emitted at Manufacturing/Exact as one grey body each (`B01.Hinge1`, `D01.Runner1L`); tested never to enter a panel across kinds × overlays, and headless (bounds match). Sample run still ~7 s headless. Picture: docs/images/blum10-stand-ins.png.
- 2026-10-24 (night) — UI-2 + UI-3 done: DocumentGroup over `.cabinetjob` files (exported UTType, JSON: job name + cabinets), so File › New/Open/Save/Open Recent, autosave and Browse All Versions come from the system; the old Application Support project.json is imported into the first new job, then renamed. Every edit registers its reverse with the window's UndoManager (⌘Z/⇧⌘Z), which is also what marks the job edited. Menu commands follow the key window. Not seen on screen here (no Screen Recording): owner to try New, Save, ⌘Z on a picker.
- 2026-10-24 (night) — UI-17 done: the job file is the core's `Project` JSON (FFI `project_json`/`open_project`; drawn cabinets kept in `placed`); older files and the Application Support job migrate with the first cabinet's standard as the job's. Construction pickers change the whole job (every cabinet not overriding that field) or, with the "Only B01" tile, just the selection; overridden options get an "Own choice" reset badge and the section a "Match job" button. Board assignments follow the same rule. Not seen on screen.
- 2026-10-24 (night) — UI-16 done: the sidebar's + menu adds fillers (F01) and end panels (E01) after the selection; the inspector has width/scribe chips and drawn to-floor/on-kick tiles. The app hands the whole job (items in order + drawn cabinets) to the core (`build_job`, `run::laid_out`), which lays out the run for the viewport, cut list, sheets and SolidCraft; the Swift layout loop is gone. Job files keep pieces at their index (`Project.pieces`). Run length in the status bar comes from the built panels. Not seen on screen.
- 2026-10-24 (night) — UI-4 built: the viewport keeps one entity per panel (named by part) and moves them instead of rebuilding; selected cabinet glows faintly in the accent colour, the panel under the pointer brighter (hover by a ray from the camera through `Scene.raycast`: RealityView has no hit test on macOS 15) with a glass tooltip (name, role, size); explode slider (up to 250 mm from each cabinet's centre); Section cuts the run through the selected cabinet's middle (boxes trimmed); Open swings doors 100° about their hinged edge (FFI `PanelBox.hinged`) and pulls drawers out, animated. Needs the owner's visual check: none of it was seen on screen here.
- 2026-10-24 (night) — UI-5 built: panels take their board's colour (FFI `PanelBox.board`/`grain`); grained decors get a seeded procedural woodgrain tile (docs/images/ui5-woodgrain-oak.png) laid along the part's grain at 0.6 m per tile, loaded async; gloss finishes are shinier. Only 4 of 459 catalog boards have a `preview_colour`, so the app guesses one from the decor name (Oak, Walnut, Grey…) and marks those cards with an eyedropper. Owner: assign an oak to the fronts and look.
- 2026-10-24 (night) — UI-6 built: a toolbar menu draws only its label's icon and title, so the connection dot is its own toolbar view beside the SolidCraft menu (green with a glow / red / grey, a spinner while sending, VoiceOver value).
- 2026-10-24 (night) — UI-9 built: View menu ⌘1–4 for the workspaces and ⌘[ / ⌘] for previous/next cabinet; in the viewport (after a click) the arrows step and ⌫ deletes; Edit › Delete Cabinet is ⌘⌫ (a bare ⌫ menu shortcut would eat text-field deletes); the sidebar list keeps its own arrows and ⌫.
- 2026-10-24 (night) — UI-11 done: asset catalog with an AppIcon (a two-door oak cabinet on a blue tile, drawn by tools/make_app_icon.swift at every macOS size; AppIcon.icns lands in the bundle) and an oak AccentColor (#c67c3c, lighter in dark mode).
- 2026-10-24 (night) — UI-13 built: saved standards ("My shop") are JSON files in Application Support/CabinetStudio/Standards, shown under Construction as tiles drawn with their top joint and back; a click makes one the job's standard (cabinets keep their overrides), Save… names the job's standard, the context menu updates or deletes. New jobs can start from one (UI-12).
- 2026-10-24 (night) — UI-12 built: Settings has General (units, SolidCraft control port with a stepper and Default), Jobs (the saved standard new jobs start from) and Folders (catalog folder in place of the bundled one, supplier CAD folder for BLUM-9). The model reads the port and catalog from there.
- 2026-10-24 (night) — UI-14 built: every drawn thumbnail is hidden from VoiceOver and its button speaks words instead ("Back: Inset in groove" with the choice's description as hint; drawer stacks by count and heights; size chips with their dimension and unit); tile and chip styles add the Selected trait; sidebar rows read as one element; the viewport and board swatches are labelled. Fonts are the semantic text styles throughout. Needs a run of Accessibility Inspector on screen.
- 2026-10-24 (night) — UI-8 built: RealityView offers no reliable 3D drag on macOS 15, so the viewport has a run strip along its bottom: each cabinet and filler to scale; drag one to move it along the run, drag its right edge (resize pointer) to change the width, snapping to 150–1200 mm cabinet sizes or 25–200 mm filler sizes; one undo step per drag; VoiceOver can step widths. Owner: is a strip acceptable, or must the drag be on the 3D cabinets?
- 2026-10-24 (night) — DATA-8 built: one BoardPicker (brand → range menus, Woodgrains/Colours, finish chips, search) over decor cards grouped by decor + finish with their thicknesses as chips. It is the Boards workspace ("Use for" menu per card) and, new, the inspector's Boards section: Carcass / Fronts / Back rows with swatches open it in a popover. A pick goes through useBoard (job-wide unless "Only B01"), so the viewport colour (UI-5) and the cut list's board and thickness follow.
- 2026-10-24 (night) — OUT-2 done: `cabinet_sheets::pdf::job_pdf` turns each sheet SVG (rect, line, circle, text) into PDF operators in standard Helvetica (WinAnsi, AFM widths for centred text; no fonts embedded, no new crates), after a cover and an index with page numbers (more index pages past 126 parts). FFI `cut_sheets_pdf`; Cut Sheets has Export PDF…. The sample base cabinet's 9 parts make 11 pages in PDFKit, text selectable; picture docs/images/out2-pdf-sheet.png.
- 2026-10-24 (night) — OUT-3 done: sheet features know their face (bores by drilling direction, slots by the face they open on). A part machined on both faces gets two views, inside and outside turned over (mirrored), the inside found by facing the cabinet's middle; table rows are tagged in/out. Each edge with bores gets an edge view strip under the views. One-face parts are unchanged (door, back, shelf, plinth snapshots identical); sides, bottom and rails snapshots reviewed. Pictures docs/images/out3-side-both-faces.png, out3-bottom-edges.png. Edge views show bores, not the biscuit slots in an edge.
- 2026-10-24 (night) — OUT-6 done: `cabinet_core::bom` counts every hardware placement per item and description with the cabinets using it; the article column comes from the hardware catalog and says "not catalogued" until the Blum/Häfele rows exist (BLUM-2, HAF-*), so today every row says that. `pdf::table_pdf` sets any table over pages. The Cut List workspace has a Parts / Hardware switch with Export CSV… and Export PDF…. Sample: docs/images/out6-hardware-pdf.png.
- 2026-10-24 (night) — OUT-7 done: `cabinet_sheets::labels` makes a 62 × 40 mm PDF page per part (continuous 62 mm tape): QR code (`qrcode` crate, MIT/Apache, no image features) of the part name, name, role, finished size, board, taped edges, job. Every label of the sample cabinet decodes back to its part name with Core Image's QR detector. Cut Sheets has Labels…. Picture docs/images/out7-label.png.
- 2026-10-24 (night) — OUT-10 done: FFI `render_png` saves a "Cabinet Studio" render setup in the design (preview mode, the only profile SolidCraft 0.20's built-in renderer has; camera aimed at the job's bounding box from SolidCraft's isometric angles, distance 1.35 × its diagonal), starts the job, polls status/export for up to two minutes and writes the PNG. SolidCraft menu › Render to PNG (⇧⌘R) saves it next to the job (or asks for an unsaved one) and shows it in the Finder. New opt-in live test (`SOLIDCRAFT_LIVE_PORT=7878`, only on an untitled empty design) built base + filler + drawer bank in the running SolidCraft 0.20 and rendered docs/images/out10-render.png. Same session: sample run at Manufacturing detail connected took 1.9 / 0.4 / 4.5 s (B01/D01/T01), so PERF-1's connected timing is confirmed under 10 s.
- 2026-10-24 (night) — SC-2/SC-3/SC-4: SolidCraft 0.20 is installed on this Mac. Started it with `--control 7878` (answers within 2 s) on an untitled empty design; new opt-in `cabinet-ffi/tests/live.rs` (SOLIDCRAFT_LIVE_PORT) draws 3 boxes, selects them (`select.set`), makes base/tall/wall cabinets in place, builds them, hides the boxes, reads them back identical, then deletes what it made. Only the menu clicks in Cabinet Studio are left for the owner.
- 2026-10-24 (night) — DATA-2 done: tools/parse_laminex.py reads the guide's Decorated MDF / Decorated Particleboard trade tables (Sep 2025, 109 pages) from word positions; merged colour/finish/substrate/thickness cells are given to the run of product rows they are centred on, nested (colour ⊇ finish ⊇ substrate ⊇ thickness). 881 Laminex boards (1074 product codes with Formica), 568 matching edgings (code, 1 mm/25 m standard or 2 mm/100 m large format; width and material are not printed, so the tape schema now allows them absent), woodgrains from the Specifier's WOODGRAINS group; 102 Formica boards (no woodgrain flag in the guide, no tape thickness: none recorded). Checks: every product code's sheet size matches the layout text (1074/1074), every thickness printed on a code's own line (882/882), and every (decor, colour code) pair the specifier pages list agrees (Laminex 132/136, 4 codes the trade tables add). Spot-checked rows in the PR.
- 2026-10-24 (night) — DATA-5 split into 5a/5b. DATA-5a done: parse_laminex.py also reads the four-tables-a-page Trade Essentials pages (Raw/White Particleboard, Raw/White MDF): 88 boards, 302 product codes, `kind` raw or melamine, in catalog/boards/trade-essentials.json. Merged substrate/thickness cells nest as before; colours set on two lines around a row (White Aquaban) are read as spans; the TYPE column is a title set on end except under MDF TYPE. 11 White MDF rows whose substrate cell gives only a grade (SELECT, SS 06MM BOW) are left out rather than guessing MR or not. Checks over all three brands: 1376/1376 sheet sizes (one by eye: its size sits a line above in the layout text), 930/930 printed thicknesses. Finish legends are now read per page, multi-word (MR Moisture Resistant).
- 2026-10-24 (night) — DATA-6 split into 6a/6b. DATA-6a done: parse_laminex.py reads the MDF Primed Panels (Surround by Laminex) page: 26 `kind: lining` records (profile as printed, e.g. Classic VJ 100, Scallop 22.5; 9/12 mm; MDF MR E0 or MDF FR; sheet sizes and product codes). The guide never says what the number in a profile name measures, so no module width is recorded. FFI BoardRecord gains `kind`; the board picker has a Boards / Linings switch when the catalog has linings.
- 2026-10-24 (night) — DATA-7 done: on top of unique ids and resolving tapes, `Catalog::validate` now refuses any board or tape whose source lacks a URL, document date or retrieval date; check.sh (and CI) load the repository catalog through it.
- 2026-10-24 (night) — PLG-3 checked: SolidCraft still has no plugin runtime (#1221 open, draft #4980 open, #4330 open; v0.20.0 latest). FEASIBILITY_PLAN updated.
- 2026-10-24 (night) — Owner's answers to PR #1 recorded: CORE-4b unblocked (bi-fold doors; hinge article split to CORE-4d), supplier-number rows stay needs-human, JOIN-2 accepted, CORE-5 question restated, JOIN-8 (Domino) and JOIN-9 (dowels) added. Not built: the run hit its usage limit.
- 2026-10-24 (night) — Blum catalogue arrived (owner-supplied, local only): BLUM-2/3/4/5/6/7a/7b, CORE-8 and CORE-10 unblocked; BLUM-11..16 and BLUM-CAD-1 added. Order for the next run: BLUM-11, BLUM-2, BLUM-3, CORE-10, BLUM-4, BLUM-6/CORE-8, BLUM-7 HF top, BLUM-CAD-1. Not started: usage limit.
- 2026-10-24 — owner added the benchtop / Surround / kitchen-style stretch goals (P2 kitchen design), to start after the cabinet systems. **Next run starts at BLUM-11** (suggested order above) and branches from **main** (PRs #1 and #2 were merged on 2026-10-24, plus the Cut Sheets crash fix 6554bbb).
- 2026-10-26 (night) — BLUM-11 done: docs/blum/webcodes.csv lists the catalogue's 153 web codes with product name, printed pages and our family grouping; no catalogue text.
- 2026-10-26 (night) — BLUM-2 + BLUM-3 done together (replacing the constants needed the rule): catalog/hardware/blum-clip-top.json holds CLIP top BLUMOTION 110° hinges (overlay/dual/inset × screw-on/INSERTA, unsprung CLIP top for TIP-ON) and cruciform 37/32 plates (system screw 0/3/6/9/18, EXPANDO 0/3) with Blum's minimum-gap and max-door-thickness tables. Hardware items gain `tables`. cabinet-core bundles the hardware files (include_str!, no I/O); `hinges::select` picks the hinge for the overlay and the thinnest plate that keeps TB in 3–7 mm (FA = TB + TO − cranking − MD; checked against every row of p.77's TB tables), warns on small gaps / thick doors. Hinge and plate are separate BOM rows with real articles. Plate pilots 13 → 11.5 deep (Blum's printed minimum). INSERTA dowel depth is not printed, so the default boss is screw-on.
- 2026-10-26 (night) — CORE-10 done: per-overlay test (straight / 9.5 / 18 mm cranked arm, TB 4 / 6 / 6.5 / 5, inset plate row at 37 + FD + 1.5).
- 2026-10-26 (night) — BLUM-4 done: catalog/hardware/blum-legrabox.json (N/M/K side sets, 750/753 profiles, fixings, planning rows). The drawer box is the tallest LEGRABOX whose space fits each drawer's pitch, NL the longest with NL + 3 ≤ inside depth; DrawerBottom (LW − 35 × NL − 10) and DrawerBack (LW − 38 × 39/63/101) parts, Ø10 × 12 front fixing bores 14 in from the sides at 45.5/+16 (N) or 51/+32 (M, K) above the drawer's datum. New `drawer_system` option.
- 2026-10-26 (night) — BLUM-6 + CORE-8 done: catalog/hardware/blum-movento-tandem.json; wooden boxes on MOVENTO or TANDEM: sides SKL = NL − 10, SKW = LW − 42 between them, base 27.5/28.5 above the runners with the sides 12 below it, front and back on the base. Box construction is ours (`drawer_board_thickness`, 16). BLUM-4b and BLUM-6b added for the drilling not read yet.
- 2026-10-26 (night) — BLUM-7 (HF top) done: catalog/hardware/blum-aventos-hf.json; `lift::front_weight` (FH × FB × FD × density, Blum's 680 chipboard / 760 MDF), power factor KH × FG, mechanism by LF range (stronger in the overlap, one mechanism for LF 1350–3000 up to 600 wide), lever arm by KH, 2/3/4 hinges. New `bi_fold_lift` front layout: two equal LiftFront parts, a 3–6 mm middle gap, cups for the 120° and centre hinges; `front_core` follows the front board's substrate. Not drilled: the mechanism's side holes (the p.24 drawing is a perspective view; check) and the lever-arm plate.
- 2026-10-26 (night) — CORE-4b done: `corner_base` kind built from two straight legs (corner.rs turns, mirrors, renames and opens a leg's end; every part stays axis-aligned), one interior, plinths meeting at the inside corner, each panel once. Bi-fold pair: door 1 on 155° hinges, door 2 on Blum's corner bi-fold hinge (data pp. 84-85, 116-117; CORE-4d left for the owner to confirm the system). Headless SolidCraft builds both hands at Manufacturing detail with every part's bounds as the core put them.
- 2026-10-26 (night) — BLUM-CAD-1: docs/blum/cad-shopping-list.md (hinges and plates, LEGRABOX and runner lengths the sample uses, AVENTOS HF top, L-corner hinges; web code, configurator choices, file names). Owner downloads.
- 2026-10-26 (night) — BLUM-12 done: TIP-ON doors take Blum's 956.1004 (overlay fronts to ~1300) or 956A1004 (taller, inset) on straight adapter plates; no add-on BLUMOTION (built into CLIP top BLUMOTION).
- 2026-10-26 (night) — BLUM-13/14/15 done together: TIP-ON BLUMOTION sets + synchronisation for LEGRABOX/MOVENTO, TANDEM TIP-ON runners, SERVO-DRIVE parts for LEGRABOX (catalog/hardware/blum-drawer-motion.json); Blum wall hanging brackets for CORE-3's hangers; docs/blum/constants.md. The hinge count now follows Blum's chart by height and weight (it allowed 2 hinges up to 900 mm; Blum says ~750 / 6 kg).
- 2026-10-26 (night) — JOIN-8 done: catalog/hardware/festool-domino.json from festool.com product pages (each tenon's thickness × length × width and pack article; DF 500 cutters and depth stops 12/15/20/25/28, DF 700 15–70) and the DF 500 manual's width settings. `domino` fixing recipe and size picker; a Mortise operation per part, half the tenon each side where the face part allows, on depths the machine sets; cut in SolidCraft, drawn on sheets, clash-checked.
- 2026-10-26 (night) — JOIN-9 done: dowel size (6×30 … 10×50) and spacing (32 mm grid from the front, or even like a jig); depth split half/rest + 2 mm glue room; too-thick dowels warn.
- 2026-10-26 (night) — BLUM-5 done: MERIVOBOX and TANDEMBOX antaro N/M/K from their planning pages; `drawers::steel_box` serves all three box systems; picker shows five drawer choices. CORE-4c set to needs-human with the hanging question.
- 2026-10-26 (night) — BLUM-7 HK top done: `stay_lift` front layout (one front, no hinges) on AVENTOS HK top, mechanism by LF = KH × (FG + 2 × handle); the Fronts tiles are a three-column grid now.
- 2026-10-26 (night) — BLUM-7 HS/HL top done: the single-front layout is `lift { kind }` (stay lift = HK top, up and over = HS top, lift up = HL top); `lift::single_front` reads each family's height/weight/power-factor ranges and takes the stronger where they overlap.
- 2026-10-26 (night) — BLUM-13 finished: TIP-ON BLUMOTION units for MERIVOBOX (T60H) and TANDEMBOX (T60B) with their synchronisation; BLUM-13b: SERVO-DRIVE for MOVENTO/TANDEM uses the same parts as the box systems (p.464), no generic rows left.
- 2026-10-26 (night) — OUT-4 built (f557f9b): ordinate dimensions from the reference corner for every bore row and column, hole table grouped by kind with counts; needs the owner's look.
- 2026-10-26 (night) — UI-10 built (94108a4): first-run welcome sheet (connect SolidCraft, pick detail with drawn tiles, start); Help › Welcome to Cabinet Studio reopens it.
- 2026-10-26 (night) — Run stopped at the usage limit. DATA-10 not started (the Polytec guide downloads fine; COMMERCIAL tables are guide p14-17, wall linings p28). Next: DATA-10, HAF-5, BLUM-4b/6b drilling once the owner confirms the readings.
- 2026-10-26 — **Owner's answers to PR #3** (applied on main):
  1. Hinge cup and plate pilot depths: keep Blum's minimums (13 / 11.5 mm).
  2. Hinges: keep screw-on bosses; no INSERTA.
  3. Corner bi-fold hinge: Blum 79T8500 + 155° hinges confirmed (CORE-4d done).
  4. Wooden drawers: bottom between the sides, 12 mm up from the lower edge, as built.
  5. LEGRABOX / runner screw positions on the cabinet sides: wait for the Blum CAD (BLUM-4b, BLUM-6b blocked).
  6. TIP-ON BLUMOTION unit by drawer depth: good enough, no load choice.
  7. Lift handle weight: default 400 g (`DEFAULT_HANDLE_WEIGHT`; the app writes 0.4 kg for new lift fronts).
  8. HF top cabinet drilling: leave until the CAD arrives.
  9. Wall cabinets: use Blum's wall hanging bracket settings (HAF-8 dropped).
  10. Corner wall L: the second leg shares the main leg's corner hanger (CORE-4c todo).
  11. DOMINO: the end part's mortise is cut on the middle setting, +6 mm (`DOMINO_PLAY`); the face part stays tight.
  12. SERVO-DRIVE power supply Z10NE04KI: confirmed.
  13. Blum CAD: the owner will download the shopping list into `vendor-cad/blum/` soon.
  14. Sink cabinet: keep "no back rail" (CORE-5 done).
- 2026-10-26b (night) — CORE-4c done: `corner_wall` kind (600 × 600 × 330 × 720 by default): the L-corner's legs built as wall cabinets with the bi-fold pair; three Blum wall hanging brackets (main leg both ends, second leg its outer end, its open end carried by the main leg's corner bracket), a rail behind each leg; a right-hand corner swaps the brackets' hands. Headless SolidCraft 0.20 builds it with every part's bounds as the core put them.
- 2026-10-26b (night) — DATA-10 done: parse_polytec.py reads COMMERCIAL boards (guide pp. 14-15) and tapes (pp. 16-17) from word positions: 252 boards (16-33 mm; MR particleboard, MR MDF, new `ldf` / `moisture_resistant_ldf` substrates), every printed board code (243/243) and every tape whose decor has a board (249/249) cross-checked against `pdftotext -raw`. Internal Carcass tapes take the width of the sized tape with the same item code (12 of 20; 8 stay width-less). 10 routed Easycraft lining panels from p. 28. Fixed on the way: the Internal Carcass SOLID MATT rows had been read as Texture (the heading shares a line with the margin title); 3 wrong ids replaced. Pack quantities not recorded (no field).
- 2026-10-26b (night) — BLUM-4b, all but the profile screw positions (7189be7): LEGRABOX pure high fronted pull-outs C (177 mm sides, space 193) and F (241, space 257) behind a new `drawer_sides` option (default unchanged: N, M or K); front bores 51/+32/+96 (C) and 51/+32/+96/+32/+32 (F) read from the drawings, F fronts on an M bracket pair below a C pair. Inner drawers (`inner_drawers` per cabinet, LEGRABOX only): the opening shared from the bottom, K where a share is ≥ 142 mm else M, a ZV7.1043C01 front piece cut to LW − 126, no shelves; the doors hang on the 155° 0-protrusion hinge (warning for half/inset overlay). Headless SolidCraft builds both.
- 2026-10-26b (night) — HAF-5 done (95d183b): `cabinet_core::lighting` with a test per rule (no 12/24 V mixing on a driver, ≤ 80 % driver load, strips ≤ distributor ports) and a planner that groups strips by voltage and picks the smallest driver per circuit; drivers and port counts are inputs until HAF-4's Loox5 data. Cabinets carry `lights` (under-cabinet / in-cabinet / plinth, voltage, W/m): each adds a strip row to the hardware list and a Ø10 cable hole (bottom, or top/back rail for inside lights; 60 mm from the left side, 25 mm in front of the back). No UI yet (HAF-6).
- 2026-10-26b (night) — DATA-3 split. DATA-3a done (ffacb39): tools/parse_egger.py reads ForestOne's EGGER Decorative Collection for trades (Nov 2024): decor rows by word positions, the availability dots (drawn, not text) from the page as SVG; 81 decors, 115 boards (73 in 18 mm, 18 in 25 mm, 7 Eurolight 50 mm, 17 compact 13 mm, as the brochure's own counts). No item or edging codes in that document.
- 2026-10-26b (night) — OUT-5 split. OUT-5a done (382339e): guillotine nesting in the core. OUT-5b done (f73e8d1): nesting PDF (summary with offcuts, a to-scale page per sheet), FFI `nesting_pdf` (each board's first catalog sheet; generic parts on 2400 × 1200) and Cut List › Nesting…. Base + tall sample: 6 sheets, see docs/images/out5-nesting-sheet-4.png. OUT-5c (in-app preview and settings) added. DATA-4 searched: nothing usable published.
- 2026-10-26b (night) — OUT-8 done (3a7abbe): exploded isometric assembly page per cabinet with its parts list (docs/images/out8-assembly.png); Cut Sheets › Assembly…. OUT-8b added for a SolidCraft-rendered version.
- 2026-10-26b (night) — OUT-9 done (4ac9391): R12 DXF per part (and `-back` for a second machined face) with layers OUTLINE, BORE_D<Ø>_Z<depth>/THRU, POCKET_<KIND>_Z<depth>, CUTOUT, EDGE_BORE_…, EDGE_<KIND>; all 11 files of the sample base cabinet audit clean in ezdxf 1.4.5 (in a scratch venv, not a project dependency). Cut Sheets › DXF… writes them into a folder.
- 2026-10-26b (night) — HAF-3 count done (a7d72a0): four `shelf-support.5mm` per adjustable shelf, none on fixed ones; the article waits for the owner (Häfele data).
- 2026-10-26b (night) — HAF-2 done (f844e59): `handle_size` (knob, 96–448 centres) and `handle_place` (corner, centre, edge pull) pickers; Ø5 handle screw holes through doors (free edge found from the hinge cups), drawer fronts and lift fronts; handle rows in the hardware list. Every golden with fronts gains them.
- 2026-10-26b (night) — BLUM-7b HK-S/HK-XS done (1b0e81f): two new lift kinds (small stay lift, stay lift on hinges) with their mechanisms by power factor (TIP-ON sets under TIP-ON), HK-XS doubled on two sides, hinges 2/3/4; drilling left as BLUM-7c (waits for the CAD like HF top).
- 2026-10-26b (night) — DATA-5b part (99dba8e): 106 Laminex compact laminate boards (guide pp. 76-77, 13/16/18 mm); product lines cross-checked 106/106. Re-run with --retrieved 2026-10-23 (same guide edition, Sep 2025): every earlier record unchanged.
- 2026-10-26b (night) — DATA-5b part (b5092fd): 28 Trade Essentials plywood boards (p.104, decorative and non-structural half); bending ply keeps both grain directions as printed.
- 2026-10-26b (night) — HAF-6 built (299d534): inspector Lights section with drawn zone tiles and voltage/power chips; not seen on screen.
- 2026-10-26b (night) — OUT-5c built (96c4edb): Cut List › Sheets shows the nesting pages with kerf and trim steppers and Export PDF; OUT-5d added for sheet size choice and offcut reuse.
- 2026-10-26b (night) — OUT-5d built (e4b4446): each board's sheet size chosen by least area among its catalog sizes; kept offcuts nested on first, stored in Application Support, kept/forgotten from the Sheets view.
- 2026-10-26b (night) — PLG-3 checked: no plugin runtime yet (#1221, draft #4980 open; #5586 replaces #4330); SolidCraft v0.23.0 is out (this Mac's CLI is 0.20.0).
- 2026-10-26b (night) — BLUM-7b HKi done (f87997e): integrated stay lift with its side pockets cut (265 × 120 × 12, 21 below the top); semi-integrated not offered.
- 2026-10-26b (night) — HAF-1 and UI-7 set to needs-human (supplier data; room planner design), UI-15 blocked on UI automation permission.
- 2026-10-26b (night) — P2-5 split; P2-5a built (ab3f118): quote from a price list the owner types in (board per m² of nested sheets, tape, hardware, labour, markup, GST); missing prices listed.
- 2026-10-26b (night) — P2-5b built (76cdcbf): customer quote PDF (price per cabinet, tax included), rates popover.
- 2026-10-26b (night) — P2-2 split; P2-2a done (0a5ea06): shaker fronts (stiles, tenoned rails, grooved panel), machining moved to the members.
- 2026-10-26b (night) — P2-2b done (e88b3c7): glass frame fronts.
- 2026-10-26b (night) — P2-3 split; P2-3a done (29a1648): wardrobe kind with hat shelf and rails.
