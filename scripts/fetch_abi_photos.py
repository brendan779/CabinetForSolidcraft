#!/usr/bin/env python3
"""Downloads ABI Interiors' product photo and finish swatch for each handle finish.

Reads the finish page URLs from catalog/hardware/abi-handles.json, one page at a time, and
saves to vendor-cad/abi-web (local only, never committed: the images are ABI's):

    vendor-cad/abi-web/<catalog row id>.jpg    the page's product photo
    vendor-cad/abi-web/swatches/<finish>.png   the round finish swatch on the page
    vendor-cad/abi-web/swatches.json           finish name -> swatch URL

Files already there are skipped, so it can be run again after a failure.

    scripts/fetch_abi_photos.py [--limit N]
"""
import html
import json
import os
import re
import sys
import subprocess
import time

CATALOG = 'catalog/hardware/abi-handles.json'
OUT = 'vendor-cad/abi-web'
PAUSE = 1.5
PHOTO_WIDTH = 640
AGENT = 'Mozilla/5.0 (Macintosh; Mac OS X) CabinetStudio-catalog/1.0'


def get(url):
    # curl, for the system's certificates: python.org builds of Python carry none.
    return subprocess.run(['curl', '-sSfL', '--max-time', '40', '-A', AGENT, url],
                          check=True, capture_output=True).stdout


def slug(s):
    return re.sub(r'[^a-z0-9]+', '-', s.lower()).strip('-')


def local_photo(item, out):
    report = json.load(open('vendor-cad/abi/import-report.json'))
    row = next((r for r in report if r['article'] == item['article']), None)
    if not row:
        return False
    folder = f"vendor-cad/abi-bim/BIMobject-{row['folder']}"
    shots = sorted(f for f in os.listdir(folder)
                   if f.lower().endswith(('.png', '.jpg')) and 'spec' not in f.lower())
    # Shots of the product alone come as PNGs on white; rooms are JPEGs.
    shots.sort(key=lambda f: not f.lower().endswith('.png'))
    if not shots:
        return False
    subprocess.run(['sips', '-s', 'format', 'jpeg', '-Z', str(PHOTO_WIDTH), f'{folder}/{shots[0]}',
                    '--out', out], check=True, capture_output=True)
    return True


def main():
    limit = int(sys.argv[sys.argv.index('--limit') + 1]) if '--limit' in sys.argv else None
    os.makedirs(OUT + '/swatches', exist_ok=True)
    items = json.load(open(CATALOG))['items']
    swatch_file = OUT + '/swatches.json'
    swatches = json.load(open(swatch_file)) if os.path.exists(swatch_file) else {}
    done = failed = 0
    for item in items:
        if limit is not None and done >= limit:
            break
        out = f"{OUT}/{item['id']}.jpg"
        if os.path.exists(out):
            continue
        finish = item['variant'].replace(' (colour to verify)', '')
        m = re.search(r'finish [^:]+: (https://\S+?),', item['source']['url'])
        if not m:
            # The Hegman timber pulls have one finish and no finish page: ABI's image from the
            # product's BIM download instead.
            if not local_photo(item, out):
                print('no page', item['id'])
            continue
        try:
            page = get(m.group(1)).decode('utf-8', 'replace')
            og = re.search(r'<meta property="og:image" content="([^"]+)"', page)
            if not og:
                raise ValueError('no og:image')
            photo = html.unescape(og.group(1))
            photo += ('&' if '?' in photo else '?') + f'width={PHOTO_WIDTH}'
            open(out, 'wb').write(get(photo))
            # The page lists every finish as a link around its swatch image.
            for link, src in re.findall(r"href='(https://[^']+)'><img[^>]*src='(https://[^']+)'", page):
                name = link.rstrip('/').rsplit('/', 1)[-1]
                swatches.setdefault(name, src.split('?')[0])
            own = m.group(1).rstrip('/').rsplit('/', 1)[-1]
            if own in swatches:
                path = f"{OUT}/swatches/{slug(finish)}.png"
                if not os.path.exists(path):
                    open(path, 'wb').write(get(swatches[own]))
            done += 1
            print('ok', item['id'], flush=True)
        except Exception as e:  # keep going: a re-run picks up what failed
            failed += 1
            print('FAILED', item['id'], e, flush=True)
        time.sleep(PAUSE)
    json.dump(swatches, open(swatch_file, 'w'), indent=1)
    print(f'{done} fetched, {failed} failed')


if __name__ == '__main__':
    main()
