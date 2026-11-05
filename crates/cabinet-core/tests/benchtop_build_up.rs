use cabinet_core::geom::Vec3;
use cabinet_core::part::{Operation, PartRole};
use cabinet_core::run::{Benchtop, EdgeProfile, Run, RunItem, ShadowTape, build_run};
use cabinet_core::{Cabinet, CabinetKind};

fn build(depth: f64, shadow: f64, setback: f64, falls: bool) -> Vec<cabinet_core::PartList> {
    let mut b: Benchtop =
        serde_json::from_str(r#"{"id":"BT1","thickness":20,"side_overhang":18}"#).unwrap();
    b.edge = EdgeProfile::MitredApron { depth };
    b.waterfall_left = falls;
    b.waterfall_right = falls;
    b.build_up.shadow_height = shadow;
    b.build_up.setback = setback;
    b.build_up.edge_tape = Some(ShadowTape {
        id: "contrast-band".into(),
        colour: "#332211".into(),
        thickness: 1.0,
    });
    build_run(&Run {
        origin: Vec3::default(),
        items: vec![
            RunItem::Cabinet(Box::new(Cabinet::new("B01", CabinetKind::Base))),
            RunItem::Benchtop(b),
        ],
    })
    .unwrap()
}

#[test]
fn waterfalls_touch_cabinet_ends_instead_of_leaving_the_end_overhang_as_a_gap() {
    let lists = build(60.0, 20.0, 0.0, true);
    let (cab, top) = (&lists[0], &lists[1]);
    assert_eq!(
        top.part("BT1.Waterfall1L").unwrap().bounds.max().x,
        cab.part("B01.LeftSide").unwrap().bounds.min.x
    );
    assert_eq!(
        top.part("BT1.Waterfall1R").unwrap().bounds.min.x,
        cab.part("B01.RightSide").unwrap().bounds.max().x
    );
}

#[test]
fn the_thick_look_raises_real_stone_on_stock_board_and_keeps_the_shadow_line() {
    for depth in [40.0, 60.0, 80.0, 100.0] {
        for shadow in [0.0, 10.0, 20.0, 30.0] {
            let lists = build(depth, shadow, 0.0, false);
            let top = &lists[1];
            let slab = top.part("BT1.Benchtop1").unwrap();
            let apron = top.part("BT1.Apron1").unwrap();
            assert_eq!(slab.thickness(), 20.0);
            assert_eq!(apron.bounds.size.z, depth);
            assert_eq!(apron.bounds.min.z, 870.0 + shadow);
            assert_eq!(slab.bounds.min.z, 870.0 + shadow + depth - 20.0);
            assert!(
                top.by_role(PartRole::BenchtopSupport)
                    .all(|p| p.bounds.min.z >= 870.0 && p.bounds.max().z <= slab.bounds.min.z)
            );
            assert!(
                top.operations
                    .iter()
                    .any(|op| matches!(op, Operation::Chamfer { .. }))
            );
        }
    }
}

#[test]
fn recess_and_edgeband_are_independent_of_the_support_stock_board() {
    for setback in [0.0, 10.0, 25.0] {
        let lists = build(60.0, 20.0, setback, false);
        let top = &lists[1];
        let band = top.part("BT1.ShadowBand1").unwrap();
        assert_eq!(band.bounds.min.y, -18.0 + setback);
        let cut = cabinet_core::cutlist::cut_list(&lists, "Generic", 1.0);
        for stone in cut.rows.iter().filter(|r| r.templated) {
            assert_eq!(
                stone.cut_length, stone.length,
                "stone gets no tape deduction"
            );
            assert_eq!(stone.cut_width, stone.width, "stone gets no tape deduction");
        }
        assert!(!cut.rows.iter().any(|r| r.part == band.name));
        let rail = cut
            .rows
            .iter()
            .find(|r| r.part == "BT1.ShadowRail1")
            .unwrap();
        assert!(!rail.templated);
        assert!(rail.description.contains("contrast-band"));
        assert_eq!(
            rail.thickness,
            lists[0].part("B01.LeftSide").unwrap().thickness()
        );
        assert!(top.part("BT1.Apron1L").is_some());
        assert!(top.part("BT1.Apron1R").is_some());
    }
}

#[test]
fn sink_openings_also_clear_the_new_supports() {
    use cabinet_core::run::{Cutout, CutoutKind};
    let mut b: Benchtop = serde_json::from_str(
        r#"{"id":"BT1","thickness":20,"edge":{"type":"mitred_apron","depth":60}}"#,
    )
    .unwrap();
    b.build_up.setback = 120.0;
    b.cutouts.push(Cutout {
        over: "S01".into(),
        kind: CutoutKind::Sink,
        width: 400.0,
        depth: 400.0,
        ..serde_json::from_str(r#"{"over":"S01","kind":"sink","width":400,"depth":400}"#).unwrap()
    });
    let lists = build_run(&Run {
        origin: Vec3::default(),
        items: vec![
            RunItem::Cabinet(Box::new(Cabinet::new("S01", CabinetKind::Sink))),
            RunItem::Benchtop(b),
        ],
    })
    .unwrap();
    let top = &lists[1];
    assert!(top.operations.iter().any(|op|matches!(op,Operation::Cutout{part,note,..} if part.contains("Packer") && note.contains("support"))));
}

#[test]
fn invalid_build_up_dimensions_are_rejected_before_geometry_is_created() {
    for values in [
        r#"{"shadow_height":-1}"#,
        r#"{"setback":-1}"#,
        r#"{"strip_width":0}"#,
        r##"{"edge_tape":{"id":"x","colour":"#000000","thickness":0}}"##,
    ] {
        let mut b: Benchtop = serde_json::from_str(
            r#"{"id":"BT1","thickness":20,"edge":{"type":"mitred_apron","depth":60}}"#,
        )
        .unwrap();
        b.build_up = serde_json::from_str(values).unwrap();
        assert!(
            build_run(&Run {
                origin: Vec3::default(),
                items: vec![
                    RunItem::Cabinet(Box::new(Cabinet::new("B01", CabinetKind::Base))),
                    RunItem::Benchtop(b)
                ]
            })
            .is_err()
        );
    }
}

#[test]
fn both_corner_hands_have_return_supports_and_stone_joints_without_overlapping_material() {
    use cabinet_core::geom::Axis;
    use cabinet_core::model::HingeSide;
    for side in [HingeSide::Left, HingeSide::Right] {
        let mut c = Cabinet::new("C01", CabinetKind::CornerBase);
        c.corner.side = side;
        let b: Benchtop = serde_json::from_str(
            r#"{"id":"BT1","thickness":20,"edge":{"type":"mitred_apron","depth":60}}"#,
        )
        .unwrap();
        let list = build_run(&Run {
            origin: Vec3::new(100.0, 200.0, 50.0),
            items: vec![RunItem::Cabinet(Box::new(c)), RunItem::Benchtop(b)],
        })
        .unwrap()
        .pop()
        .unwrap();
        assert!(list.part("BT1.ShadowRail1Leg").is_some());
        assert!(list.part("BT1.Apron1Leg").is_some());
        let stone: Vec<_> = list.by_role(PartRole::Benchtop).collect();
        let contains = |p: &cabinet_core::part::Part, at: Vec3| {
            if Axis::ALL
                .into_iter()
                .any(|a| at.get(a) <= p.bounds.min.get(a) || at.get(a) >= p.bounds.max().get(a))
            {
                return false;
            }
            !list.operations.iter().any(|op| match op {
                Operation::Chamfer {
                    part,
                    bounds,
                    at: corner,
                } if part == &p.name => {
                    if Axis::ALL
                        .into_iter()
                        .any(|a| at.get(a) < bounds.min.get(a) || at.get(a) > bounds.max().get(a))
                    {
                        return false;
                    }
                    let along = Axis::ALL
                        .into_iter()
                        .find(|&a| {
                            corner.get(a) > bounds.min.get(a) + 1e-6
                                && corner.get(a) < bounds.max().get(a) - 1e-6
                        })
                        .unwrap();
                    along
                        .others()
                        .iter()
                        .map(|&a| (at.get(a) - corner.get(a)).abs() / bounds.size.get(a))
                        .sum::<f64>()
                        <= 1.0 + 1e-6
                }
                _ => false,
            })
        };
        for (i, a) in stone.iter().enumerate() {
            for b in &stone[i + 1..] {
                let lo = Vec3::new(
                    a.bounds.min.x.max(b.bounds.min.x),
                    a.bounds.min.y.max(b.bounds.min.y),
                    a.bounds.min.z.max(b.bounds.min.z),
                );
                let hi = Vec3::new(
                    a.bounds.max().x.min(b.bounds.max().x),
                    a.bounds.max().y.min(b.bounds.max().y),
                    a.bounds.max().z.min(b.bounds.max().z),
                );
                if Axis::ALL.into_iter().any(|a| hi.get(a) <= lo.get(a)) {
                    continue;
                }
                for x in 0..7 {
                    for y in 0..7 {
                        for z in 0..7 {
                            let p = Vec3::new(
                                lo.x + (hi.x - lo.x) * (x as f64 + 0.5) / 7.0,
                                lo.y + (hi.y - lo.y) * (y as f64 + 0.5) / 7.0,
                                lo.z + (hi.z - lo.z) * (z as f64 + 0.5) / 7.0,
                            );
                            assert!(
                                !(contains(a, p) && contains(b, p)),
                                "{side:?}: {} and {} overlap at {p:?}",
                                a.name,
                                b.name
                            );
                        }
                    }
                }
            }
        }
    }
}
