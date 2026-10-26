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
    /// A benchtop over every stretch of base-height cabinets in the run. It takes no run length.
    Benchtop(Benchtop),
    /// An open shelf on concealed brackets, in a gap between wall cabinets.
    FloatingShelf(FloatingShelf),
}

/// A floating shelf: `width` of the run, its underside `height` above the floor.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FloatingShelf {
    pub id: String,
    pub width: f64,
    #[serde(default = "FloatingShelf::default_depth")]
    pub depth: f64,
    #[serde(default = "FloatingShelf::default_thickness")]
    pub thickness: f64,
    #[serde(default = "FloatingShelf::default_height")]
    pub height: f64,
    /// An LED strip under its front edge (24 V, at this power per metre; 0: none).
    #[serde(default)]
    pub light_watts_per_metre: f64,
    /// Catalog board id (a timber), or empty for generic.
    #[serde(default)]
    pub board: String,
}

impl FloatingShelf {
    /// Our defaults: a 250 deep, 40 thick shelf, its underside at 1500.
    fn default_depth() -> f64 {
        250.0
    }

    fn default_thickness() -> f64 {
        40.0
    }

    fn default_height() -> f64 {
        1500.0
    }
}

/// Our choice: a floating shelf has a concealed bracket at most this far apart (at least two).
const SHELF_BRACKET_SPACING: f64 = 400.0;

/// A benchtop: one piece over each stretch of base cabinets (and the fillers and end panels
/// between and beside them), overhanging the door faces and, if asked, the stretch's ends.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Benchtop {
    pub id: String,
    #[serde(default = "Benchtop::default_thickness")]
    pub thickness: f64,
    /// Past the door faces.
    #[serde(default = "Benchtop::default_front_overhang")]
    pub front_overhang: f64,
    /// Past each end of a stretch.
    #[serde(default)]
    pub side_overhang: f64,
    /// Short of the wall at the back.
    #[serde(default)]
    pub back_clearance: f64,
    /// Catalog board id, or empty for generic.
    #[serde(default)]
    pub board: String,
    /// The profile of the front edge (and ends, where they show).
    #[serde(default)]
    pub edge: EdgeProfile,
    /// Holes through the top for a sink or a cooktop, each centred over a cabinet.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cutouts: Vec<Cutout>,
    /// Past the cabinets' backs (an island's seating side, a breakfast bar); with it there is
    /// no back clearance or splashback.
    #[serde(default)]
    pub back_overhang: f64,
    /// The top turns down to the floor at the left / right end of each stretch, mitred at the
    /// corner.
    #[serde(default)]
    pub waterfall_left: bool,
    #[serde(default)]
    pub waterfall_right: bool,
    /// A slab splashback this high standing on each piece against the wall (0: none), in the
    /// benchtop's board.
    #[serde(default)]
    pub splashback: f64,
    #[serde(default = "Benchtop::default_splashback_thickness")]
    pub splashback_thickness: f64,
    /// Recesses through the splashback into the wall, each over a cabinet, lined in the
    /// splashback's board.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub niches: Vec<Niche>,
    /// Cut-outs in the splashback for double power points, each centred over a cabinet.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub power_points: Vec<PowerPoint>,
}

/// An open recess in the splashback, centred over a cabinet: an opening through the slab, lined
/// (top, bottom, sides, back) and with shelves, set into the wall cavity.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Niche {
    /// The cabinet's id.
    pub over: String,
    /// The opening's outside size; 0 wide: the cabinet's width less a margin each side.
    #[serde(default)]
    pub width: f64,
    #[serde(default = "Niche::default_height")]
    pub height: f64,
    /// From the splashback's face to the back of the lining.
    #[serde(default = "Niche::default_depth")]
    pub depth: f64,
    /// From the benchtop to the opening.
    #[serde(default = "Niche::default_sill")]
    pub sill: f64,
    #[serde(default)]
    pub shelves: u32,
}

impl Niche {
    /// Our defaults: 400 high, 150 above the top, 110 deep (a 90 mm stud wall behind a 20 mm
    /// slab).
    fn default_height() -> f64 {
        400.0
    }

    fn default_depth() -> f64 {
        110.0
    }

    fn default_sill() -> f64 {
        150.0
    }
}

/// A double power point's cut-out in the splashback.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PowerPoint {
    /// The cabinet's id.
    pub over: String,
    /// The cut-out's centre above the benchtop.
    #[serde(default = "PowerPoint::default_height")]
    pub height: f64,
}

impl PowerPoint {
    /// Our default: centred 150 above the top.
    fn default_height() -> f64 {
        150.0
    }
}

/// Our choices, VERIFY with the electrician: the cut-out for a double power point's mounting
/// block (it is covered by the plate, 116 × 73), and the niche's margin to its cabinet's sides.
const POWER_POINT_CUTOUT: (f64, f64) = (100.0, 60.0);
const NICHE_MARGIN: f64 = 50.0;

/// A benchtop's edge.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum EdgeProfile {
    #[default]
    Square,
    PencilRound,
    Bullnose,
    Bevel,
    /// A strip mitred under the front edge so the top looks `depth` thick.
    MitredApron {
        depth: f64,
    },
}

impl EdgeProfile {
    fn label(self) -> String {
        match self {
            EdgeProfile::Square => "square".into(),
            EdgeProfile::PencilRound => "pencil round".into(),
            EdgeProfile::Bullnose => "bullnose".into(),
            EdgeProfile::Bevel => "bevel".into(),
            EdgeProfile::MitredApron { depth } => format!("mitred apron, {depth:.0} mm deep"),
        }
    }
}

