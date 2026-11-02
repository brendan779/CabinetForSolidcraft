//! SPLASH-1: a splashback's tiles. The field is laid out from the benchtop up and centred along
//! the wall, so the cuts fall evenly at the ends and at the top under the wall cabinets, as a tiler
//! sets it out.

use serde::{Deserialize, Serialize};

use crate::geom::Aabb;

/// A cut narrower than this share of a tile looks like a mistake, and tends to break when cut.
const SMALLEST_CUT: f64 = 1.0 / 3.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TilePattern {
    /// Joints in line both ways.
    #[default]
    Stack,
    /// Every other row moved along by half a tile.
    Brick,
}

/// What a splashback is finished in.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TileSpec {
    /// Across, in mm. 0: one sheet with no joints (glass, stainless, a slab).
    pub tile_width: f64,
    pub tile_height: f64,
    pub grout: f64,
    pub pattern: TilePattern,
    /// `#RRGGBB`.
    pub colour: String,
    pub grout_colour: String,
}

/// A stretch of wall finished in tile, in the run's frame: it faces the room, towards -Y.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TileField {
    pub name: String,
    pub bounds: Aabb,
    pub spec: TileSpec,
}

/// One tile on a field, from the field's bottom left corner, cut to fit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TileRect {
    pub x: f64,
    pub z: f64,
    pub width: f64,
    pub height: f64,
}

impl TileSpec {
    pub fn is_sheet(&self) -> bool {
        self.tile_width <= 0.0 || self.tile_height <= 0.0
    }

    /// The tiles of a field `width` × `height`, whole and cut.
    pub fn layout(&self, width: f64, height: f64) -> Vec<TileRect> {
        if self.is_sheet() {
            return vec![TileRect {
                x: 0.0,
                z: 0.0,
                width,
                height,
            }];
        }
        let (px, pz) = (self.tile_width + self.grout, self.tile_height + self.grout);
        let start = self.first_joint(width);
        let mut out = Vec::new();
        let mut row = 0;
        loop {
            let z = row as f64 * pz;
            if z >= height {
                break;
            }
            let shift = if self.pattern == TilePattern::Brick && row % 2 == 1 {
                px / 2.0
            } else {
                0.0
            };
            let top = (z + self.tile_height).min(height);
            let mut x = start + shift - px * ((start + shift) / px).ceil();
            while x < width {
                let (x0, x1) = (x.max(0.0), (x + self.tile_width).min(width));
                // A sliver of a tile is not laid: the grout takes it.
                if x1 - x0 >= 1.0 && top - z >= 1.0 {
                    out.push(TileRect {
                        x: x0,
                        z,
                        width: x1 - x0,
                        height: top - z,
                    });
                }
                x += px;
            }
            row += 1;
        }
        out
    }

    /// Where a tile starts, so the end cuts match: a tile or a joint on the centre, whichever
    /// leaves the larger cuts.
    fn first_joint(&self, width: f64) -> f64 {
        let px = self.tile_width + self.grout;
        let joint_centred = width / 2.0 + self.grout / 2.0;
        let tile_centred = (width - self.tile_width) / 2.0;
        let cut = |start: f64| {
            let c = start.rem_euclid(px) - self.grout;
            if c <= 0.0 { self.tile_width } else { c }
        };
        if cut(tile_centred) > cut(joint_centred) {
            tile_centred
        } else {
            joint_centred
        }
    }

