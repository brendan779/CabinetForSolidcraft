//! The collision checker: machining that runs into other machining in the same part, or out of
//! the part where it should not. The builder avoids what it knows about; this is the check that
//! every rule together still leaves a part that can be made.

use crate::geom::{Aabb, Axis, Vec3};
use crate::part::{BorePurpose, Operation, Part, PartList};

/// Thinnest material left at the bottom of a blind bore or between a bore and a panel edge (mm).
pub const MIN_WALL: f64 = 2.0;
/// Overlaps smaller than this (mm) are rounding, not clashes.
const EPS: f64 = 1e-6;

#[derive(Clone, Debug, PartialEq)]
pub struct Clash {
    pub part: String,
    pub what: String,
}

impl std::fmt::Display for Clash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.part, self.what)
    }
}

/// A machining volume in one part.
#[derive(Clone, Debug)]
enum Cut {
    /// A bore: the cylinder from `start` along +`axis` for `length`.
    Bore {
        axis: Axis,
        start: Vec3,
        length: f64,
        radius: f64,
        /// A clearance hole, meant to go right through.
        through: bool,
        label: String,
    },
    Box {
        bounds: Aabb,
        label: String,
    },
}

impl Cut {
    fn label(&self) -> &str {
        match self {
            Cut::Bore { label, .. } | Cut::Box { label, .. } => label,
        }
    }

    fn bounds(&self) -> Aabb {
        match self {
            Cut::Box { bounds, .. } => *bounds,
            Cut::Bore {
                axis,
                start,
                length,
                radius,
                ..
            } => {
                let mut min = *start;
                let mut size = Vec3::new(2.0 * radius, 2.0 * radius, 2.0 * radius);
                for a in axis.others() {
                    min = min.with(a, start.get(a) - radius);
                }
                size = size.with(*axis, *length);
                Aabb::new(min, size)
            }
        }
    }
}

/// Every clash in the part list.
pub fn check(list: &PartList) -> Vec<Clash> {
    let mut out = Vec::new();
    for part in &list.parts {
        let cuts = cuts_in(list, part);
        for c in &cuts {
            if let Some(why) = leaves_part(part, c) {
                out.push(Clash {
                    part: part.name.clone(),
                    what: format!("{} {why}", c.label()),
                });
            }
        }
        for (i, a) in cuts.iter().enumerate() {
            for b in &cuts[i + 1..] {
                if intersects(a, b) && !seat_and_hole(a, b) {
                    out.push(Clash {
                        part: part.name.clone(),
                        what: format!("{} runs into {}", a.label(), b.label()),
                    });
                }
            }
        }
    }
    out
}

fn cuts_in(list: &PartList, part: &Part) -> Vec<Cut> {
    let mut cuts = Vec::new();
    for op in &list.operations {
        match op {
            Operation::Bore {
                part: p,
                at,
                direction,
                diameter,
                depth,
                purpose,
            } if *p == part.name => {
                let start = if direction.positive {
                    *at
                } else {
                    at.with(direction.axis, at.get(direction.axis) - depth)
                };
                cuts.push(Cut::Bore {
                    axis: direction.axis,
                    start,
                    length: *depth,
                    radius: diameter / 2.0,
                    through: matches!(
                        purpose,
                        BorePurpose::Screw
                            | BorePurpose::CableHole
                            | BorePurpose::HandleScrew
                            | BorePurpose::TapHole
                    ),
                    label: format!("{purpose:?} Ø{diameter} at {}", show(*at)),
                });
            }
            Operation::BiscuitSlot { parts, bounds, .. } if parts.contains(&part.name) => {
                cuts.push(Cut::Box {
                    bounds: *bounds,
                    label: format!("biscuit slot at {}", show(bounds.center())),
                });
            }
            Operation::Groove { part: p, bounds }
            | Operation::Cutout {
                part: p, bounds, ..
            } if *p == part.name => {
                cuts.push(Cut::Box {
                    bounds: *bounds,
                    label: format!("groove at {}", show(bounds.min)),
                });
            }
            Operation::Mortise {
                part: p, bounds, ..
            } if *p == part.name => {
                cuts.push(Cut::Box {
                    bounds: *bounds,
                    label: format!("mortise at {}", show(bounds.center())),
                });
            }
            _ => {}
        }
    }
    cuts
}

fn show(v: Vec3) -> String {
    format!("({:.0}, {:.0}, {:.0})", v.x, v.y, v.z)
}

/// Why a bore leaves its part where it should not: through the far face (unless a clearance hole
/// meant to), too thin a floor, or out through an edge.
fn leaves_part(part: &Part, cut: &Cut) -> Option<String> {
    let Cut::Bore {
        axis,
        start,
        length,
        radius,
        through: clearance,
        ..
    } = cut
    else {
        return None;
    };
    let b = &part.bounds;
    let (lo, hi) = (b.min.get(*axis), b.max().get(*axis));
    let (s, e) = (start.get(*axis), start.get(*axis) + length);
    let through = s <= lo + EPS && e >= hi - EPS;
    if through && !clearance {
        return Some("goes right through".into());
    }
    if !through {
        if s < lo - EPS || e > hi + EPS {
            return Some("breaks out of the far face".into());
        }
        let floor = if s <= lo + EPS { hi - e } else { s - lo };
        if floor < MIN_WALL - EPS {
            return Some(format!("leaves a {floor:.1} mm floor"));
        }
    }
    for a in axis.others() {
        let c = start.get(a);
        let wall = (c - radius - b.min.get(a)).min(b.max().get(a) - c - radius);
        if wall < -EPS {
            return Some("breaks out of an edge".into());
        }
    }
    None
}

