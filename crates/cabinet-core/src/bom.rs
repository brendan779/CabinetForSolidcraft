//! The hardware bill of materials: every hardware placement of a job, counted per item.

use crate::cutlist::csv;
use crate::part::PartList;

/// One line of the bill: an item (with one description) and how many the job needs.
#[derive(Clone, Debug, PartialEq)]
pub struct BomRow {
    /// Hardware id the builder placed (`blum.clip-top-blumotion.110.full`).
    pub item: String,
    pub description: String,
    pub quantity: u32,
    /// The cabinets that use it, in job order.
    pub cabinets: Vec<String>,
}

/// The job's hardware, one row per item and description (a rail cut to 600 and one cut to 900
/// are two rows), sorted by item.
pub fn hardware_bom(lists: &[PartList]) -> Vec<BomRow> {
    let mut rows: Vec<BomRow> = Vec::new();
    for list in lists {
        for h in &list.hardware {
            match rows
                .iter_mut()
                .find(|r| r.item == h.item && r.description == h.description)
            {
                Some(r) => {
                    r.quantity += h.quantity;
                    if !r.cabinets.contains(&list.cabinet) {
                        r.cabinets.push(list.cabinet.clone());
                    }
                }
                None => rows.push(BomRow {
                    item: h.item.clone(),
                    description: h.description.clone(),
                    quantity: h.quantity,
                    cabinets: vec![list.cabinet.clone()],
                }),
            }
        }
    }
    rows.sort_by(|a, b| (&a.item, &a.description).cmp(&(&b.item, &b.description)));
    rows
}

/// The bill as CSV. `article` gives an item's brand and article number from the hardware catalog;
/// items not catalogued yet are marked so.
pub fn bom_csv(rows: &[BomRow], article: impl Fn(&str) -> Option<(String, String)>) -> String {
    let mut out = String::from("Item,Brand,Article,Description,Qty,Cabinets\n");
    for r in rows {
        let (brand, art) =
            article(&r.item).unwrap_or_else(|| (String::new(), NOT_CATALOGUED.into()));
        out.push_str(&format!(
            "{},{},{},{},{},{}\n",
            csv(&r.item),
            csv(&brand),
            csv(&art),
            csv(&r.description),
            r.quantity,
            csv(&r.cabinets.join(" "))
        ));
    }
    out
}

/// The article column for an item the hardware catalog has no record of yet.
pub const NOT_CATALOGUED: &str = "not catalogued";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{FrontLayout, ToeKick};
    use crate::{Cabinet, CabinetKind, build_cabinet};

    /// Legs only under B02; B01 has one door and B02 a pair.
    fn job() -> Vec<PartList> {
        let cab = |id: &str, kind, kick| {
            let mut c = Cabinet::new(id, kind);
            c.standard.toe_kick = kick;
            c
        };
        let mut b01 = cab("B01", CabinetKind::Base, ToeKick::IntegratedNotch);
        b01.fronts = FrontLayout::Door {
            hinge_side: crate::model::HingeSide::Left,
        };
        let mut b02 = cab("B02", CabinetKind::Base, ToeKick::LegsAndPlinth);
        b02.fronts = FrontLayout::DoorPair;
        let d01 = cab("D01", CabinetKind::DrawerBank, ToeKick::IntegratedNotch);
        [b01, b02, d01]
            .iter()
            .map(|c| build_cabinet(c).unwrap())
            .collect()
    }

    #[test]
    fn the_bill_counts_every_placement_once_and_names_the_cabinets() {
        let lists = job();
        let rows = hardware_bom(&lists);
        let placed: u32 = lists
            .iter()
            .flat_map(|l| &l.hardware)
            .map(|h| h.quantity)
            .sum();
        assert_eq!(rows.iter().map(|r| r.quantity).sum::<u32>(), placed);
        let hinges: u32 = rows
            .iter()
            .filter(|r| r.item.starts_with("blum.clip-top"))
            .map(|r| r.quantity)
            .sum();
        assert_eq!(
            hinges,
            2 + 4,
            "one door of two hinges and a pair of two each"
        );
        let legs = rows
            .iter()
            .find(|r| r.item == "hafele.leg.adjustable")
            .unwrap();
        assert_eq!(legs.cabinets, ["B02"]);
        let runners: u32 = rows
            .iter()
            .filter(|r| r.item.starts_with("blum.legrabox.cabinet-profile."))
            .map(|r| r.quantity)
            .sum();
        assert_eq!(runners, 3);
    }

    #[test]
    fn items_without_a_catalog_record_say_so_in_the_csv() {
        let rows = hardware_bom(&job());
        let csv = bom_csv(&rows, |item| {
            (item == "hafele.leg.adjustable").then(|| ("Brand".to_string(), "TEST-1".to_string()))
        });
        assert!(csv.starts_with("Item,Brand,Article,Description,Qty,Cabinets\n"));
        assert!(csv.contains("hafele.leg.adjustable,Brand,TEST-1,"), "{csv}");
        assert!(csv.contains(&format!(",,{NOT_CATALOGUED},")));
    }
}
