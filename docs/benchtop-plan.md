# Benchtop plan (on hold)

Written 2026-11-02 at the owner's request: planning only, no code. Benchtops wait until cabinets
work well (BENCH-HOLD). This comes from the backlog and `docs/benchtops.md`, not from testing.

## Where it stands

More exists than the hold suggests, but most has never been looked at.

| Area | State | Gap |
|---|---|---|
| Top over each stretch of base cabinets (BENCH-1) | built, not seen on screen | Stone and compact not offered as kinds; material picker limited to the board slot |
| Edge profiles (BENCH-2) | built | 3D view draws every edge square |
| Waterfall ends, breakfast bar, rounded ends (BENCH-3a, c, e) | built | Support spacing is our rule of thumb |
| L-corner (BENCH-3b) | butt joint only | No mitre, no choice of joint position, no U shapes |
| Cut-outs, appliance library (BENCH-4, BENCH-9) | done | Sizes marked VERIFY; no power pop-ups |
| Template sheets, quoting figures (BENCH-5) | done | Edge profiles on the sheet to confirm |
| Splashbacks, niches, power points (BENCH-6, BENCH-8) | done | Unverified supplier data |
| Material rules (BENCH-7) | done | No compact laminate or timber rows, no sources yet; all `verified: false` |

## What must come first

1. **ROOM-2, walls that stay put.** A top is cut to the walls, not to the cabinets. It needs the
   real wall positions before it can be right.
2. **LAYOUT-1, wall cabinets in position.** Splashback height, niches and rangehoods depend on
   what hangs above the top.
3. **Cabinet design styles working.** Sharknose, frames and seen end boards change where the front
   face is, and the front overhang is measured from it.

## Proposed order once the hold lifts

**Stage 1, check what's there (S).** Build a test kitchen, look at every BENCH feature on screen and
in SolidCraft, and turn the "not seen" rows into a list of real faults. This comes first because
it may change everything below.

**Stage 2, tops that fit the room (M).**
- Cut to configured walls, with a scribe allowance at the back and at return walls.
- Check the gap between a top's end and an open end, seen end board or end panel. The top already
  measures from all of a stretch's parts, so a seen end board should be included: confirm it.
- Warn when a top is too long for one slab, and place joins so they avoid cut-outs.

**Stage 3, materials (M).** Stone, Zenith, polytec and Laminex rows from real supplier documents, each
with its slab sizes, allowed finishes and support rules, and a material picker as pictures.

**Stage 4, edges (L, split first).** All the front and end cuts, the shadow line with its edgeband, and
mitred build-ups as real parts. Drawn in the 3D view and the elevation, and on the elevation drawing.

**Stage 4b, appliances and power (M).** ABI sinks, Fisher & Paykel and Smeg cooktops, pop-ups, and power
points in waterfall edges.

**Stage 5, shapes (L, split first).** Mitred L-corners, a choice of joint position, and U shapes.
L and U depend on the room planner, which is after 1.0, so this probably waits too.

## The owner's answers (2026-11-02)

1. **Materials:** stone, Zenith compact porcelain, and polytec and Laminex benchtops.
2. **The owner cuts the tops.** So cut files and DXF must be workshop-accurate, and every piece of a
   build-up has to be its own part on the cut list and nesting. Quote figures are secondary.
3. **Edges:** all the major front and end cuts. Shadow line settings for how the top meets the
   cabinet, with the visible shadow line face either in a contrasting custom edgeband or the same
   band as the fronts. Each material allows different finishes, so options come from its catalog
   row. Mitred edges that look 20, 40, 60, 80 or 100 mm thick (only to the eye: the top stays its real
   thickness). Waterfall edges.
4. **Support rules:** right, but they depend on the material, so they live on its catalog row.
5. **Appliances:** ABI Interiors sinks for now, more from the owner after 1.0. Cooktops from the
   Fisher & Paykel and Smeg BIM libraries, to be fetched.
6. **Power:** pop-ups through the top, and an option for power points in waterfall edges.

New work is in the backlog: BENCH-10 to BENCH-16 and SPLASH-1.

## Not planned

Engineered stone: banned in Australia since 1 July 2024.
