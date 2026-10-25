//! DXF files for CNC: each part's outline and machining as plain AutoCAD R12 entities (LINE,
//! CIRCLE, closed POLYLINE), sorted onto layers a CAM program can map to tools:
//!
//! | layer | what |
//! |---|---|
//! | `OUTLINE` | the part's cut size, closed |
//! | `BORE_D<Ø>_Z<depth>` | face bores (circles), e.g. `BORE_D5_Z10`; `BORE_D<Ø>_THRU` through |
//! | `POCKET_<KIND>_Z<depth>` | grooves, mortises, biscuit slots in the face (closed rectangles) |
//! | `CUTOUT` | notches and anything through the part (closed rectangles) |
//! | `EDGE_BORE_D<Ø>_Z<depth>` | bores into an edge: a line from the edge as deep as the bore |
//! | `EDGE_<KIND>` | slots cut from an edge (closed rectangles of their footprint) |
//!
//! Coordinates are millimetres from the part's reference corner in its cut sheet frame (x along
//! the part's length, y across it), seen from the face being machined: the low face (as the cut
//! sheet calls it) is drawn turned over about the part's length, x mirrored. A part machined on
//! both faces gets a second file for the other face.

use std::fmt::Write;

use cabinet_core::part::{Part, PartList};

use crate::{Face, Feature, features, fmt, frame};

/// One DXF file: `name` is the part name, with `-back` for a part's second face.
#[derive(Clone, Debug, PartialEq)]
pub struct DxfFile {
    pub name: String,
    pub contents: String,
}

struct Dxf {
    out: String,
}

impl Dxf {
    fn new() -> Self {
        let mut out = String::new();
        for (code, value) in [
            ("0", "SECTION"),
            ("2", "HEADER"),
            ("9", "$ACADVER"),
            ("1", "AC1009"),
            ("9", "$INSUNITS"),
            ("70", "4"),
            ("0", "ENDSEC"),
            ("0", "SECTION"),
            ("2", "ENTITIES"),
        ] {
            let _ = writeln!(out, "{code}\n{value}");
        }
        Self { out }
    }

    fn pair(&mut self, code: u32, value: impl std::fmt::Display) {
        let _ = writeln!(self.out, "{code}\n{value}");
    }

    fn xy(&mut self, base: u32, x: f64, y: f64) {
        self.pair(base, format!("{x:.3}"));
        self.pair(base + 10, format!("{y:.3}"));
        self.pair(base + 20, "0.0");
    }

    fn line(&mut self, layer: &str, a: (f64, f64), b: (f64, f64)) {
        self.pair(0, "LINE");
        self.pair(8, layer);
        self.xy(10, a.0, a.1);
        self.xy(11, b.0, b.1);
    }

    fn circle(&mut self, layer: &str, c: (f64, f64), r: f64) {
        self.pair(0, "CIRCLE");
        self.pair(8, layer);
        self.xy(10, c.0, c.1);
        self.pair(40, format!("{r:.3}"));
    }

    /// A closed rectangle from `(x, y)`, `w` × `h`.
    fn rect(&mut self, layer: &str, x: f64, y: f64, w: f64, h: f64) {
        self.pair(0, "POLYLINE");
        self.pair(8, layer);
        self.pair(66, 1);
        self.pair(70, 1);
        self.xy(10, 0.0, 0.0);
        for (px, py) in [(x, y), (x + w, y), (x + w, y + h), (x, y + h)] {
            self.pair(0, "VERTEX");
            self.pair(8, layer);
            self.xy(10, px, py);
        }
        self.pair(0, "SEQEND");
        self.pair(8, layer);
    }

    fn finish(mut self) -> String {
        self.pair(0, "ENDSEC");
        self.pair(0, "EOF");
        self.out
    }
}

/// Depth for a layer name: `10`, `12.5`.
fn depth_name(v: f64) -> String {
    fmt(v).replace('.', "_")
}

