# Checks against Blum E-SERVICES

The owner plans a cabinet in Blum's Product Configurator (E-SERVICES), and we build the same
cabinet in the core and compare: panel sizes, every bore, the hardware. Blum's files stay local
(they are Blum's output, like vendor-cad).

To build ours: write the cabinet's settings as JSON overrides and run
`cargo run -q -p cabinet-core --example part_list -- overrides.json`.

## Check 1: door base, CLIP top BLUMOTION (2026-10-27)

Blum: standard cabinet 600 × 850 × 580, 16 mm sides, top and base, a 16 mm lay-on back, one
shelf, one door hinged left. Top gap 2, bottom gap 0, side gaps 1.5, gap between fronts 3.
Blum chose CLIP top BLUMOTION 110° 71B358E (EXPANDO boss), plate 174H7100E (0 mm, EXPANDO),
cover caps 70.1503.BP.

Ours: base, `toe_kick: none`, `top_joint: over_sides`, `bottom_joint: under_sides`,
`back: applied` (16), 16 mm fronts, `front_gap: 3`, `fixings: dowel`.

| What | Blum | Ours | Verdict |
|---|---|---|---|
| Sides | 818 × 564 × 16 | same | match |
| Top, base | 600 × 564 × 16 | same | match |
| Back | 850 × 600 × 16 | same | match |
| Door width | 597 (1.5 each side) | 597 | match (`front_gap` 3 means 1.5 at each cabinet edge, as Blum) |
| Door height | 848 (top gap 2, bottom 0) | 847 (top gap = front gap 3) | **differs**: we have no separate top gap |
| Cup | Ø35, 13 deep, centre 21 from the edge (TB 3.5) | same | match: TB follows the overlay (14.5) through Blum's FA table |
| Hinges | 2 | 3 | **differs**: our rule is Blum's printed chart (p.75 / p.718: 2 hinges to 750 mm); the configurator gives 2 at 848 |
| Hinge from door ends | 80 | 100 | **differs** |
| Hinge boss | EXPANDO: two Ø8 × 13 holes, 45 apart, 9.5 behind the cup | screw-on 71B3550, no boss holes | **option missing**: we can't pick EXPANDO or INSERTA |
| Plate | 37 from the front, holes ±16 about the hinge height, Ø5 × 11.5 | same (175H9100, system screws) | match. 174H7100E drills the same holes |
| Cover caps | 70.1503.BP per hinge | none | **missing** |
| Shelf | 568 × 564 (full interior), underside 401 above the base, pins 2.5 below it | 566 × 560 (1 mm side, 2 mm front and back clearance), underside on the pin centre, snapped to 32 mm | **differs**: our shelf sits on the pins' centre line, 2.5 mm into the pin |
| Shelf pins | one hole per row at the shelf, 37 from front and back | 7 per row around the shelf, 37 from front and back | fine: more holes on purpose (WF-6) |
| Carcass joints | Ø8 dowels in threes on 32 mm (8 from the face; Ø8/12 face, Ø8/30 edge) | Ø8 dowels at 160 mm (8/13 face, 8/24 edge) | joinery is ours to choose, not Blum hardware |

### What changed (2026-10-27)

Rebuilt with `top_gap: 2`, `bottom_gap: 0`, `hinge_boss: expando`, our part list now matches
Blum's in everything above but the hinge count and the shelf's height:

1. **Top and bottom gaps** are settings (`top_gap`, default 2; `bottom_gap`, default 0), shown
   under Door mounting with the front gap.
2. **Hinge count** stays on the printed chart (p.75 / p.718), the safer reading, so this door
   keeps a middle hinge. The configurator's own limit is still to learn: check a 900 and a
   1000 mm high door.
3. **End hinges** sit 80 from the door's ends (`blum.clip-top.hinge-position`, from this
   report). Under an arched top, the top hinge keeps its 80 from where the arch meets its
   edge. Middle hinges put their plate screws on the 32 mm grid, in the shelf-pin rows' holes,
   as Blum's 37/32 system intends.
4. **Hinge fixing** is a choice: screw-on, EXPANDO (71B358E with 174H7100E/7130E plates,
   overlay only) or INSERTA (71B3590/3690/3790 with 174H7100I/7130I). EXPANDO and INSERTA drill
   the two Ø8 × 13 boss holes.
5. **Cover caps**: 70.1503.BP on overlay hinges, 70.1663.BP on dual and inset ones.
6. **Shelves** rest on their pins: the underside a pin's radius (2.5) above the holes. Ours
   still snaps to the 32 mm grid, so it sits at 434.5 where Blum's is at 417; Blum drills one
   hole per row at the shelf instead.

### Learnings to act on

1. **Top gap.** Add a top gap (and bottom gap) to the standard, apart from the gap between fronts.
   Blum defaults: top 2, bottom 0.
