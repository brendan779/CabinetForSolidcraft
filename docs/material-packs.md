# Material packs

A material pack carries one manufacturer's materials to Cabinet Studio: the photographs of its
decors and slabs, the catalog records that describe them, and how each picture maps onto a sheet.
The app installs the official packs from GitHub on first run; the same format is what a user's own
pack will be, made for a brand that isn't in the repository and shared as a file.

## The file

A pack is a zip named `<id>-<version>.zip`, holding:

```
pack.json                      the manifest (below)
catalog/textures/<brand>.json  how each picture maps (the format of catalog/textures in the repo)
catalog/boards/<file>.json     optional: board records (the format of catalog/boards)
images/<name>.jpg              the pictures: .jpg or .png
```

Nothing else: any other path, a path that climbs out (`..`), or a name with characters other than
letters, digits, `.`, `-` and `_` makes the pack refused, as does a file the manifest doesn't list or
whose checksum doesn't match. Images are stored, not recompressed, at the size they were made.

## pack.json

```json
{
  "format": "cabinet-studio-material-pack",
  "format_version": 1,
  "id": "polytec",
  "name": "polytec decors",
  "brand": "Polytec",
  "version": "2026.11.27",
  "description": "Full-sheet photographs of polytec's board decors.",
  "source": "polytec.com.au, fetched by scripts/fetch_board_images.py",
  "files": { "catalog/textures/polytec.json": "<sha256>", "images/soft-walnut.jpg": "<sha256>" }
}
```

- `id`: lower case letters, digits and `-`; one pack per id is installed, a newer `version` replacing
  an older.
- `files`: every file in the zip but `pack.json`, with its SHA-256.
- A textures file's records name their pictures (`name`, and `normal` for a normal map) without the
  extension: each must be in `images/`.

## Installing

`install_material_pack` (cabinet-ffi) checks the whole pack before writing anything, then puts the
pictures in `~/Library/Application Support/CabinetStudio/decors` and the pack's catalog files under
`.../CabinetStudio/packs/<id>/catalog`, and records the pack in `.../packs/installed.json`.

The official packs are the assets of the GitHub release `material-packs`, listed by its
`packs.json`: each pack's id, name, version, file, size and SHA-256. On first run, and whenever that
list has a newer version, the app downloads what it's missing and installs it.
`scripts/make_material_packs.py` builds them from the catalog and the decors folder.

## Later

An import button for a pack file, and the catalog reading the records of an installed pack whose
brand the bundled catalog doesn't have, so a user's pack adds boards and slabs of its own.
