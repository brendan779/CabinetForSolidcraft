//! Golden part lists: one per cabinet kind at the default construction standard. A change to any
//! construction rule shows here as a reviewable diff (`cargo insta review`).

use cabinet_core::{Cabinet, CabinetKind, build_cabinet};

#[test]
fn every_kind_builds_its_golden_part_list() {
    for kind in CabinetKind::ALL {
        let list = build_cabinet(&Cabinet::new("C01", kind)).unwrap();
        insta::assert_json_snapshot!(format!("{kind:?}"), list);
    }
}

/// SEC-1: a sectioned wardrobe, built from JSON as the app sends it: drawers under hanging on the
/// left, a shoe rack under shelves on the right.
#[test]
fn a_sectioned_wardrobe_builds_its_golden_part_list() {
    let mut cab = Cabinet::new("W01", CabinetKind::Tall);
    cab.width = 1200.0;
    cab.adjustable_shelves = 0;
    cab.fronts = serde_json::from_value(serde_json::json!({
        "type": "sections",
        "columns": [
            {"width": 600, "rows": [
                {"height": 600, "front": {"type": "drawers", "count": 2}},
                {"front": {"type": "door_pair"}, "inside": {"type": "hanging"}}
            ]},
            {"rows": [
                {"height": 900, "front": {"type": "door", "hinge_side": "right"},
                 "inside": {"type": "shoe_shelves", "count": 4}},
                {"front": {"type": "door", "hinge_side": "right"},
                 "inside": {"type": "shelves", "count": 2}}
            ]}
        ]
    }))
    .unwrap();
    let list = build_cabinet(&cab).unwrap();
    assert!(
        !list.warnings.iter().any(|w| w.contains("clash")),
        "{:?}",
        list.warnings
    );
    insta::assert_json_snapshot!("SectionedWardrobe", list);
}
