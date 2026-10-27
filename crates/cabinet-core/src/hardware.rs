//! The supplier hardware data the builders read, compiled in from `catalog/hardware/` so the core
//! stays free of I/O. Every supplier number a rule uses comes from here, with its source.

use std::sync::OnceLock;

use cabinet_catalog::{HardwareFile, HardwareItem};

/// The hardware files the builders depend on. The app loads the same files for the BOM.
const FILES: &[(&str, &str)] = &[
    (
        "blum-clip-top.json",
        include_str!("../../../catalog/hardware/blum-clip-top.json"),
    ),
    (
        "blum-legrabox.json",
        include_str!("../../../catalog/hardware/blum-legrabox.json"),
    ),
    (
        "blum-aventos-hk.json",
        include_str!("../../../catalog/hardware/blum-aventos-hk.json"),
    ),
    (
        "blum-ambia-line.json",
        include_str!("../../../catalog/hardware/blum-ambia-line.json"),
    ),
    (
        "blum-cabloxx.json",
        include_str!("../../../catalog/hardware/blum-cabloxx.json"),
    ),
    (
        "blum-orga-line.json",
        include_str!("../../../catalog/hardware/blum-orga-line.json"),
    ),
    (
        "blum-aventos-hki.json",
        include_str!("../../../catalog/hardware/blum-aventos-hki.json"),
    ),
    (
        "blum-aventos-hk-s-xs.json",
        include_str!("../../../catalog/hardware/blum-aventos-hk-s-xs.json"),
    ),
    (
        "blum-aventos-hs-hl.json",
        include_str!("../../../catalog/hardware/blum-aventos-hs-hl.json"),
    ),
    (
        "blum-aventos-hf.json",
        include_str!("../../../catalog/hardware/blum-aventos-hf.json"),
    ),
    (
        "blum-hanging.json",
        include_str!("../../../catalog/hardware/blum-hanging.json"),
    ),
    (
        "blum-drawer-motion.json",
        include_str!("../../../catalog/hardware/blum-drawer-motion.json"),
    ),
    (
        "festool-domino.json",
        include_str!("../../../catalog/hardware/festool-domino.json"),
    ),
    (
        "blum-tip-on.json",
        include_str!("../../../catalog/hardware/blum-tip-on.json"),
    ),
    (
        "blum-merivobox.json",
        include_str!("../../../catalog/hardware/blum-merivobox.json"),
    ),
    (
        "blum-tandembox.json",
        include_str!("../../../catalog/hardware/blum-tandembox.json"),
    ),
    (
        "blum-movento-tandem.json",
        include_str!("../../../catalog/hardware/blum-movento-tandem.json"),
    ),
    (
        "hafele-axilo.json",
        include_str!("../../../catalog/hardware/hafele-axilo.json"),
    ),
    (
        "clipsal-power-points.json",
        include_str!("../../../catalog/hardware/clipsal-power-points.json"),
    ),
];

/// Every bundled hardware item. The files are checked by this crate's tests, so a parse failure
/// here is a broken build, not a user error.
pub fn items() -> &'static [HardwareItem] {
    static ITEMS: OnceLock<Vec<HardwareItem>> = OnceLock::new();
    ITEMS.get_or_init(|| {
        FILES
            .iter()
            .flat_map(|(name, text)| {
                serde_json::from_str::<HardwareFile>(text)
                    .unwrap_or_else(|e| panic!("bundled catalog/hardware/{name}: {e}"))
                    .items
            })
            .collect()
    })
}

/// A bundled item by id, or an error naming it.
pub fn item(id: &str) -> Result<&'static HardwareItem, String> {
    items()
        .iter()
        .find(|h| h.id == id)
        .ok_or_else(|| format!("hardware catalog has no {id}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_hardware_parses_with_unique_ids_and_sources() {
        let all = items();
        assert!(!all.is_empty());
        let mut ids: Vec<_> = all.iter().map(|h| h.id.as_str()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), all.len());
        assert!(
            all.iter()
                .all(|h| !h.source.url.is_empty() && !h.source.verified)
        );
    }
}
