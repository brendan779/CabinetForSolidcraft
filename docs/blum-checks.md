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
