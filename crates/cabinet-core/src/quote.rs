//! Quotes: what a job costs to make, from the shop's own price list. Board is priced by the
//! sheets the nesting opens (kept offcuts cost nothing), edge tape by the metre, hardware by the
//! piece, labour by time per part, per cabinet and per hardware item; then markup and tax.
//!
//! Prices are the shop's, never supplier data: a price list is the owner's file.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::bom::BomRow;
use crate::cutlist::CutList;
use crate::nesting::Nesting;

/// The shop's prices and rates. Any price left out is reported as missing, not guessed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PriceList {
    pub currency: String,
    /// Board price per square metre of sheet, by catalog board id (`Generic` for parts with
    /// none).
    pub boards: BTreeMap<String, f64>,
    /// Edge tape per metre.
    pub edge_tape_per_m: Option<f64>,
    /// Hardware price each, by hardware item id.
    pub hardware: BTreeMap<String, f64>,
    pub labour_per_hour: Option<f64>,
    pub minutes_per_part: f64,
    pub minutes_per_cabinet: f64,
    pub minutes_per_hardware_item: f64,
    /// Added to the cost, as a fraction (0.3 = 30 %).
    pub markup: f64,
    /// Sales tax on the marked-up price, as a fraction (Australian GST 0.1).
    pub tax: f64,
}

