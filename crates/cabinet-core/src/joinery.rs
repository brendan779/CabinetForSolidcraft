//! Joinery: where biscuits, screws, dowels and DOMINO tenons go along a butt joint.
//!
//! A butt joint is the end (edge) of one panel, the *end part*, against the face of another, the
//! *face part*: a bottom between two sides, a rail under a top. Fixings are spread along the line
//! where they meet, and every fixing becomes machining operations on both parts.

use cabinet_catalog::HardwareItem;

use crate::geom::{Aabb, Axis, Vec3};
use crate::hardware;
use crate::model::{ConstructionStandard, DowelSize, DowelSpacing, FixingRecipe};
use crate::part::{BorePurpose, Dir, Operation, Part};

/// Distance from each end of the joint to the first fixing.
pub const END_SETBACK: f64 = 50.0;
/// Biscuits in panels thinner than this would break through a face.
pub const MIN_BISCUIT_PANEL: f64 = 12.0;

/// Chipboard screw: clearance hole in the face part, pilot in the end part.
const SCREW_CLEARANCE: f64 = 4.5;
/// Countersunk head of a 4 mm chipboard screw (DIN 7997 style, 90°).
const SCREW_HEAD: f64 = 8.0;
/// Countersunk head of a 7 mm confirmat.
const CONFIRMAT_HEAD: f64 = 10.0;
const SCREW_PILOT: f64 = 3.0;
const SCREW_PILOT_DEPTH: f64 = 35.0;
/// Confirmat 7 × 50.
const CONFIRMAT_CLEARANCE: f64 = 7.0;
const CONFIRMAT_PILOT: f64 = 5.0;
const CONFIRMAT_PILOT_DEPTH: f64 = 40.0;
/// Room for glue at the bottom of the two dowel holes together, beyond the dowel's length.
const DOWEL_GLUE_ROOM: f64 = 2.0;
/// The 32 mm system: dowels on multiples of this from the joint's front end.
const SYSTEM32: f64 = 32.0;

/// Fixing centres along a joint of `length`, at least `setback` from each end and at most
/// `max_spacing` apart. Short joints get one fixing in the middle, or none when even that would
/// not fit `min_length`.
pub fn fixing_positions(length: f64, setback: f64, max_spacing: f64, min_length: f64) -> Vec<f64> {
    if length < min_length {
        return Vec::new();
    }
    if length < 2.0 * setback + 1.0 {
        return vec![length / 2.0];
    }
    let span = length - 2.0 * setback;
    let gaps = (span / max_spacing).ceil().max(1.0) as usize;
    (0..=gaps)
        .map(|i| setback + span * i as f64 / gaps as f64)
        .collect()
}

/// Dowel centres on the 32 mm grid from the joint's front end: every whole number of pitches
/// that keeps them at most `max_spacing` apart, the first and last at least one pitch in. A joint
/// too short for two gets one in the middle.
pub fn grid_positions(length: f64, max_spacing: f64) -> Vec<f64> {
    if length < 2.0 * SYSTEM32 + 1e-9 {
        return if length >= 20.0 {
            vec![length / 2.0]
        } else {
            Vec::new()
        };
    }
    let step = SYSTEM32 * (max_spacing / SYSTEM32).floor().max(1.0);
    let last = SYSTEM32 * ((length - SYSTEM32) / SYSTEM32).floor();
    let mut out = Vec::new();
    let mut p = SYSTEM32;
    while p < last - 1e-9 {
        out.push(p);
        p += step;
    }
    out.push(last);
    out
}

/// Hole depths for a dowel in the face part (thickness `face`) and the end part: half the
/// dowel in the face part where that leaves a skin, the rest in the end part, plus glue room.
pub fn dowel_depths(size: DowelSize, face: f64) -> (f64, f64) {
    let face_d = (size.length() / 2.0).min(face - MIN_SKIN);
    (face_d, size.length() - face_d + DOWEL_GLUE_ROOM)
}

/// Midpoints between consecutive positions.
pub fn between(positions: &[f64]) -> Vec<f64> {
    positions.windows(2).map(|w| (w[0] + w[1]) / 2.0).collect()
}

