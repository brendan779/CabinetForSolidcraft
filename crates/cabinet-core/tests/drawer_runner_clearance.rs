use cabinet_core::cad::cad_placements;
use cabinet_core::hardware;
use cabinet_core::part::{BorePurpose, Operation, PartRole};
use cabinet_core::{Project, build_cabinet};

#[test]
fn high_fronted_legrabox_uses_its_own_base_lift_for_every_overlay() {
    use cabinet_core::model::{DoorOverlay, DrawerSides, FrontLayout};
    for overlay in [DoorOverlay::Full, DoorOverlay::Half, DoorOverlay::Inset] {
        let mut cab = cabinet_core::Cabinet::new("D01", cabinet_core::CabinetKind::DrawerBank);
        cab.fronts = FrontLayout::Drawers { ratios: vec![1.0] };
        cab.standard.drawer_sides = DrawerSides::HighFronted;
        cab.standard.overlay = overlay;
        let list = build_cabinet(&cab).unwrap();
        assert!(
            list.hardware
                .iter()
                .any(|h| h.item.contains(".drawer-side.f."))
        );
        let floor = list
            .by_role(PartRole::Bottom)
            .next()
            .unwrap()
            .bounds
            .max()
            .z;
        let runners: Vec<_> = cad_placements(&list)
            .into_iter()
            .filter(|p| p.name.contains("Runner"))
            .collect();
        assert_eq!(runners.len(), 2);
        for runner in runners {
            let screw = hardware::item(&runner.item).unwrap().drilling[0].y;
            assert!(
                (runner.origin.z - floor - screw).abs() < 1e-6,
                "{overlay:?}"
            );
        }
    }
}

#[test]
fn wardrobe_runner_models_align_with_their_mounting_holes_above_the_floor() {
    for (system, model_bottom) in [
        ("legrabox", -38.1),
        ("merivobox", -53.75),
        ("tandembox", -32.8),
        ("movento", -38.1),
        ("tandem", -36.3),
    ] {
        let mut json: serde_json::Value =
            serde_json::from_str(include_str!("fixtures/wardrobe-runners.cabinetjob")).unwrap();
        json["standard"]["drawer_system"] = system.into();
        for cab in json["cabinets"].as_array_mut().unwrap() {
            for row in cab["fronts"]["columns"][0]["rows"].as_array_mut().unwrap() {
                if row["front"]["type"] == "drawers" {
                    row["front"]["system"] = system.into();
                }
            }
        }
        let project: Project = serde_json::from_value(json).unwrap();
        for cab in project.resolved().unwrap() {
            let list = build_cabinet(&cab).unwrap();
            let placements = cad_placements(&list);
            let runners: Vec<_> = placements
                .iter()
                .filter(|p| p.name.contains("Runner"))
                .collect();
            assert_eq!(runners.len(), 8, "{} {system}", cab.id);
            let floor = list
                .by_role(PartRole::Bottom)
                .next()
                .unwrap()
                .bounds
                .max()
                .z;
            for runner in runners {
                let support = list
                    .parts
                    .iter()
                    .filter(|p| {
                        matches!(p.role, PartRole::Bottom | PartRole::Shelf)
                            && p.bounds.min.x <= runner.origin.x
                            && p.bounds.max().x >= runner.origin.x
                            && p.bounds.max().z < runner.origin.z
                    })
                    .map(|p| p.bounds.max().z)
                    .fold(floor, f64::max);
                // Bounds measured from the supplier's NL 400 STEP models, Y up.
                // LEGRABOX/MOVENTO's nominal 38 mm datum includes a 0.1 mm model overrun.
                assert!(
                    runner.origin.z + model_bottom >= support - 0.11,
                    "{} {system}: runner bottom {} below support {support}",
                    runner.name,
                    runner.origin.z + model_bottom
                );
                let item = hardware::item(&runner.item).unwrap();
                assert!(
                    list.operations.iter().any(|op| matches!(op,
                        Operation::Bore { at, purpose: BorePurpose::RunnerScrew, .. }
                        if (at.z - runner.origin.z).abs() < 1e-6
                            && (at.y - runner.origin.y - item.drilling[0].x).abs() < 1e-6
                            && (at.x - runner.origin.x).abs() < 1e-6
                    )),
                    "{} {system}: model screw line misses cabinet drilling",
                    runner.name
                );
            }
        }
    }
}

#[test]
fn inner_drawers_use_the_same_runner_datum_without_a_drawer_front() {
    let mut cab = cabinet_core::Cabinet::new("W01", cabinet_core::CabinetKind::Wardrobe);
    cab.fronts = cabinet_core::model::FrontLayout::Open;
    cab.inner_drawers = 2;
    let list = build_cabinet(&cab).unwrap();
    let floor = list
        .by_role(PartRole::Bottom)
        .next()
        .unwrap()
        .bounds
        .max()
        .z;
    let placements = cad_placements(&list);
    let runners: Vec<_> = placements
        .iter()
        .filter(|p| p.name.contains("Runner"))
        .collect();
    assert_eq!(runners.len(), 4);
    for (pair, base) in runners.chunks(2).zip(list.by_role(PartRole::DrawerBottom)) {
        let datum = base.bounds.min.z
            - hardware::item("blum.legrabox.inner-m")
                .unwrap()
                .param("base_above_datum")
                .unwrap();
        for runner in pair {
            let screw = hardware::item(&runner.item).unwrap().drilling[0].y;
            assert!((runner.origin.z - datum - screw).abs() < 1e-6);
            assert!(runner.origin.z - 38.1 >= floor - 0.11);
        }
    }
}
