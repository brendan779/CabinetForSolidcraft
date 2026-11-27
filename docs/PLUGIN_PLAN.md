# The SolidCraft plugin: workflow and interface

Plan of record for PLUG-0 to PLUG-6 in [BACKLOG.md](BACKLOG.md). Written 2026-11-27 against
SolidCraft 0.50.0. The owner's brief:

- The interface comes first: a simple, logical workflow, with fewer options than Cabinet Studio,
  each in one obvious place. Cabinet Studio's inspector is "a bit disjointed in the amount of
  options and their locations relative to each other".
- Boards can be text or simple swatches. Benchtops can be simplified. Handles and leg plinths can be
  simple shapes if meshes can't be brought in. The splashback is parametric only.
- No Blender or Revit for now.

## What carries over and what doesn't

The plugin is a thin shell around the crates we already have. `cabinet-core`, `cabinet-catalog`,
`cabinet-sheets` and `solidcraft-bridge` all compile for `wasm32-unknown-unknown` unchanged
(checked 2026-11-27). The Blum, Häfele and ABI data is built into the core with `include_str!`.

- **Shared, never copied:** cabinet rules, joinery, hardware, the part list, cut list, nesting, cut
  sheets, and the commands that build a cabinet in SolidCraft. A fix lands once and both the app and
  the plugin get it. The plugin crate never holds a cabinet rule. If it needs one, the rule goes
  into the core with its test.
- **New:** `crates/cabinet-plugin` (the WebAssembly component: entries, dialog hooks, panel data,
  design storage) and `plugin/` (the manifest, icons, option pictures, `tests/*.json`).
- **Stays in Cabinet Studio:** Blender, Revit, material packs, quotes, the elevation, lighting and
  lift systems (on hold anyway). The app keeps working throughout and is where interface ideas are
  tried first while the plugin catches up.

## What SolidCraft 0.50 gives an interface

