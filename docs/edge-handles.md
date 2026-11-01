# Plan: over-the-edge pulls, sharknose and bevelled fronts

Written 2026-11-02 at the owner's request. Nothing here is built yet, apart from one fix noted
below. Decisions for the owner are at the end.

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
| `HandlePlace::Sharknose` | A chamfer, 20 down × 12 in, both fixed in `build.rs` | Not a choice; nothing checks there's room for fingers |
| Sharknose on cut sheets | **Was missing**: `features()` ignored it, though WF-3 asked for it | **Fixed in this branch** |
| Sharknose in the viewport | Drawn as a step, not a slope (`cabinet-ffi`) | Looks wrong; SolidCraft gets a true chamfer |
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

Do these after SEC-3 merges: they touch the same part of `build.rs`.

| Row | Size | What | Done when |
|---|---|---|---|
| EDGE-1 | S | Catalogue: a `mount` parameter (`face`, `back_leg`, `top_plate`) on handles; import Rappana and Scalo with their frames from the drawings, not guessed; add a catalogue row for the generic edge pull | All five are in the catalogue with screw patterns, `verified: false` |
| EDGE-2 | M | Builder: fit an edge-mounted pull centred on the gripping edge. Blind pilots in the back face (Rappana) or down into the top edge (Scalo), shown on the cut sheet. A pull wider than the front is left off, with a warning | A Rappana's pilots are 10 below the top, in the back, and don't break through |
| EDGE-3 | M | The room rules above, in one function, used by EDGE-2 and SHARK-2 | A Scalo under a top gap of 2 warns; at 3 it doesn't |
| EDGE-4 | S | Models: the real mesh over the edge, in the viewport and SolidCraft, with its lip on the face; replaces the placeholder lip when a real pull is chosen | Checked by eye in both, on a drawer and a wall door |
| EDGE-5 | S | Inspector: picking an edge-mounted pull by photo sets the place to Edge pull. Picking Corner or Centre with one chosen asks for a pull that suits it. No number fields | Can't reach a Rappana bored through the face |
| EDGE-6 | S | Side mounting: on a door's free edge at hand height, as in ABI's photo of tall doors. Needs the frame rotated and the hinge side known | A Scalo on a tall door's free edge |
| SHARK-1 | S | Sharknose becomes a picked profile: legs and angle as catalog-style data with pictures (for example 30°, 45°, 60°), not constants | Picture tiles change the chamfer, and golden part lists follow |
| SHARK-2 | S | Finger room: raise or warn on the gaps a grip needs | A sharknose under a 2 mm top gap warns |
| SHARK-3 | S | Draw the true slope in the viewport (the FFI's mesh, not a step) | The viewport matches SolidCraft |
| SHARK-4 | S | Edge finish: a bevelled edge isn't banded; say on the cut list how it's finished | The cut list doesn't tape a bevelled edge |
| BEVEL-1 | M | Depends on question 3: a bevel on the face, or an applied bevelled profile | Set after the answer |

Later, after SEC-3: all of this as a per-section choice, as `opening` is now. A cabinet could have a
Scalo on its drawers and sharknose doors.

## Decisions for the owner

1. **Scalo's position.** Is the plate's back flush with the front's back, overhanging forward? It's
   my reading of the drawing. A photo of a real one or ABI's fitting note would settle it.
2. **Rappana's leg behind a top drawer or door.** The leg and screw heads need room in front of the
   rail. Recommended: rout a shallow rebate in the back of the front so the leg sits flush, cut as
   an ordinary groove and shown on the sheet. Alternatives: a gap behind the front, or a warning
   only.
3. **What "bevelled edge" means to you.** Either the board's own edge cut at an angle, which is
   sharknose with other angles and on the face side; or an applied aluminium bevel profile fixed to
   the edge, which would be catalogue hardware on the EDGE rows and needs a supplier. Recommended:
   do the board's own edge first.
4. **Finger room for sharknose.** How big a gap do you want above a sharknose front: say 20 to 30
   under a benchtop? And between stacked drawers, where 3 gives no grip: bigger gaps, or a sharknose
   on the top drawer only? No supplier gives this number, so it's your call.
5. **Top gap with an edge pull.** Raise it automatically to clear the plate, or leave it and warn?
   Recommended: warn, since the gap is yours to set.
6. **Wide fronts.** ABI's photos show one Scalo centred on each 600-odd drawer. Should wider fronts
   get two?
