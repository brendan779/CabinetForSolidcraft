use cabinet_core::geom::{Axis, Vec3};
use cabinet_core::part::{Operation, PartRole};
use cabinet_core::run::{Benchtop, Run, RunItem, build_run};
use cabinet_core::{Cabinet, CabinetKind, PartList};

fn cabinet(id: &str, width: f64) -> Cabinet {
    let mut c = Cabinet::new(id, CabinetKind::Base);
    c.width = width;
    c.cladding.back = true;
    c.cladding.thickness = 18.0;
    c
}
fn top(falls: bool, overhang: f64) -> Benchtop {
    serde_json::from_value(serde_json::json!({"id":"BT1","thickness":20,"front_overhang":0,"side_overhang":0,
        "back_overhang":overhang,"waterfall_left":falls,"waterfall_right":falls,
        "edge":{"type":"mitred_apron","depth":60},"build_up":{"shadow_height":15,"setback":10,"strip_width":30,
        "board_thickness":18,"edge_tape":{"id":"accent","colour":"#B59A61","thickness":1}}})).unwrap()
}
fn build(cabs: Vec<Cabinet>, b: Option<Benchtop>) -> Vec<PartList> {
    let mut items: Vec<_> = cabs
        .into_iter()
        .map(|c| RunItem::Cabinet(Box::new(c)))
        .collect();
    if let Some(b) = b {
        items.push(RunItem::Benchtop(b));
    }
    build_run(&Run {
        origin: Vec3::new(100.0, 200.0, 50.0),
        items,
    })
    .unwrap()
}

#[test]
fn rear_supports_stay_on_the_cabinet_when_the_stone_overhang_changes() {
    let mut previous = None;
    for overhang in [0.0, 300.0, 600.0] {
        let lists = build(
            vec![cabinet("B01", 900.0), cabinet("B02", 900.0)],
            Some(top(true, overhang)),
        );
        let top = lists.last().unwrap();
        let back = top.part("BT1.ShadowRail1Back").unwrap();
        let packer = top.part("BT1.Packer1Back").unwrap();
        let face = lists[0].part("B01.SeenBack").unwrap().bounds;
        assert_eq!(
            back.bounds.min.z,
            face.max().z,
            "rail must sit on the cabinet"
        );
        assert_eq!(
            packer.bounds.min.z,
            top.part("BT1.Underlining1")
                .map_or(back.bounds.max().z, |p| p.bounds.max().z),
            "rear packer must touch the rail or its stone lining"
        );
        assert_eq!(packer.bounds.min.y, back.bounds.min.y);
        assert_eq!(
            top.part("BT1.ShadowBand1Back").unwrap().bounds.max().y,
            face.max().y - 10.0
        );
        let shape = (
            back.bounds.min.y,
            back.bounds.max().y,
            packer.bounds.min.y,
            packer.bounds.max().y,
        );
        if let Some(previous) = previous {
            assert_eq!(shape, previous, "stone overhang moved its support");
        }
        previous = Some(shape);
        assert!(
            top.part("BT1.Apron1Back").is_some(),
            "exposed rear needs a stone return"
        );
    }
}

#[test]
fn removing_waterfalls_keeps_all_four_shadow_edges_and_supported_returns() {
    let lists = build(
        vec![cabinet("B01", 900.0), cabinet("B02", 900.0)],
        Some(top(false, 300.0)),
    );
    let list = lists.last().unwrap();
    for side in ["", "Back", "Left", "Right"] {
        let rail = list.part(&format!("BT1.ShadowRail1{side}")).unwrap();
        assert_eq!(rail.bounds.min.z, 920.0);
        assert!(list.part(&format!("BT1.ShadowBand1{side}")).is_some());
        assert!(
            list.part(&format!(
                "BT1.Packer1{}",
                if side.is_empty() { "Front" } else { side }
            ))
            .is_some()
        );
    }
    assert!(list.part("BT1.Waterfall1L").is_none());
    assert!(list.part("BT1.Apron1L").is_some());
    assert!(
        list.operations.iter().any(
            |op| matches!(op,Operation::Groove{part,..} if part.starts_with("BT1.ShadowRail"))
        ),
        "flush stone needs a clearance notch, not intersecting wood"
    );
    let cut = cabinet_core::cutlist::cut_list(&lists, "Generic", 1.0);
    for rail in cut.rows.iter().filter(|r| r.part.contains("ShadowRail")) {
        assert!(rail.description.contains("edgeband accent"));
        assert_eq!(
            rail.thickness, 18.0,
            "notches do not invent a different stock board"
        );
    }
}

