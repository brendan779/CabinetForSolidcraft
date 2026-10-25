# Our constants against Blum's general information

Checked on 2026-10-26 against the Blum *Catalogue and technical manual 2024/2025*, General
Information and planning pages (printed pp. 710–731, with the hinge and plate pages they point
to). Supplier numbers now live in `catalog/hardware/blum-*.json`; what is left in code is ours.

## Agrees with Blum

| Constant (file) | Value | Blum |
|---|---|---|
| `SYSTEM32_SETBACK` (build.rs) | 37 mm | Cruciform plates are 37/32: hole row 37 from the front edge (pp. 152–153) |
| `SYSTEM32_PITCH` (build.rs) | 32 mm | The plates' hole pitch, 32 (pp. 152–153) |
| Plate hole Ø (catalog) | 5 mm | "5 ±0.1" on every system-screw / EXPANDO plate (pp. 152–153) |

## Was wrong, now from Blum's data

| Rule | Was | Now |
|---|---|---|
| Hinges per door (build.rs) | 2 up to 900 mm, 3 to 1600, 4 to 2000, then 5, by height only | Blum's chart (p.718): 2 to about 750 mm / 6 kg, 3 to 1500 / 12 kg, 4 to 2100 / 17 kg, 5 to 2500 / 22 kg, by height **and** weight (`blum.clip-top.hinge-count`) |
| Hinge cup depth | 13 | 13 (110°, "min 13"), 11.7 (155°), 11 (bi-fold): per hinge in the catalog |
| Plate pilot depth | 13 | 11.5, Blum's printed minimum for system screws and EXPANDO |
| Boring distance | 4 for every door | From the overlay, cranking and plate spacing (`hinges::select`) |

## Ours, not Blum's (no Blum figure applies)

| Constant | Value | Why ours |
|---|---|---|
| `HINGE_END_OFFSET` | 100 mm | Where the end hinges sit; Blum only asks for the hinges to be as far apart as possible (p.718) |
| `SHELF_PIN_DIAMETER` / `_DEPTH` | 5 / 10 | Shelf supports are Häfele (HAF-3) |
| `TALL_DOOR` | 1200 mm | Two TIP-ON units above this height; Blum's short TIP-ON unit stops at about 1300 mm high fronts (p.192), which picks the unit, not the count |
| `DEEP_WOOD_DRAWER` | 150 mm | Which TIP-ON BLUMOTION load range a wooden drawer gets; Blum ranges by pull-out weight (p.504), which we do not know |
| LEGRABOX K → 15–40 kg TIP-ON unit | | Same reason: the load is a guess from the box height |
| `BACK_INSET`, `BACK_GROOVE_DEPTH` | 16 / 8 | Our carcass construction |

## To read later

- The LEGRABOX cabinet profile and MOVENTO/TANDEM runner screw positions (pp. 248, 428, 460):
  BLUM-4b, BLUM-6b.
- The AVENTOS HF top mechanism's side drilling (p.24) is a perspective drawing.