/// A hole through a benchtop, centred across the cabinet it sits over.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Cutout {
    /// The cabinet's id.
    pub over: String,
    pub kind: CutoutKind,
    /// Along the run, and front to back.
    pub width: f64,
    pub depth: f64,
    /// From the benchtop's front edge to the cut-out.
    #[serde(default = "Cutout::default_setback")]
    pub setback: f64,
}

impl Cutout {
    /// Our default: 70 mm of top in front of a sink or cooktop.
    fn default_setback() -> f64 {
        70.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CutoutKind {
    /// A sink, with a tap hole behind it.
    Sink,
    Cooktop,
}

/// Our choices: overhangs up to this need no support, and brackets go at most this far apart.
const BAR_OVERHANG_UNSUPPORTED: f64 = 300.0;
const BAR_BRACKET_SPACING: f64 = 600.0;

/// Our choices: a tap hole's diameter and how far behind the sink cut-out its centre sits.
const TAP_HOLE_DIAMETER: f64 = 35.0;
const TAP_HOLE_BEHIND: f64 = 40.0;

impl Benchtop {
    /// Our defaults: a 33 mm laminate top 20 mm past the doors.
    fn default_thickness() -> f64 {
        33.0
    }

    fn default_front_overhang() -> f64 {
        20.0
    }

    /// Our default: a 20 mm slab.
    fn default_splashback_thickness() -> f64 {
        20.0
    }
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
    let cabinets = cabinets(run);
    if cabinets.is_empty() {
        return Err(Error::Invalid("a run needs at least one cabinet".into()));
    }
    let neighbour = |i: usize| nearest(&cabinets, i);
    let mut out = Vec::new();
    for (i, (item, at)) in run.items.iter().zip(lefts(run, &neighbour)).enumerate() {
        match item {
            RunItem::Cabinet(c) => out.push(build_cabinet(&placed(c, at))?),
            RunItem::Filler(f) => out.push(filler(f, neighbour(i), at)?),
            RunItem::EndPanel(e) => out.push(end_panel(e, neighbour(i), at)),
            // Built from the rest, below.
            RunItem::Benchtop(b) => out.push(PartList {
                cabinet: b.id.clone(),
                ..PartList::default()
            }),
            RunItem::FloatingShelf(f) => out.push(floating_shelf(f, at, neighbour(i).depth)),
        }
    }
    for (i, item) in run.items.iter().enumerate() {
        if let RunItem::Benchtop(b) = item {
            let under: Vec<bool> = run
                .items
                .iter()
                .enumerate()
                .map(|(j, it)| match it {
                    RunItem::Cabinet(c) => takes_benchtop(c),
                    RunItem::Filler(_) | RunItem::EndPanel(_) => takes_benchtop(neighbour(j)),
                    RunItem::Benchtop(_) | RunItem::FloatingShelf(_) => false,
                })
                .collect();
            let mut top = benchtop(b, &out, &under);
            cutouts(b, &mut top, &out);
            splashback_openings(b, &mut top, &out);
            out[i] = top;
        }
    }
    Ok(out)
}

/// The run with each cabinet's origin where the run puts it, as [`build_run`] builds it.
pub fn laid_out(run: &Run) -> Run {
    let cabinets: Vec<(usize, &Cabinet)> = cabinets(run);
    let items = if cabinets.is_empty() {
        run.items.clone()
    } else {
        let neighbour = |i: usize| nearest(&cabinets, i);
        run.items
            .iter()
            .zip(lefts(run, &neighbour))
            .map(|(item, at)| match item {
                RunItem::Cabinet(c) => RunItem::Cabinet(Box::new(placed(c, at))),
                other => other.clone(),
            })
            .collect()
    };
    Run {
        origin: run.origin,
        items,
    }
}

fn cabinets(run: &Run) -> Vec<(usize, &Cabinet)> {
    run.items
        .iter()
        .enumerate()
        .filter_map(|(i, item)| match item {
            RunItem::Cabinet(c) => Some((i, c.as_ref())),
            _ => None,
        })
        .collect()
}

/// The cabinet a filler or end panel at `i` belongs against: the nearest one, left first.
fn nearest<'a>(cabinets: &[(usize, &'a Cabinet)], i: usize) -> &'a Cabinet {
    cabinets
        .iter()
        .min_by_key(|(j, _)| (i.abs_diff(*j), *j > i))
        .map(|(_, c)| *c)
        .expect("a run with a cabinet")
}

/// Where each item's left end sits, side by side from the run's origin.
fn lefts<'a>(run: &Run, neighbour: &dyn Fn(usize) -> &'a Cabinet) -> Vec<Vec3> {
    let mut x = run.origin.x;
    run.items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let at = Vec3::new(x, run.origin.y, run.origin.z);
            x += match item {
                RunItem::Cabinet(c) => c.width,
                RunItem::Filler(f) => f.width,
                RunItem::EndPanel(_) => neighbour(i).standard.front_thickness,
                RunItem::Benchtop(_) => 0.0,
                RunItem::FloatingShelf(f) => f.width,
            };
            at
        })
        .collect()
}

