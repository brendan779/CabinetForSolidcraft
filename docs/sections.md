# Sections (SEC-1)

The owner's ask (2026-10-27): build a cabinet the way Blum's configurator does. Start from an open
carcass, place vertical dividers, and set each section on its own: open, shelves, a shoe rack,
drawers, a hanging rail. A section isn't a separate cabinet.

## Model

A new front layout, `FrontLayout::Sections { columns }`, replaces the cabinet's single front
layout and its shelves:

```text
Sections
  columns: [Column]            left to right
Column
  width: mm                    the column's share of the fronts' width (0: shares what is left)
  rows: [Section]              bottom to top
Section
  height: mm                   its fronts' height (0: shares what is left), as Stack does
  front: open | door{hinge_side} | door_pair | drawers{count}
  inside: empty | shelves{count} | shoe_shelves{count} | hanging | inner_drawers{count}
```

- **Dividers:** a vertical panel stands between neighbouring columns. It runs from the bottom up to
  the top (or rails) and from the carcass front to the interior's back. It's jointed to the bottom
  and the top like a side.
- **Fixed shelves:** one goes between neighbouring rows of a column, across that column only.
  It's jointed to the side or divider on each hand and placed as Stack places them: its top sits
  where the next row's fronts start.
- **Fronts:** a row's fronts span its column's share of the front width. The front gap separates
  them from the next column's fronts, and the divider's centre lies in that gap. A door hinges on
  the side or divider on its hinge side, with half overlay on a divider (two doors share it, as
  Blum's middle panel). Drawer boxes, runners and front fixings are sized to the column's bay
  (the inner faces either side), not the cabinet's sides.
- **Inside:**
  - *shelves:* adjustable shelves, evenly spaced, each on four pins.
  - *shoe shelves:* fixed shelves at an even pitch.
  - *hanging:* a wardrobe rail under the row's top.
  - *inner drawers:* LEGRABOX inner drawers stacked from the row's floor (doors over them get
    Blum's 0-protrusion hinge, as now).
- **Kinds:** sections work on every carcass kind. A wardrobe with sections takes its interior from
  them, not from its hanging preset.

The existing layouts stay. Stack is a one-column Sections whose rows have no inside settings, and
it keeps building as before. The app starts a Sections layout from one column and one row
holding the cabinet's current front.

## Inspector

A **Sections** tile joins the front pictures. Picking it shows the cabinet's elevation drawn to
scale. Click a cell to select it, and under the drawing:

- **Front:** tiles for Open, Door left, Door right, Pair, Drawers (with a count).
- **Inside:** tiles for Empty, Shelves, Shoe, Hanging, Inner drawers (with a count).
- **Height** (the row) and **Width** (the column): rows of usual sizes plus Auto.
- **Split:** a divider to the selected cell's right, or a shelf over it. **Remove** merges the
  cell away.

## Left for later

- Adjustable shelves in a section get their four supports, but no pin holes. The holes need the
  pin rows on dividers, which come later.
- A divider is always full height. A divider inside one row only (a split within a drawer
  section) would be a tree, not columns of rows.
- Sloped shoe shelves: today they're flat fixed shelves.
