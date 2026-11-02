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

**Stage 3, materials (M).** Add compact laminate and timber rows from real supplier documents, a
proper material picker as pictures, and slab sizes per material. Needs the answers below.

**Stage 4, how it looks (M).** Edge profiles drawn in the 3D view and the elevation. Benchtops drawn
on the front elevation.

**Stage 5, shapes (L, split first).** Mitred L-corners, a choice of joint position, and U shapes.
L and U depend on the room planner, which is after 1.0, so this probably waits too.

## Questions for the owner

1. **What do you specify most?** Stone, porcelain, laminate or timber decides what gets built first.
2. **Who makes the tops?** If a stonemason templates on site, the app needs quote figures and a
   layout. If you cut them yourself, it needs accurate cut files and DXF.
3. **Which default overhangs do you use?** Front, ends, and how far back from the wall. Today's
   figures are published conventions, not yours.
4. **Are the support rules right?** 300 mm unsupported and 600 mm bracket spacing are our rules of
   thumb for breakfast bars.
5. **Which sink and cooktop do you fit most?** Real templates would replace the VERIFY sizes.
6. **Are power pop-ups needed before 1.0?**

## Not planned

Engineered stone: banned in Australia since 1 July 2024.
