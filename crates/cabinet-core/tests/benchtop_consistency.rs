use cabinet_core::geom::Vec3;
use cabinet_core::model::HingeSide;
use cabinet_core::part::Operation;
use cabinet_core::run::{Benchtop, Cutout, CutoutKind, Gap, GapLine, Run, RunItem, build_run};
use cabinet_core::{Cabinet, CabinetKind};

fn top() -> Benchtop {
    serde_json::from_str(r#"{"id":"BT1"}"#).unwrap()
}

#[test]
fn edge_profiles_are_thickness_finishes_and_leave_wall_and_waterfall_joints_square() {
    use cabinet_core::run::EdgeProfile;
    for profile in [
        EdgeProfile::PencilRound,
        EdgeProfile::Bullnose,
        EdgeProfile::Bevel,
    ] {
        let mut b = top();
        b.edge = profile;
        b.side_overhang = 20.0;
        b.waterfall_right = true;
        let wall = serde_json::from_value(serde_json::json!({"id":"W1", "left":true})).unwrap();
        let run = Run {
            origin: Vec3::default(),
            items: vec![
                cabinet(CabinetKind::Base),
                RunItem::Benchtop(b),
                RunItem::Walls(wall),
            ],
        };
        let list = &build_run(&run).unwrap()[1];
        let finishes: Vec<_> = list
            .operations
            .iter()
            .filter_map(|op| match op {
                Operation::EdgeFinish {
                    part,
                    edges,
                    size,
                    bevel,
                } => Some((part, edges, size, bevel)),
                _ => None,
            })
            .collect();
        assert_eq!(finishes.len(), 1);
        let (part, edges, size, bevel) = finishes[0];
        assert_eq!(part, "BT1.Benchtop1");
        assert_eq!(
            *size,
            if profile == EdgeProfile::Bullnose {
                16.5
            } else {
                2.0
            }
        );
        assert_eq!(*bevel, profile == EdgeProfile::Bevel);
        assert_eq!(edges.len(), if *bevel { 1 } else { 2 });
        assert!(
            edges
                .iter()
                .all(|at| at.x == list.parts[0].bounds.center().x)
        );
        assert!(
            !list
                .operations
                .iter()
                .any(|op| matches!(op, Operation::Round { .. }))
        );
    }
}

#[test]
fn unsupported_bullnose_on_rounded_ends_warns_instead_of_breaking_the_export() {
    let mut b = top();
    b.edge = cabinet_core::run::EdgeProfile::Bullnose;
    b.end_radius = 60.0;
    let run = Run {
        origin: Vec3::default(),
        items: vec![cabinet(CabinetKind::Base), RunItem::Benchtop(b)],
    };
    let list = build_run(&run).unwrap().pop().unwrap();
    assert!(
        list.warnings
            .iter()
            .any(|w| w.contains("full bullnose with rounded ends"))
    );
    assert!(
        !list
            .operations
            .iter()
            .any(|op| matches!(op, Operation::EdgeFinish { .. }))
    );
    assert!(
        list.operations
            .iter()
            .any(|op| matches!(op, Operation::Round { .. }))
    );
}

fn cabinet(kind: CabinetKind) -> RunItem {
    RunItem::Cabinet(Box::new(Cabinet::new("B01", kind)))
}

#[test]
fn moving_a_run_keeps_straight_and_corner_benchtops_the_same_shape() {
    for kind in [CabinetKind::Base, CabinetKind::CornerBase] {
        for side in [HingeSide::Left, HingeSide::Right] {
            let mut cab = Cabinet::new("B01", kind);
            cab.corner.side = side;
            let mut b = top();
            b.waterfall_left = true;
            b.waterfall_right = true;
            let mut run = Run {
                origin: Vec3::default(),
                items: vec![RunItem::Cabinet(Box::new(cab)), RunItem::Benchtop(b)],
            };
            let before = build_run(&run).unwrap().pop().unwrap();
            for shift in [
                Vec3::new(1000.0, -2000.0, 100.0),
                Vec3::new(0.0, 2000.0, 0.0),
            ] {
                run.origin = shift;
                let after = build_run(&run).unwrap().pop().unwrap();
                assert_eq!(before.parts.len(), after.parts.len(), "{kind:?} {side:?}");
                for (a, b) in before.parts.iter().zip(&after.parts) {
                    assert_eq!(a.name, b.name);
                    assert_eq!(a.bounds.size, b.bounds.size, "{} {kind:?} {side:?}", a.name);
                    assert_eq!(a.bounds.min + shift, b.bounds.min, "{}", a.name);
                }
            }
        }
    }
}

#[test]
fn a_floor_gap_without_a_base_cabinet_does_not_make_an_invalid_benchtop() {
    let run = Run {
        origin: Vec3::default(),
        items: vec![
            cabinet(CabinetKind::Wall),
            RunItem::Gap(Gap {
                id: "G01".into(),
                width: 600.0,
                line: GapLine::Floor,
            }),
            RunItem::Benchtop(top()),
        ],
    };
    let list = build_run(&run).unwrap().pop().unwrap();
    assert!(list.parts.is_empty(), "{:?}", list.parts);
    assert!(list.warnings.iter().any(|w| w.contains("no base cabinets")));
}

#[test]
fn a_cutout_cannot_start_in_front_of_the_benchtop() {
    let mut b = top();
    let mut cutout = Cutout::new("B01", CutoutKind::Cooktop, 400.0, 400.0);
    cutout.setback = -20.0;
    b.cutouts.push(cutout);
    let run = Run {
        origin: Vec3::default(),
        items: vec![cabinet(CabinetKind::Base), RunItem::Benchtop(b)],
    };
    let list = build_run(&run).unwrap().pop().unwrap();
    assert!(
        !list
            .operations
            .iter()
            .any(|o| matches!(o, Operation::Cutout { .. }))
    );
    assert!(list.warnings.iter().any(|w| w.contains("does not fit")));
}

#[test]
fn invalid_cutout_dimensions_do_not_reach_the_cad_export() {
    for (width, depth) in [(0.0, 400.0), (-100.0, 400.0), (400.0, 0.0), (400.0, -100.0)] {
        let mut b = top();
        b.cutouts
            .push(Cutout::new("B01", CutoutKind::Cooktop, width, depth));
        let run = Run {
            origin: Vec3::default(),
            items: vec![cabinet(CabinetKind::Base), RunItem::Benchtop(b)],
        };
        let list = build_run(&run).unwrap().pop().unwrap();
        assert!(
            list.operations.is_empty(),
            "{width} × {depth}: {:?}",
            list.operations
        );
        assert!(!list.warnings.is_empty());
    }
}

#[test]
fn waterfall_panels_reach_the_run_floor_even_under_raised_cabinets() {
    let mut cab = Cabinet::new("B01", CabinetKind::Base);
    cab.origin.z = 100.0;
    let mut b = top();
    b.waterfall_left = true;
    let run = Run {
        origin: Vec3::new(0.0, 0.0, 250.0),
        items: vec![RunItem::Cabinet(Box::new(cab)), RunItem::Benchtop(b)],
    };
    let list = build_run(&run).unwrap().pop().unwrap();
    assert_eq!(
        list.part("BT1.Waterfall1L").unwrap().bounds.min.z,
        run.origin.z
    );
}

#[test]
fn rounded_ends_stay_square_where_the_top_meets_a_return_wall() {
    for left in [true, false] {
        let wall = serde_json::from_value(serde_json::json!({
            "id": "W1", "left": left, "right": !left,
        }))
        .unwrap();
        let mut b = top();
        b.end_radius = 100.0;
        let run = Run {
            origin: Vec3::default(),
            items: vec![
                cabinet(CabinetKind::Base),
                RunItem::Benchtop(b),
                RunItem::Walls(wall),
            ],
        };
        let lists = build_run(&run).unwrap();
        let list = &lists[1];
        let bounds = list.part("BT1.Benchtop1").unwrap().bounds;
        let rounds: Vec<_> = list
            .operations
            .iter()
            .filter_map(|o| match o {
                Operation::Round { at, .. } => Some(at),
                _ => None,
            })
            .collect();
        assert_eq!(rounds.len(), 1);
        assert_eq!(
            rounds[0].x,
            if left { bounds.max().x } else { bounds.min.x }
        );
    }
}

#[test]
fn a_waterfall_does_not_extend_through_a_return_wall() {
    let wall = serde_json::from_value(serde_json::json!({"id": "W1", "left": true})).unwrap();
    let mut b = top();
    b.waterfall_left = true;
    b.waterfall_right = true;
    let run = Run {
        origin: Vec3::default(),
        items: vec![
            cabinet(CabinetKind::Base),
            RunItem::Benchtop(b),
            RunItem::Walls(wall),
        ],
    };
    let lists = build_run(&run).unwrap();
    let list = &lists[1];
    assert!(list.part("BT1.Waterfall1L").is_none());
    assert!(list.part("BT1.Waterfall1R").is_some());
    assert!(
        list.warnings
            .iter()
            .any(|w| w.contains("waterfall") && w.contains("wall"))
    );
}
