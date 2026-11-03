#!/usr/bin/env python3
"""Takes Laminex's decors from its downloaded Revit library, where they beat the website's.

    scripts/import_laminex_library.py "<folder holding Material Images>"

Files are named <Type>_<Use>_Laminex_<Range>_..._<DecorName>[_Normal].jpg. A library file replaces what
scripts/fetch_board_images.py found only where it's better:

- a whole sheet (twice as long as wide) with more pixels: its scale is true;
- any picture of a plain colour, largest first: a plain sheet looks the same everywhere;
- a tiling square where the website only had a swatch. Its size is an estimate from comparing grain.

A grained decor with a whole sheet from the website keeps it, however small: a library square has
different colour and figure, and no stated size. Files go to the local decor cache, not to git.
"""
import json
import os
import re
import shutil
import subprocess
import sys

CACHE = os.path.expanduser("~/Library/Application Support/CabinetStudio/decors")
OUT = "catalog/textures/laminex.json"
SQUARE_MM = [1200.0, 1200.0]


def key(text):
    return re.sub(r"[^a-z0-9]", "", text.lower())


def pixels(path):
    out = subprocess.run(
        ["/usr/bin/sips", "-g", "pixelWidth", "-g", "pixelHeight", path], capture_output=True, text=True
    ).stdout
    return [int(v) for v in re.findall(r"pixel(?:Width|Height): (\d+)", out)]


def whole_sheet(size):
    return len(size) == 2 and min(size) > 0 and max(size) / min(size) >= 1.5


def copy_jpeg(src, dst):
    if src.lower().endswith(".png"):
        subprocess.run(
            ["/usr/bin/sips", "-s", "format", "jpeg", "-s", "formatOptions", "95", src, "--out", dst],
            capture_output=True,
        )
    else:
        shutil.copyfile(src, dst)


def main():
    folder = sys.argv[1]
    pictures, normals = {}, {}
    for name in os.listdir(folder):
        if not re.search(r"\.(jpe?g|png)$", name, re.I) or not name.startswith("Laminate_"):
            continue
        stem = name.rsplit(".", 1)[0]
        if stem.endswith("_Normal"):
            normals[key(stem[: -len("_Normal")].rsplit("_", 1)[-1])] = os.path.join(folder, name)
            continue
        size = pixels(os.path.join(folder, name))
        k = key(stem.rsplit("_", 1)[-1])
        if len(size) == 2 and (k not in pictures or size[0] * size[1] > pictures[k][1][0] * pictures[k][1][1]):
            pictures[k] = (os.path.join(folder, name), size)
    boards = json.load(open("catalog/boards/laminex.json"))["boards"]
    grained = {}
    for b in boards:
        grained[b["decor"]] = grained.get(b["decor"], False) or b.get("grained", False)
    record = json.load(open(OUT))
    images = record["images"]
    taken = {"whole sheet": 0, "plain colour": 0, "swatch replaced": 0, "new decor": 0}
    kept, used = 0, set()
    for decor in sorted(grained):
        k = key(decor)
        if k not in pictures:
            continue
        path, size = pictures[k]
        now = images.get(decor)
        sheet_now = now and "covers" not in now
        area_now = now["pixels"][0] * now["pixels"][1] if now else 0
        if whole_sheet(size):
            better = not sheet_now or size[0] * size[1] > area_now
            why = "whole sheet"
        elif not grained[decor]:
            better, why = size[0] * size[1] > area_now, "plain colour"
        else:
            better, why = not sheet_now, "swatch replaced"
        if not better:
            kept += 1
            continue
        if not now:
            why = "new decor"
        name = now["name"] if now else "laminex-" + re.sub(r"[^a-z0-9]+", "-", decor.lower()).strip("-")
        copy_jpeg(path, os.path.join(CACHE, name + ".jpg"))
        # Its height map was made from the picture this replaces.
        stale = [f for f in os.listdir(CACHE) if re.fullmatch(re.escape(name) + r"-h\d+\.png", f)]
        for f in stale:
            os.remove(os.path.join(CACHE, f))
        entry = {"name": name, "url": None, "pixels": size}
        if not whole_sheet(size):
            entry["covers"] = SQUARE_MM
        if k in normals:
            copy_jpeg(normals[k], os.path.join(CACHE, name + "-normal.jpg"))
            entry["normal"] = name + "-normal"
        images[decor] = entry
        if decor in record["no_image"]:
            record["no_image"].remove(decor)
        taken[why] += 1
        used.add(k)
    record["source"] = "Laminex's Revit library, else https://www.laminex.com.au: the product page's Full Sheet Image or colour picture"
    with open(OUT, "w") as f:
        json.dump(record, f, indent=1, ensure_ascii=False)
        f.write("\n")
    print(taken, "kept the website's:", kept)
    print("normal maps:", sum(1 for v in images.values() if "normal" in v))
    print("still no image:", record["no_image"])
    print("library decors not in the catalog:", sorted(set(pictures) - {key(d) for d in grained})[:60])


main()