/// A countersink and the clearance hole it is the seat of: one axis, one mouth.
fn seat_and_hole(a: &Cut, b: &Cut) -> bool {
    match (a, b) {
        (
            Cut::Bore {
                axis: x,
                start: s,
                label: la,
                ..
            },
            Cut::Bore {
                axis: y,
                start: t,
                label: lb,
                ..
            },
        ) => {
            x == y
                && x.others()
                    .iter()
                    .all(|&o| (s.get(o) - t.get(o)).abs() < EPS)
                && (la.starts_with("Countersink") || lb.starts_with("Countersink"))
        }
        _ => false,
    }
}

fn intersects(a: &Cut, b: &Cut) -> bool {
    let (ba, bb) = (a.bounds(), b.bounds());
    if !ba.overlaps(&bb) {
        return false;
    }
    // Boxes that overlap meet. A bore meets a volume only where its round section does.
    round_section_meets(a, &bb) && round_section_meets(b, &ba)
}

/// For a bore: does its circular section reach the box's footprint in the section plane? For a
/// box: always (the bounding boxes already overlap).
fn round_section_meets(cut: &Cut, other: &Aabb) -> bool {
    let Cut::Bore {
        axis,
        start,
        radius,
        ..
    } = cut
    else {
        return true;
    };
    let mut d2 = 0.0;
    for a in axis.others() {
        let c = start.get(a);
        let nearest = c.clamp(other.min.get(a), other.max().get(a));
        d2 += (c - nearest).powi(2);
    }
    // Two bores side by side on the same axis meet only where their circles do, which the
    // nearest point of the other bore's box over-reports by the corner of its square section.
    d2 < (radius - EPS).powi(2)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;
    use crate::options::option_groups;
    use crate::{Cabinet, build_cabinet};

    #[test]
    fn two_bores_in_one_place_clash() {
        let mut list = build_cabinet(&Cabinet::new("B01", CabinetKind::Base)).unwrap();
        let first = list
            .operations
            .iter()
            .find(|o| {
                matches!(
                    o,
                    Operation::Bore {
                        purpose: BorePurpose::ShelfPin,
                        ..
                    }
                )
            })
            .unwrap()
            .clone();
        list.operations.push(first);
        assert!(!check(&list).is_empty());
    }

    #[test]
    fn a_bore_out_of_an_edge_is_reported() {
        let mut list = build_cabinet(&Cabinet::new("B01", CabinetKind::Base)).unwrap();
        let side = list.part("B01.LeftSide").unwrap().clone();
        list.operations.push(Operation::Bore {
            part: side.name.clone(),
            at: Vec3::new(16.0, 1.0, 400.0),
            direction: crate::part::Dir::new(Axis::X, false),
            diameter: 5.0,
            depth: 10.0,
            purpose: BorePurpose::ShelfPin,
        });
        let clashes = check(&list);
        assert!(
            clashes.iter().any(|c| c.what.contains("edge")),
            "{clashes:?}"
        );
    }

    #[test]
    fn every_kind_with_fixed_shelves_and_exposed_ends_builds_without_clashes() {
        for kind in CabinetKind::ALL {
            let mut cab = Cabinet::new("C01", kind);
            cab.adjustable_shelves = cab.adjustable_shelves.max(2);
            cab.fixed_shelves = (1..=cab.adjustable_shelves).collect();
            cab.exposed = Exposed {
                left: true,
                right: true,
            };
            let list = build_cabinet(&cab).unwrap();
            assert_eq!(check(&list), [], "{kind:?}");
        }
    }

    /// Every kind built with every construction choice, one field at a time, has no clashes.
    #[test]
    fn every_kind_and_every_choice_builds_without_clashes() {
        let base = serde_json::to_value(ConstructionStandard::default()).unwrap();
        let mut failures = Vec::new();
        for kind in CabinetKind::ALL {
            for g in option_groups() {
                for c in &g.choices {
                    let mut v = base.clone();
                    v[&g.field] = serde_json::Value::String(c.id.clone());
                    let mut cab = Cabinet::new("C01", kind);
                    cab.standard = serde_json::from_value(v).unwrap();
                    let Ok(list) = build_cabinet(&cab) else {
                        continue;
                    };
                    for clash in check(&list) {
                        failures.push(format!("{kind:?} {}={}: {clash}", g.field, c.id));
                    }
                }
            }
        }
        failures.sort();
        failures.dedup();
        assert!(
            failures.is_empty(),
            "{} clashes:\n{}",
            failures.len(),
            failures.join("\n")
        );
    }
}
