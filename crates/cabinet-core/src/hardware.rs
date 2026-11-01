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
    (
        "benchtop-materials.json",
        include_str!("../../../catalog/hardware/benchtop-materials.json"),
    ),
    (
        "generic-handles.json",
        include_str!("../../../catalog/hardware/generic-handles.json"),
    ),
    (
        "abi-handles.json",
        include_str!("../../../catalog/hardware/abi-handles.json"),
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

/// The handle a job bores and orders. A knob has no centres.
#[derive(Clone, Debug, PartialEq)]
pub struct ChosenHandle {
    pub item: String,
    /// The span of the screw pattern (mm). `None` is a single screw: a knob or T pull.
    pub centres: Option<f64>,
    /// Each screw from the pattern's centre, (along the handle, across it), in order along it.
    pub holes: Vec<(f64, f64)>,
    /// Overall length (mm), which the front must have room for. 0 where only the holes matter.
    pub length: f64,
    pub description: String,
    /// True when this came from the ABI range, so a size that does not fit is left off rather
    /// than bored past the front.
    pub abi: bool,
}

/// Centres below this are a knob: ABI names some knobs with centres 0.
const KNOB_CENTRES: f64 = 1.0;

/// The handle the standard asks for. An ABI style the catalog does not have is an error, so the
/// builder can say so and fall back to the generic handle instead of boring a guess.
pub fn chosen_handle(s: &crate::model::ConstructionStandard) -> Result<ChosenHandle, String> {
    if s.handle_range == crate::model::HandleRange::Abi {
        return abi_handle(&s.handle_style);
    }
    Ok(generic_handle(s))
}

/// The generic bar or knob, from the size and finish enums.
pub fn generic_handle(s: &crate::model::ConstructionStandard) -> ChosenHandle {
    let finish = s.handle_finish.label();
    match s.handle_size.centres() {
        Some(cc) => ChosenHandle {
            item: format!("handle.cc{cc:.0}"),
            centres: Some(cc),
            holes: vec![(-cc / 2.0, 0.0), (cc / 2.0, 0.0)],
            length: 0.0,
            description: format!("Handle, {cc:.0} mm hole centres, {finish}"),
            abi: false,
        },
        None => ChosenHandle {
            item: "handle.knob".into(),
            centres: None,
            holes: vec![(0.0, 0.0)],
            length: 0.0,
            description: format!("Knob, {finish}"),
            abi: false,
        },
    }
}

fn abi_handle(style: &str) -> Result<ChosenHandle, String> {
    if style.is_empty() {
        return Err("ABI handles are chosen but no style is set; using the generic handle".into());
    }
    let item = item(style).map_err(|e| format!("{e}; using the generic handle"))?;
    let centres = item.parameters.get("centres").copied().unwrap_or(0.0);
    let centres = (centres >= KNOB_CENTRES).then_some(centres);
    let sku = item
        .parameters
        .get("sku")
        .map(|n| format!("{n:.0}"))
        .unwrap_or_default();
    // ABI names a pull by its overall length; the screws are what a maker needs to read.
    let length = item.parameters.get("length").copied().unwrap_or(0.0);
    let size = match centres {
        Some(cc) => format!("{length:.0} mm, screws {cc:.0} mm apart"),
        None if item.parameters.contains_key("knob") => "knob".into(),
        None => format!("{length:.0} mm T pull"),
    };
    let description = if sku.is_empty() {
        format!("{}, {size}, {}", item.family, item.variant)
    } else {
        format!("{}, {size}, {}, SKU {sku}", item.family, item.variant)
    };
    Ok(ChosenHandle {
        item: item.id.clone(),
        centres,
        holes: item.drilling.iter().map(|h| (h.x, h.y)).collect(),
        length,
        description,
        abi: true,
    })
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