| Piece | What it can do | What it can't |
|---|---|---|
| Ribbon | One group in the Model tab, with buttons in our order. `ribbon_groups` puts several entries behind one drop-down button | Our own ribbon tab, or a workspace |
| Dialogs (an entry's form) | Sections that fold, declared field order, fields shown or enabled by other fields, choices narrowed by other fields, pictures on choices (`x-solidcraft-image`, PNG up to 512 px), swatch grids with search, viewport pick fields, live preview on a scratch copy, a hook that computes defaults and hints from the design | Choices that aren't written in the manifest (a hook can only narrow them), pictures outside a dropdown, more than 64 KiB of manifest |
| Re-editing | `forms.open` reopens an entry's form filled with stored values: Edit is the same dialog as Add | — |
| Docked panel | Up to 8 panels, right or bottom, with tabs, splits, forms, toolbars, buttons, lists and tables up to 100,000 rows. Buttons and row clicks run our entries | Images, trees, charts (phase 3), refreshing itself when the design changes: it needs a Refresh button or one of our entries to run |
| Storage | `storage@0.1` keeps the job in the design. `user-storage@0.1` keeps the shop's own standard on the Mac (64 KiB) | — |
| Long jobs | 10 minutes, a million engine calls, progress and Cancel | Preview |
| Files | Writes into folders the user grants at install | Network, other programs |

What this means for the design:

1. **The catalogue can't be a dropdown.** 769 decors and 3,049 boards don't fit in a 64 KiB
   manifest or in 256 swatches. Boards are picked in a searchable table in the panel, then given a
   job role. The dialogs only refer to roles ("Fronts board"), which are a short, fixed list.
2. **Shapes are pictures and data is text.** Cabinet types, front layouts, how fronts open, edge
   profiles and corner shapes are picture dropdowns. Boards, sinks and hardware articles are text.
3. **One dialog per thing, reopened to edit.** Add and Edit are the same form, so an option is
   always where it was last time.

## The workflow

Five steps, in the order a cabinetmaker works. The ribbon buttons and the panel's tabs follow
this order, left to right.

```text
 1 Set up the job ─▶ 2 Lay out cabinets ─▶ 3 Add the run's pieces ─▶ 4 Check ─▶ 5 Make it
   Job setup          Cabinet                Benchtop, Panel,         Panel:      Panel: Cut list
   Boards tab         From boxes             Splashback               Job tab     Export
```

1. **Set up the job, once.** Boards for carcass, fronts and backs (Boards tab), and the job
   standard (Job setup): construction, fronts and hardware, the base and wall, standard sizes. A
   new job starts from the shop standard, so most jobs skip this step.
2. **Lay out the cabinets.** Add each cabinet beside the one picked, or draw boxes and turn them
   into cabinets. A cabinet asks only for its type, size and fronts.
3. **Add the run's pieces:** benchtop, end panels and fillers, splashback. Kick and plinth come
   with each run from the job setup.
4. **Check** in the panel's Job tab: every cabinet in a table with its warnings. Click a row to
   edit that cabinet.
5. **Make it:** the cut list, sheets and hardware in the panel, exported to a folder.

Going back is always the same move: pick the thing in the viewport or in the panel's table, and
its dialog opens filled in.

## Where every decision lives

The rule: **each option is decided at one level, by whoever decides it in a real job.**

- **Job:** everything a cabinetmaker sets once per job, in Job setup and the Boards tab.
- **Run:** what belongs to a row of cabinets: benchtop, kick, end panels.
- **Cabinet:** only what differs from cabinet to cabinet: type, size, fronts, and a few type
  extras.

A cabinet can differ from the job in only four things, kept in one folded section: door style,
how its fronts open, fronts board and hinge side. Everything else is job-only in the plugin. In
Cabinet Studio, every construction option can also be set per cabinet with the scope switch. That
doubles every option's location and is the main source of the disjointed feel.

### Job setup (one dialog, five folding sections)

| Section | Fields (pictures marked ▣) | Replaces in Cabinet Studio |
|---|---|---|
| **Boards** | Carcass, Fronts, Backs, Benchtop: read-only text showing the job's choices, with a hint "Change in the panel's Boards tab". Edge tape: same as board / contrasting (text) | Build tab › Boards, Boards workspace |
| **Fronts** | Door style ▣ (slab, shaker, glass). Door mounting ▣ (full, half, inset). **How fronts open** ▣ (handle, knob, edge pull, finger pull, sharknose, TIP-ON, SERVO-DRIVE). Shown only when they apply: handle size and finish (handle, knob), handle position ▣ (corner, centre), bevel ▣ and door edge ▣ (sharknose). Front grain ▣ | Fronts tab: Opening, Handle range, Handle, Handle finish, Handle place, Sharknose bevel, Sharknose on doors, Door style, Door mounting, Front grain |
| **Drawers and hinges** | Drawer system ▣ (LEGRABOX, MERIVOBOX, TANDEMBOX, MOVENTO, TANDEM). Sides (when it has a choice). Drawer frame. Hinge fixing | Fittings tab: Drawers, Drawer sides, Insert design, Hinge fixing; Fronts tab: Drawer frame |
| **Carcass** | Top ▣, bottom ▣, back ▣. Joinery ▣ (biscuit + screw, biscuits, screws, confirmat, dowels, cam lock, DOMINO), with its size and spacing shown only for the method chosen. Carcass thickness | Build tab: Top, Bottom, Back, Fixings, Dowel size, Dowel spacing, DOMINO size, Biscuit size, Fixing spacing |
| **Base, wall and sizes** | Base ▣ (legs and plinth, notched sides, none) and kick height. Wall hanging ▣. Standard height and depth for base, wall and tall cabinets. Benchtop height. Cut list: saw kerf and sheet trim | Cabinet tab: Base, Leg fixing, Wall hanging; Preferences; Cut List settings |

**How fronts open** merges four of Cabinet Studio's pickers (Opening, Handle range, Handle place
and the sharknose pickers) into one picture choice. That's why the sharknose was hard to find: it
was the fifth tile of the third picker, shown only after the first one was set to Handles.