    /// What a tiler would want to know before starting.
    pub fn warnings(&self, name: &str, width: f64, height: f64) -> Vec<String> {
        if self.is_sheet() {
            return Vec::new();
        }
        let mut out = Vec::new();
        let tiles = self.layout(width, height);
        let narrowest = tiles.iter().map(|t| t.width).fold(f64::MAX, f64::min);
        if narrowest < self.tile_width * SMALLEST_CUT {
            out.push(format!(
                "{name}: the end tiles are cut to {narrowest:.0} mm of {:.0}: a different tile or pattern would look better",
                self.tile_width
            ));
        }
        let lowest = tiles.iter().map(|t| t.height).fold(f64::MAX, f64::min);
        if lowest < self.tile_height * SMALLEST_CUT {
            out.push(format!(
                "{name}: the top row is cut to {lowest:.0} mm of {:.0}, under the wall cabinets",
                self.tile_height
            ));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(w: f64, h: f64, grout: f64, pattern: TilePattern) -> TileSpec {
        TileSpec {
            tile_width: w,
            tile_height: h,
            grout,
            pattern,
            colour: "#FFFFFF".into(),
            grout_colour: "#888888".into(),
        }
    }

    #[test]
    fn tiles_stay_on_the_field_with_a_grout_joint_between_them() {
        let s = spec(300.0, 75.0, 2.0, TilePattern::Stack);
        let tiles = s.layout(1000.0, 600.0);
        for t in &tiles {
            assert!(t.x >= 0.0 && t.x + t.width <= 1000.0 + 1e-9);
            assert!(t.z >= 0.0 && t.z + t.height <= 600.0 + 1e-9);
        }
        let row: Vec<_> = tiles.iter().filter(|t| t.z == 0.0).collect();
        for pair in row.windows(2) {
            assert!(((pair[1].x - (pair[0].x + pair[0].width)) - 2.0).abs() < 1e-9);
        }
    }

    #[test]
    fn the_end_cuts_match_because_the_field_is_centred() {
        let s = spec(300.0, 75.0, 2.0, TilePattern::Stack);
        let tiles = s.layout(1000.0, 75.0);
        let (first, last) = (tiles.first().unwrap(), tiles.last().unwrap());
        assert!(
            (first.width - last.width).abs() < 1e-9,
            "{first:?} {last:?}"
        );
    }

    #[test]
    fn rows_start_whole_on_the_benchtop_and_the_cut_row_is_at_the_top() {
        let s = spec(100.0, 100.0, 2.0, TilePattern::Stack);
        let tiles = s.layout(510.0, 250.0);
        assert!(
            tiles
                .iter()
                .filter(|t| t.z == 0.0)
                .all(|t| t.height == 100.0)
        );
        let top = tiles.iter().map(|t| t.z).fold(0.0, f64::max);
        assert_eq!(top, 204.0);
        assert!(
            tiles
                .iter()
                .filter(|t| t.z == top)
                .all(|t| t.height == 46.0)
        );
    }

    #[test]
    fn a_brick_pattern_moves_every_other_row_by_half_a_tile() {
        let s = spec(300.0, 75.0, 2.0, TilePattern::Brick);
        let tiles = s.layout(2000.0, 200.0);
        let whole = |z: f64| {
            tiles
                .iter()
                .find(|t| t.z == z && t.width == 300.0)
                .unwrap()
                .x
        };
        assert!(((whole(77.0) - whole(0.0)).rem_euclid(302.0) - 151.0).abs() < 1e-9);
    }

    #[test]
    fn changing_the_tile_changes_the_layout() {
        let a = spec(300.0, 75.0, 2.0, TilePattern::Stack).layout(1200.0, 600.0);
        let b = spec(100.0, 100.0, 2.0, TilePattern::Stack).layout(1200.0, 600.0);
        assert!(b.len() > a.len());
    }

    #[test]
    fn a_thin_cut_warns_and_an_even_fit_does_not() {
        let s = spec(100.0, 100.0, 0.0, TilePattern::Stack);
        assert!(s.warnings("S", 400.0, 400.0).is_empty());
        let top = s.warnings("S", 400.0, 410.0);
        assert!(top.iter().any(|w| w.contains("top row")), "{top:?}");
    }

    #[test]
    fn a_sheet_has_no_joints() {
        let s = spec(0.0, 0.0, 0.0, TilePattern::Stack);
        assert_eq!(s.layout(1200.0, 600.0).len(), 1);
    }
}
