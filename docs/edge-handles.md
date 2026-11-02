# Plan: over-the-edge pulls, sharknose and bevelled fronts

Written 2026-11-02 at the owner's request. The sharknose rows are since built (SEC-8 to SEC-10, see
docs/sections.md), apart from SHARK-4; the edge pull rows are built for the ABI pulls (ABI-5), apart from side mounting (EDGE-6). Decisions for the owner are at
the end.

Two different things share the top edge of a front:

- **Over-the-edge pulls** are hardware you order and fit (ABI Rappana and Scalo).
- **Sharknose and bevelled fronts** have no hardware: the board itself is the grip.

Both need the same thing nothing has yet: **a way to check there is room around that edge.**

## What the drawings say

Read from ABI's spec drawings, product notes and photos (local only, in `vendor-cad/abi-bim`). All
are nominal, as ABI's drawings say.

| | Rappana 50, 100, 500, 800 | Scalo |
|---|---|---|
| Shape | Folded U, about 1.2 thick, 36 across the top | Folded L, 2 thick, 35 across the top, 50 wide |
| Fixing | Back leg 18 long, screwed to the **back** of the front, screws 10 below the top | Plate screwed **down into the top edge**, Ø4 holes 32 apart, 9 from the ends |
| Hole distance from | The top of the leg | The back of the plate, 10 |
| On show | A lip 10 deep, standing proud of the face | A half disc, R25, 30 deep, standing proud of the face |
| Screws | 2 on the 50 and 100, 3 on the 500, 4 on the 800 | 2 (M3 × 14, 16 or 18, in the box) |
| Mounts on | Top or side edge | Top or side edge |

My reading of Scalo: the holes are 10 from the plate's back, so the plate sits on the board with
its back flush with the front's back, and overhangs forward to make the finger gap. A 35 plate on an
18 board leaves 17 for fingers. ABI's drawing doesn't state this, so check it on a real one
(question 1).

## Where the code stands

| Item | Today | Problem |
|---|---|---|
| Rappana, Scalo | Skipped by `scripts/import_abi_handles.py` (`edge_mount`) | Not in the catalogue |
| `HandlePlace::EdgePull` | A generic lip drawn by the FFI, `handle.edge-pull` | No catalogue row, no bores, nothing checks it fits |
| `HandlePlace::Sharknose` | Done: 30, 45 or 60° bevel, a finger gap (default 15), on the door edge chosen | The finger gap is a setting, not checked against the top gap |
| Sharknose on cut sheets | Done | |
| Sharknose in the viewport | Done: a true slope | |
| Edge tape on a bevel | The front's edges are banded as usual | The bevel cuts through the tape on its edge |

## Room around the edge

This is the core of the work, one rule set shared by all of it:

| Check | Over-the-edge pull | Sharknose or bevel |
|---|---|---|
| Above the front | The gap must clear the plate: Scalo 2, Rappana 1.2, plus 1 to run. **The default top gap of 2 is too small for both.** The 3 between fronts is enough | Fingers have to get in: 3 between fronts isn't a grip |
| Behind the front | Rappana's leg and screw heads sit between the front and whatever is behind it: the rail on a top drawer, the top panel behind a door | Nothing |
| Along the edge | Must fit the front's width. A long Rappana must also clear hinges and front fixings | Nothing |
| Board | Screws must stop inside it; Scalo's holes need board around them (10 from the back, so at least 16 thick) | The bevel must leave a nose: at least 4 thick, so 12 in needs 16 |
| Front style | Scalo on a shaker bores into the rail's end grain: allow it, warn | Slab only, as now |

Each failed check gives a warning that names the fix, in the same way as the handle fit and SERVO-DRIVE
bracket warnings.

## Work, in order

These touch the same part of `build.rs` as sections and sharknose.

| Row | Size | What | Done when |
|---|---|---|---|
| EDGE-1 | S | Catalogue: a `mount` parameter (`face`, `back_leg`, `top_plate`) on handles; import Rappana and Scalo with their frames from the drawings, not guessed; add a catalogue row for the generic edge pull | All five are in the catalogue with screw patterns, `verified: false` |
| EDGE-2 | M | Builder: fit an edge-mounted pull centred on the gripping edge. Blind pilots in the back face (Rappana) or down into the top edge (Scalo), shown on the cut sheet. A pull wider than the front is left off, with a warning | A Rappana's pilots are 10 below the top, in the back, and don't break through |
| EDGE-3 | M | The room rules above, in one function, used by EDGE-2 and SHARK-2 | A Scalo under a top gap of 2 warns; at 3 it doesn't |
| EDGE-4 | S | Models: the real mesh over the edge, in the viewport and SolidCraft, with its lip on the face; replaces the placeholder lip when a real pull is chosen | Checked by eye in both, on a drawer and a wall door |
| EDGE-5 | S | Inspector: picking an edge-mounted pull by photo sets the place to Edge pull. Picking Corner or Centre with one chosen asks for a pull that suits it. No number fields | Can't reach a Rappana bored through the face |
| EDGE-6 | S | Side mounting: on a door's free edge at hand height, as in ABI's photo of tall doors. Needs the frame rotated and the hinge side known | A Scalo on a tall door's free edge |
| SHARK-1 | done (SEC-8) | Sharknose becomes a picked profile: legs and angle as catalog-style data with pictures (for example 30°, 45°, 60°), not constants | Picture tiles change the chamfer, and golden part lists follow |
| SHARK-2 | done (SEC-8) | Finger room: raise or warn on the gaps a grip needs | A sharknose under a 2 mm top gap warns |
| SHARK-3 | done (SEC-8) | Draw the true slope in the viewport (the FFI's mesh, not a step) | The viewport matches SolidCraft |
| SHARK-4 | S | Edge finish: a bevelled edge isn't banded; say on the cut list how it's finished | The cut list doesn't tape a bevelled edge |
| BEVEL-1 | M | Depends on question 3: a bevel on the face, or an applied bevelled profile | Set after the answer |

Later: all of this as a per-section choice, as `opening` is now. A cabinet could have a
Scalo on its drawers and sharknose doors.

## Decisions from the owner (2026-11-02)

The owner's Rappana 100 specification: plate 36 deep, lip 10 down at the face, back leg 18 long and
2 thick, holes 10 below the top, 75 apart, 13 in from the ends; mounts from top and side; 2 × 16 mm
screws, driven from behind through the leg into the back of the front.

1. **Scalo's position.** The plate's back is flush with the front's back, overhanging forward.
2. **Rappana's leg behind a front.** Rebate the back of the front so the leg sits flush.
3. **Bevelled edge.** Sharknose, built (SEC-8 to SEC-10).
4. **Finger room for sharknose.** A Finger gap setting, 10 to 30, default 15 (our choice), with rails
   behind the bevel.
5. **Clearance over an edge pull.** 2 mm between the plate and whatever is above it, raised
   automatically: the pull sits on the top of the front and needs that room.
6. **Wide fronts.** One pull, centred. Longer fronts take a longer Rappana.
