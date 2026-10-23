//! Runs: cabinets side by side along a wall, with the fillers and end panels between and beside
//! them. A run lays its items out left to right along X and builds each into its own part list.

use serde::{Deserialize, Serialize};

use crate::geom::{Aabb, Axis, Vec3};
use crate::model::Cabinet;
use crate::part::{EdgeBanding, Part, PartList, PartRole};
use crate::{Error, Result, build_cabinet};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Run {
    /// Where the run's left end sits (mm): its front at `origin.y`, floor at `origin.z`.
    #[serde(default)]
    pub origin: Vec3,
    pub items: Vec<RunItem>,
}

/// One thing in a run, left to right.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum RunItem {
    Cabinet(Box<Cabinet>),
    Filler(Filler),
    EndPanel(EndPanel),
}

/// A strip in the plane of the fronts, closing a gap between cabinets or to a wall.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Filler {
    /// Short stable id, used in the part name (`F01.Filler`).
    pub id: String,
    /// Its finished width once fitted.
    pub width: f64,
    /// Extra width to cut to an out-of-true wall on site. Not part of the run's length.
    #[serde(default)]
    pub scribe: f64,
}

/// A decorative panel on the end of a run, covering the carcass side and the edge of the fronts.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EndPanel {
    /// Short stable id, used in the part name (`E01.EndPanel`).
    pub id: String,
    /// Extra depth at the back to cut to the wall on site.
    #[serde(default)]
    pub scribe: f64,
    /// Down to the floor across the toe kick, rather than stopping on top of it.
    #[serde(default = "yes")]
    pub to_floor: bool,
}

fn yes() -> bool {
    true
}

/// Builds every item of the run in place. Fillers and end panels take their height, depth and
/// board from the nearest cabinet, so a run needs at least one.
pub fn build_run(run: &Run) -> Result<Vec<PartList>> {
    let cabinets: Vec<(usize, &Cabinet)> = run
        .items
        .iter()
        .enumerate()
        .filter_map(|(i, item)| match item {
            RunItem::Cabinet(c) => Some((i, c.as_ref())),
            _ => None,
        })
        .collect();
    if cabinets.is_empty() {
        return Err(Error::Invalid("a run needs at least one cabinet".into()));
    }
    // The cabinet a filler or end panel at `i` belongs against: the nearest one, left first.
    let neighbour = |i: usize| {
        cabinets
            .iter()
            .min_by_key(|(j, _)| (i.abs_diff(*j), *j > i))
            .map(|(_, c)| *c)
            .unwrap()
    };
    let mut x = run.origin.x;
    let mut out = Vec::new();
    for (i, item) in run.items.iter().enumerate() {
        match item {
            RunItem::Cabinet(c) => {
                let mut c = c.as_ref().clone();
                c.origin = Vec3::new(x, run.origin.y, run.origin.z + c.origin.z);
                x += c.width;
                out.push(build_cabinet(&c)?);
            }
            RunItem::Filler(f) => {
                let list = filler(f, neighbour(i), Vec3::new(x, run.origin.y, run.origin.z))?;
                x += f.width;
                out.push(list);
            }
            RunItem::EndPanel(e) => {
                let c = neighbour(i);
                let list = end_panel(e, c, Vec3::new(x, run.origin.y, run.origin.z));
                x += c.standard.front_thickness;
                out.push(list);
            }
        }
    }
    Ok(out)
}

/// The run's front region of `c` in Z: where its doors start and stop.
fn front_z(c: &Cabinet) -> (f64, f64) {
    let kick = if c.has_toe_kick() {
        c.standard.toe_kick_height
    } else {
        0.0
    };
    (c.origin.z + kick, c.origin.z + c.height)
}

fn filler(f: &Filler, c: &Cabinet, at: Vec3) -> Result<PartList> {
    if !(f.width > 0.0 && f.scribe >= 0.0) {
        return Err(Error::Invalid(format!("filler {} needs a width", f.id)));
    }
    let ft = c.standard.front_thickness;
    let (z0, z1) = front_z(c);
    // In the plane of the doors; the scribe allowance is cut off on site.
    let bounds = Aabb::new(
        Vec3::new(at.x, at.y - ft, at.z + z0),
        Vec3::new(f.width + f.scribe, ft, z1 - z0),
    );
    Ok(single(
        &f.id,
        PartRole::Filler,
        bounds,
        Axis::Y,
        Axis::Z,
        &c.standard.front_board,
    ))
}

