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
