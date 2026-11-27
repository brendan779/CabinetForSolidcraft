//! BENCH-19: a stone top longer than its slab is joined over a cabinet joint, clear of cut-outs.
use cabinet_core::geom::Vec3;
use cabinet_core::part::Operation;
use cabinet_core::run::{Benchtop, Run, RunItem, build_run};
use cabinet_core::{Cabinet, CabinetKind, PartList};

fn run(top: serde_json::Value, cabinets: usize) -> PartList {
    let mut items: Vec<RunItem> = (0..cabinets)
        .map(|i| {
            let mut c = Cabinet::new(format!("B0{}", i + 1), CabinetKind::Base);
            c.width = 900.0;
            RunItem::Cabinet(Box::new(c))
        })
        .collect();
    items.push(RunItem::Benchtop(
        serde_json::from_value::<Benchtop>(top).unwrap(),
    ));
    build_run(&Run {
        origin: Vec3::default(),
        items,
    })
    .unwrap()
    .pop()
    .unwrap()
}

fn joins(list: &PartList) -> Vec<f64> {
    list.operations
        .iter()
        .filter_map(|o| match o {
            Operation::SlabJoin { at, .. } => Some(at.x),
            _ => None,
        })
        .collect()
}

#[test]
fn a_top_within_its_slab_has_no_join() {
    let list = run(
        serde_json::json!({"id": "BT1", "thickness": 20, "slab": [3200, 1600]}),
        3,
    );
    assert!(joins(&list).is_empty());
}

#[test]
fn a_top_longer_than_its_slab_joins_over_the_last_cabinet_joint_a_slab_reaches() {
    // 4500 long: the joints are 900 apart, a slab reaches 3200, so the join is at 2700.
    let list = run(
        serde_json::json!({"id": "BT1", "thickness": 20, "slab": [3200, 1600]}),
        5,
    );
    assert_eq!(joins(&list), vec![2700.0]);
    assert!(
        list.notes.iter().any(|n| n.contains("joined 2700 mm")),
        "{:?}",
        list.notes
    );
}

#[test]
fn a_join_keeps_clear_of_a_cut_out() {
    // A cooktop centred over B03 (1800 to 2700) keeps a join 100 clear of it, so neither of
    // B03's joints will do: the join falls back to 900, and the 3600 left needs a second.
    let list = run(
        serde_json::json!({"id": "BT1", "thickness": 20, "slab": [3200, 1600],
            "cutouts": [{"over": "B03", "kind": "cooktop", "name": "Cooktop 900", "width": 860, "depth": 490}]}),
        5,
    );
    assert_eq!(joins(&list), vec![900.0, 3600.0]);
}

#[test]
fn a_waterfall_comes_out_of_its_end_of_the_top_slab() {
    // The left drop takes about 890 of the first slab: the join comes back a cabinet.
    let list = run(
        serde_json::json!({"id": "BT1", "thickness": 20, "slab": [3200, 1600], "waterfall_left": true}),
        5,
    );
    let drop = list.part("BT1.Waterfall1L").unwrap().bounds.size.z;
    let at = joins(&list);
    assert_eq!(at.len(), 1);
    let x0 = list.part("BT1.Benchtop1").unwrap().bounds.min.x;
    assert!(drop + (at[0] - x0) <= 3200.0, "{at:?} after a {drop} drop");
    assert_eq!(at[0], 1800.0);
}