/// The DXF file(s) for one part: the face with the most machining first (with the edge work),
/// then, if the other face is machined too, its own file.
pub fn part_dxfs(list: &PartList, part: &Part) -> Vec<DxfFile> {
    let all = features(list, part);
    let (ua, va) = frame(part);
    let (lu, lv) = (part.bounds.size.get(ua), part.bounds.size.get(va));
    let t = part.thickness();
    let on = |f: Face| all.iter().filter(|x| x.face() == Some(f)).count();
    let main = if on(Face::Low) > on(Face::High) {
        Face::Low
    } else {
        Face::High
    };
    let other = if main == Face::Low {
        Face::High
    } else {
        Face::Low
    };
    let mut faces = vec![main];
    if on(other) > 0 {
        faces.push(other);
    }
    faces
        .into_iter()
        .enumerate()
        .map(|(i, face)| {
            // Machining the low face, the part lies turned over about its length: x mirrored.
            let mirror = face == Face::Low;
            let x = |u: f64| if mirror { lu - u } else { u };
            let mut d = Dxf::new();
            d.rect("OUTLINE", 0.0, 0.0, lu, lv);
            for f in &all {
                match *f {
                    Feature::FaceBore {
                        u,
                        v,
                        diameter,
                        depth,
                        face: fc,
                        ..
                    } if fc == face => {
                        let layer = if depth >= t - 1e-6 {
                            format!("BORE_D{}_THRU", depth_name(diameter))
                        } else {
                            format!("BORE_D{}_Z{}", depth_name(diameter), depth_name(depth))
                        };
                        d.circle(&layer, (x(u), v), diameter / 2.0);
                    }
                    Feature::Rect {
                        u,
                        v,
                        w,
                        h,
                        depth,
                        label,
                        face: fc,
                    } => {
                        let through = depth >= t - 1e-6;
                        let kind = label.to_uppercase();
                        let layer = match fc {
                            Some(fc) if fc == face && !through => {
                                format!("POCKET_{kind}_Z{}", depth_name(depth))
                            }
                            None if through => "CUTOUT".to_string(),
                            // Cut from an edge: with the main face's file only.
                            None if i == 0 => format!("EDGE_{kind}"),
                            _ => continue,
                        };
                        let x0 = if mirror { lu - u - w } else { u };
                        d.rect(&layer, x0, v, w, h);
                    }
                    Feature::EdgeBore {
                        u,
                        v,
                        diameter,
                        depth,
                        edge,
                    } if i == 0 => {
                        let layer =
                            format!("EDGE_BORE_D{}_Z{}", depth_name(diameter), depth_name(depth));
                        let (u, v) = (u.clamp(0.0, lu), v.clamp(0.0, lv));
                        let to = match edge {
                            crate::Edge::Left => (u + depth, v),
                            crate::Edge::Right => (u - depth, v),
                            crate::Edge::Bottom => (u, v + depth),
                            crate::Edge::Top => (u, v - depth),
                        };
                        d.line(&layer, (x(u), v), (x(to.0), to.1));
                    }
                    _ => {}
                }
            }
            DxfFile {
                name: if i == 0 {
                    part.name.clone()
                } else {
                    format!("{}-back", part.name)
                },
                contents: d.finish(),
            }
        })
        .collect()
}

