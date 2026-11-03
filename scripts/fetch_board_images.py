#!/usr/bin/env python3
"""Downloads each board brand's largest published decor images, and records which decor has which.

Goes through the decors in catalog/boards, one request a second, asking only for pages each brand's
robots.txt allows, and writes:

    catalog/textures/<brand>.json                       decor -> image name, address, what it covers
    ~/Library/Application Support/CabinetStudio/decors  <name>.jpg (local only: the images are the
                                                        brands' own, and are not ours to publish)

`pixels` is what the file really holds: names can't be trusted ("-3600x1800" may be 3000 x 1500, and
"-1200x1200" 1200 x 2400). `covers` is the image's size on the board in mm, when its shape says it isn't
a whole sheet. Files already there are skipped, so it can be run again after a failure.

    scripts/fetch_board_images.py [polytec laminex formica trade-essentials nikpol egger]
"""
import html
import json
import os
import re
import subprocess
import sys
import time
import unicodedata
import urllib.parse

CACHE = os.path.expanduser("~/Library/Application Support/CabinetStudio/decors")
AGENT = "CabinetStudio/1.0 (decor images for a cabinet tool)"
# A square picture is part of a board, at about this size. An estimate.
SWATCH_MM = [1300.0, 1300.0]


def slug(text):
    plain = unicodedata.normalize("NFKD", text).encode("ascii", "ignore").decode()
    return re.sub(r"[^a-z0-9]+", "-", plain.lower()).strip("-")


def curl(url, *args):
    time.sleep(1)
    return subprocess.run(
        ["/usr/bin/curl", "-sSfL", "--max-time", "90", "-A", AGENT, *args, url], capture_output=True
    )


def page(url):
    done = curl(url)
    return done.stdout.decode("utf-8", "ignore") if done.returncode == 0 else ""


def size_of(url):
    done = curl(url, "-I")
    found = re.findall(r"(?im)^content-length:\s*(\d+)", done.stdout.decode("latin1", "ignore"))
    return int(found[-1]) if found else 0


def fetch(url, name):
    path = os.path.join(CACHE, name + ".jpg")
    if os.path.exists(path):
        return True
    if curl(url, "-o", path + ".part").returncode != 0:
        return False
    with open(path + ".part", "rb") as f:
        head = f.read(4)
    if head == b"\x89PNG":
        # Some brands' pictures are PNGs. One format in the cache, at a quality that hides nothing.
        subprocess.run(
            ["/usr/bin/sips", "-s", "format", "jpeg", "-s", "formatOptions", "95", path + ".part", "--out", path + ".part"],
            capture_output=True,
        )
    elif head[:2] != b"\xff\xd8":
        os.remove(path + ".part")
        return False
    os.replace(path + ".part", path)
    return True


def pixels(name):
    done = subprocess.run(
        ["/usr/bin/sips", "-g", "pixelWidth", "-g", "pixelHeight", os.path.join(CACHE, name + ".jpg")],
        capture_output=True, text=True,
    )
    return [int(v) for v in re.findall(r"pixel(?:Width|Height): (\d+)", done.stdout)]


def decors(brand):
    return sorted({
        b["decor"]
        for b in json.load(open(f"catalog/boards/{brand}.json"))["boards"]
        if b["kind"] != "raw"
    })


def polytec(names):
    template = "https://www.polytec.com.au/img/products/fullsheet/{}.jpg"
    for decor in names:
        name = slug(decor)
        url = template.format(name)
        if os.path.exists(os.path.join(CACHE, name + ".jpg")):
            yield decor, name, url, None
            continue
        # The colour's own page says where its sheet is, where the name alone doesn't.
        found = re.search(r"/img/products/fullsheet/[^'\"]+\.jpg", page(f"https://www.polytec.com.au/colour/{name}/"))
        yield decor, name, ("https://www.polytec.com.au" + found.group(0)) if found else url, None


def laminex_group(brand, names):
    index = page("https://www.laminex.com.au/sitemap.xml")
    sitemap = re.search(r"<loc>([^<]*ProductAU[^<]*)", index)
    urls = re.findall(r"<loc>([^<]*/products/[^<]*)", page(html.unescape(sitemap.group(1))))
    by_slug = {}
    for u in urls:
        by_slug.setdefault(u.split("/products/")[1].split("/")[0], []).append(u)
    for decor in names:
        # Every picture its pages offer, measured: neither names nor bytes say which is largest (a
        # plain colour's sheet can be a tenth of the bytes of its swatch). A whole sheet beats a swatch.
        best, score, tried = None, (False, 0, 0), set()
        for url in by_slug.get(slug(decor), [])[:4]:
            text = page(url)
            title = re.search(r"<title>([^<]*)", text)
            title = title.group(1) if title else ""
            # A decor name can be in both ranges: Formica's pages say so.
            if ("Formica" in title) != (brand == "formica"):
                continue
            for link in {
                "https://www.laminex.com.au" + html.unescape(l)
                for l in re.findall(r"/medias/[^\"' ]*\.jpg\?context=[^\"' ]*attachment=true", text)
            }:
                file = link.split("/medias/")[1].split("?")[0]
                # Not its thumbnails, its low resolution copies, or other decors' pictures.
                if re.match(r"\d+Wx\d+H-|p-", file) or file.endswith("-LR.jpg") or file in tried:
                    continue
                tried.add(file)
                probe = os.path.join(CACHE, f".probe-{brand}.jpg")
                if curl(link, "-o", probe).returncode != 0:
                    continue
                size = pixels(f".probe-{brand}")
                if len(size) == 2:
                    candidate = (max(size) / min(size) >= 1.5, size[0] * size[1], max(size))
                    if candidate > score:
                        best, score = link, candidate
            # Laminex's best density, about 0.83 px/mm whatever the sheet's shape: stop looking.
            if score[0] and score[2] >= 3000:
                break
        if best:
            yield decor, f"{brand}-{slug(decor)}", best, None
        else:
            yield decor, None, None, None