/// A butt joint between two parts.
pub struct ButtJoint<'a> {
    pub face_part: &'a Part,
    pub end_part: &'a Part,
    /// Direction from the face part towards the end part, across the contact plane.
    pub normal: Dir,
    /// The axis the joint line runs along.
    pub along: Axis,
    /// Where along the line fixings may go, when less than the whole contact: a back panel's
    /// groove crosses the joint behind this, and a biscuit there would cut into it.
    pub limit: Option<(f64, f64)>,
    /// The face part's outside is seen from the room: nothing may come through it. Biscuits and
    /// glue where they fit, blind dowels where they do not.
    pub hidden: bool,
}

impl ButtJoint<'_> {
    /// The contact plane's coordinate on `normal.axis`.
    fn contact(&self) -> f64 {
        let b = &self.face_part.bounds;
        if self.normal.positive {
            b.max().get(self.normal.axis)
        } else {
            b.min.get(self.normal.axis)
        }
    }

    /// The outside face of the face part, where screws go in.
    fn outside(&self) -> f64 {
        let b = &self.face_part.bounds;
        if self.normal.positive {
            b.min.get(self.normal.axis)
        } else {
            b.max().get(self.normal.axis)
        }
    }

    /// The shared extent along the joint line.
    fn range(&self) -> (f64, f64) {
        let (a, b) = (&self.face_part.bounds, &self.end_part.bounds);
        let lo = a.min.get(self.along).max(b.min.get(self.along));
        let hi = a.max().get(self.along).min(b.max().get(self.along));
        match self.limit {
            Some((l, h)) => (lo.max(l), hi.min(h)),
            None => (lo, hi),
        }
    }

    /// The axis across the end part's thickness, and its middle.
    fn mid_thickness(&self) -> (Axis, f64) {
        let t = self.end_part.thickness_axis;
        let b = &self.end_part.bounds;
        (t, b.min.get(t) + b.size.get(t) / 2.0)
    }

    fn point(&self, normal_at: f64, along_at: f64) -> Vec3 {
        let (t, mid) = self.mid_thickness();
        Vec3::default()
            .with(self.normal.axis, normal_at)
            .with(self.along, along_at)
            .with(t, mid)
    }

    /// The machining for this joint under `std`'s fixing recipe. Problems go to `warnings`.
    pub fn operations(
        &self,
        std: &ConstructionStandard,
        warnings: &mut Vec<String>,
    ) -> Vec<Operation> {
        let (lo, hi) = self.range();
        let length = hi - lo;
        if length <= 0.0 {
            return Vec::new();
        }
        let spacing = std.fixing_density.max_spacing();
        let biscuits_fit = self.end_part.thickness() >= MIN_BISCUIT_PANEL
            && self.face_part.thickness() >= MIN_BISCUIT_PANEL;
        let mut ops = Vec::new();
        let biscuit_positions = fixing_positions(
            length,
            END_SETBACK,
            spacing,
            std.biscuit.slot_length() + 10.0,
        );

        let wants_biscuits = matches!(
            std.fixings,
            FixingRecipe::BiscuitAndScrew | FixingRecipe::BiscuitOnly
        );
        if wants_biscuits && !biscuits_fit {
            warnings.push(format!(
                "{} to {}: panel too thin for biscuits, screwed instead",
                self.end_part.name, self.face_part.name
            ));
        }
        // A lone biscuit leaves no room for screws beside it; short joints (rails) are screwed.
        let biscuits = wants_biscuits && biscuits_fit && biscuit_positions.len() >= 2;
        if biscuits {
            for &p in &biscuit_positions {
                ops.push(self.biscuit_slot(std, lo + p));
            }
        }

        if std.fixings == FixingRecipe::Domino {
            match self.dominos(std, lo, length) {
                Ok(d) => ops.extend(d),
                Err(e) => warnings.push(format!(
                    "{} to {}: {e}",
                    self.end_part.name, self.face_part.name
                )),
            }
            return ops;
        }
        let screw_positions = match std.fixings {
            _ if self.hidden => Vec::new(),
            FixingRecipe::BiscuitAndScrew if biscuits => between(&biscuit_positions),
            FixingRecipe::BiscuitOnly if biscuits => Vec::new(),
            FixingRecipe::Dowel | FixingRecipe::CamLock => Vec::new(),
            _ => fixing_positions(length, END_SETBACK.min(length / 4.0), spacing, 20.0),
        };
        let (clearance, head, pilot, pilot_depth) = match std.fixings {
            FixingRecipe::Confirmat => (
                CONFIRMAT_CLEARANCE,
                CONFIRMAT_HEAD,
                CONFIRMAT_PILOT,
                CONFIRMAT_PILOT_DEPTH,
            ),
            _ => (SCREW_CLEARANCE, SCREW_HEAD, SCREW_PILOT, SCREW_PILOT_DEPTH),
        };
        for p in screw_positions {
            ops.extend(self.screw(lo + p, clearance, head, pilot, pilot_depth));
        }

        let dowels = std.fixings == FixingRecipe::Dowel || (self.hidden && !biscuits);
        match std.fixings {
            _ if dowels => {
                let d = std.dowel.diameter();
                if d > self.end_part.thickness() - 2.0 * MIN_SKIN {
                    warnings.push(format!(
                        "{} to {}: a {d} mm dowel leaves less than {MIN_SKIN} mm each side",
                        self.end_part.name, self.face_part.name
                    ));
                    return ops;
                }
                let spacing = std.fixing_density.max_spacing();
                let at = match std.dowel_spacing {
                    DowelSpacing::System32 => grid_positions(length, spacing),
                    DowelSpacing::Jig => fixing_positions(length, SYSTEM32, spacing, 20.0),
                };
                for p in at {
                    ops.extend(self.dowel(std.dowel, lo + p));
                }
            }
            FixingRecipe::CamLock => warnings.push(format!(
                "{} to {}: cam-lock fixings are not generated yet",
                self.end_part.name, self.face_part.name
            )),
            _ => {}
        }
        ops
    }

    /// Chipboard screws alone, at most `spacing` apart, piloted `pilot_depth` into the end part:
    /// for a thin back screwed onto the carcass's rear edges, which takes no biscuits.
    pub fn screws_every(&self, spacing: f64, pilot_depth: f64) -> Vec<Operation> {
        let (lo, hi) = self.range();
        let length = hi - lo;
        if length <= 0.0 {
            return Vec::new();
        }
        fixing_positions(length, END_SETBACK.min(length / 4.0), spacing, 20.0)
            .into_iter()
            .flat_map(|p| {
                self.screw(
                    lo + p,
                    SCREW_CLEARANCE,
                    SCREW_HEAD,
                    SCREW_PILOT,
                    pilot_depth,
                )
            })
            .collect()
    }

    /// DOMINO mortises along the joint, spaced like biscuits: one in each part per tenon, the
    /// tight width (the tenon's own), as deep as the face part allows and the rest in the end
    /// part, on the depths the machine can set.
    fn dominos(
        &self,
        std: &ConstructionStandard,
        lo: f64,
        length: f64,
    ) -> Result<Vec<Operation>, String> {
        let tenon = hardware::item(&std.domino.catalog_id())?;
        let (thick, wide) = (tenon.param("thickness")?, tenon.param("width")?);
        let (face_t, end_t) = (self.face_part.thickness(), self.end_part.thickness());
        if thick > end_t - 2.0 * MIN_SKIN {
            return Err(format!(
                "a {thick} mm DOMINO leaves less than {MIN_SKIN} mm each side in {end_t} mm board"
            ));
        }
        let (face_d, end_d) = domino_depths(tenon, face_t - MIN_SKIN)?;
        let positions = fixing_positions(
            length,
            END_SETBACK,
            std.fixing_density.max_spacing(),
            wide + 10.0,
        );
        if positions.is_empty() {
            return Err(format!("too short for a {wide} mm wide DOMINO"));
        }
        let (t, mid) = self.mid_thickness();
        let c = self.contact();
        let sign = self.normal.sign();
        let mut ops = Vec::new();
        for p in positions {
            let at = lo + p;
            for (part, from, depth) in [
                (self.face_part, c - sign * face_d, face_d),
                (self.end_part, c, end_d),
            ] {
                let n0 = if sign > 0.0 { from } else { from - depth };
                ops.push(Operation::Mortise {
                    part: part.name.clone(),
                    bounds: Aabb::new(
                        Vec3::default()
                            .with(self.normal.axis, n0)
                            .with(self.along, at - wide / 2.0)
                            .with(t, mid - thick / 2.0),
                        Vec3::default()
                            .with(self.normal.axis, depth)
                            .with(self.along, wide)
                            .with(t, thick),
                    ),
                    tenon: tenon.id.clone(),
                });
            }
        }
        Ok(ops)
    }

    fn biscuit_slot(&self, std: &ConstructionStandard, at: f64) -> Operation {
        let size = std.biscuit;
        let depth = size.slot_depth();
        let len = size.slot_length();
        let (t, mid) = self.mid_thickness();
        let c = self.contact();
        let min = Vec3::default()
            .with(self.normal.axis, c - depth)
            .with(self.along, at - len / 2.0)
            .with(t, mid - size.thickness() / 2.0);
        let extent = Vec3::default()
            .with(self.normal.axis, 2.0 * depth)
            .with(self.along, len)
            .with(t, size.thickness());
        Operation::BiscuitSlot {
            parts: [self.face_part.name.clone(), self.end_part.name.clone()],
            bounds: Aabb::new(min, extent),
            along: self.along,
            size,
        }
    }

    fn screw(
        &self,
        at: f64,
        clearance: f64,
        head: f64,
        pilot: f64,
        pilot_depth: f64,
    ) -> [Operation; 3] {
        [
            Operation::Bore {
                part: self.face_part.name.clone(),
                at: self.point(self.outside(), at),
                direction: self.normal,
                diameter: clearance,
                depth: self.face_part.thickness(),
                purpose: BorePurpose::Screw,
            },
            // A 90° seat: as deep as the head is wider than the hole, halved.
            Operation::Bore {
                part: self.face_part.name.clone(),
                at: self.point(self.outside(), at),
                direction: self.normal,
                diameter: head,
                depth: (head - clearance) / 2.0,
                purpose: BorePurpose::Countersink,
            },
            Operation::Bore {
                part: self.end_part.name.clone(),
                at: self.point(self.contact(), at),
                direction: self.normal,
                diameter: pilot,
                depth: pilot_depth.min(self.end_part.bounds.size.get(self.normal.axis)),
                purpose: BorePurpose::ScrewPilot,
            },
        ]
    }

    fn dowel(&self, size: DowelSize, at: f64) -> [Operation; 2] {
        let back = Dir::new(self.normal.axis, !self.normal.positive);
        let (face, end) = dowel_depths(size, self.face_part.thickness());
        [
            Operation::Bore {
                part: self.face_part.name.clone(),
                at: self.point(self.contact(), at),
                direction: back,
                diameter: size.diameter(),
                depth: face,
                purpose: BorePurpose::Dowel,
            },
            Operation::Bore {
                part: self.end_part.name.clone(),
                at: self.point(self.contact(), at),
                direction: self.normal,
                diameter: size.diameter(),
                depth: end,
                purpose: BorePurpose::Dowel,
            },
        ]
    }
}

