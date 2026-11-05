use cabinet_core::geom::{Aabb, Axis, Vec3};
use cabinet_core::part::{BorePurpose, Operation, Part, PartRole};
use cabinet_core::run::{Run, RunItem, build_run};
use cabinet_core::{Cabinet, CabinetKind, PartList};

fn build(depth: f64, shadow: f64, seating: f64, falls: bool, holes: bool) -> Vec<PartList> {
    let mut cab = Cabinet::new("B01", CabinetKind::Base);
    cab.width = 900.0;
    cab.cladding.back = true;
    let top = serde_json::from_value(serde_json::json!({
        "id":"BT1", "thickness":20, "edge":{"type":"mitred_apron","depth":depth},
        "back_overhang":seating,"waterfall_left":falls,"waterfall_right":falls,
        "build_up":{"shadow_height":shadow,"setback":10,"strip_width":45,
            "board_thickness":18,"edge_tape":{"id":"gold","colour":"#aa7722","thickness":1}},
        "cutouts":if holes {serde_json::json!([{"over":"B01","kind":"sink","width":500,"depth":520,"setback":60,
            "corner_radius":20,"tap_hole":35,"tap_hole_behind":55}])} else {serde_json::json!([])}
    }))
    .unwrap();
    build_run(&Run {
        origin: Vec3::default(),
        items: vec![RunItem::Cabinet(Box::new(cab)), RunItem::Benchtop(top)],
    })
    .unwrap()
}

fn contains(list: &PartList, p: &Part, at: Vec3) -> bool {
    let inside = |b: Aabb| {
        Axis::ALL
            .iter()
            .all(|&a| at.get(a) > b.min.get(a) + 1e-6 && at.get(a) < b.max().get(a) - 1e-6)
    };
    inside(p.bounds)
        && !list.operations.iter().any(|op| match op {
            Operation::Groove { part, bounds } if part == &p.name => inside(*bounds),
            Operation::Chamfer {
                part,
                at: corner,
                bounds,
            } if part == &p.name && inside(*bounds) => {
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
}

#[test]
fn seating_lining_closes_only_the_cantilever_and_bears_on_the_rear_support() {
    for depth in [40.0, 60.0, 80.0] {
        for shadow in [15.0, 20.0, 30.0] {
            for falls in [false, true] {
                let lists = build(depth, shadow, 300.0, falls, false);
                let list = lists.last().unwrap();
                let cap = list.part("BT1.Underlining1").unwrap();
                let slab = list.part("BT1.Benchtop1").unwrap();
                let rail = list.part("BT1.ShadowRail1Back").unwrap();
                assert_eq!(cap.thickness(), slab.thickness());
                assert_eq!(cap.bounds.min.z, slab.bounds.max().z - depth);
                assert_eq!(cap.bounds.max().y, slab.bounds.max().y);
                assert_eq!(cap.bounds.min.y, rail.bounds.min.y);
                assert!(
                    cap.bounds.min.y > slab.bounds.min.y + 400.0,
                    "lining covered the cabinet interior"
                );
                let bearing = Vec3::new(450.0, cap.bounds.min.y + 5.0, cap.bounds.min.z - 0.01);
                assert!(
                    list.parts
                        .iter()
                        .any(|p| p.role == PartRole::BenchtopSupport && contains(list, p, bearing)),
                    "no bearing at {depth}/{shadow}/{falls}"
                );
                if let Some(upper) = list.part("BT1.Packer1Back") {
                    assert_eq!(upper.bounds.min.z, cap.bounds.max().z);
                }
                let cut = cabinet_core::cutlist::cut_list(&lists, "Generic", 1.0);
                let row = cut.rows.iter().find(|r| r.part == cap.name).unwrap();
                assert!(row.templated);
                assert_eq!(row.cut_length, row.length);
                // Sample all intersections between stone/supports/tape, including finite lower mitres.
                let parts: Vec<_> = list
                    .parts
                    .iter()
                    .filter(|p| {
                        matches!(
                            p.role,
                            PartRole::Benchtop | PartRole::BenchtopSupport | PartRole::EdgeBand
                        )
                    })
                    .collect();
                for (i, a) in parts.iter().enumerate() {
                    for b in &parts[i + 1..] {
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
                        if Axis::ALL.iter().any(|&v| hi.get(v) <= lo.get(v)) {
                            continue;
                        }
                        for x in 0..5 {
                            for y in 0..5 {
                                for z in 0..5 {
                                    let at = Vec3::new(
                                        lo.x + (hi.x - lo.x) * (x as f64 + 0.37) / 5.0,
                                        lo.y + (hi.y - lo.y) * (y as f64 + 0.37) / 5.0,
                                        lo.z + (hi.z - lo.z) * (z as f64 + 0.37) / 5.0,
                                    );
                                    assert!(
                                        !(contains(list, a, at) && contains(list, b, at)),
                                        "{depth}/{shadow}/{falls}: {} overlaps {} at {at:?}",
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
    }
    assert!(
        build(60.0, 20.0, 0.0, false, false)
            .last()
            .unwrap()
            .part("BT1.Underlining1")
            .is_none()
    );
}

#[test]
fn exposed_shadow_rail_ends_are_banded_and_stock_includes_all_tape_deductions() {
    let lists = build(60.0, 20.0, 300.0, false, false);
    let list = lists.last().unwrap();
    let cut = cabinet_core::cutlist::cut_list(&lists, "Generic", 1.0);
    for side in ["", "Back"] {
        let rail = list.part(&format!("BT1.ShadowRail1{side}")).unwrap();
        assert!(rail.edges.left && rail.edges.right);
        for end in ["Left", "Right"] {
            assert!(
                list.part(&format!("BT1.ShadowBand1{side}End{end}"))
                    .is_some()
            );
        }
        let row = cut.rows.iter().find(|r| r.part == rail.name).unwrap();
        assert_eq!(row.length - row.cut_length, 2.0);
        assert_eq!(row.width - row.cut_width, 1.0);
        assert_eq!(row.cut_length, rail.bounds.size.x);
        assert_eq!(row.cut_width, rail.bounds.size.y);
    }
}

#[test]
fn appliance_and_tap_holes_continue_through_the_lower_skin() {
    let lists = build(60.0, 20.0, 300.0, false, true);
    let list = lists.last().unwrap();
    let cap = list.part("BT1.Underlining1").unwrap();
    assert!(list.operations.iter().any(|o|matches!(o,Operation::Cutout{part,bounds,radius,..} if part==&cap.name && *radius==20.0 && bounds.min.y<cap.bounds.min.y && bounds.size.z==20.0)));
    assert!(list.operations.iter().any(|o|matches!(o,Operation::Bore{part,purpose:BorePurpose::TapHole,at,depth,..} if part==&cap.name && at.z==cap.bounds.max().z && *depth==20.0)));
}

#[test]
fn waterfall_rails_reach_the_stone_despite_the_shadow_recess() {
    let lists = build(60.0, 15.0, 300.0, true, false);
    let list = lists.last().unwrap();
    for rail in ["BT1.ShadowRail1", "BT1.ShadowRail1Back"] {
        let p = list.part(rail).unwrap();
        assert_eq!(
            p.bounds.min.x - 1.0,
            list.part("BT1.Waterfall1L").unwrap().bounds.max().x
        );
        assert_eq!(
            p.bounds.max().x + 1.0,
            list.part("BT1.Waterfall1R").unwrap().bounds.min.x
        );
    }
    assert_eq!(list.part("BT1.ShadowBand1").unwrap().bounds.min.y, -8.0);
}