2. **Hinge count.** Keep the catalogue chart as the safe rule, but learn the configurator's
   thresholds (more checks: 900 and 1000 high doors) and say which one we follow.
3. **Hinge from door ends.** 80 mm, as the configurator places them, from catalog data with its
   source instead of the 100 mm constant.
4. **Boss fixing.** An option: screw-on, EXPANDO or INSERTA. It picks the hinge and plate
   articles, and EXPANDO and INSERTA drill the two Ø8 boss holes.
5. **Cover caps.** Add 70.1503.BP (one per hinge) to the hardware list.
6. **Shelf on its pins.** The shelf's underside sits a pin radius (2.5) above the hole centre.

## Check 2: drawer base, built section by section (2026-11-02)

Blum's session of 2026-10-27 (screen recording and output files, local only:
`vendor-cad/blum-sessions/2026-10-27-drawer-base.zip`). A 600 wide, 870 high, 580 deep base with
three LEGRABOX drawers, all on SERVO-DRIVE, and one fixed shelf over the bottom one. Ours rebuilt
as sections with the same settings.

How Blum's configurator works, and what SEC-3 took from it:

- The cabinet comes first, then **Fronts** splits it into A/1, A/2, A/3, each with a height and an
  open-compartment toggle. **Internal design** (fixed shelf, central panel, adjustable shelf, line
  drilling) is a separate step, so a split between fronts doesn't imply a shelf. Ours: `shelf_above`
  on each section.
- **Fittings** are chosen per section: lift, hinge or box system, each drawer with its own motion.
  Ours: `opening` and the drawers' `system` on each section.

| Item | Blum's session | Ours | Result |
|---|---|---|---|
| Fronts | 287.33 × 597 | same | match |
| Driven drawer | NL 500, base 490 × 533, back 530 wide | same | match: SERVO-DRIVE takes the shorter box |
| Front fixing | 28.5 from the front's edge, first hole 67 | same | match |
| Cross members | 568 × 80 | same | match |
| Bracket | one per compartment, clear height − 10 | was one for the whole cabinet | **fixed** |
| Bracket stock | Z10T670AA for both cuts | was the shortest that reaches (650) | **fixed**: shortest stock is catalog data |
| Parts per bracket | 1 attachment, 1 cable, 1 node | 2, 1 and 1 per cabinet | **fixed** |
| Drive units, bumpers | 3 units, 8 bumpers (2 + 4 + 2) | 3 units, per each front's height and runner | match by rule; ours gives 6 here because all three of ours are on the 40 kg profile |
| Bumper position | datum on the front's back face (582.5), shank in the front, head towards the carcass | was outside the front's outer face | **fixed** (2026-11-02): the owner saw them as stray screws |
| Upright's datum | 4 above its floor (20 over a 16 base, 591.17 over a shelf top at 587.17) | was 20 | **fixed**. Z10T670AA is drawn as Blum's model where it fits uncut, otherwise a bar at its cut length |
| Front to carcass | fronts' back 2.5 off the carcass (582.5, carcass front at 580) | 0 | **differs**: the bumper's 2.6 head needs that gap; all our fronts sit on the carcass |
| Power supply | not listed | 1 per cabinet | ours stays: a cabinet can't run without one, and Blum's session may leave it to the room |
| Fixed shelf height | 282.83 to 298.83, cuts 256.83 and 545.16 | 290.33 to 306.33, cuts 264.3 and 537.7 | **differs**: Blum's sits 7.5 lower; ours keeps every drawer's base the same height above its front |
| Cabinet profile holes | drilled on the sides: 37 from the front, the third hole at 54, 328.33 and 625.17 | 54, 328.33 and 38 over the shelf, 632.67 (the shelf sits 7.5 higher in ours) | **fixed** (SEC-4), LEGRABOX only. 38 above the datum |
| Datum of a drawer above another | its own front's bottom edge: front holes 51 and 147 up, against 67 and 163 on the lowest | was 16 higher, copying the lowest | **fixed** (SEC-4) |
| Box height, length | the user picked C (back height 148) and NL 500 on all drawers | auto takes the tallest and longest that fit; each section can pick its own | **fixed** (SEC-4) |
| Carcass | sides 870, base and cross members between them | a construction choice in `bottom_joint` | ours to choose |
| Glass design elements, AMBIA-LINE | listed | not modelled | the user's choices, not a fault |

### Still to confirm

- Attachments: Blum's catalogue calls Z10D01E0.01 "top/bottom", and this session lists one per
  bracket on two brackets. We follow the session. A session with one tall bracket would settle
  whether one is always enough.
- The 650's own ICH range (660 to 669) still picks the 650, per the catalogue. Blum's choice of 670
  for much shorter cuts is one observation. `verified` stays false.