Two buttons at the foot of the dialog: **Save as shop standard** (to user storage, so new jobs
start from it) and **Use shop standard**. Applying the setup rebuilds every cabinet as a long job,
with progress and Cancel. A long job has no preview, so the dialog shows what will change as text
(the hook's hints: "12 cabinets rebuild; hinges change to EXPANDO").

### Cabinet (one dialog for add and edit)

```text
┌ Cabinet ─────────────────────────────────────────────┐
│ Picked:  B03 · Base 600          [ Edit ▾ ]          │  edit / new beside it / new above it
│                                                      │
│ Type     ▣ Base  ▣ Drawers  ▣ Sink  ▣ Corner  ...    │  picture grid
│ Size     Width 600   Height job (720)   Depth job    │  job values unless typed
│ Fronts   ▣ 1 door ▣ 2 doors ▣ drawer+door ▣ 3 drw …  │  only layouts the type allows
│          Hinges on  ▣ left ▣ right                    │  1 door only
│          Shelves 1                                    │
│ ▸ Sink   pipe cut-out ✓   liner ✓                     │  shown for the type only
│ ▸ Different from the job (none)                       │  folded: 4 fields
│                                   ☑ Preview   [ Run ] │
└──────────────────────────────────────────────────────┘
```

- **Type:** base, drawers, sink, blind corner, L-corner, wall, wall corner, tall, wardrobe,
  rangehood, open shelf. Pictures drawn from the cabinet itself.
- **Mode (the first field):** the dialog hook reads the selection. A picked cabinet defaults to
  **Edit it**. The other choices are **New beside it** (on the side picked) and **New above it**
  (a wall cabinet over a base). With nothing picked, it's **New**. Woodwork guesses the mode from
  which part was picked. We say what will happen instead, and the user can change it.
- **Fronts:** presets drawn as pictures, narrowed by type (a drawer bank offers drawer stacks, a
  wall cabinet offers doors and lift-ups when LIFT-HOLD ends). Finer splits are a later option
  (SEC-*).
- **Type extras:** one folding section, only for its type: sink (pipe cut-out, liner), blind
  corner (blind side and width), L-corner (the two depths), wardrobe (hanging, shelves and drawers
  presets), rangehood (insert).
- **Different from the job:** door style, how fronts open, fronts board (a role or "as the job")
  and hinge side. Each starts at "As the job".

That is at most 9 visible fields. Height and depth are "job" until typed, so most cabinets are a
type, a width and a front layout: three clicks.

### From boxes

Draw boxes for the room layout with SolidCraft's own tools, pick them, then **From boxes**. The
dialog asks only for a type for all of them (or "guess from height": base, wall or tall) and
builds a cabinet in each box (SC-2's logic). Then edit cabinets one by one. This is our fastest
route for a whole kitchen, and Woodwork doesn't have it.

### The run's pieces

All simplified for the plugin's first version, as agreed:

- **Benchtop:** pick the cabinets (or one, and the run is found). Material: laminate, stone or
  timber (text). Thickness, front and side overhangs, edge profile ▣ (square, pencil round,
  bullnose, bevel). Cut-outs: a sink from the ABI list (text, the 16 sinks), or a size, and a
  cooktop size. No slabs, joins, veins, mitred aprons or waterfalls yet.
- **Panel:** pick a cabinet's side. End panel ▣, filler ▣ or seen end ▣, its width and board role.
- **Splashback:** pick the benchtop. Height and thickness, board role. Parametric only.
- **Kick and plinth:** automatic for each run, from Job setup.
- **Handles and legs:** supplier meshes if `file.insert_mesh` works within the plugin's grants
  (PLUG-0 checks), otherwise simple shapes at the right size and drilling: a bar on two posts, a
  knob, a cylinder leg. The drilling and the hardware list are right either way.

### The panel: "Cabinets", docked right

One panel with four tabs, in workflow order. A table row click selects that thing in the viewport.
A double-click opens its dialog.

| Tab | Shows | Toolbar |
|---|---|---|
| **Job** | Table: id, type, width × height × depth, fronts, warnings count. Below it, the warnings list (empty state "No problems found") | Add, Edit, From boxes, Rebuild all, Refresh |
| **Boards** | The job's boards on top (role, brand, decor, finish, thickness). Below, search and brand fields and the catalogue table (brand, range, decor, finish, thickness, sheet) | Use for carcass, Use for fronts, Use for backs, Use for benchtop |
| **Cut list** | Table: part, board, length × width × thickness, quantity, edges, grain | Export folder… then Cut list CSV, Cut sheets (SVG/PDF), DXF per sheet, Labels |
| **Hardware** | Table: item, article, supplier, quantity, cabinet | Export CSV |

The panel can't refresh itself when the design changes yet (phase 3 of RFC 0015). Every entry
of ours updates it when it runs, and Refresh covers edits made with SolidCraft's own tools.

### The ribbon

One group in the Model tab, in workflow order:

| Button | Entries |
|---|---|
| **Job setup** | `setup` |
| **Cabinet** | `cabinet` |
| **From boxes** | `from_boxes` |
| **Run ▾** | `benchtop`, `panel`, `splashback` |
| **Cabinets panel** | `panel_show`: opens the panel |
| **Rebuild** | `rebuild` (a long job) |

Exports are in the panel's Cut list tab, where the user checks what they're exporting.

## Data

- **In the design (`storage@0.1`):** the job: setup, board roles, each cabinet's spec (the same
  JSON as a `.cabinetjob` item), the run pieces. SolidCraft's bodies are rebuilt from it, so the
  design stays the source of truth, and it opens without the plugin as ordinary bodies.
- **On the Mac (`user-storage@0.1`):** the shop standard.
- **In the component:** the catalogue (boards, edge tape, hardware, sinks), built in.
- **Pictures:** PNGs in `plugin/pictures/`, made by a script from the drawings Cabinet Studio
  already uses for its tiles, so the two look alike. At most 256 in the package.
- **A job file from Cabinet Studio** opens in the plugin: an `import_job` entry reads a
  `.cabinetjob` from a granted folder into the design's storage, and builds it.

## Milestones

Each ends with `solidcraft-cli plugin test --require-cases` green in `scripts/check.sh`, and an
on-screen walk through the new steps for the owner.

| id | Size | What | Done when |
|---|---|---|---|
| PLUG-0 | S | **Spike.** The component builds from the core and makes one base cabinet. Measure: engine calls per cabinet, and whether `engine.batch` runs from a plugin; preview time; `forms.open` from the panel; `ui.set-data` from an entry run from a dialog; a picture dropdown; design and user storage; `file.insert_mesh` with a folder grant; writing an export. Write the answers into this file | We know each limit from a run, not from the docs |
| PLUG-1 | M | **Job and cabinets:** Job setup, Cabinet (add and edit, all types, front presets), Rebuild, design storage, the shop standard, the panel's Job tab | A kitchen run is laid out, edited and rebuilt in SolidCraft from the plugin alone |
| PLUG-2 | S | **Boards:** the Boards tab, roles, swatches from preview colours | A job's carcass and fronts are chosen from the catalogue |
| PLUG-3 | M | **Run pieces:** benchtop (simplified), panels and fillers, splashback, kick and plinth, handles and legs (meshes or simple shapes) | The kitchen run is complete to the benchtop |
| PLUG-4 | M | **Outputs:** Cut list and Hardware tabs, exports of CSV, cut sheets, DXF and labels | A job's cut list and sheets come out of SolidCraft |
| PLUG-5 | S | **From boxes** and **import a Cabinet Studio job** | A room drawn as boxes becomes cabinets; an existing job opens |
| PLUG-6 | S | **Polish and publish:** icons, help links, agent-tool descriptions, the Workshop listing (the owner publishes) | Installed from the Workshop on a clean Mac |

Rules for the work:

- Every cabinet rule stays in the core with its test. A bug found in the plugin that is a core bug
  is fixed in the core, and both get it.
- Every entry has `plugin test` cases: its inputs, and what `inspect.measure` should give.
- The plugin's option names and order follow this file. A change to the interface changes this
  file first.
- When SolidCraft adds image widgets, trees, workspaces or design-change listeners for panels
  (the PLG-3 weekly check), revisit the panel: a Cabinets workspace and pictures in the
  panel are the next step.
