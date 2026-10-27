#!/usr/bin/env python3
"""Builds the generic handle models (WF-7) as STEP files with SolidCraft's command-line tool.

One family in one style, so a job's handles match whatever the size: a 12 mm round bar on two
10 mm legs standing it 32 mm off the front, running 20 mm past each hole, for each hole-centre
size the app offers; and a 30 mm knob on a 12 mm post. These are our own shapes, not a
supplier's: catalog/hardware/generic-handles.json lists them.

The models are drawn in the frame the app places them in: the screw holes on the X axis at
+/- centres / 2, the front's face at Y = 0, the handle standing out towards -Y.

    scripts/make-generic-handles.py [vendor-cad/generic/handles]
"""

import json
import subprocess
import sys
import tempfile
from pathlib import Path

CLI = "/Applications/SolidCraft.app/Contents/MacOS/solidcraft-cli"
CENTRES = [96, 128, 160, 192, 224, 256, 320, 448]
BAR, LEG, STANDOFF, END = 12.0, 10.0, 32.0, 20.0
KNOB, KNOB_POST, KNOB_STEM, KNOB_CAP = 30.0, 12.0, 20.0, 12.0


def cylinder(name, base, axis, diameter, height, join=None):
    params = {"base": base, "axis": axis, "diameter": diameter, "height": height}
    if join:
        params.update({"operation": "join", "targets": [join]})
    else:
        params.update({"operation": "new", "body_name": name, "name": name})
    return {"command": "solid.cylinder", "params": params}


def bar(cc):
    name = f"GEN-H-CC{cc}"
    half = cc / 2 + END
    return name, [
        cylinder(name, [-half, -STANDOFF, 0], "X", BAR, 2 * half),
        cylinder(name, [-cc / 2, 0, 0], [0, -1, 0], LEG, STANDOFF, join=name),
        cylinder(name, [cc / 2, 0, 0], [0, -1, 0], LEG, STANDOFF, join=name),
    ]


def knob():
    name = "GEN-H-KNOB"
    return name, [
        cylinder(name, [0, 0, 0], [0, -1, 0], KNOB_POST, KNOB_STEM),
        cylinder(name, [0, -KNOB_STEM, 0], [0, -1, 0], KNOB, KNOB_CAP, join=name),
        # Its outer rim rounded.
        {"command": "solid.fillet",
         "params": {"body": name, "edges": [[KNOB / 2, -KNOB_STEM - KNOB_CAP, 0]], "radius": 4}},
    ]


def main():
    out = Path(sys.argv[1] if len(sys.argv) > 1 else "vendor-cad/generic/handles")
    out.mkdir(parents=True, exist_ok=True)
    models = [bar(cc) for cc in CENTRES] + [knob()]
    with tempfile.TemporaryDirectory() as tmp:
        for name, commands in models:
            script = Path(tmp) / f"{name}.json"
            script.write_text(json.dumps({"commands": commands}))
            step = out / f"{name}.step"
            subprocess.run([CLI, "run", str(script), "--out", str(step), "--quiet"], check=True)
            print(step)


if __name__ == "__main__":
    main()
