#!/usr/bin/env python3
"""Takes the articles our catalog lists out of a Blum E-SERVICES "CAD/CAM package" zip.

    scripts/import_blum_package.py ~/Downloads/Individual-product-*_STEP.zip

A package holds every article of a programme line, over a thousand files, so only exact matches
go to vendor-cad/blum/<article>__<id>.step. One article's files can include its sub-parts, so only
those near the largest's size are taken: the whole part, in each hand. An article already in the
folder is left alone. The zip goes to vendor-cad/blum/catalogue/packages, which is not committed.
"""
import glob
import json
import os
import shutil
import sys
import zipfile

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, "vendor-cad", "blum")


def catalog_articles():
    found = set()
    for path in glob.glob(os.path.join(ROOT, "catalog", "hardware", "**", "*.json"), recursive=True):
        with open(path, encoding="utf-8") as f:
            for item in json.load(f).get("items", []):
                if item.get("brand") == "Blum" and " " not in str(item.get("article", " ")):
                    found.add(item["article"])
    return found


def main():
    wanted = catalog_articles()
    have = {
        os.path.basename(f).split("__")[0].removesuffix(".step").removesuffix("_L").removesuffix("_R")
        for f in glob.glob(os.path.join(OUT, "*.step"))
    }
    for zip_path in sys.argv[1:]:
        by_article = {}
        with zipfile.ZipFile(zip_path) as z:
            for info in z.infolist():
                stem, ext = os.path.splitext(os.path.basename(info.filename).strip())
                article, _, file_id = stem.partition("_")
                if ext.lower() in (".step", ".stp") and article in wanted and article not in have:
                    by_article.setdefault(article, []).append((info, file_id))
            count = 0
            for files in by_article.values():
                largest = max(i.file_size for i, _ in files)
                for info, file_id in files:
                    if info.file_size >= 0.9 * largest:
                        article = os.path.basename(info.filename).strip().partition("_")[0]
                        target = os.path.join(OUT, f"{article}__{file_id}.step")
                        with z.open(info) as src, open(target, "wb") as dst:
                            shutil.copyfileobj(src, dst)
                        count += 1
        have.update(by_article)
        print(f"{os.path.basename(zip_path)}: {len(by_article)} articles, {count} files")
        keep = os.path.join(OUT, "catalogue", "packages")
        os.makedirs(keep, exist_ok=True)
        if os.path.dirname(os.path.abspath(zip_path)) != keep:
            shutil.move(zip_path, os.path.join(keep, os.path.basename(zip_path)))


if __name__ == "__main__":
    main()