/// Material left beside a bore or mortise in a panel's face or across its thickness.
const MIN_SKIN: f64 = 3.0;

/// Mortise depths in the face and end part for `tenon`, with at most `face_max` in the face part
/// (its thickness less a skin): half the tenon each where the face part allows (Festool's
/// guide), on the depths the machine sets: the DF 500 on its stops, the DF 700 anywhere in its
/// range. Together they hold the tenon's nominal length; Festool's tenons are 2 mm shorter, which
/// leaves the glue room.
pub fn domino_depths(tenon: &HardwareItem, face_max: f64) -> Result<(f64, f64), String> {
    let long = tenon.param("length")?;
    if tenon.param("df700")? > 0.0 {
        let m = hardware::item("festool.domino.df700")?;
        let (lo, hi) = (m.param("depth_min")?, m.param("depth_max")?);
        let face = face_max.min(long / 2.0).min(hi);
        let end = (long - face).max(lo);
        if face < lo || end > hi {
            return Err(format!(
                "the DF 700 cuts {lo}–{hi} mm deep: no room for {} in this board",
                tenon.variant
            ));
        }
        return Ok((face, end));
    }
    let m = hardware::item("festool.domino.df500")?;
    let stops: Vec<f64> = (1..=5)
        .filter_map(|i| m.param(&format!("depth_stop_{i}")).ok())
        .collect();
    let face = stops
        .iter()
        .copied()
        .filter(|s| *s <= face_max + 1e-9)
        .reduce(f64::max)
        .ok_or_else(|| {
            format!(
                "the face part is too thin for the DF 500's {} mm stop",
                stops[0]
            )
        })?;
    let end = stops
        .iter()
        .copied()
        .find(|s| *s >= long - face - 1e-9)
        .ok_or_else(|| {
            format!(
                "a {long} mm DOMINO needs {} mm in the end part, past the DF 500's deepest stop",
                long - face
            )
        })?;
    Ok((face, end))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positions_respect_setback_and_spacing() {
        let p = fixing_positions(544.0, 50.0, 180.0, 60.0);
        assert_eq!(p.first(), Some(&50.0));
        assert_eq!(p.last(), Some(&494.0));
        assert!(p.windows(2).all(|w| w[1] - w[0] <= 180.0 + 1e-9));
        assert_eq!(p.len(), 4);
    }

    #[test]
    fn short_joints_get_one_fixing_or_none() {
        assert_eq!(fixing_positions(80.0, 50.0, 180.0, 60.0), vec![40.0]);
        assert!(fixing_positions(40.0, 50.0, 180.0, 60.0).is_empty());
    }

    #[test]
    fn screws_sit_between_biscuits() {
        assert_eq!(between(&[50.0, 150.0, 250.0]), vec![100.0, 200.0]);
    }

    /// JOIN-8: each DOMINO size in 16 mm and 19 mm board: the face and end depths Festool's
    /// machines can set, or why not.
    #[test]
    fn domino_depths_per_size_on_16_and_19_mm_board() {
        use crate::model::DominoSize::*;
        let depths = |size: crate::model::DominoSize, board: f64| {
            domino_depths(
                hardware::item(&size.catalog_id()).unwrap(),
                board - MIN_SKIN,
            )
            .ok()
        };
        let rows = [
            (D4x20, 16.0, Some((12.0, 12.0))),
            (D5x30, 16.0, Some((12.0, 20.0))),
            (D6x40, 16.0, Some((12.0, 28.0))),
            (D8x40, 16.0, Some((12.0, 28.0))),
            (D8x50, 16.0, None),
            (D10x50, 16.0, None),
            (D8x40, 19.0, Some((15.0, 25.0))),
            (D10x50, 19.0, None),
            (D8x80, 16.0, None),
            (D8x80, 25.0, Some((22.0, 58.0))),
            // 78 mm in the end part is past the DF 700's 70.
            (D12x100, 25.0, None),
            (D12x100, 40.0, Some((37.0, 63.0))),
            (D14x100, 40.0, Some((37.0, 63.0))),
        ];
        for (size, board, want) in rows {
            assert_eq!(depths(size, board), want, "{size:?} in {board}");
        }
    }

    #[test]
    fn every_domino_size_has_a_tenon_in_the_catalog() {
        use crate::model::DominoSize::*;
        for size in [
            D4x20, D5x30, D6x40, D8x40, D8x50, D10x50, D8x80, D10x80, D12x100, D14x100,
        ] {
            let t = hardware::item(&size.catalog_id()).unwrap();
            assert!(t.param("width").unwrap() > t.param("thickness").unwrap());
        }
    }

    /// JOIN-9: the 32 mm grid keeps every dowel on a multiple of 32 from the front, at most the
    /// fixing spacing apart; the jig spreads them evenly from 32 in.
    #[test]
    fn dowels_sit_on_the_32_mm_grid_or_evenly_for_a_jig() {
        let grid = grid_positions(544.0, 180.0);
        assert_eq!(grid, [32.0, 192.0, 352.0, 512.0]);
        assert!(grid_positions(300.0, 96.0).iter().all(|p| p % 32.0 == 0.0));
        assert!(
            grid_positions(300.0, 96.0)
                .windows(2)
                .all(|w| w[1] - w[0] <= 96.0)
        );
        assert_eq!(grid_positions(50.0, 180.0), [25.0]);
        let jig = fixing_positions(544.0, SYSTEM32, 180.0, 20.0);
        assert_eq!((jig[0], *jig.last().unwrap()), (32.0, 512.0));
        assert!(
            jig.windows(2)
                .all(|w| (w[1] - w[0] - (jig[1] - jig[0])).abs() < 1e-9)
        );
    }

    /// The face part takes half the dowel where it leaves a 3 mm skin; the end part the rest
    /// and 2 mm of glue room.
    #[test]
    fn dowel_depth_split_between_face_and_end() {
        use crate::model::DowelSize::*;
        assert_eq!(dowel_depths(D8x35, 16.0), (13.0, 24.0));
        assert_eq!(dowel_depths(D8x30, 18.0), (15.0, 17.0));
        assert_eq!(dowel_depths(D6x30, 25.0), (15.0, 17.0));
        assert_eq!(dowel_depths(D10x50, 18.0), (15.0, 37.0));
    }
}
