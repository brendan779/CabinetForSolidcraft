# Sections (SEC-1, SEC-3)

The owner's ask: build a cabinet the way Blum's configurator does. Start from a carcass, split its
front, and set each section on its own. A section isn't a separate cabinet. SEC-3 made this the way
**all** fronts work, not one front type among many.

## Model

`FrontLayout::Sections { columns }` holds the cabinet's fronts and what's behind them:

```text
Sections
  columns: [Column]            left to right
Column
  width: mm                    the column's share of the fronts' width (0: shares what is left)
  rows: [Section]              bottom to top
Section
  height: mm                   its fronts' height (0: shares what is left)
  shelf_above: bool            a fixed shelf between this section and the next (jobs saved before
                               the choice existed: true)
  opening: handles | tip_on | servo_drive     where it differs from the job's
  front: open | door{hinge_side} | door_pair | drawers{count, ratios, system}
         | lift{kind, handle_weight} | bi_fold{handle_weight} | niche
  inside: empty | shelves{count, fixed} | shoe_shelves{count} | hanging | inner_drawers{count}
          | wardrobe
```

- **Every old layout converts.** `Cabinet::sectioned` turns each kind's front layout into sections
  and builds the same cabinet (`every_old_layout_builds_the_same_cabinet_as_its_sections`). The
  app converts on new cabinets, opened projects and selections. Corner kinds and rangehoods keep
  their own fronts.
- **Dividers:** a vertical panel stands between neighbouring columns, from the bottom up to the top
  (or rails) and from the carcass front to the interior's back, jointed like a side.
- **Fixed shelves:** only where `shelf_above` asks. A split between fronts doesn't make one, so a
  drawer cabinet has no shelf behind its drawers.
- **Fronts:** a row's fronts span its column's share of the front width. A door hinges on the side
  or divider on its hinge side, with half overlay on a divider. Drawer boxes, runners and front
  fixings fit the column's bay. A lift or bi-fold section goes across the top of the cabinet, one
  to a cabinet; elsewhere it warns and says why.
- **Inside:**
  - *shelves:* adjustable shelves on pins in the bay's panels, on the 32 mm grid.
  - *shoe shelves:* fixed shelves at an even pitch.
  - *hanging:* a wardrobe rail under the row's top.
  - *inner drawers:* LEGRABOX inner drawers from the row's floor.
  - *wardrobe:* the wardrobe preset's interior.
- **SERVO-DRIVE:** a drive unit and bumpers per driven drawer; a bracket, attachment, cable and node
  per compartment holding driven drawers, where a compartment runs between fixed shelves; one power
  supply per cabinet. An adjustable shelf in the way of a bracket gets a warning.

## Sliding doors

A `sliding` section has 2 to 4 doors sharing its width, each overlapping the next by `overlap`, every
other one in the front plane, between a top track and a bottom guide. `track` names a family of
hardware rows (`<family>.top`, `.bottom`, `.roller`, `.pull`); only the generic one exists yet, with
sizes of our choosing. They stand further forward than hinged fronts by a door and the plane gap.

## Drawer boxes

A section of drawers can pick `box_height` (n, m, k, c or f) and `nominal_length`; left out, it takes
the tallest and longest that fit. A pick that doesn't fit warns and uses what does. The lowest drawer
in a compartment stands on the floor; one above another stands on the level of its own front's bottom
edge, as in Blum's session. Every system's runner screws are drilled in the panels either side, from
`drilling` on the runner's catalog row.

## Drawer frame

`drawer_frame: board` puts one board (`<id>.DrawerFrame`, 16 or 18, of the front board) over the
carcass's front. Each drawer gets an opening across its bay, from its datum to `drawer_frame_overlap`
(default 25, adjustable, our choice) below its front's top, and each open section its clear opening.
The rails between them close the gaps, and come down behind the fronts past a sharknose's bevel (21
at 30°): stopped at the front's top, the inside shows down the bevel. Boxes are chosen to fit under
the rail, and a rail too short for the bevel gets a warning.

The carcass moves back behind the board and loses its thickness from its depth, so the fronts, the
back and the cabinet's overall depth stay where they were (the owner's call), and benchtops and
neighbours are unaffected. Boxes, runners and the SERVO-DRIVE line measure from the front's back, so
drawers keep their length; runners stand proud of the sides by the board's thickness, through the
openings, which the owner has accepted. The plinth stays in line with its neighbours' (except in
notched sides, where it goes back with them). Half-overlay fronts work the same way. Inset, separate rails (`<id>.DrawerFrame<n>`) stand inside the carcass
behind the fronts, so nothing moves; with more than one column dividers would cross them, so that gets a warning and no
rails. Doors and lifts need the carcass edge the board covers, so a cabinet with any gets no frame
and a warning.

## Sharknose doors

A sharknose door bevels one edge: `sharknose_door_edge` is `auto` (top, or bottom on a wall door),
`top`, `bottom` or `opening` (opposite the hinges, the finger gap taken off that edge; half each for
a pair). Behind that edge alone stands a `<id>.GripRail<n>`, in the door's back plane inside the
opening, from `drawer_frame_overlap` behind the door out to the carcass, so the hinges and carcass
don't change. Shelves stand behind a rail down the opening side. The carcass's own edges still show
in the finger gap, on purpose: for these jobs the owner uses a decorative carcass board.

## Inspector

**Fronts** is always the section editor: quick-start layouts (door, pair, drawers, lift, bi-fold,
oven, fridge) that are ordinary sections to carry on from, then the elevation drawn to scale. Click
a cell to select it, and under the drawing:

- **Split up**, **Split across**, **Remove**, and **Fixed shelf above**.
- **Front:** tiles for each front. Drawers add the stack picker and the drawer system. Lifts add
  the movement. Fronts that open choose handles, TIP-ON or SERVO-DRIVE, or "As the job".
- **Inside:** tiles for what can go behind this front (drawers and niches leave no room).
- **Height** (the row) and **Width** (the column): rows of usual sizes plus Auto.

## Left for later

- After 1.0: a divider is always full height. A divider inside one row only (a split within a drawer
  section) would be a tree, not columns of rows.
- Shoe shelves are flat fixed shelves today. The owner wants two types: sloped decorative board with
  a lip, or flat glass (SEC-2c).
