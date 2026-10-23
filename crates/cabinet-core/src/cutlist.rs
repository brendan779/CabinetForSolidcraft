//! Cut lists: finished and cut sizes, edge tape and totals per board.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::geom::Axis;
use crate::part::{Part, PartList};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CutRow {
    pub cabinet: String,
    pub part: String,
    pub description: String,
    pub board: String,
    pub thickness: f64,
    /// Finished size; length runs along the grain.
    pub length: f64,
    pub width: f64,
    /// Size to cut before edge tape: finished size less the tape on banded edges.
    pub cut_length: f64,
    pub cut_width: f64,
    /// Banded edges as `L1 L2 W1 W2` flags: the two length edges and the two width edges.
    pub edges: [bool; 4],
    pub grain: bool,
    pub quantity: u32,
}

impl CutRow {
    /// Metres of edge tape this row needs.
    pub fn edge_metres(&self) -> f64 {
        let [l1, l2, w1, w2] = self.edges;
        let along_length = (l1 as u8 + l2 as u8) as f64 * self.length;
        let along_width = (w1 as u8 + w2 as u8) as f64 * self.width;
        (along_length + along_width) * self.quantity as f64 / 1000.0
    }

    pub fn area_m2(&self) -> f64 {
        self.length * self.width * self.quantity as f64 / 1e6
    }
}

/// Totals for one board and thickness.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BoardTotal {
    pub board: String,
    pub thickness: f64,
    pub parts: u32,
    pub area_m2: f64,
    pub edge_metres: f64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CutList {
    pub rows: Vec<CutRow>,
    pub totals: Vec<BoardTotal>,
}

/// The cut list for some part lists. `default_board` names parts with no board of their own;
/// `tape` is the edge tape thickness taken off cut sizes.
pub fn cut_list(lists: &[PartList], default_board: &str, tape: f64) -> CutList {
    let mut rows = Vec::new();
    for list in lists {
        for p in &list.parts {
            rows.push(row(&list.cabinet, p, default_board, tape));
        }
    }
    let mut totals: BTreeMap<(String, i64), BoardTotal> = BTreeMap::new();
    for r in &rows {
        let key = (r.board.clone(), (r.thickness * 100.0).round() as i64);
        let t = totals.entry(key).or_insert_with(|| BoardTotal {
            board: r.board.clone(),
            thickness: r.thickness,
            ..BoardTotal::default()
        });
        t.parts += r.quantity;
        t.area_m2 += r.area_m2();
        t.edge_metres += r.edge_metres();
    }
    CutList {
        rows,
        totals: totals.into_values().collect(),
    }
}

fn row(cabinet: &str, p: &Part, default_board: &str, tape: f64) -> CutRow {
    let (length, width) = p.finished_size();
    let [a, b] = p.thickness_axis.others();
    let length_axis: Axis =
        if (p.bounds.size.get(a) - length).abs() < 1e-9 && p.grain_axis != Some(b) {
            a
        } else {
            b
        };
    let width_axis = if length_axis == a { b } else { a };
    // Edges at the ends of the length axis run across the width, and the other way round.
    let (w1, w2) = p.edges.on_axis(length_axis);
    let (l1, l2) = p.edges.on_axis(width_axis);
    let cut_length = length - tape * (w1 as u8 + w2 as u8) as f64;
    let cut_width = width - tape * (l1 as u8 + l2 as u8) as f64;
    CutRow {
        cabinet: cabinet.to_string(),
        part: p.name.clone(),
        description: p.role.label().to_string(),
        board: if p.board.is_empty() {
            default_board.to_string()
        } else {
            p.board.clone()
        },
        thickness: p.thickness(),
        length,
        width,
        cut_length,
        cut_width,
        edges: [l1, l2, w1, w2],
        grain: p.grain_axis.is_some(),
        quantity: 1,
    }
}

/// CSV with a header row, for spreadsheets and panel saw software.
pub fn to_csv(list: &CutList) -> String {
    let mut out = String::from(
        "Cabinet,Part,Description,Board,Thickness,Length,Width,Cut length,Cut width,Edge L1,Edge L2,Edge W1,Edge W2,Grain,Qty\n",
    );
    for r in &list.rows {
        let yn = |b: bool| if b { "Y" } else { "" };
        out.push_str(&format!(
            "{},{},{},{},{},{:.1},{:.1},{:.1},{:.1},{},{},{},{},{},{}\n",
            csv(&r.cabinet),
            csv(&r.part),
            csv(&r.description),
            csv(&r.board),
            r.thickness,
            r.length,
            r.width,
            r.cut_length,
            r.cut_width,
            yn(r.edges[0]),
            yn(r.edges[1]),
            yn(r.edges[2]),
            yn(r.edges[3]),
            yn(r.grain),
            r.quantity
        ));
    }
    out
}

fn csv(s: &str) -> String {
    if s.contains([',', '"', '\n']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Cabinet, CabinetKind, build_cabinet};

    #[test]
    fn side_cut_size_takes_tape_off_the_front_edge() {
        let c = Cabinet::new("B01", CabinetKind::Base);
        let list = build_cabinet(&c).unwrap();
        let cl = cut_list(&[list], "Generic 16", 1.0);
        let side = cl.rows.iter().find(|r| r.part == "B01.LeftSide").unwrap();
        // Grain runs up the side: length is the side height, width its depth.
        assert_eq!(side.length, 870.0 - 150.0);
        assert_eq!(side.width, 580.0);
        assert_eq!(side.cut_width, 579.0);
        assert_eq!(side.cut_length, side.length);
        assert!((side.edge_metres() - 0.72).abs() < 1e-9);
    }

    #[test]
    fn doors_are_taped_all_round() {
        let c = Cabinet::new("B01", CabinetKind::Base);
        let cl = cut_list(&[build_cabinet(&c).unwrap()], "", 1.0);
        let door = cl.rows.iter().find(|r| r.part == "B01.Door1").unwrap();
        assert_eq!(door.edges, [true; 4]);
        assert_eq!(door.cut_length, door.length - 2.0);
    }

    #[test]
    fn csv_has_a_row_per_part() {
        let c = Cabinet::new("B01", CabinetKind::Base);
        let list = build_cabinet(&c).unwrap();
        let n = list.parts.len();
        let csv = to_csv(&cut_list(&[list], "", 1.0));
        assert_eq!(csv.lines().count(), n + 1);
    }
}