#[test]
fn matching_seen_backs_are_one_panel_with_across_run_grain() {
    let lists = build(vec![cabinet("B01", 900.0), cabinet("B02", 900.0)], None);
    let backs: Vec<_> = lists
        .iter()
        .flat_map(|l| &l.parts)
        .filter(|p| p.name.contains(".SeenBack"))
        .collect();
    assert_eq!(backs.len(), 1);
    assert_eq!(backs[0].bounds.size, Vec3::new(1800.0, 18.0, 870.0));
    assert_eq!(backs[0].grain_axis, Some(Axis::X));
    assert_eq!(backs[0].role, PartRole::Cladding);
    let cut = cabinet_core::cutlist::cut_list(&lists, "Generic", 1.0);
    let back = cut
        .rows
        .iter()
        .find(|r| r.part.contains(".SeenBack"))
        .unwrap();
    assert_eq!((back.length, back.width), (1800.0, 870.0));
}

#[test]
fn long_seen_backs_join_at_a_cabinet_boundary_within_the_chosen_maximum() {
    for (maximum, widths) in [
        (2700.0, vec![2700.0, 900.0]),
        (1800.0, vec![1800.0, 1800.0]),
    ] {
        let cabs = (1..=4)
            .map(|i| {
                let mut c = cabinet(&format!("B{i:02}"), 900.0);
                c.cladding.back_max_width = maximum;
                c
            })
            .collect();
        let lists = build(cabs, None);
        let backs: Vec<_> = lists
            .iter()
            .flat_map(|l| &l.parts)
            .filter(|p| p.name.contains(".SeenBack"))
            .collect();
        assert_eq!(
            backs.iter().map(|p| p.bounds.size.x).collect::<Vec<_>>(),
            widths
        );
        assert_eq!(backs[0].bounds.max().x, backs[1].bounds.min.x);
    }
}

#[test]
fn rear_faces_stop_at_unselected_cabinets_and_changes_of_material_or_height() {
    for change in [0, 1, 2, 3, 4, 5] {
        let a = cabinet("B01", 900.0);
        let mut b = cabinet("B02", 900.0);
        match change {
            0 => b.cladding.back = false,
            1 => b.cladding.board = "other".into(),
            2 => b.height += 20.0,
            3 => b.cladding.back_grain = Axis::Z,
            4 => b.cladding.thickness += 4.0,
            _ => b.cladding.back_max_width += 100.0,
        };
        let lists = build(vec![a, b], None);
        assert_eq!(
            lists[0].part("B01.SeenBack").unwrap().bounds.size.x,
            900.0,
            "change {change}"
        );
    }
}

#[test]
fn different_depth_cabinets_still_share_the_same_rear_plane() {
    let a = cabinet("B01", 900.0);
    let mut b = cabinet("B02", 900.0);
    b.depth += 20.0;
    let lists = build(vec![a, b], None);
    assert_eq!(lists[0].part("B01.SeenBack").unwrap().bounds.size.x, 1800.0);
}

#[test]
fn a_wall_unit_above_the_run_does_not_split_the_continuous_seen_back() {
    let a = cabinet("B01", 900.0);
    let b = cabinet("B02", 900.0);
    let mut wall = Cabinet::new("W01", CabinetKind::Wall);
    wall.cladding.back = true;
    let lists = build(vec![a, wall, b], None);
    assert_eq!(lists[0].part("B01.SeenBack").unwrap().bounds.size.x, 1800.0);
    assert!(lists[1].part("W01.SeenBack").is_some());
}

#[test]
fn a_shared_rear_panel_does_not_move_the_sink_cutout_off_its_own_cabinet() {
    let mut a = cabinet("S01", 900.0);
    a.kind = CabinetKind::Sink;
    let mut top = top(false, 300.0);
    top.cutouts.push(
        serde_json::from_value(
            serde_json::json!({"over":"S01","kind":"sink","width":600,"depth":400}),
        )
        .unwrap(),
    );
    let lists = build(vec![a, cabinet("B02", 900.0)], Some(top));
    let hole = lists
        .last()
        .unwrap()
        .operations
        .iter()
        .find_map(|op| match op {
            Operation::Cutout { part, bounds, .. } if part == "BT1.Benchtop1" => Some(bounds),
            _ => None,
        })
        .unwrap();
    assert_eq!(hole.center().x, 550.0);
}
