#!/usr/bin/env python3
"""Takes EGGER's decors and normal maps from a downloaded Revit material folder (ForestOne's, from
bimstore), where the web's pictures have no relief and only a guessed scale.

    scripts/import_egger_materials.py <folder holding Materials/>

Files are named ForestOne_Egger_<Range>_<Name>_<CODE>_<FINISH>[_Normal].jpg. Each is matched to the
catalog by code and finish, and copied to the local decor cache: they are not committed. Decors it has
no file for keep what scripts/fetch_board_images.py found.
"""
import json
import os
import re
import shutil
import subprocess
import sys

CACHE = os.path.expanduser("~/Library/Application Support/CabinetStudio/decors")
OUT = "catalog/textures/egger.json"
# EGGER's page labels its 16:9 decor pictures "approx. 2.311 x 1.300 mm". Square ones state no size.
WIDE_MM = [2311.0, 1300.0]
SQUARE_MM = [1300.0, 1300.0]
NAME = re.compile(r"ForestOne_Egger_[A-Za-z]+_[^_]+_([A-Z]\d{3,4})_([A-Z0-9]+?)(_Normal)?\.jpe?g$", re.I)


def slug(text):
    return re.sub(r"[^a-z0-9]+", "-", text.lower()).strip("-")


def pixels(path):
    out = subprocess.run(
        ["/usr/bin/sips", "-g", "pixelWidth", "-g", "pixelHeight", path], capture_output=True, text=True
    ).stdout
    return [int(v) for v in re.findall(r"pixel(?:Width|Height): (\d+)", out)]


def main():
    files = {}
    for root, _, names in os.walk(os.path.join(sys.argv[1], "Materials")):
        for name in names:
            m = NAME.match(name)
            if m:
                key = (m.group(1).upper(), m.group(2).upper())
                files.setdefault(key, {})["normal" if m.group(3) else "colour"] = os.path.join(root, name)
    boards = json.load(open("catalog/boards/egger.json"))["boards"]
    record = json.load(open(OUT))
    used = set()
    for decor in sorted({b["decor"] for b in boards}):
        code = re.search(r"\b([A-Z]\d{3,4})\b", decor)
        if not code:
            continue
        finishes = [b["finish"].upper() for b in boards if b["decor"] == decor]
        # One image to a decor: its own finish, the one with relief first.
        found = [files[(code.group(1), f)] for f in finishes if (code.group(1), f) in files]
        found.sort(key=lambda f: "normal" not in f)
        found += [f for k, f in files.items() if k[0] == code.group(1) and f not in found]
        if not found or "colour" not in found[0]:
            continue
        chosen = found[0]
        used.add(chosen["colour"])
        name = "egger-" + slug(decor)
        shutil.copyfile(chosen["colour"], os.path.join(CACHE, name + ".jpg"))
        size = pixels(chosen["colour"])
        entry = {
            "name": name,
            "url": None,
            "pixels": size,
            "covers": WIDE_MM if max(size) / min(size) > 1.5 else SQUARE_MM,
        }
        if "normal" in chosen:
            shutil.copyfile(chosen["normal"], os.path.join(CACHE, name + "-normal.jpg"))
            entry["normal"] = name + "-normal"
        record["images"][decor] = entry
        if decor in record["no_image"]:
            record["no_image"].remove(decor)
    record["source"] = "ForestOne's EGGER Revit materials (bimstore), else https://forest.one's picture of each decor"
    with open(OUT, "w") as f:
        json.dump(record, f, indent=1, ensure_ascii=False)
        f.write("\n")
    images = record["images"]
    print(f'{len(used)} files used of {len(files)}; {sum(1 for v in images.values() if "normal" in v)} decors with normal maps')
    print(f'{len(images)} decors with images, no image: {record["no_image"]}')
    print("not matched:", sorted(k for k, f in files.items() if f.get("colour") not in used and "colour" in f))


main()
