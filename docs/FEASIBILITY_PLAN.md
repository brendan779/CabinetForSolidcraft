# Cabinet plugin for SolidCraft: feasibility and plan

Working name: **CabinetForSolidcraft** (app: Cabinet Studio). First written 2026-10-23.

## Revision 2 (2026-10-23, later the same day)

- **Plugin status re-checked against SolidCraft v0.20.0** (released today, installed here).
  Preferences › Plugins exists, but per its source (`crates/ui-egui/src/plugins.rs`) it only lists
  `plugins/<id>/plugin.json` folders and turns them on or off: "It does not load or run anything".
  The runtime is still the open draft (#1221, PR #4980). The conclusion below stands.
- **Decision: native macOS app first.** It is written in Rust (the core) plus SwiftUI built in
  Xcode, and talks to SolidCraft over its control API. Functionality moves into a SolidCraft plugin
  as that becomes possible (BACKLOG P3).
- **Australian boards are in scope:** Polytec, Laminex and others, including specialty boards,
  linings and edge tape (BACKLOG DATA-1…9).
- **Supplier CAD:** the owner supplies Blum and Häfele downloads. They're used locally from
  `vendor-cad/` and are never committed to the public repo.
- **Built and verified today:** see the README. A live build into SolidCraft 0.20 works. At
  Manufacturing detail, each cut slows as panels gain features (tall cabinet: 293 commands in
  121 s), so batching per panel (PERF-1) is the top performance item.

The rest of this document is the original feasibility study. Where it disagrees with Revision 2
or BACKLOG.md, those win.

## 1. Verdict

**It can be built, but SolidCraft can't host it as a normal plugin yet.** The geometry side is ready:
SolidCraft has sketch/extrude, booleans, holes, patterns, components, parameters, configurations,
materials, STEP import, a product render workspace and an automation surface (MCP, a control
socket, JSON scripts). The *plugin* side is not ready. The WebAssembly plugin runtime is still a
design proposal: issue #1221 and draft PR #4980 are open and no code has merged. The proposed panel
API also has no images, so it can't draw the visual pickers you want.

So the plan does not wait on upstream:

1. Write all the cabinet logic as a host-independent Rust library (`cabinet-core`).
2. Ship it first as a **companion app** with its own visual configurator. The app drives the
   running SolidCraft through its control protocol, the same channel MCP uses. This works today.
3. Package the same core as a **SolidCraft WASM plugin** when the runtime lands, and
   upstream a "visual choice" widget so the pickers can move inside SolidCraft.

Matching the commercial products (Cabinet Vision, Microvellum, Mozaik, Polyboard, imos) is roughly a
**12–18 month** effort for one focused developer working with AI help. A useful MVP (boxes to frameless
carcasses with doors, drawers, Blum hardware, biscuit/screw joinery, cut list and cut sheets) is
about **10–14 weeks**. Section 10 has the phasing.

## 2. What SolidCraft is (findings)

| | |
|---|---|
| Repo | [tangency-labs/solidcraft](https://github.com/tangency-labs/solidcraft): pure Rust, MIT/Apache-2.0, ~9.4k stars, v0.15, very active (commits daily) |
| Kernel | Its own B-rep kernel. 75/76 reference parts match analytic or FreeCAD volumes |
| Apps | Desktop (egui + wgpu, Win/macOS/Linux), browser (wasm), `solidcraft-cli`, MCP server |
| Modelling | Sketches with a constraint solver, extrude/revolve/sweep/loft, fillet/chamfer, holes, threads, patterns, mirror, shell, split, combine |
| Parameters | Unit-aware expressions, dependency graph, CSV/JSON import/export, configuration tables (`config.*`) |
| Assemblies | Components, occurrences (instances), joints, interference checks, mass properties, `component.insert_design` (linked designs) |
| Materials/looks | `material.*`, `appearance.*`, per-body and per-face looks |
| Render | A **Render workspace** with a CPU product-image renderer and PNG export (`render.start/export`). Opaque PBR is opt-in. Texture *decoding* exists but texture *rendering* (wood grain) is not wired yet |
| Drawings | **Deferred (M11).** Only "Export current view as DXF" (hidden-line projection) exists |
| Files | STEP/IGES in and out, 3MF, STL, OBJ, glTF, DXF |
| Automation | Every UI action is an engine command. The same JSON goes through MCP, the control socket (`--control 7878`, token auth) and `solidcraft-cli run script.json` |
| Data in designs | `document.metadata.set` / `component.metadata.set` with namespaced keys (64 KiB per value, 256 KiB per document) |

### Plugin system status (the critical constraint)

- `docs/plugins.md`: design only. A future `FeatureKind::Plugin` timeline row would rebuild from
  stored params, but that is "phase 2".
- PR #4980 (draft, 2026-10-22) proposes WIT `solidcraft:plugin@0.1.0` with two worlds:
  - `command-plugin`: transactions that run engine commands, one undo step each, 10 s default
    (max 120 s), 10 000 commands per transaction.
  - `panel-plugin`: a host-drawn panel. Its widgets are **text, buttons, checkboxes, number inputs and
    dropdowns only. No images or plugin artwork** (an explicit non-goal for 0.1). It can also place
    pins in the viewport.
- Today only `plugins/<id>/plugin.json` discovery and an enable/disable page exist. Nothing runs.
- Parametric plugin features, exporters, custom workspaces and background jobs are all listed as
  "later".
- **Prior art / competition:** a team announced **KorpusPRO**, a furniture/cabinet plugin for
  SolidCraft, in #1221 (2026-10-19). It's worth watching, and possibly collaborating with them on the
  upstream widget work.

### What this means for the requirements

| Your requirement | SolidCraft today | How we cover it |
|---|---|---|
| Turn boxes into cabinets | Boxes can be read through `document.inspect`, `model.faces` and `measure` | Box recognizer in the plugin (section 4) |
| Fully parametric | Parameters/configs exist, but there's no plugin timeline feature | The plugin owns the cabinet spec, stores it in design metadata, and rebuilds the component on change |
| Graphical options, no typing numbers | Panel API has no images | Companion configurator now; upstream "image choice" widget later |
| Blum and Häfele hardware | STEP import, components, instances | Parametric stand-in models plus the user's own downloaded STEP (section 7) |
| Biscuits and screws | Holes, cuts, patterns | Joinery engine emits slot cuts, biscuit bodies and screw holes (section 6) |
| Fully rendered solids | Render workspace, solid colours/PBR | Real B-rep panels and hardware. Wood-grain textures wait for upstream texture rendering |
| Technical cut sheets | No drawing workspace | The plugin generates its own sheets from its part model (PDF/SVG/DXF/CSV). This is how commercial tools do it anyway |

## 3. Architecture

```text
┌──────────────────────────────────────────────────────────────────────┐
│  cabinet-core (Rust library, no I/O, builds for native + wasm32)      │
│   domain model  →  construction rules  →  parts + machining ops       │
│   Cabinet, Carcass, Opening, Door, Drawer, Shelf, Joint, HardwareRef  │
│   catalog: Blum / Häfele / Lamello / screws (data files, versioned)   │
│   outputs: PartList, CutList, Nesting, DrillMap, BOM, SVG thumbnails  │
└───────────────┬──────────────────────────────┬───────────────────────┘
                │                              │
     ┌──────────▼──────────┐        ┌──────────▼─────────────┐
     │ solidcraft-emitter   │        │ sheets (PDF/SVG/DXF/   │
     │ parts → engine cmds  │        │ CSV/labels/BXF-ish)    │
     │ (sketch/extrude/cut/ │        └────────────────────────┘
     │  hole/component/...) │
     └──────────┬──────────┘
        ┌───────┴────────────────────────┐
┌───────▼──────────────┐      ┌──────────▼──────────────────────┐
│ Phase 1: Companion    │      │ Phase 2: SolidCraft WASM plugin │
│ app (Tauri or egui)   │      │ command-plugin + panel-plugin   │
│ visual configurator,  │      │ (when PR #4980 lands)           │
│ talks to SolidCraft   │      │ + upstream "visual choice"      │
│ over control socket   │      │ widget, or a Cabinet workspace  │
└───────────────────────┘      └─────────────────────────────────┘
```

Key decisions:

- **The cabinet model is the source of truth, not the B-rep.** The cut list, sheets, BOM and CNC data come
  from `cabinet-core`'s part list. SolidCraft geometry is a faithful *rendering* of that list. All
  commercial tools work this way, and it sidesteps the missing drawings workspace.
- **Rust**, to match SolidCraft. We can reuse its expression and unit conventions and later move
  into the WASM plugin unchanged.
- **One cabinet = one SolidCraft component.** Each panel is a body named by role (`B01.LeftSide`).
  Hardware goes in as linked component instances (`component.insert_design`), so 40 hinges share one
  model.
- **Spec storage.** A compact JSON spec goes in `component.metadata` under `plugin.cabinetcraft.*`.
  Shared "construction standards" go in document metadata and cabinets store only overrides. This
  keeps a 40-cabinet kitchen inside the 256 KiB metadata budget. The full project file can also live
  alongside the design as `.cabinet.json`.
- **Rebuild = regenerate.** With no parametric plugin feature yet, editing a cabinet deletes and re-emits its
  component in a single transaction (one undo step). Persistent names stay stable because body names come
  from part roles.
- **Level of detail.** *Design* LOD has plain panels and simplified hardware. *Manufacturing* LOD
  adds every bore, groove, biscuit slot and screw. A kitchen at manufacturing LOD means thousands of
  boolean ops, so it's generated on demand, per cabinet or for export.

## 4. Box → cabinet workflow

1. The user blocks out the room with plain boxes in SolidCraft (`solid.box` or sketch + extrude), or
   draws them in the configurator's room planner.
2. **Select box(es) → "Make cabinet".** The recognizer checks that each body is a cuboid (6 planar faces,
   orthogonal) and reads its size and placement through `model.faces` and `measure`.
3. **Pick the front face** with a click in the viewport. Suggestion: the face pointing away from the nearest wall,
   or −Y.
4. **Pick a cabinet type** from a visual gallery (base, wall, tall, sink, corner, drawer bank,
   appliance housing, open shelf, vanity, wardrobe…). Size-aware defaults apply: a 2100 mm tall box
   suggests a tall pantry, 720 mm above 1400 mm off the floor suggests a wall cabinet.
5. The plugin hides the original box and emits the cabinet component in its place, keeping its
   transform. The box stays as a hidden *envelope*: resizing the box and choosing "Update" rebuilds.
6. **Runs of boxes:** select several adjacent boxes and you get a run, with shared fillers, continuous
   worktop, scribes, end panels and plinth.

## 5. Visual option system (no typing numbers)

Every construction choice is a **picker of thumbnails**. The thumbnails are SVG/raster images *generated
by `cabinet-core` from the live cabinet*, so they show your actual proportions and not a stock icon.
Numbers are still there in an "Advanced" fold for people who want them, but defaults come from the
construction standard.

| Group | Visual choices (each one a thumbnail) |
|---|---|
| Carcass construction | Frameless (32 mm/European) · Face-frame (US) · Inset face-frame |
| Top-to-side joint | Top between sides · Top over sides · Mitred · Rebated · Rails (front + back stretchers) |
| Bottom-to-side joint | Bottom between sides · Bottom under sides · Rebated · Dadoed |
| Back panel | Inset in groove · Rebated · Applied (screwed on) · Full-thickness structural · Hanging rail + thin back · None |
| Base / toe kick | Integrated notch · Separate plinth box · Adjustable legs + clip plinth · Feet · Floating/wall-hung |
| Door style | Slab · Shaker (5-piece) · Raised panel · Glass frame · Profile-frame aluminium · Handleless (J-pull, TIP-ON, SERVO-DRIVE) |
| Door mounting | Full overlay · Half overlay · Inset · With visualised reveal gaps (choose the gap from 1.5/2/3 mm swatches) |
| Door split | Single L/R · Pair · Lift-up · Bi-fold up · Stay lift · Up-and-over · Pocket/sliding |
| Drawer stack | Drag dividers on a cabinet thumbnail; presets 1/2/3/4, 1-2-3 graduated, inner drawers |
| Drawer box | LEGRABOX · MERIVOBOX · TANDEMBOX · Wood box on MOVENTO/TANDEM · Wood box on Häfele Matrix runners |
| Drawer joinery (wood) | Dovetail · Biscuit + screw · Dowel · Rebate + pin · Locked rabbet |
| Shelves | Adjustable on 32 mm pins (choose the row graphically) · Fixed (biscuited) · Pull-out · Glass |
| Edge banding | Click edges on a panel diagram to toggle banding; pick a band from swatches |
| Grain direction | Arrows on a panel thumbnail: vertical / horizontal / continuous across doors |
| Fixings | Choose a fixing recipe per joint type (section 6) as icons: biscuit, screw, confirmat, dowel, Clamex, Minifix |
| Handles | Gallery from catalog; placement chosen on a door thumbnail (top-corner, centre, edge-pull) |
| Lighting | Click where light goes on a cabinet sketch: under-cabinet, in-cabinet top, shelf edge, plinth, drawer interior |

A cabinet-wide **"construction standard"** (for example "My shop: 18 mm ply, biscuit + screw, inset
back, Blum CLIP top BLUMOTION") is defined once as a visual preset and applied to every
cabinet. This is what makes it feel commercial-grade.

## 6. Joinery engine (biscuits + screws)

Joints are first-class objects between two panels. A **joint recipe** decides what gets made:

| Fixing | Geometry emitted | Rules |
|---|---|---|
| **Biscuit** (Lamello-style #0 / #10 / #20, nominal 47×15×4, 53×19×4, 56×23×4 mm) | Arc-ended slot cut in both panels (≈100 mm cutter, 4 mm kerf, depth ≈ half biscuit width + 1 mm), plus a biscuit solid (lens shape) | Size chosen from joint length and panel thickness. End setback ≥ 50 mm. Spacing 100–200 mm (user-picked "dense / normal / sparse"). Slot centreline at panel mid-thickness, or offset for thin panels |
| **Screw** (generic Ø4 × 50 chipboard/wood, or Ø7 confirmat) | Countersunk clearance hole + pilot in the mating panel. Optional cap/cover | Placed *between* biscuits, so they never collide. Hidden faces preferred (end panels get "no visible screws" automatically) |
| Pocket screw | Angled pocket + pilot | Face-frame option |
| Dowel Ø8 × 35 | Paired bores on 32 mm grid | Alternative recipe |
| Lamello Clamex P / Tenso P | P-System slot (biscuit-like, CNC/Zeta P2) | Knock-down option. Same slot logic as biscuits |
| Häfele Minifix / Rafix / Ixconnect cam | Cam bore Ø15 + bolt bore | Flat-pack option |

- **Recipe example ("Biscuit + screw", default):** biscuits for alignment, plus one screw between each
  pair of biscuits, on top/bottom-to-side, fixed shelves and stretchers. Backs get screws on 150 mm
  centres into the rebate.
- **Collision check:** joint features are tested against hinge cups, runner screw lines, 32 mm shelf-pin
  rows and edge setbacks before emitting. Conflicts are moved automatically and flagged on the cut sheet.
- **Output:** each biscuit slot and screw hole goes on the part's machining list. It shows on cut sheets
  (dimensioned from the reference corner) and in CNC/DXF exports, and as real cuts in Manufacturing LOD.

## 7. Hardware catalogs (Blum, Häfele)

### What exists

- **Blum** publishes its catalogue and technical manual online (`publications.blum.com`) and planning
  "TD" sheets per product. 3D/2D CAD is in **E-SERVICES** (Product Configurator, Product Database),
  which **needs a customer login**; guests get a basic range. The Configurator exports **BXF** (Blum
  Exchange Format, hardware plus wood-part cut sizes and drill positions). DYNALOG holds Blum's own
  AutoCAD/SolidWorks interfaces.
- **Häfele** runs a CAD database (≈50 000 CAD and 18 000 CAM data sets per its services page) with 2D/3D
  downloads per article. **Loox5** is its 12 V / 24 V LED system: strips in aluminium profiles,
  drivers 20–240 W, modular distributors, switches (motion, IR, touch dimmer), all with Häfele article numbers.

### Legal and practical constraint

We **cannot bundle Blum or Häfele CAD files or copy their catalogue content** into an open-source
plugin. That would be redistribution of copyrighted data, and SolidCraft's own clean-room rules also
forbid it. The plan:

1. **Rules from public planning data, re-expressed as our own data.** For each product family we
   store *functional* parameters (article number, opening angle, overlay range, cup Ø35, cup
   boring distance, mounting-plate drilling, runner nominal lengths, width deductions, load class)
   in our own JSON schema, with a source reference and a "verified by" field. Facts and dimensions
   aren't copyrightable, but they must be checked against the current Blum/Häfele document
   before release.
2. **Our own parametric stand-in 3D models** (simplified hinge, plate, runner, box sides, lift
   mechanism, LED profile) generated by the plugin. They're good enough for renders and clash checks.
3. **"Bring your own CAD":** the user downloads STEP from their own Blum/Häfele account and drops it
   into the plugin's library folder. The plugin maps it to the article number and uses it in place of the
   stand-in (STEP import already works in SolidCraft).
4. **Trademarks** used only nominatively ("compatible with Blum CLIP top"), with no logos.
5. Optional later: **BXF import/export** so shops can round-trip with Blum's configurator. This needs the
   format spec from Blum, so treat it as a partnership item.

### Families to model (MVP in **bold**)

| Category | Blum | Häfele |
|---|---|---|
| Hinges | **CLIP top BLUMOTION 110°/107°/155°/170°, inset/half/full overlay**, MODUL, blind-corner, glass-door, thick-door, angled | Metalla 510 / Duomatic hinges |
| Mounting plates | **CLIP wing / cruciform, 0/3/9 mm, screw-on/INSERTA/EXPANDO** | matching plates |
| Drawer systems | **LEGRABOX pure/free**, **MERIVOBOX**, **TANDEMBOX antaro/intivo**, METABOX | Matrix Box / Matrix Box P |
| Runners (wood drawers) | **MOVENTO**, **TANDEM** (width deduction 42 mm, drawer length = NL − 10 per Blum planning sheet) | Matrix runners A / S |
| Lift systems | AVENTOS HK top, HK-S, HK-XS, HF top, HS top, HL top, HKi (power-factor based selection) | Free flap / Free fold / Free swing / Free up |
| Motion | TIP-ON, TIP-ON BLUMOTION, SERVO-DRIVE | Push-to-open |
| Inner dividers / organisation | AMBIA-LINE, ORGA-LINE, SPACE TOWER, SPACE CORNER | Kitchen interior fittings, corner carousels |
| Connectors / knock-down | n/a | **Minifix / Rafix / Ixconnect**, dowels, shelf supports |
| Legs / plinths | n/a | **Adjustable legs + plinth clips** |
| Handles | n/a | Handle gallery (dimensions, CC spacing) |
| Lighting | n/a | **Loox5 strips + profiles (surface / recessed / shelf-edge), drivers, distributors, switches/sensors** |

### Lighting planning (Loox5)

- Lights are placed visually (under-cabinet, in-cabinet, shelf, plinth, drawer). The plugin
  picks a profile compatible with the panel thickness (recessed needs a groove, which it adds to the
  machining list) and cuts strip length to the run.
- **Electrical rules:** keep 12 V and 24 V separate. Total watts per driver ≤ ~80 % of rating. Count
  distributor ports, lead lengths and switch per zone. Each cable route through panels needs a cable
  hole (Ø8–10) or grommet, added automatically to the parts.
- Outputs: a lighting BOM (strip m, profiles, end caps, drivers, distributors, leads, switches) and a
  wiring diagram sheet.
- Render: an emissive strip shows only after SolidCraft's renderer supports emissive materials. Until
  then it's shown as a bright appearance. Track this upstream.

## 8. Outputs

1. **3D solids in SolidCraft:** each panel is a real body with material and appearance, plus hardware
   instances and (at Manufacturing LOD) all bores, grooves and slots. Export to STEP/3MF/glTF/OBJ works
   for free.
2. **Rendered images** through SolidCraft's Render workspace (`render.start` / `render.export`). Preset
   camera setups per run. Wood grain arrives when texture rendering lands (#900 track).
3. **Cut sheets (per part):** a dimensioned 2D view of each face that has machining. Includes part ID,
   cabinet, material, thickness, finished size and cut size (edge band compensation), grain arrow,
   edge band legend, a hole/slot table from a reference corner, biscuits and screws shown, and a QR/label.
   Output as PDF (multi-page) and SVG.
4. **Cut list:** CSV/XLSX grouped by material, with edge banding metres per band type.
5. **Sheet nesting / optimisation:** a guillotine panel-saw optimiser with grain and kerf rules, plus
   offcut tracking, and PDF nesting diagrams per sheet. CNC nesting (true shape) comes later.
6. **Hardware BOM and order list:** Blum/Häfele article numbers, quantities, and per-cabinet packing
   lists.
7. **Assembly sheets:** an exploded isometric per cabinet (SolidCraft exploded views + "Export view as
   DXF/PNG") with the fixing sequence.
8. **Machine output (later):** DXF per part with layer conventions (cuts, bores by Ø/depth, slots).
   Optional BXF / MPR / CIX post-processors.
9. **Quote / costing (later):** material, hardware and labour rates.

## 9. Commercial parity checklist

Grouped by what Cabinet Vision, Microvellum, Mozaik, Polyboard and imos all have:

- Room planner: walls, openings, appliances, ceiling height, wall-aware runs. **(Phase 3)**
- Cabinet library of 60+ types with corner solutions. **(MVP has 12 types)**
- Construction standards and material libraries. **(MVP)**
- Doors/drawer fronts with styles and reveal control. **(MVP: slab + shaker)**
- Hardware rules and auto-selection by size and weight. **(MVP: Blum hinges/drawers)**
- Worktops, end panels, fillers, scribes, plinths, cornices, light rails. **(Phase 2–3)**
- Cut list, nesting, labels, BOM, costing. **(MVP: cut list and sheets; nesting in Phase 2)**
- CNC output. **(Phase 4)**
- Customer presentation renders. **(depends on upstream rendering)**

## 10. Phased plan

| Phase | Scope | Est. (1 dev + AI) | Depends on |
|---|---|---|---|
| **0. Spike** (1–2 wk) | Install SolidCraft, connect via MCP/control socket. Emit a hard-coded 600 × 720 × 560 base carcass as named bodies. Measure command throughput and boolean cost for 50 biscuit slots. Prototype one SVG picker | 1–2 wk | — |
| **1. MVP core** | `cabinet-core` model; frameless base/wall/tall carcasses; doors (slab), drawer fronts; Blum CLIP top + LEGRABOX/MOVENTO rules; biscuit + screw joiner; box recognizer; regenerate-on-edit; cut list CSV; per-part cut sheets PDF/SVG | 8–10 wk | Phase 0 |
| **1b. Visual configurator** | Companion app (Tauri + three.js preview, or egui) with all section 5 pickers, construction-standard presets, live link to SolidCraft | 3–4 wk (overlaps) | Phase 1 model |
| **2. Hardware breadth + nesting** | AVENTOS, TANDEMBOX, MERIVOBOX, TIP-ON; Häfele connectors, legs, handles, Loox5 lighting + electrical rules; guillotine nesting; hardware BOM; face-frame construction; shaker/glass doors | 10–12 wk | — |
| **3. Rooms and runs** | Room planner, runs, corner cabinets, fillers/scribes, worktops, end panels, cornices; assembly sheets; quoting | 10–12 wk | — |
| **4. Plugin + manufacturing** | Port to `command-plugin`/`panel-plugin` when #4980 ships; upstream PR for an image-choice widget; DXF/CNC export; label printing; BXF if Blum cooperates | 8–12 wk | Upstream plugin runtime |
| **5. Polish** | Textured renders (when upstream supports them), large-project performance, docs, sample kitchen | ongoing | Upstream render |

Roughly **12–18 months to commercial parity**. The MVP (Phases 0–1b) is about **10–14 weeks**.

## 11. Risks

| Risk | Impact | Mitigation |
|---|---|---|
| Plugin runtime slips or changes shape | Can't ship "inside" SolidCraft | Companion app first. Keep the core host-agnostic. Engage on #1221 / #4980 early (especially an image-choice widget) |
| No images in the plugin panel API | Visual pickers can't live in SolidCraft | Upstream a `choice` variant with host-rendered SVG thumbnails, or a plugin workspace (both are listed as "later" in #4980) |
| Boolean performance at Manufacturing LOD | Slow rebuilds on big kitchens | LOD system. Machining only on demand. Instanced hardware. Batch commands |
| Metadata size cap (256 KiB per document) | Big projects overflow | Shared standards + overrides. Per-component metadata. Sidecar `.cabinet.json` |
| Hardware data accuracy / licensing | Wrong drillings ruin parts; legal exposure | Every catalog row cites its source document and must be verified. No vendor CAD bundled. "Bring your own STEP" |
| SolidCraft is pre-1.0 and changes fast (file format bumps) | Breakage | Pin a tested version. Run CI against SolidCraft nightly. Use only documented commands |
| Competing KorpusPRO effort | Duplicate work | Differentiate on joinery depth, hardware rules and cut sheets, or collaborate |
| macOS: control socket needs the app running with `--control` | Setup friction | Configurator launches SolidCraft with the flag and reads the token file |

## 12. Decisions for you

1. **Companion-app-first** (works now) vs **waiting** for the plugin runtime. Recommended: companion first.
2. Primary construction style for the MVP: **frameless 32 mm** (recommended; Blum-native) or face-frame.
3. Units and market: metric-first with imperial display? This affects default materials (18 mm vs ¾") and
   which Blum article numbers are listed.
4. Do you have **Blum E-SERVICES / Häfele** accounts? They're needed to verify data and fetch your own STEP models.
5. Open-source the plugin (fits SolidCraft's ecosystem) or keep it commercial? This affects the KorpusPRO
   collaboration question and the licence of the catalog data.

## Sources

- SolidCraft repo, README, ROADMAP, `docs/plugins.md`, `docs/control-protocol.md`, render and drawing docs: <https://github.com/tangency-labs/solidcraft>
- Plugin issue #1221 and draft WIT PR #4980: <https://github.com/tangency-labs/solidcraft/issues/1221>, <https://github.com/tangency-labs/solidcraft/pull/4980>
- Hackaday on SolidCraft: <https://hackaday.com/2026/10/17/solidcraft-is-parametric-automation-friendly-cad-for-desktop-and-browser/>
- Blum E-SERVICES FAQ (CAD access, BXF, DYNALOG): <https://www.blum.com/gb/en/services/faq/>
- Blum product database: <https://blum.com/us/en/services/planning-construction-product-selection/product-database>
- Blum catalogue and technical manual: <https://publications.blum.com/2022/catalogue/en/423/>
- Blum MOVENTO/TANDEM planning sheet (width −42, length NL −10): <https://d2.blum.com/services/BEC003/me13029704_td_dok_bau_$sen_$aof_$v1.pdf>
- Blum product families overview: <https://www.blum.com/th/en/company/tradeshows/virtual-booth/>
- Häfele design tools / CAD services: <https://hafele.com/us/en/info/services/project-planning-customization/design-tools/143861>
- Häfele Loox5 launch: <https://www.hafele.com/us/en/info/about-haefele/press-and-media/press-releases/loox5-led-lighting-unveiling/105201/>
- Loox5 drivers and switches (retailer listings, verify against Häfele): <https://www.electrical4less.co.uk/product-category/hafele-loox-system/>
- Cabinet software landscape: <https://www.myarchitectai.com/blog/cabinet-design-software>
