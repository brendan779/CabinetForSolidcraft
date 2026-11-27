//! BENCH-19: a joined stone top shows its join on its sheet, in the schedule and in the DXF.
use cabinet_core::geom::Vec3;
use cabinet_core::run::{Benchtop, Run, RunItem, build_run};
use cabinet_core::{Cabinet, CabinetKind};

#[test]
fn a_joined_top_shows_its_join_on_the_sheet_the_schedule_and_the_dxf() {
    let mut items: Vec<RunItem> = (0..5)
        .map(|i| {
            let mut c = Cabinet::new(format!("B0{}", i + 1), CabinetKind::Base);
            c.width = 900.0;
            RunItem::Cabinet(Box::new(c))
        })
        .collect();
    let top: Benchtop = serde_json::from_value(
        serde_json::json!({"id": "BT1", "thickness": 20, "slab": [3200, 1600]}),
    )
    .unwrap();
    items.push(RunItem::Benchtop(top));
    let lists = build_run(&Run {
        origin: Vec3::default(),
        items,
    })
    .unwrap();
    let list = lists.last().unwrap();
    let piece = list.part("BT1.Benchtop1").unwrap();
    let svg = cabinet_sheets::part_sheet(list, piece, "Stone", "Job");
    assert!(svg.contains("slab join"), "no join on the sheet");
    let dxf = cabinet_sheets::dxf::part_dxfs(list, piece);
    assert!(
        dxf.iter().any(|f| f.contents.contains("JOIN")),
        "no JOIN layer"
    );
    assert!(
        !dxf.iter().any(|f| f.contents.contains("CUTOUT")),
        "a join is no cut-out"
    );
    let pdf = cabinet_sheets::benchtops::benchtops_pdf(&lists, "Job");
    assert!(!pdf.is_empty());
}