def nikpol(names):
    listing = page("https://www.nikpol.com.au/colourPage/index")
    colours = {html.unescape(c).strip() for c in re.findall(r"colorName=([^\"'&<]+)", listing)}
    for decor in names:
        # The catalog's name ends in a code: "Acero 9116".
        plain = re.sub(r"\s+[A-Z]?\d+[A-Z]?$", "", decor)
        # Either of the site's names may be ours: "Dusk Blue (Spring Green)".
        def names_of(colour):
            return [slug(n) for n in re.split(r"[()]", colour) if n.strip()]

        match = next((c for c in colours if {slug(decor), slug(plain)} & set(names_of(c))), None)
        best, most = None, 0
        if match:
            text = page("https://www.nikpol.com.au/ColourPage/ColourPageDetails?colorName=" + urllib.parse.quote(match))
            # The names say nothing, and a colour may have several: measure them.
            for link in sorted(set(re.findall(r"/Uploads/FinishImage/SheetDesign/[^\"']+", text))):
                url = "https://www.nikpol.com.au" + urllib.parse.quote(link)
                probe = os.path.join(CACHE, ".probe-nikpol.jpg")
                if curl(url, "-o", probe).returncode != 0:
                    continue
                size = pixels(".probe-nikpol")
                if len(size) == 2 and size[0] * size[1] > most:
                    best, most = url, size[0] * size[1]
        if best:
            yield decor, f"nikpol-{slug(decor)}", best, None
        else:
            yield decor, None, None, None


def egger(names):
    boards = json.load(open("catalog/boards/egger.json"))["boards"]
    for decor in names:
        finish = next(b["finish"] for b in boards if b["decor"] == decor)
        text = page(f"https://forest.one/{slug(decor + ' ' + finish)}")
        # The page's main picture: other decors' are in the /cache/ of its "More Decors".
        found = re.search(r"https://forest\.one/media/catalog/product/\w/\w/[^\"' ]+\.jpg", text)
        if found:
            yield decor, f"egger-{slug(decor)}", found.group(0), None
        else:
            yield decor, None, None, None


SOURCES = {
    "polytec": ("https://www.polytec.com.au: each colour's full sheet image", polytec),
    "laminex": ("https://www.laminex.com.au: the product page's Full Sheet Image or colour picture", lambda n: laminex_group("laminex", n)),
    "formica": ("https://www.laminex.com.au: the product page's Full Sheet Image or colour picture", lambda n: laminex_group("formica", n)),
    "trade-essentials": ("https://www.laminex.com.au: the product page's Full Sheet Image or colour picture", lambda n: laminex_group("trade-essentials", n)),
    "nikpol": ("https://www.nikpol.com.au: each colour's Sheet Design image", nikpol),
    "egger": ("https://forest.one: EGGER's Australian distributor's picture of each decor", egger),
}


def run(brand):
    source, find = SOURCES[brand]
    names = decors(brand)
    images, missing = {}, []
    for decor, name, url, _ in find(names):
        if name and fetch(url, name):
            size = pixels(name)
            images[decor] = {"name": name, "url": url, "pixels": size}
            # A sheet is about twice as long as it's wide. Square is a swatch.
            if len(size) == 2 and max(size) / min(size) < 1.5:
                images[decor]["covers"] = SWATCH_MM
        else:
            missing.append(decor)
        print(f"{brand} {len(images) + len(missing)}/{len(names)} {decor}: {'ok' if decor in images else 'NOT FOUND'}", flush=True)
    with open(f"catalog/textures/{brand}.json", "w") as f:
        json.dump({"source": source, "verified": False, "images": images, "no_image": missing}, f, indent=1, ensure_ascii=False)
        f.write("\n")
    print(f"{brand}: {len(images)} found, {len(missing)} not found: {missing}", flush=True)


os.makedirs(CACHE, exist_ok=True)
os.makedirs("catalog/textures", exist_ok=True)
for brand in sys.argv[1:] or SOURCES:
    run(brand)
