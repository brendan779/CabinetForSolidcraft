#!/usr/bin/env python3
"""Downloads Polytec's full sheet decor images and records which decor has which.

Reads the decors from catalog/boards/polytec.json, tries each at Polytec's full sheet address one
at a time, and writes:

    catalog/textures/polytec.json                        decor -> image name
    ~/Library/Application Support/CabinetStudio/polytec  <name>.jpg (local only: the images are Polytec's)

Files already there are skipped, so it can be run again after a failure.

    scripts/fetch_polytec_images.py [--limit N]
"""
import json
import os
import re
import subprocess
import sys
import time
import unicodedata

URL = "https://www.polytec.com.au/img/products/fullsheet/{name}.jpg"
CACHE = os.path.expanduser("~/Library/Application Support/CabinetStudio/polytec")
OUT = "catalog/textures/polytec.json"
AGENT = "CabinetStudio/1.0 (decor images for a cabinet tool)"


def slug(decor):
    plain = unicodedata.normalize("NFKD", decor).encode("ascii", "ignore").decode()
    return re.sub(r"[^a-z0-9]+", "-", plain.lower()).strip("-")


def fetch(name):
    path = os.path.join(CACHE, name + ".jpg")
    if os.path.exists(path):
        return True
    # curl for the system's certificates: python.org's Python has none of its own.
    done = subprocess.run(
        ["/usr/bin/curl", "-sSfL", "--max-time", "60", "-A", AGENT, "-o", path + ".part", URL.format(name=name)],
        capture_output=True,
    )
    if done.returncode != 0:
        return False
    with open(path + ".part", "rb") as f:
        if f.read(2) != b"\xff\xd8":
            os.remove(path + ".part")
            return False
    os.replace(path + ".part", path)
    time.sleep(0.5)
    return True


def main():
    limit = int(sys.argv[sys.argv.index("--limit") + 1]) if "--limit" in sys.argv else None
    os.makedirs(CACHE, exist_ok=True)
    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    boards = json.load(open("catalog/boards/polytec.json"))["boards"]
    decors = sorted({b["decor"] for b in boards})[:limit]
    found, missing = {}, []
    for decor in decors:
        name = slug(decor)
        if fetch(name):
            found[decor] = name
        else:
            missing.append(decor)
        print(f"{len(found) + len(missing)}/{len(decors)} {decor}: {'ok' if decor in found else 'NOT FOUND'}", flush=True)
    old = json.load(open(OUT)) if os.path.exists(OUT) else {}
    old.update({
        "source": "https://www.polytec.com.au (each colour's page links its full sheet image)",
        "verified": False,
        "image_url": URL,
        "images": found,
        "no_image": missing,
    })
    with open(OUT, "w") as f:
        json.dump(old, f, indent=1, ensure_ascii=False)
        f.write("\n")
    print(f"{len(found)} found, {len(missing)} not found: {missing}")


main()
