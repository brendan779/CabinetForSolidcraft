# Hardware catalog

One JSON file per brand and family (`blum-clip-top.json`, `hafele-legs.json`), loaded by
`cabinet-catalog` (`Catalog::hardware`). The builders take every supplier number from here by
name, so a missing number stops a build instead of becoming a guess.

```json
{
  "brand": "Blum",
  "website": "https://www.blum.com",
  "notes": "Where the rows came from and anything odd about them.",
  "items": [
    {
      "id": "blum.clip-top-blumotion.110.full",
      "brand": "Blum",
      "article": "the article number as printed",
      "family": "CLIP top BLUMOTION",
      "variant": "110°, full overlay",
      "kind": "hinge",
      "parameters": { "cup_diameter": 35.0, "cup_depth": 13.0 },
      "drilling": [{ "x": 0.0, "y": 0.0, "diameter": 35.0, "depth": 13.0, "note": "cup" }],
      "source": {
        "url": "the supplier document's URL",
        "document_date": "its date or version",
        "retrieved": "YYYY-MM-DD",
        "verified": false
      }
    }
  ]
}
```

- `kind`: hinge, hinge_plate, runner, drawer_system, lift_system, leg, handle, hanger, connector,
  shelf_support, lighting, other.
- `parameters`: named numbers in mm unless the name says otherwise (`max_weight_kg`).
- `drilling`: the pattern in the item's frame, `x` along the edge it is set out from and `y` in
  from that edge, from the item's reference point.
- `verified` stays `false` until the owner has checked the row against the document (BLUM-2,
  DATA-9). Nothing from `vendor-cad/` or the suppliers' PDFs is committed: facts only.
