# Blum CAD shopping list

The STEP models BLUM-9 needs, in the order to fetch them. Each is a part the app places today
(the article numbers come from `catalog/hardware/blum-*.json`). Download them from Blum
E-SERVICES / the Product Configurator (enter the web code), as **STEP, millimetres**, one file per
part, and save each as `vendor-cad/blum/<part-no>.step` with the part number exactly as printed
(dots kept, for example `750.5001S.step`). That folder stays on your Mac: it is never committed.

Where a configurator asks for options not listed here, take Blum's default, nickel (NI) finish
for hinges and silk white (SW-M) for LEGRABOX, and note your choice next to the row.

## 1. Hinges and plates (every door)

| Part no. | What | Web code | Configurator choices |
|---|---|---|---|
| 71B3550 | CLIP top BLUMOTION 110°, overlay, screw-on boss | DQDELA | Overlay application, screw-on, with spring |
| 71B3650 | CLIP top BLUMOTION 110°, dual (half overlay) | DQDELA | Dual application, screw-on |
| 71B3750 | CLIP top BLUMOTION 110°, inset | DQDELA | Inset application, screw-on |
| 70T3550.TL | CLIP top 110° overlay, unsprung (TIP-ON doors) | DQDELA | Overlay, screw-on, without spring |
| 175H9100 | Cruciform plate 37/32, 0 mm, system screws | DQDELA (plates p.153) | 0 mm spacing |
| 175H9130 | Cruciform plate 37/32, 3 mm | DQDELA | 3 mm spacing |
| 175H9160 | Cruciform plate 37/32, 6 mm | DQDELA | 6 mm spacing |
| 175H9190 | Cruciform plate 37/32, 9 mm | DQDELA | 9 mm spacing |

## 2. Drawers

| Part no. | What | Web code | Configurator choices |
|---|---|---|---|
| 750.5001S | LEGRABOX cabinet profile, NL 500, 40 kg | DQBN9A | NL 500, BLUMOTION S |
| 750.5501S | LEGRABOX cabinet profile, NL 550, 40 kg | DQBN9A | NL 550 |
| 770M5002S | LEGRABOX drawer side set M, NL 500 | DQBN9A | Height M (90.5), NL 500, SW-M |
| 770K5502S | LEGRABOX drawer side set K, NL 550 | DQBNYM | Height K (128.5), NL 550, SW-M |
| 770N5002S | LEGRABOX drawer side set N, NL 500 | DQBMJY | Height N (66.5), NL 500, SW-M |
| ZF7M70E2 | LEGRABOX front fixing bracket M, EXPANDO | DQBN9A | EXPANDO |
| 760H5500S | MOVENTO runner, NL 550, 40 kg | DQCZ3Y | NL 550, BLUMOTION S |
| 560H5500B | TANDEM 560H runner, NL 550, 30 kg | DQD1XA | NL 550 |

## 3. AVENTOS HF top (bi-fold lift)

| Part no. | What | Web code | Configurator choices |
|---|---|---|---|
| 22F2510 | Lift mechanism set, LF 2700–13500, system screws | DQDXLC | Standard (not SERVO-DRIVE) |
| 22F3500 | Lever arm set, KH 600–910 | DQDXLC | Cabinet height 720 |
| 70T5550.TL | CLIP top 120° hinge, unsprung | DQDXLC | Screw-on |
| 78Z5500T | CLIP top centre hinge | DQDXLC | Screw-on |
| 22.8000 | Cover cap set | DQDXLC | Silk white |

## 4. L-corner bi-fold doors

| Part no. | What | Web code | Configurator choices |
|---|---|---|---|
| 71B7550 | CLIP top BLUMOTION 155° hinge, overlay, screw-on | DQDHEM | Overlay (0-protrusion), screw-on |
| 79T8500 | Corner cabinet bi-fold hinge, screw-on | DQDQJY | Screw-on, with spring |
| 173L6100 | Cruciform plate 37/32, 0 mm, chipboard screws | DQDQJY (plates p.153) | 0 mm |

## After downloading

Tell the next run the files are there: BLUM-9 then converts each to a linked SolidCraft component
in place of the grey stand-ins. The drawer rows are the lengths the sample job uses (a 580 deep
base gets NL 550); other lengths can follow when a job needs them.
