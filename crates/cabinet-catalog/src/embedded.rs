//! The board and edge tape catalogue built into the program, for the SolidCraft plugin, which has
//! no catalogue folder to read (CabinetStudioPlugin's docs/PLAN.md, PLUG-2). Cabinet Studio reads
//! the folder with [`crate::Catalog::load`].
use crate::{BrandFile, Catalog};

/// Every file under `catalog/boards` and `catalog/edge-tape`. A test checks none is left out.
const FILES: &[(&str, &str)] = &[
    (
        "boards/dekton.json",
        include_str!("../../../catalog/boards/dekton.json"),
    ),
    (
        "boards/egger.json",
        include_str!("../../../catalog/boards/egger.json"),
    ),
    (
        "boards/formica.json",
        include_str!("../../../catalog/boards/formica.json"),
    ),
    (
        "boards/laminex-benchtops.json",
        include_str!("../../../catalog/boards/laminex-benchtops.json"),
    ),
    (
        "boards/laminex.json",
        include_str!("../../../catalog/boards/laminex.json"),
    ),
    (
        "boards/nikpol.json",
        include_str!("../../../catalog/boards/nikpol.json"),
    ),
    (
        "boards/polytec-benchtops.json",
        include_str!("../../../catalog/boards/polytec-benchtops.json"),
    ),
    (
        "boards/polytec.json",
        include_str!("../../../catalog/boards/polytec.json"),
    ),
    (
        "boards/smartstone.json",
        include_str!("../../../catalog/boards/smartstone.json"),
    ),
    (
        "boards/trade-essentials.json",
        include_str!("../../../catalog/boards/trade-essentials.json"),
    ),
    (
        "boards/zenith.json",
        include_str!("../../../catalog/boards/zenith.json"),
    ),
];

/// The built-in boards and edge tapes, checked as a loaded folder is.
pub fn catalog() -> Result<Catalog, crate::Error> {
    let mut cat = Catalog::default();
    for (name, text) in FILES {
        let file: BrandFile = serde_json::from_str(text).map_err(|source| crate::Error::Json {
            path: (*name).into(),
            source,
        })?;
        cat.boards.extend(file.boards);
        cat.edge_tapes.extend(file.edge_tapes);
    }
    Ok(cat)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_board_and_tape_file_is_built_in() {
        let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../catalog");
        for sub in ["boards", "edge-tape"] {
            for e in std::fs::read_dir(format!("{root}/{sub}")).unwrap() {
                let name = e.unwrap().file_name().into_string().unwrap();
                if name.ends_with(".json") {
                    let key = format!("{sub}/{name}");
                    assert!(
                        FILES.iter().any(|(n, _)| *n == key),
                        "{key} is not in embedded::FILES"
                    );
                }
            }
        }
    }

    #[test]
    fn the_built_in_catalogue_is_the_folders() {
        let root = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../catalog"));
        let loaded = Catalog::load(root).unwrap();
        let built = catalog().unwrap();
        assert_eq!(built.boards, loaded.boards);
        assert_eq!(built.edge_tapes, loaded.edge_tapes);
    }
}