fn end_panel(e: &EndPanel, c: &Cabinet, at: Vec3) -> PartList {
    let ft = c.standard.front_thickness;
    let (z0, z1) = front_z(c);
    let z0 = if e.to_floor { c.origin.z } else { z0 };
    // From the face of the doors back to the wall, plus the scribe.
    let bounds = Aabb::new(
        Vec3::new(at.x, at.y - ft, at.z + z0),
        Vec3::new(ft, c.depth + ft + e.scribe, z1 - z0),
    );
    single(
        &e.id,
        PartRole::EndPanel,
        bounds,
        Axis::X,
        Axis::Z,
        &c.standard.front_board,
    )
}

fn single(
    id: &str,
    role: PartRole,
    bounds: Aabb,
    thickness: Axis,
    grain: Axis,
    board: &str,
) -> PartList {
    PartList {
        cabinet: id.to_string(),
        parts: vec![Part {
            name: format!("{id}.{}", role.code()),
            role,
            bounds,
            thickness_axis: thickness,
            grain_axis: Some(grain),
            board: board.to_string(),
            edges: EdgeBanding {
                front: true,
                top: true,
                ..EdgeBanding::default()
            },
        }],
        ..PartList::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::CabinetKind;

    fn end(id: &str) -> RunItem {
        RunItem::EndPanel(EndPanel {
            id: id.into(),
            scribe: 20.0,
            to_floor: true,
        })
    }

    #[test]
    fn a_run_with_an_end_panel_each_side_builds_in_order() {
        let run = Run {
            origin: Vec3::default(),
            items: vec![
                end("E01"),
                RunItem::Cabinet(Box::new(Cabinet::new("B01", CabinetKind::Base))),
                RunItem::Cabinet(Box::new(Cabinet::new("B02", CabinetKind::Base))),
                end("E02"),
            ],
        };
        let lists = build_run(&run).unwrap();
        assert_eq!(lists.len(), 4);
        let base = Cabinet::new("B01", CabinetKind::Base);
        let ft = base.standard.front_thickness;
        let left = &lists[0].parts[0];
        assert_eq!(left.name, "E01.EndPanel");
        assert!(left.role.is_front());
        assert_eq!(
            left.bounds.min,
            Vec3::new(0.0, -ft, 0.0),
            "to the floor, flush with the doors"
        );
        assert_eq!(
            left.bounds.size,
            Vec3::new(ft, base.depth + ft + 20.0, base.height)
        );
        // The cabinets follow the panel, side by side.
        let b1 = lists[1].part("B01.LeftSide").unwrap();
        assert_eq!(b1.bounds.min.x, ft);
        let b2 = lists[2].part("B02.LeftSide").unwrap();
        assert_eq!(b2.bounds.min.x, ft + base.width);
        let right = &lists[3].parts[0];
        assert_eq!(right.bounds.min.x, ft + 2.0 * base.width);
    }

    #[test]
    fn a_filler_sits_in_the_door_plane_and_carries_its_scribe() {
        let run = Run {
            origin: Vec3::default(),
            items: vec![
                RunItem::Filler(Filler {
                    id: "F01".into(),
                    width: 50.0,
                    scribe: 15.0,
                }),
                RunItem::Cabinet(Box::new(Cabinet::new("B01", CabinetKind::Base))),
            ],
        };
        let lists = build_run(&run).unwrap();
        let f = &lists[0].parts[0];
        let base = Cabinet::new("B01", CabinetKind::Base);
        assert_eq!(f.name, "F01.Filler");
        assert_eq!(f.bounds.size.x, 65.0, "width + scribe");
        assert_eq!(
            f.bounds.min.z, base.standard.toe_kick_height,
            "above the toe kick, like the doors"
        );
        // The scribe is cut away on site: the cabinet starts after the finished width.
        assert_eq!(lists[1].part("B01.LeftSide").unwrap().bounds.min.x, 50.0);
    }

    #[test]
    fn a_run_without_cabinets_is_refused() {
        let run = Run {
            origin: Vec3::default(),
            items: vec![end("E01")],
        };
        assert!(build_run(&run).is_err());
    }
}