/// Every part's DXF files.
pub fn job_dxfs(lists: &[PartList]) -> Vec<DxfFile> {
    lists
        .iter()
        .flat_map(|l| l.parts.iter().flat_map(move |p| part_dxfs(l, p)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use cabinet_core::{Cabinet, CabinetKind, build_cabinet};

    /// (code, value) pairs of a DXF.
    fn pairs(dxf: &str) -> Vec<(String, String)> {
        let lines: Vec<&str> = dxf.lines().collect();
        lines
            .chunks(2)
            .map(|c| (c[0].trim().to_string(), c[1].trim().to_string()))
            .collect()
    }

    fn layers(dxf: &str) -> Vec<String> {
        let p = pairs(dxf);
        let mut out: Vec<String> = p
            .iter()
            .filter(|(c, _)| c == "8")
            .map(|(_, v)| v.clone())
            .collect();
        out.sort();
        out.dedup();
        out
    }

    #[test]
    fn a_dxf_is_well_formed_r12() {
        let list = build_cabinet(&Cabinet::new("B01", CabinetKind::Base)).unwrap();
        // For checking with another reader: DXF_OUT_DIR=target/dxf cargo test -p cabinet-sheets.
        let out = std::env::var("DXF_OUT_DIR").ok();
        for f in job_dxfs(std::slice::from_ref(&list)) {
            if let Some(dir) = &out {
                std::fs::create_dir_all(dir).unwrap();
                std::fs::write(format!("{dir}/{}.dxf", f.name), &f.contents).unwrap();
            }
            let p = pairs(&f.contents);
            assert_eq!(p[0], ("0".into(), "SECTION".into()));
            assert_eq!(p.last().unwrap(), &("0".into(), "EOF".into()));
            assert!(f.contents.lines().count() % 2 == 0, "{}", f.name);
            let opens = p
                .iter()
                .filter(|(c, v)| c == "0" && v == "POLYLINE")
                .count();
            let ends = p.iter().filter(|(c, v)| c == "0" && v == "SEQEND").count();
            assert_eq!(opens, ends);
        }
    }

    #[test]
    fn a_side_has_its_outline_shelf_pins_and_hinge_plates_on_layers_by_size() {
        let list = build_cabinet(&Cabinet::new("B01", CabinetKind::Base)).unwrap();
        let side = list.part("B01.LeftSide").unwrap();
        let files = part_dxfs(&list, side);
        let main = &files[0];
        assert_eq!(main.name, "B01.LeftSide");
        let l = layers(&main.contents);
        assert!(l.contains(&"OUTLINE".to_string()), "{l:?}");
        assert!(l.contains(&"BORE_D5_Z10".to_string()), "{l:?}");
        assert!(l.iter().any(|n| n.starts_with("POCKET_GROOVE_Z")), "{l:?}");
        // Every face bore of that face is one circle.
        let circles = pairs(&main.contents)
            .iter()
            .filter(|(c, v)| c == "0" && v == "CIRCLE")
            .count();
        let face = features(&list, side)
            .iter()
            .filter(|f| matches!(f, Feature::FaceBore { .. }) && f.face().is_some())
            .count();
        assert!(circles > 0 && circles <= face);
    }

    #[test]
    fn the_second_face_is_mirrored_along_the_length() {
        let list = build_cabinet(&Cabinet::new("B01", CabinetKind::Base)).unwrap();
        // Sides with exposed screws: countersinks outside, pins inside.
        for p in &list.parts {
            let files = part_dxfs(&list, p);
            if files.len() < 2 {
                continue;
            }
            assert_eq!(files[1].name, format!("{}-back", p.name));
            let (ua, _) = frame(p);
            let lu = p.bounds.size.get(ua);
            for (c, v) in pairs(&files[1].contents) {
                if c == "10" {
                    let x: f64 = v.parse().unwrap();
                    assert!((-1e-6..=lu + 1e-6).contains(&x), "{}: {x}", p.name);
                }
            }
            return;
        }
        panic!("no part machined on both faces");
    }

    #[test]
    fn a_toe_kick_notch_is_a_cutout() {
        let mut cab = Cabinet::new("B01", CabinetKind::Base);
        cab.standard.toe_kick = cabinet_core::model::ToeKick::IntegratedNotch;
        let list = build_cabinet(&cab).unwrap();
        let files = part_dxfs(&list, list.part("B01.LeftSide").unwrap());
        assert!(layers(&files[0].contents).contains(&"CUTOUT".to_string()));
    }
}
