//! BENCH-15a: a catalog sink sets its cut-out from its data and is drawn under or on the top.
use cabinet_core::cad::cad_placements;
use cabinet_core::geom::Vec3;
use cabinet_core::part::Operation;
use cabinet_core::run::{Appliance, Benchtop, Run, RunItem, build_run};
use cabinet_core::{Cabinet, CabinetKind, PartList};

fn top_with(cutout: serde_json::Value) -> PartList {
    let mut sink = Cabinet::new("S01", CabinetKind::Sink);
    sink.width = 900.0;
    let top: Benchtop = serde_json::from_value(
        serde_json::json!({"id": "BT1", "thickness": 20, "cutouts": [cutout]}),
    )
    .unwrap();
    build_run(&Run {
        origin: Vec3::default(),
        items: vec![RunItem::Cabinet(Box::new(sink)), RunItem::Benchtop(top)],
    })
    .unwrap()
    .pop()
    .unwrap()
}

fn cutout(list: &PartList) -> (Vec3, f64, String) {
    list.operations
        .iter()
        .find_map(|o| match o {
            Operation::Cutout {
                bounds,
                radius,
                note,
                ..
            } => Some((bounds.size, *radius, note.clone())),
            _ => None,
        })
        .unwrap()
}

#[test]
fn every_catalog_sink_is_an_appliance_with_its_opening() {
    let sinks = Appliance::catalog_sinks();
    // 16 undermount, and the 5 granite sinks whose body under the flange is printed on top.
    assert_eq!(sinks.len(), 21);
    let vienna = sinks
        .iter()
        .find(|a| a.product == "sink.abi.vienna-double-sink-760mm")
        .unwrap();
    assert_eq!(
        (vienna.width, vienna.depth, vienna.corner_radius),
        (710.0, 400.0, 5.0)
    );
}

#[test]
fn an_undermount_sink_cuts_its_bowls_opening_and_hangs_under_the_top() {
    let list = top_with(
        serde_json::json!({"over": "S01", "kind": "sink", "width": 1, "depth": 1,
        "product": "sink.abi.vienna-double-sink-760mm"}),
    );
    let (size, radius, note) = cutout(&list);
    assert_eq!((size.x, size.y, radius), (710.0, 400.0, 5.0));
    assert!(
        note.contains("Vienna Double Sink 760mm") && note.contains("undermount"),
        "{note}"
    );
    let top = list.part("BT1.Benchtop1").unwrap().bounds;
    let sink = list
        .hardware
        .iter()
        .find(|h| h.item == "sink.abi.vienna-double-sink-760mm")
        .unwrap();
    assert_eq!(sink.at.z, top.min.z, "the rim's top at the top's underside");
    let placed = cad_placements(&list);
    let p = placed.iter().find(|p| p.name == "BT1.Sink1").unwrap();
    assert_eq!(p.article, "ABI-SINK-VIENNA-DOUBLE-SINK-760MM");
}

#[test]
fn a_top_mounted_granite_sink_cuts_its_body_and_sits_on_the_top() {
    let list = top_with(
        serde_json::json!({"over": "S01", "kind": "sink", "width": 1, "depth": 1,
        "mount": "top_mount", "product": "sink.abi.vienna-double-sink-760mm"}),
    );
    let (size, _, _) = cutout(&list);
    assert_eq!((size.x, size.y), (738.0, 428.0));
    let top = list.part("BT1.Benchtop1").unwrap().bounds;
    assert_eq!(
        list.hardware
            .iter()
            .find(|h| h.item.starts_with("sink."))
            .unwrap()
            .at
            .z,
        top.max().z
    );
}

#[test]
fn a_sink_with_no_body_size_to_cut_is_undermounted_with_a_warning() {
    let list = top_with(
        serde_json::json!({"over": "S01", "kind": "sink", "width": 1, "depth": 1,
        "mount": "top_mount", "product": "sink.abi.zalo-double-kitchen-sink-855mm"}),
    );
    let (size, radius, _) = cutout(&list);
    assert_eq!((size.x, size.y, radius), (805.0, 400.0, 10.0));
    assert!(
        list.warnings.iter().any(|w| w.contains("undermounted")),
        "{:?}",
        list.warnings
    );
}