fn placed(c: &Cabinet, at: Vec3) -> Cabinet {
    let mut c = c.clone();
    c.origin = Vec3::new(at.x, at.y, at.z + c.origin.z);
    c
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

/// Base-height cabinets carry a benchtop.
fn takes_benchtop(c: &Cabinet) -> bool {
    use crate::model::CabinetKind as K;
    matches!(
        c.kind,
        K::Base | K::DrawerBank | K::Sink | K::BlindCorner | K::CornerBase
    )
}

/// One benchtop part per stretch of consecutive items `under` marks, from their built parts:
/// across their width (plus the side overhangs), from the door faces less the front overhang to
/// the back less the clearance, on top.
fn benchtop(b: &Benchtop, lists: &[PartList], under: &[bool]) -> PartList {
    let mut out = PartList {
        cabinet: b.id.clone(),
        ..PartList::default()
    };
    let mut stretches: Vec<Vec<&PartList>> = Vec::new();
    let mut open = false;
    for (list, &u) in lists.iter().zip(under) {
        if u {
            if !open {
                stretches.push(Vec::new());
            }
            stretches.last_mut().unwrap().push(list);
        }
        open = u;
    }
    if stretches.is_empty() {
        out.warnings
            .push("no base cabinets in the run to put the benchtop on".into());
    }
    out.notes.push(format!("Edge profile: {}", b.edge.label()));
    for (n, stretch) in stretches.iter().enumerate() {
        let parts: Vec<&Part> = stretch.iter().flat_map(|l| &l.parts).collect();
        // An L-corner's second leg comes forward of the doors: it gets a piece of its own.
        let in_line: Vec<&&Part> = parts.iter().filter(|p| p.bounds.min.y >= -100.0).collect();
        let leg: Vec<&&Part> = parts.iter().filter(|p| p.bounds.min.y < -100.0).collect();
        let lo = |f: fn(&Part) -> f64| in_line.iter().map(|p| f(p)).fold(f64::MAX, f64::min);
        let hi = |f: fn(&Part) -> f64| in_line.iter().map(|p| f(p)).fold(f64::MIN, f64::max);
        let x0 = lo(|p| p.bounds.min.x) - b.side_overhang;
        let x1 = hi(|p| p.bounds.max().x) + b.side_overhang;
        let y0 = lo(|p| p.bounds.min.y) - b.front_overhang;
        // The cabinets' backs: end panels and fillers carry a scribe allowance.
        let y1 = in_line
            .iter()
            .filter(|p| !matches!(p.role, PartRole::EndPanel | PartRole::Filler))
            .map(|p| p.bounds.max().y)
            .fold(f64::MIN, f64::max)
            + if b.back_overhang > 0.0 {
                b.back_overhang
            } else {
                -b.back_clearance
            };
        let z = hi(|p| p.bounds.max().z);
        out.parts.push(Part {
            name: format!("{}.{}{}", b.id, PartRole::Benchtop.code(), n + 1),
            role: PartRole::Benchtop,
            bounds: Aabb::new(
                Vec3::new(x0, y0, z),
                Vec3::new(x1 - x0, y1 - y0, b.thickness),
            ),
            thickness_axis: Axis::Z,
            grain_axis: Some(Axis::X),
            board: b.board.clone(),
            edges: EdgeBanding {
                front: true,
                left: b.side_overhang > 0.0,
                right: b.side_overhang > 0.0,
                ..EdgeBanding::default()
            },
        });
        // The leg's piece butts against the main top's front edge (the main top runs through the
        // corner); its front, facing along X, overhangs like the run's.
        if !leg.is_empty() {
            let lx0 = leg.iter().map(|p| p.bounds.min.x).fold(f64::MAX, f64::min);
            let lx1 = leg
                .iter()
                .map(|p| p.bounds.max().x)
                .fold(f64::MIN, f64::max);
            let ly0 = leg.iter().map(|p| p.bounds.min.y).fold(f64::MAX, f64::min);
            // A leg on the run's left faces +X, on its right −X.
            let faces_plus_x = (lx0 + lx1) / 2.0 < (x0 + x1) / 2.0;
            let (bx0, bx1) = if faces_plus_x {
                (lx0, lx1 + b.front_overhang)
            } else {
                (lx0 - b.front_overhang, lx1)
            };
            out.parts.push(Part {
                name: format!("{}.{}{}Leg", b.id, PartRole::Benchtop.code(), n + 1),
                role: PartRole::Benchtop,
                bounds: Aabb::new(
                    Vec3::new(bx0, ly0 - b.side_overhang, z),
                    Vec3::new(bx1 - bx0, y0 - (ly0 - b.side_overhang), b.thickness),
                ),
                thickness_axis: Axis::Z,
                grain_axis: Some(Axis::Y),
                board: b.board.clone(),
                edges: EdgeBanding {
                    right: faces_plus_x,
                    left: !faces_plus_x,
                    front: b.side_overhang > 0.0,
                    ..EdgeBanding::default()
                },
            });
        }
        // A mitred apron under the front edge, the top's thickness deep, down to the apron's
        // depth below the top's upper face.
        if let EdgeProfile::MitredApron { depth } = b.edge {
            let drop = depth - b.thickness;
            if drop > 0.0 && b.thickness > b.front_overhang {
                out.warnings.push(format!(
                    "the apron reaches back past the door faces: give the top at least {:.0} mm \
                     front overhang, or shorten the fronts by {drop:.0} mm",
                    b.thickness
                ));
            }
            if drop > 0.0 {
                out.parts.push(Part {
                    name: format!("{}.Apron{}", b.id, n + 1),
                    role: PartRole::Benchtop,
                    bounds: Aabb::new(
                        Vec3::new(x0, y0, z - drop),
                        Vec3::new(x1 - x0, b.thickness, drop),
                    ),
                    thickness_axis: Axis::Y,
                    grain_axis: Some(Axis::X),
                    board: b.board.clone(),
                    edges: EdgeBanding::default(),
                });
            } else {
                out.warnings.push(format!(
                    "a {depth:.0} mm apron is no deeper than the {:.0} mm top",
                    b.thickness
                ));
            }
        }
        // Waterfall ends: from the floor up the stretch's ends to the top's upper face, the
        // depth of the top. Mitred with the top: both pieces are cut to their outer faces.
        let floor = in_line
            .iter()
            .map(|p| p.bounds.min.z)
            .fold(f64::MAX, f64::min);
        for (on, side, wx0) in [
            (b.waterfall_left, "L", x0 - b.thickness),
            (b.waterfall_right, "R", x1),
        ] {
            if on {
                out.parts.push(Part {
                    name: format!("{}.Waterfall{}{side}", b.id, n + 1),
                    role: PartRole::Benchtop,
                    bounds: Aabb::new(
                        Vec3::new(wx0, y0, floor),
                        Vec3::new(b.thickness, y1 - y0, z + b.thickness - floor),
                    ),
                    thickness_axis: Axis::X,
                    grain_axis: Some(Axis::Z),
                    board: b.board.clone(),
                    edges: EdgeBanding {
                        front: true,
                        ..EdgeBanding::default()
                    },
                });
            }
        }
        // Our rule of thumb: an overhang past this needs brackets under it, one per this spacing.
        if b.back_overhang > BAR_OVERHANG_UNSUPPORTED {
            let brackets = ((x1 - x0) / BAR_BRACKET_SPACING).ceil().max(2.0) as u32;
            out.hardware.push(crate::part::HardwarePlacement {
                item: "benchtop.support-bracket".into(),
                description: format!(
                    "Concealed benchtop support bracket for a {:.0} mm overhang",
                    b.back_overhang
                ),
                at: Vec3::new((x0 + x1) / 2.0, y1, z),
                part: format!("{}.{}{}", b.id, PartRole::Benchtop.code(), n + 1),
                quantity: brackets,
            });
        }
        if b.splashback > 0.0 && b.back_overhang <= 0.0 {
            let t = b.splashback_thickness;
            out.parts.push(Part {
                name: format!("{}.{}{}", b.id, PartRole::Splashback.code(), n + 1),
                role: PartRole::Splashback,
                bounds: Aabb::new(
                    Vec3::new(x0, y1 - t, z + b.thickness),
                    Vec3::new(x1 - x0, t, b.splashback),
                ),
                thickness_axis: Axis::Y,
                grain_axis: Some(Axis::X),
                board: b.board.clone(),
                edges: EdgeBanding {
                    top: true,
                    ..EdgeBanding::default()
                },
            });
        }
    }
    out
}

/// Cuts each cut-out through the benchtop piece over its cabinet, and a tap hole behind a sink.
fn cutouts(b: &Benchtop, top: &mut PartList, lists: &[PartList]) {
    use crate::part::{BorePurpose, Dir, Operation};
    for c in &b.cutouts {
        let Some(cab) = lists.iter().find(|l| l.cabinet == c.over) else {
            top.warnings.push(format!(
                "{:?} cut-out: no cabinet {} in the run",
                c.kind, c.over
            ));
            continue;
        };
        let (x0, x1) = span_x(cab);
        let mid = (x0 + x1) / 2.0;
        let Some(piece) = top
            .parts
            .iter()
            .find(|p| p.bounds.min.x <= mid && mid <= p.bounds.max().x)
            .cloned()
        else {
            top.warnings
                .push(format!("{:?} cut-out: no benchtop over {}", c.kind, c.over));
            continue;
        };
        let pb = piece.bounds;
        let y0 = pb.min.y + c.setback;
        let bounds = Aabb::new(
            Vec3::new(mid - c.width / 2.0, y0, pb.min.z),
            Vec3::new(c.width, c.depth, pb.size.z),
        );
        if bounds.min.x < pb.min.x || bounds.max().x > pb.max().x || bounds.max().y > pb.max().y {
            top.warnings.push(format!(
                "{:?} cut-out over {} does not fit the benchtop",
                c.kind, c.over
            ));
            continue;
        }
        top.operations.push(Operation::Groove {
            part: piece.name.clone(),
            bounds,
        });
        if c.kind == CutoutKind::Sink {
            let y = bounds.max().y + TAP_HOLE_BEHIND;
            if y + TAP_HOLE_DIAMETER / 2.0 < pb.max().y {
                top.operations.push(Operation::Bore {
                    part: piece.name.clone(),
                    at: Vec3::new(mid, y, pb.max().z),
                    direction: Dir::new(Axis::Z, false),
                    diameter: TAP_HOLE_DIAMETER,
                    depth: pb.size.z,
                    purpose: BorePurpose::TapHole,
                });
            } else {
                top.warnings.push(format!(
                    "no room behind the sink over {} for a tap hole",
                    c.over
                ));
            }
        }
    }
}

/// From the left to the right of a cabinet's parts.
fn span_x(cab: &PartList) -> (f64, f64) {
    let x0 = cab
        .parts
        .iter()
        .map(|p| p.bounds.min.x)
        .fold(f64::MAX, f64::min);
    let x1 = cab
        .parts
        .iter()
        .map(|p| p.bounds.max().x)
        .fold(f64::MIN, f64::max);
    (x0, x1)
}

/// Cuts each niche and power point through the splashback over its cabinet, and lines each
/// niche: top and bottom across, sides between them, a back behind them all, shelves inside.
fn splashback_openings(b: &Benchtop, top: &mut PartList, lists: &[PartList]) {
    let t = b.splashback_thickness;
    let openings = b
        .niches
        .iter()
        .map(|n| (&n.over, Some(n)))
        .chain(b.power_points.iter().map(|p| (&p.over, None)));
    for (k, (over, niche)) in openings.enumerate() {
        let what = if niche.is_some() {
            "niche"
        } else {
            "power point"
        };
        let Some(cab) = lists.iter().find(|l| &l.cabinet == over) else {
            top.warnings
                .push(format!("{what}: no cabinet {over} in the run"));
            continue;
        };
        let (x0, x1) = span_x(cab);
        let mid = (x0 + x1) / 2.0;
        let Some(back) = top
            .parts
            .iter()
            .find(|p| {
                p.role == PartRole::Splashback && p.bounds.min.x <= mid && mid <= p.bounds.max().x
            })
            .cloned()
        else {
            top.warnings
                .push(format!("{what} over {over}: no splashback there"));
            continue;
        };
        let sb = back.bounds;
        let (w, h, z) = match niche {
            Some(n) => {
                let w = if n.width > 0.0 {
                    n.width
                } else {
                    x1 - x0 - 2.0 * NICHE_MARGIN
                };
                (w, n.height, sb.min.z + n.sill)
            }
            None => {
                let p = &b.power_points[k - b.niches.len()];
                let (w, h) = POWER_POINT_CUTOUT;
                (w, h, sb.min.z + p.height - h / 2.0)
            }
        };
        let hole = Aabb::new(
            Vec3::new(mid - w / 2.0, sb.min.y, z),
            Vec3::new(w, sb.size.y, h),
        );
        if hole.min.x < sb.min.x || hole.max().x > sb.max().x || hole.max().z > sb.max().z {
            top.warnings
                .push(format!("{what} over {over} does not fit the splashback"));
            continue;
        }
        top.operations.push(crate::part::Operation::Groove {
            part: back.name.clone(),
            bounds: hole,
        });
        let Some(n) = niche else { continue };
        if n.depth <= 2.0 * t || w <= 2.0 * t || h <= 2.0 * t {
            top.warnings
                .push(format!("niche over {over} is too small to line"));
            continue;
        }
        // The linings start at the slab's face; the back closes them at the niche's depth.
        let (y0, deep) = (sb.min.y, n.depth - t);
        let name = |what: &str| format!("{}.Niche{}{what}", b.id, k + 1);
        let lining = |name: String, min: Vec3, size: Vec3, axis: Axis, grain: Axis| Part {
            name,
            role: PartRole::Splashback,
            bounds: Aabb::new(min, size),
            thickness_axis: axis,
            grain_axis: Some(grain),
            board: b.board.clone(),
            edges: EdgeBanding {
                front: axis != Axis::Y,
                ..EdgeBanding::default()
            },
        };
        let (hx, hz) = (hole.min.x, hole.min.z);
        top.parts.extend([
            lining(
                name("Bottom"),
                Vec3::new(hx, y0, hz),
                Vec3::new(w, deep, t),
                Axis::Z,
                Axis::X,
            ),
            lining(
                name("Top"),
                Vec3::new(hx, y0, hz + h - t),
                Vec3::new(w, deep, t),
                Axis::Z,
                Axis::X,
            ),
            lining(
                name("SideL"),
                Vec3::new(hx, y0, hz + t),
                Vec3::new(t, deep, h - 2.0 * t),
                Axis::X,
                Axis::Z,
            ),
            lining(
                name("SideR"),
                Vec3::new(hx + w - t, y0, hz + t),
                Vec3::new(t, deep, h - 2.0 * t),
                Axis::X,
                Axis::Z,
            ),
            lining(
                name("Back"),
                Vec3::new(hx, y0 + deep, hz),
                Vec3::new(w, t, h),
                Axis::Y,
                Axis::X,
            ),
        ]);
        // Shelves share the clear height evenly.
        let clear = h - 2.0 * t;
        let gap = (clear - n.shelves as f64 * t) / (n.shelves + 1) as f64;
        if n.shelves > 0 && gap < 60.0 {
            top.warnings.push(format!(
                "niche over {over}: {} shelves leave {gap:.0} mm between them",
                n.shelves
            ));
        }
        for s in 0..n.shelves {
            let sz = hz + t + gap * (s + 1) as f64 + t * s as f64;
            top.parts.push(lining(
                name(&format!("Shelf{}", s + 1)),
                Vec3::new(hx + t, y0, sz),
                Vec3::new(w - 2.0 * t, deep, t),
                Axis::Z,
                Axis::X,
            ));
        }
    }
}

/// A floating shelf against the wall (the wall line is where its neighbouring cabinet's back is),
/// its brackets in the hardware list, and its strip.
fn floating_shelf(f: &FloatingShelf, at: Vec3, wall: f64) -> PartList {
    let back = at.y + wall;
    let bounds = Aabb::new(
        Vec3::new(at.x, back - f.depth, at.z + f.height),
        Vec3::new(f.width, f.depth, f.thickness),
    );
    let name = format!("{}.FloatingShelf", f.id);
    let mut list = PartList {
        cabinet: f.id.clone(),
        parts: vec![Part {
            name: name.clone(),
            role: PartRole::Shelf,
            bounds,
            thickness_axis: Axis::Z,
            grain_axis: Some(Axis::X),
            board: f.board.clone(),
            edges: EdgeBanding {
                front: true,
                left: true,
                right: true,
                ..EdgeBanding::default()
            },
        }],
        ..PartList::default()
    };
    let brackets = ((f.width / SHELF_BRACKET_SPACING).ceil() as u32).max(2);
    list.hardware.push(crate::part::HardwarePlacement {
        item: "shelf.floating-bracket".into(),
        description: format!(
            "Concealed floating shelf bracket for a {:.0} mm thick shelf",
            f.thickness
        ),
        at: bounds.center(),
        part: name.clone(),
        quantity: brackets,
    });
    if f.light_watts_per_metre > 0.0 {
        let length = f.width - 20.0;
        list.hardware.push(crate::part::HardwarePlacement {
            item: "lighting.strip.24v".into(),
            description: format!(
                "LED strip 24 V, {length:.0} mm, {:.1} W ({} W/m)",
                f.light_watts_per_metre * length / 1000.0,
                f.light_watts_per_metre
            ),
            at: bounds.center(),
            part: name,
            quantity: 1,
        });
    }
    list
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
    fn laying_out_a_run_gives_each_cabinet_the_origin_it_is_built_at() {
        let run = Run {
            origin: Vec3::default(),
            items: vec![
                end("E01"),
                RunItem::Cabinet(Box::new(Cabinet::new("B01", CabinetKind::Base))),
                RunItem::Cabinet(Box::new(Cabinet::new("B02", CabinetKind::Base))),
            ],
        };
        let lists = build_run(&run).unwrap();
        for (item, list) in laid_out(&run).items.iter().zip(&lists).skip(1) {
            let RunItem::Cabinet(c) = item else {
                panic!("a cabinet")
            };
            assert_eq!(build_cabinet(c).unwrap(), *list);
        }
    }

    #[test]
    fn a_run_without_cabinets_is_refused() {
        let run = Run {
            origin: Vec3::default(),
            items: vec![end("E01")],
        };
        assert!(build_run(&run).is_err());
    }

    fn top(id: &str, side: f64) -> RunItem {
        RunItem::Benchtop(Benchtop {
            id: id.into(),
            thickness: 33.0,
            front_overhang: 20.0,
            side_overhang: side,
            back_clearance: 0.0,
            board: String::new(),
            edge: EdgeProfile::Square,
            cutouts: Vec::new(),
            back_overhang: 0.0,
            waterfall_left: false,
            waterfall_right: false,
            splashback: 0.0,
            splashback_thickness: 20.0,
            niches: Vec::new(),
            power_points: Vec::new(),
        })
    }

    fn cab(id: &str, kind: CabinetKind) -> RunItem {
        RunItem::Cabinet(Box::new(Cabinet::new(id, kind)))
    }

    /// BENCH-1: one piece over each stretch of base cabinets (with their end panels), 20 past
    /// the doors, on top, taking no run length; a tall cabinet splits the stretches.
    #[test]
    fn a_benchtop_covers_each_stretch_of_base_cabinets() {
        let run = Run {
            origin: Vec3::default(),
            items: vec![
                end("E01"),
                cab("B01", CabinetKind::Base),
                cab("D01", CabinetKind::DrawerBank),
                cab("T01", CabinetKind::Tall),
                cab("B02", CabinetKind::Sink),
                top("BT1", 10.0),
            ],
        };
        let lists = build_run(&run).unwrap();
        assert_eq!(lists.len(), 6);
        let bt = &lists[5];
        assert!(bt.warnings.is_empty(), "{:?}", bt.warnings);
        assert_eq!(bt.parts.len(), 2);
        let (a, b) = (&bt.parts[0].bounds, &bt.parts[1].bounds);
        let ft = 18.0;
        // E01 (18) + B01 + D01, plus 10 each end.
        assert!((a.min.x - (-10.0)).abs() < 1e-9);
        assert!(
            (a.size.x - (ft + 1200.0 + 20.0)).abs() < 1e-9,
            "{}",
            a.size.x
        );
        assert!((a.min.y - (-ft - 20.0)).abs() < 1e-9);
        assert!((a.max().y - 580.0).abs() < 1e-9);
        assert!((a.min.z - 870.0).abs() < 1e-9 && (a.size.z - 33.0).abs() < 1e-9);
        // The sink after the tall cabinet.
        let sink_x = ft + 1200.0 + 600.0;
        assert!((b.min.x - (sink_x - 10.0)).abs() < 1e-9);
        // The run's length is unchanged by the benchtop.
        let laid = laid_out(&run);
        let RunItem::Cabinet(sink) = &laid.items[4] else {
            panic!()
        };
        assert!((sink.origin.x - sink_x).abs() < 1e-9);
        assert!(bt.parts.iter().all(|p| p.role == PartRole::Benchtop));
    }

    #[test]
    fn a_benchtop_with_no_base_cabinets_says_so() {
        let run = Run {
            origin: Vec3::default(),
            items: vec![cab("W01", CabinetKind::Wall), top("BT1", 0.0)],
        };
        let lists = build_run(&run).unwrap();
        assert!(lists[1].parts.is_empty());
        assert!(lists[1].warnings[0].contains("no base cabinets"));
    }

    /// BENCH-4: a sink cut-out over the sink cabinet with a tap hole behind it, a cooktop over a
    /// base cabinet; both through the top.
    #[test]
    fn cut_outs_go_through_the_top_over_their_cabinets() {
        let mut bt = Benchtop {
            id: "BT1".into(),
            thickness: 33.0,
            front_overhang: 20.0,
            side_overhang: 0.0,
            back_clearance: 0.0,
            board: String::new(),
            edge: EdgeProfile::Square,
            back_overhang: 0.0,
            waterfall_left: false,
            waterfall_right: false,
            splashback: 600.0,
            splashback_thickness: 20.0,
            niches: Vec::new(),
            power_points: Vec::new(),
            cutouts: vec![
                Cutout {
                    over: "S01".into(),
                    kind: CutoutKind::Sink,
                    width: 760.0,
                    depth: 440.0,
                    setback: 70.0,
                },
                Cutout {
                    over: "B01".into(),
                    kind: CutoutKind::Cooktop,
                    width: 560.0,
                    depth: 490.0,
                    setback: 70.0,
                },
            ],
        };
        let run = |bt: &Benchtop| Run {
            origin: Vec3::default(),
            items: vec![
                cab("B01", CabinetKind::Base),
                cab("S01", CabinetKind::Sink),
                RunItem::Benchtop(bt.clone()),
            ],
        };
        let lists = build_run(&run(&bt)).unwrap();
        let top = &lists[2];
        assert!(top.warnings.is_empty(), "{:?}", top.warnings);
        let holes: Vec<_> = top
            .operations
            .iter()
            .filter_map(|o| match o {
                crate::part::Operation::Groove { bounds, .. } => Some(*bounds),
                _ => None,
            })
            .collect();
        assert_eq!(holes.len(), 2);
        // The sink centred on S01 (600 + 900 / 2), through the 33 mm top.
        assert!((holes[0].center().x - 1050.0).abs() < 1e-9);
        assert_eq!(holes[0].size.z, 33.0);
        assert!(top.operations.iter().any(|o| matches!(
            o,
            crate::part::Operation::Bore {
                purpose: crate::part::BorePurpose::TapHole,
                ..
            }
        )));
        assert!(crate::collisions::check(top).is_empty());
        // Too wide for its top.
        bt.cutouts[1].width = 2000.0;
        let lists = build_run(&run(&bt)).unwrap();
        assert!(lists[2].warnings.iter().any(|w| w.contains("does not fit")));
    }

    /// BENCH-6: a splashback stands on each piece at the back, across the same length.
    #[test]
    fn a_splashback_stands_on_the_top_against_the_wall() {
        let mut item = top("BT1", 0.0);
        if let RunItem::Benchtop(b) = &mut item {
            b.splashback = 600.0;
        }
        let run = Run {
            origin: Vec3::default(),
            items: vec![cab("B01", CabinetKind::Base), item],
        };
        let lists = build_run(&run).unwrap();
        let top = lists[1].part("BT1.Benchtop1").unwrap().bounds;
        let back = lists[1].part("BT1.Splashback1").unwrap().bounds;
        assert_eq!(back.min.z, top.max().z);
        assert_eq!(back.max().y, top.max().y);
        assert_eq!(
            (back.size.x, back.size.y, back.size.z),
            (top.size.x, 20.0, 600.0)
        );
    }

    /// BENCH-6: a niche through the splashback over its cabinet, lined and shelved, and a power
    /// point cut-out beside it; one that does not fit says so.
    #[test]
    fn a_niche_is_cut_through_the_splashback_and_lined() {
        let mut item = top("BT1", 0.0);
        if let RunItem::Benchtop(b) = &mut item {
            b.splashback = 600.0;
            b.niches.push(Niche {
                over: "B02".into(),
                width: 0.0,
                height: 400.0,
                depth: 110.0,
                sill: 150.0,
                shelves: 1,
            });
            b.power_points.push(PowerPoint {
                over: "B01".into(),
                height: 150.0,
            });
        }
        let run = |item: &RunItem| Run {
            origin: Vec3::default(),
            items: vec![
                cab("B01", CabinetKind::Base),
                cab("B02", CabinetKind::Base),
                item.clone(),
            ],
        };
        let lists = build_run(&run(&item)).unwrap();
        let bt = &lists[2];
        assert!(bt.warnings.is_empty(), "{:?}", bt.warnings);
        let holes: Vec<_> = bt
            .operations
            .iter()
            .filter_map(|o| match o {
                crate::part::Operation::Groove { part, bounds } => Some((part.as_str(), *bounds)),
                _ => None,
            })
            .collect();
        assert_eq!(holes.len(), 2);
        assert!(holes.iter().all(|(p, _)| *p == "BT1.Splashback1"));
        // Over B02 (600..1200), 50 in from each side, 150 above the 33 mm top at 870.
        let (_, niche) = holes[0];
        assert_eq!((niche.min.x, niche.size.x), (650.0, 500.0));
        assert_eq!((niche.min.z, niche.size.z), (870.0 + 33.0 + 150.0, 400.0));
        let (_, power) = holes[1];
        assert_eq!(power.center().x, 300.0);
        assert_eq!(power.center().z, 870.0 + 33.0 + 150.0);
        let names: Vec<_> = bt.parts.iter().map(|p| p.name.as_str()).collect();
        for n in ["Bottom", "Top", "SideL", "SideR", "Back", "Shelf1"] {
            assert!(
                names.contains(&format!("BT1.Niche1{n}").as_str()),
                "{names:?}"
            );
        }
        // The back closes the lining 110 behind the slab's face; the shelf is halfway up.
        let face = bt.part("BT1.Splashback1").unwrap().bounds.min.y;
        let back = bt.part("BT1.Niche1Back").unwrap().bounds;
        assert_eq!(back.max().y - face, 110.0);
        let shelf = bt.part("BT1.Niche1Shelf1").unwrap().bounds;
        assert_eq!(shelf.center().z, niche.center().z);
        assert!(crate::collisions::check(bt).is_empty());
        // Taller than the splashback.
        if let RunItem::Benchtop(b) = &mut item {
            b.niches[0].height = 600.0;
        }
        let lists = build_run(&run(&item)).unwrap();
        assert!(lists[2].warnings.iter().any(|w| w.contains("does not fit")));
        assert!(lists[2].part("BT1.Niche1Back").is_none());
    }

    /// BENCH-3a/3c: an island top: waterfall ends down to the floor and a 400 mm seating
    /// overhang at the back, which needs brackets; no splashback with an overhang.
    #[test]
    fn an_island_top_has_waterfall_ends_and_a_supported_overhang() {
        let mut item = top("BT1", 0.0);
        if let RunItem::Benchtop(b) = &mut item {
            b.waterfall_left = true;
            b.waterfall_right = true;
            b.back_overhang = 400.0;
            b.splashback = 600.0;
        }
        let run = Run {
            origin: Vec3::default(),
            items: vec![
                cab("B01", CabinetKind::Base),
                cab("B02", CabinetKind::Base),
                item,
            ],
        };
        let lists = build_run(&run).unwrap();
        let bt = &lists[2];
        let top = bt.part("BT1.Benchtop1").unwrap().bounds;
        assert!((top.max().y - 980.0).abs() < 1e-9);
        let (l, r) = (
            bt.part("BT1.Waterfall1L").unwrap().bounds,
            bt.part("BT1.Waterfall1R").unwrap().bounds,
        );
        assert_eq!((l.max().x, r.min.x), (top.min.x, top.max().x));
        assert_eq!((l.min.z, l.max().z), (0.0, top.max().z));
        assert_eq!(l.size.y, top.size.y);
        assert!(bt.part("BT1.Splashback1").is_none());
        let brackets = bt
            .hardware
            .iter()
            .find(|h| h.item == "benchtop.support-bracket")
            .unwrap();
        assert_eq!(brackets.quantity, 2);
    }

    /// BENCH-3b: an L-corner's second leg gets its own piece, butted against the main top's
    /// front edge, overhanging the leg's doors; both hands.
    #[test]
    fn an_l_corner_leg_gets_a_piece_butted_to_the_main_top() {
        for side in [
            crate::model::HingeSide::Left,
            crate::model::HingeSide::Right,
        ] {
            let mut corner = Cabinet::new("C01", CabinetKind::CornerBase);
            corner.corner.side = side;
            let run = Run {
                origin: Vec3::default(),
                items: vec![RunItem::Cabinet(Box::new(corner)), top("BT1", 0.0)],
            };
            let lists = build_run(&run).unwrap();
            let bt = &lists[1];
            assert!(bt.warnings.is_empty(), "{:?}", bt.warnings);
            let main = bt.part("BT1.Benchtop1").unwrap().bounds;
            let leg = bt.part("BT1.Benchtop1Leg").unwrap().bounds;
            assert!((leg.max().y - main.min.y).abs() < 1e-9, "butted");
            assert!(!leg.overlaps(&main));
            // 900 − 580 forward of the main run's front.
            assert!((leg.min.y - (-320.0)).abs() < 1e-9, "{leg:?}");
            // The leg's doors face ±X; the top passes them by the overhang.
            let doors = lists[0].part("C01.Door2").unwrap().bounds;
            match side {
                crate::model::HingeSide::Left => {
                    assert!((leg.max().x - (doors.max().x + 20.0)).abs() < 1e-9)
                }
                crate::model::HingeSide::Right => {
                    assert!((leg.min.x - (doors.min.x - 20.0)).abs() < 1e-9)
                }
            }
        }
    }

    /// BENCH-2: the edge profile is noted for the maker; a mitred apron adds a strip under the
    /// front edge to the look's depth.
    #[test]
    fn a_mitred_apron_hangs_under_the_front_edge() {
        let mut item = top("BT1", 0.0);
        if let RunItem::Benchtop(b) = &mut item {
            b.thickness = 20.0;
            b.edge = EdgeProfile::MitredApron { depth: 60.0 };
        }
        let run = Run {
            origin: Vec3::default(),
            items: vec![cab("B01", CabinetKind::Base), item],
        };
        let lists = build_run(&run).unwrap();
        let bt = &lists[1];
        let top = bt.part("BT1.Benchtop1").unwrap().bounds;
        let apron = bt.part("BT1.Apron1").unwrap().bounds;
        assert_eq!((apron.min.y, apron.size.y), (top.min.y, 20.0));
        assert_eq!((apron.min.z, apron.max().z), (top.min.z - 40.0, top.min.z));
        assert_eq!(bt.notes, ["Edge profile: mitred apron, 60 mm deep"]);
        // 20 mm overhang takes a 20 mm apron; a 33 mm top's apron would reach the doors.
        assert!(bt.warnings.is_empty(), "{:?}", bt.warnings);
    }

    /// STYLE-2: a floating shelf takes its width in the run, sits at its height against the
    /// wall line, with brackets every 400 mm and its strip.
    #[test]
    fn a_floating_shelf_sits_in_the_run_on_brackets() {
        let shelf = RunItem::FloatingShelf(FloatingShelf {
            id: "FS1".into(),
            width: 900.0,
            depth: 250.0,
            thickness: 40.0,
            height: 1500.0,
            light_watts_per_metre: 9.6,
            board: String::new(),
        });
        let run = Run {
            origin: Vec3::default(),
            items: vec![
                cab("W01", CabinetKind::Wall),
                shelf,
                cab("W02", CabinetKind::Wall),
            ],
        };
        let lists = build_run(&run).unwrap();
        let s = lists[1].parts[0].bounds;
        assert_eq!(
            (s.min.x, s.size.x, s.min.z, s.size.z),
            (600.0, 900.0, 1500.0, 40.0)
        );
        assert_eq!(
            s.max().y,
            330.0,
            "against the wall, like the wall cabinets' backs"
        );
        let laid = laid_out(&run);
        let RunItem::Cabinet(w2) = &laid.items[2] else {
            panic!()
        };
        assert_eq!(w2.origin.x, 1500.0);
        let brackets = lists[1]
            .hardware
            .iter()
            .find(|h| h.item == "shelf.floating-bracket")
            .unwrap();
        assert_eq!(brackets.quantity, 3);
        assert!(
            lists[1]
                .hardware
                .iter()
                .any(|h| h.item == "lighting.strip.24v")
        );
    }
}