impl Default for PriceList {
    /// No prices; the times and GST are ours, for the owner to change.
    fn default() -> Self {
        Self {
            currency: "AUD".into(),
            boards: BTreeMap::new(),
            edge_tape_per_m: None,
            hardware: BTreeMap::new(),
            labour_per_hour: None,
            minutes_per_part: 6.0,
            minutes_per_cabinet: 30.0,
            minutes_per_hardware_item: 2.0,
            markup: 0.0,
            tax: 0.1,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Section {
    Board,
    EdgeTape,
    Hardware,
    Labour,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct QuoteLine {
    pub section: Section,
    /// What its price is kept under: the board or hardware id (`edge_tape`, `labour` for those).
    pub key: String,
    pub description: String,
    pub quantity: f64,
    pub unit: String,
    /// `None` when the price list has no price for it.
    pub unit_price: Option<f64>,
    pub total: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Quote {
    pub currency: String,
    pub lines: Vec<QuoteLine>,
    pub cost: f64,
    pub markup: f64,
    pub tax: f64,
    pub total: f64,
    /// What has no price yet: the quote is short by these.
    pub missing: Vec<String>,
}

/// The quote for a job: its nesting (board), cut list (edge tape, parts), hardware bill and the
/// number of cabinets.
pub fn quote(
    nesting: &Nesting,
    cut: &CutList,
    bom: &[BomRow],
    cabinets: usize,
    prices: &PriceList,
) -> Quote {
    let mut lines = Vec::new();
    let mut missing = Vec::new();
    let mut line =
        |section, key: &str, description: String, quantity: f64, unit: &str, price: Option<f64>| {
            if price.is_none() {
                missing.push(description.clone());
            }
            lines.push(QuoteLine {
                section,
                key: key.into(),
                description,
                quantity,
                unit: unit.into(),
                unit_price: price,
                total: price.map_or(0.0, |p| p * quantity),
            });
        };
    // Board: the area of the new sheets each board and thickness takes.
    let mut sheets: BTreeMap<(String, i64), (f64, f64, u32)> = BTreeMap::new();
    for s in nesting.sheets.iter().filter(|s| !s.from_offcut) {
        let e = sheets
            .entry((s.board.clone(), (s.thickness * 100.0).round() as i64))
            .or_insert((s.thickness, 0.0, 0));
        e.1 += s.stock.length * s.stock.width / 1e6;
        e.2 += 1;
    }
    for ((board, _), (thickness, m2, count)) in sheets {
        line(
            Section::Board,
            &board,
            format!("{board}, {thickness} mm ({count} sheets)"),
            round2(m2),
            "m²",
            prices.boards.get(&board).copied(),
        );
    }
    let tape: f64 = cut.rows.iter().map(|r| r.edge_metres()).sum();
    if tape > 0.0 {
        line(
            Section::EdgeTape,
            "edge_tape",
            "Edge tape".into(),
            round2(tape),
            "m",
            prices.edge_tape_per_m,
        );
    }
    let mut items = 0;
    for r in bom {
        items += r.quantity;
        line(
            Section::Hardware,
            &r.item,
            r.description.clone(),
            r.quantity as f64,
            "each",
            prices.hardware.get(&r.item).copied(),
        );
    }
    let parts: u32 = cut.rows.iter().map(|r| r.quantity).sum();
    let minutes = parts as f64 * prices.minutes_per_part
        + cabinets as f64 * prices.minutes_per_cabinet
        + items as f64 * prices.minutes_per_hardware_item;
    line(
        Section::Labour,
        "labour",
        format!("Labour: {parts} parts, {cabinets} cabinets, {items} hardware items"),
        round2(minutes / 60.0),
        "h",
        prices.labour_per_hour,
    );
    let cost: f64 = lines.iter().map(|l| l.total).sum();
    let markup = cost * prices.markup;
    let tax = (cost + markup) * prices.tax;
    Quote {
        currency: prices.currency.clone(),
        lines,
        cost: round2(cost),
        markup: round2(markup),
        tax: round2(tax),
        total: round2(cost + markup + tax),
        missing,
    }
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nesting::{Settings, Stock, nest};
    use crate::{Cabinet, CabinetKind, build_cabinet};

    fn job() -> (Nesting, CutList, Vec<BomRow>) {
        let lists = vec![build_cabinet(&Cabinet::new("B01", CabinetKind::Base)).unwrap()];
        let cut = crate::cutlist::cut_list(&lists, "Generic", 1.0);
        let n = nest(
            &cut.rows,
            |_, _| {
                vec![Stock {
                    length: 2400.0,
                    width: 1200.0,
                    grained: false,
                }]
            },
            &Settings::default(),
        );
        (n, cut, crate::bom::hardware_bom(&lists))
    }

    #[test]
    fn board_is_priced_by_the_sheets_the_nesting_opens() {
        let (n, cut, bom) = job();
        let mut prices = PriceList::default();
        prices.boards.insert("Generic".into(), 20.0);
        let q = quote(&n, &cut, &bom, 1, &prices);
        let board: f64 = q
            .lines
            .iter()
            .filter(|l| l.section == Section::Board)
            .map(|l| l.total)
            .sum();
        let sheets = n.sheets.len() as f64;
        assert!((board - sheets * 2.88 * 20.0).abs() < 0.05, "{board}");
    }

    #[test]
    fn missing_prices_are_listed_and_count_nothing() {
        let (n, cut, bom) = job();
        let q = quote(&n, &cut, &bom, 1, &PriceList::default());
        assert_eq!(q.cost, 0.0);
        // Every board, the tape, every hardware line and the labour rate.
        assert_eq!(q.missing.len(), q.lines.len());
    }

    #[test]
    fn labour_markup_and_tax_add_up() {
        let (n, cut, bom) = job();
        let prices = PriceList {
            labour_per_hour: Some(60.0),
            minutes_per_part: 6.0,
            minutes_per_cabinet: 30.0,
            minutes_per_hardware_item: 0.0,
            markup: 0.5,
            tax: 0.1,
            ..PriceList::default()
        };
        let q = quote(&n, &cut, &bom, 1, &prices);
        let parts = cut.rows.len() as f64;
        let labour = (parts * 6.0 + 30.0) / 60.0 * 60.0;
        assert!((q.cost - labour).abs() < 0.01);
        assert!((q.markup - labour * 0.5).abs() < 0.01);
        assert!((q.total - labour * 1.5 * 1.1).abs() < 0.02);
    }

    #[test]
    fn kept_offcuts_cost_nothing() {
        let (mut n, cut, bom) = job();
        for s in &mut n.sheets {
            s.from_offcut = true;
        }
        let mut prices = PriceList::default();
        prices.boards.insert("Generic".into(), 20.0);
        let q = quote(&n, &cut, &bom, 1, &prices);
        assert!(!q.lines.iter().any(|l| l.section == Section::Board));
    }
}
