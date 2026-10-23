//! Joinery: where biscuits, screws and dowels go along a butt joint.
//!
//! A butt joint is the end (edge) of one panel, the *end part*, against the face of another, the
//! *face part*: a bottom between two sides, a rail under a top. Fixings are spread along the line
//! where they meet, and every fixing becomes machining operations on both parts.

use crate::geom::{Aabb, Axis, Vec3};
use crate::model::{ConstructionStandard, FixingRecipe};
use crate::part::{BorePurpose, Dir, Operation, Part};

/// Distance from each end of the joint to the first fixing.
pub const END_SETBACK: f64 = 50.0;
/// Biscuits in panels thinner than this would break through a face.
pub const MIN_BISCUIT_PANEL: f64 = 12.0;

/// Chipboard screw: clearance hole in the face part, pilot in the end part.
const SCREW_CLEARANCE: f64 = 4.5;
const SCREW_PILOT: f64 = 3.0;
const SCREW_PILOT_DEPTH: f64 = 35.0;
/// Confirmat 7 × 50.
const CONFIRMAT_CLEARANCE: f64 = 7.0;
const CONFIRMAT_PILOT: f64 = 5.0;
const CONFIRMAT_PILOT_DEPTH: f64 = 40.0;
const DOWEL_DIAMETER: f64 = 8.0;
const DOWEL_FACE_DEPTH: f64 = 12.0;
const DOWEL_END_DEPTH: f64 = 25.0;

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
        (lo, hi)
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

        let screw_positions = match std.fixings {
            FixingRecipe::BiscuitAndScrew if biscuits => between(&biscuit_positions),
            FixingRecipe::BiscuitOnly if biscuits => Vec::new(),
            FixingRecipe::Dowel | FixingRecipe::CamLock => Vec::new(),
            _ => fixing_positions(length, END_SETBACK.min(length / 4.0), spacing, 20.0),
        };
        let (clearance, pilot, pilot_depth) = match std.fixings {
            FixingRecipe::Confirmat => {
                (CONFIRMAT_CLEARANCE, CONFIRMAT_PILOT, CONFIRMAT_PILOT_DEPTH)
            }
            _ => (SCREW_CLEARANCE, SCREW_PILOT, SCREW_PILOT_DEPTH),
        };
        for p in screw_positions {
            ops.extend(self.screw(lo + p, clearance, pilot, pilot_depth));
        }

        match std.fixings {
            FixingRecipe::Dowel => {
                for p in fixing_positions(length, 32.0, 96.0, 20.0) {
                    ops.extend(self.dowel(lo + p));
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
        }
    }

    fn screw(&self, at: f64, clearance: f64, pilot: f64, pilot_depth: f64) -> [Operation; 2] {
        [
            Operation::Bore {
                part: self.face_part.name.clone(),
                at: self.point(self.outside(), at),
                direction: self.normal,
                diameter: clearance,
                depth: self.face_part.thickness(),
                purpose: BorePurpose::Screw,
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

    fn dowel(&self, at: f64) -> [Operation; 2] {
        let back = Dir::new(self.normal.axis, !self.normal.positive);
        [
            Operation::Bore {
                part: self.face_part.name.clone(),
                at: self.point(self.contact(), at),
                direction: back,
                diameter: DOWEL_DIAMETER,
                depth: DOWEL_FACE_DEPTH.min(self.face_part.thickness() - 3.0),
                purpose: BorePurpose::Dowel,
            },
            Operation::Bore {
                part: self.end_part.name.clone(),
                at: self.point(self.contact(), at),
                direction: self.normal,
                diameter: DOWEL_DIAMETER,
                depth: DOWEL_END_DEPTH,
                purpose: BorePurpose::Dowel,
            },
        ]
    }
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
}
