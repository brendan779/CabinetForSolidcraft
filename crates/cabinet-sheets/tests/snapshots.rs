//! Cut sheet snapshots: every part of the default base cabinet, so a layout or dimensioning change
//! shows as an SVG diff (`cargo insta review`).

use cabinet_core::{Cabinet, CabinetKind, build_cabinet};

#[test]
fn base_cabinet_sheets_match_their_snapshots() {
    let list = build_cabinet(&Cabinet::new("B01", CabinetKind::Base)).unwrap();
    for part in &list.parts {
        let svg = cabinet_sheets::part_sheet(
            &list,
            part,
            "Polytec Classic White Matt 16 MR MDF",
            "Snapshot job",
        );
        insta::assert_snapshot!(part.name.clone(), svg);
    }
}
