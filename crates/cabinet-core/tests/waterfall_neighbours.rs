use cabinet_core::geom::Vec3;
use cabinet_core::run::{Benchtop, Run, RunItem, build_run};
use cabinet_core::{Cabinet, CabinetKind};

fn top(left: bool, right: bool) -> Benchtop {
    serde_json::from_value(serde_json::json!({"id": "BT1", "thickness": 20,
        "waterfall_left": left, "waterfall_right": right}))
    .unwrap()
}

#[test]
fn a_waterfall_against_a_cabinet_beyond_the_top_is_left_out_with_a_note() {
    let run = Run {
        origin: Vec3::default(),
        items: vec![
            RunItem::Cabinet(Box::new(Cabinet::new("B01", CabinetKind::Base))),
            RunItem::Cabinet(Box::new(Cabinet::new("O01", CabinetKind::OpenShelf))),
            RunItem::Benchtop(top(true, true)),
        ],
    };
    let lists = build_run(&run).unwrap();
    let bt = lists.iter().find(|l| l.cabinet == "BT1").unwrap();
    assert!(
        bt.part("BT1.Waterfall1L").is_some(),
        "the free end keeps its waterfall"
    );
    assert!(bt.part("BT1.Waterfall1R").is_none());
    assert!(
        bt.warnings
            .iter()
            .any(|w| w.contains("R waterfall would run into O01")),
        "{:?}",
        bt.warnings
    );
}
