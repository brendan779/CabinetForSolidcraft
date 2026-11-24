use cabinet_core::model::{DoorOverlay, FrontLayout, HingeSide};
use cabinet_core::part::PartRole;
use cabinet_core::{Cabinet, CabinetKind, build_cabinet};

#[test]
fn no_loose_shelf_sits_where_a_hinge_is() {
    // A CLIP top hinge and its plate take 57 mm of the side (Blum's models).
    let half = 28.5;
    let mut checked = 0;
    for kind in [CabinetKind::Base, CabinetKind::Wall, CabinetKind::Tall] {
        for overlay in [DoorOverlay::Full, DoorOverlay::Inset] {
            for shelves in 1..=4 {
                for (width, layout) in [
                    (
                        600.0,
                        FrontLayout::Door {
                            hinge_side: HingeSide::Left,
                        },
                    ),
                    (900.0, FrontLayout::DoorPair),
                ] {
                    let mut cab = Cabinet::new("C01", kind);
                    cab.width = width;
                    cab.fronts = layout;
                    cab.standard.overlay = overlay;
                    cab.adjustable_shelves = shelves;
                    let cab = cab.sectioned();
                    let list = build_cabinet(&cab).unwrap();
                    let hinges: Vec<f64> = list
                        .hardware
                        .iter()
                        .filter(|h| h.item.starts_with("blum.clip-top") && h.item.contains(".110"))
                        .map(|h| h.at.z)
                        .collect();
                    for s in list.by_role(PartRole::Shelf) {
                        for &z in &hinges {
                            let clear = s.bounds.max().z <= z - half || s.bounds.min.z >= z + half;
                            assert!(
                                clear,
                                "{kind:?} {overlay:?} {shelves} shelves: {} at {} meets a hinge at {z}",
                                s.name, s.bounds.min.z
                            );
                        }
                        checked += 1;
                    }
                }
            }
        }
    }
    assert!(checked > 50);
}
