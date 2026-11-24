use cabinet_core::model::{
    FrontLayout, Hanging, Section, SectionColumn, SectionFront, SectionInside,
};
use cabinet_core::part::PartRole;
use cabinet_core::{Cabinet, CabinetKind, build_cabinet};

fn row(height: f64, front: SectionFront, inside: SectionInside) -> Section {
    Section {
        height,
        front,
        inside,
        shelf_above: true,
        opening: None,
    }
}

/// The owner's walk-in robe: double hanging under a 250 mm open top section.
fn walk_in(hanging: Hanging) -> cabinet_core::PartList {
    let mut cab = Cabinet::new("W01", CabinetKind::Wardrobe);
    cab.height = 2400.0;
    cab.wardrobe = hanging;
    cab.fronts = FrontLayout::Sections {
        columns: vec![SectionColumn {
            width: 0.0,
            rows: vec![
                row(0.0, SectionFront::Open, SectionInside::Wardrobe),
                row(250.0, SectionFront::Open, SectionInside::Empty),
            ],
        }],
    };
    build_cabinet(&cab).unwrap()
}

#[test]
fn hanging_under_a_section_shelf_adds_no_hat_shelf_above_it() {
    for hanging in [Hanging::Long, Hanging::Double] {
        let list = walk_in(hanging);
        let shelves: Vec<_> = list.by_role(PartRole::Shelf).collect();
        // The section shelf is the top of the hanging; double hanging adds its lower shelf.
        let section_shelf = shelves.iter().map(|p| p.bounds.min.z).fold(0.0, f64::max);
        assert_eq!(
            shelves.len(),
            if hanging == Hanging::Double { 2 } else { 1 },
            "{hanging:?}"
        );
        let rails: Vec<_> = list
            .hardware
            .iter()
            .filter(|h| h.item == "wardrobe-rail.oval")
            .collect();
        assert!(
            rails.iter().all(|r| r.at.z < section_shelf),
            "{hanging:?}: a rail above the section"
        );
        assert_eq!(rails.len(), shelves.len(), "{hanging:?}");
    }
}

#[test]
fn every_shelf_of_a_sectioned_wardrobe_has_its_own_name() {
    let list = walk_in(Hanging::Double);
    let mut names: Vec<_> = list.parts.iter().map(|p| p.name.as_str()).collect();
    let all = names.len();
    names.sort();
    names.dedup();
    assert_eq!(names.len(), all);
}

#[test]
fn hanging_under_the_carcass_top_gets_its_hat_shelf() {
    let mut cab = Cabinet::new("W01", CabinetKind::Wardrobe);
    cab.height = 2400.0;
    cab.fronts = FrontLayout::Sections {
        columns: vec![SectionColumn {
            width: 0.0,
            rows: vec![row(0.0, SectionFront::DoorPair, SectionInside::Wardrobe)],
        }],
    };
    let list = build_cabinet(&cab).unwrap();
    let t = cab.standard.carcass_thickness;
    let hat = list.by_role(PartRole::Shelf).next().unwrap();
    assert_eq!(
        hat.bounds.min.z,
        2400.0 - t - cab.standard.wardrobe_hat_shelf_drop
    );
}
