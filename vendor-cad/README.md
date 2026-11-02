# Supplier CAD

Blum, Häfele and ABI models, committed at the owner's decision (2026-11-02) so every machine has
them. They are the suppliers' files and remain theirs, under the suppliers' own terms of use:
this repository's licence does not cover them.

```
vendor-cad/blum/<article>__<id>.step        scripts/import_blum_package.py takes these from Blum's packages
vendor-cad/hafele/minifix, axilo, shelf-supports, lighting, catalogue (drawings and raw files)
vendor-cad/abi/<article>.stl                scripts/import_abi_handles.py
vendor-cad/generic/
```

The app reads this folder at run time and falls back to its own stand-ins for anything missing.

Not committed, in `.gitignore`: `mesh/` (rebuilt from these by Settings › Folders › Prepare Now),
`abi-bim/` (the 2.2 GB library the handles came from), `abi-web/` (photos), `blum-sessions/` and
`blum/catalogue/` (the catalogue PDF and package zips, some over GitHub's 100 MB file limit).
