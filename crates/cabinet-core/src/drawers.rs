//! Drawer boxes: which drawer system box fits an opening, and the parts it needs cut, from the
//! Blum data in `catalog/hardware/`.

use cabinet_catalog::HardwareItem;

use crate::hardware;

/// LEGRABOX heights, tallest first: the box chosen is the tallest whose space requirement fits.
const LEGRABOX_HEIGHTS: [&str; 3] = ["k", "m", "n"];

/// A LEGRABOX drawer sized for its opening.
#[derive(Clone, Debug, PartialEq)]
pub struct Legrabox {
    /// The planning row (`blum.legrabox.m`): side height, deductions, front drilling.
    pub system: &'static HardwareItem,
    /// N, M or K.
    pub height: String,
    pub nominal_length: f64,
    pub profile: &'static HardwareItem,
    pub sides: &'static HardwareItem,
    pub back_fixing: &'static HardwareItem,
    pub front_fixing: &'static HardwareItem,
    /// Cut sizes in 16 mm board: base width × length, back width × height.
    pub base: (f64, f64),
    pub back: (f64, f64),
    pub board_thickness: f64,
}

/// The tallest LEGRABOX of the longest nominal length that fits an opening `inner_width` (LW)
/// wide, `inner_depth` (LT) deep from the back of the front, with `space` mm from the drawer's
/// datum (the surface its profiles stand on) to the next one up.
pub fn legrabox(inner_width: f64, inner_depth: f64, space: f64) -> Result<Legrabox, String> {
    let mut last_err = format!("no LEGRABOX drawer fits in {space:.0} mm of height");
    for h in LEGRABOX_HEIGHTS {
        let system = hardware::item(&format!("blum.legrabox.{h}"))?;
        if system.param("space_height")? > space + 1e-9 {
            continue;
        }
        let allowance = system.param("depth_allowance")?;
        let side_prefix = format!("blum.legrabox.drawer-side.{h}.");
        let mut lengths: Vec<(f64, &HardwareItem)> = hardware::items()
            .iter()
            .filter(|i| i.id.starts_with(&side_prefix))
            .filter_map(|i| Some((i.param("nominal_length").ok()?, i)))
            .filter(|(nl, _)| nl + allowance <= inner_depth + 1e-9)
            .collect();
        lengths.sort_by(|a, b| a.0.total_cmp(&b.0));
        let Some(&(nl, sides)) = lengths.last() else {
            last_err = format!(
                "no LEGRABOX {} drawer is short enough for {inner_depth:.0} mm inside depth",
                h.to_uppercase()
            );
            continue;
        };
        let profile = profile_for(nl)?;
        let p = |name: &str| system.param(name);
        return Ok(Legrabox {
            system,
            height: h.to_uppercase(),
            nominal_length: nl,
            profile,
            sides,
            back_fixing: hardware::item(&format!("blum.legrabox.back-fixing.{h}"))?,
            front_fixing: hardware::item(&format!("blum.legrabox.front-fixing.{h}"))?,
            base: (
                inner_width - p("base_width_deduction")?,
                nl - p("base_length_deduction")?,
            ),
            back: (inner_width - p("back_width_deduction")?, p("back_height")?),
            board_thickness: p("board_thickness")?,
        });
    }
    Err(last_err)
}

/// The 40 kg cabinet profile for a nominal length, or the 70 kg one where only it comes that
/// long.
fn profile_for(nl: f64) -> Result<&'static HardwareItem, String> {
    ["750", "753"]
        .iter()
        .find_map(|series| {
            hardware::item(&format!("blum.legrabox.cabinet-profile.{series}.{nl}")).ok()
        })
        .ok_or_else(|| format!("no LEGRABOX cabinet profile of nominal length {nl}"))
}

/// The wooden-drawer runner systems.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WoodRunner {
    Movento,
    Tandem,
}

/// A wooden drawer box on Blum runners. The runner rules are Blum's; the box itself is our
/// construction: base between the sides at Blum's minimum recess, front and back standing on the
/// base between the sides, all in `board` mm board.
#[derive(Clone, Debug, PartialEq)]
pub struct WoodBox {
    /// The planning row (`blum.tandem.560h`).
    pub system: &'static HardwareItem,
    pub runner: &'static HardwareItem,
    pub locking_device: &'static HardwareItem,
    pub nominal_length: f64,
    /// SKW: between the box sides.
    pub inside_width: f64,
    /// SKL: the sides' length.
    pub length: f64,
    pub side_height: f64,
    /// From the runners' datum up to the sides' bottom edges.
    pub side_lift: f64,
    /// How far the sides reach below the base.
    pub base_recess: f64,
    pub board: f64,
}

impl WoodBox {
    /// Height of the box front and back standing on the base.
    pub fn end_height(&self) -> f64 {
        self.side_height - self.base_recess - self.board
    }
}

/// A wooden drawer on `runner` for an opening `inner_width` (LW) wide and `inner_depth` (LT)
/// deep, with `space` mm from the runners' datum to the next one up, in `board` mm board.
pub fn wood(
    runner: WoodRunner,
    inner_width: f64,
    inner_depth: f64,
    space: f64,
    board: f64,
) -> Result<WoodBox, String> {
    let (system_id, series, lock) = match runner {
        WoodRunner::Movento => ("blum.movento.760h", &["760h", "766h"][..], "movento"),
        WoodRunner::Tandem => ("blum.tandem.560h", &["560h"][..], "tandem"),
    };
    let system = hardware::item(system_id)?;
    let p = |name: &str| system.param(name);
    let (min_side, max_side) = (
        p("side_thickness_min").unwrap_or(0.0),
        p("side_thickness_max")?,
    );
    if !(min_side - 1e-9..=max_side + 1e-9).contains(&board) {
        return Err(format!(
            "{} takes drawer sides {}{max_side} mm thick, not {board} mm",
            system.family,
            if min_side > 0.0 {
                format!("{min_side}–")
            } else {
                "up to ".to_string()
            }
        ));
    }
    let allowance = p("depth_allowance")?;
    // The longest length that fits, from the first series that makes it (the lighter runner).
    let family = system_id.rsplit_once('.').map_or("", |(f, _)| f);
    let mut best: Option<(f64, &'static HardwareItem)> = None;
    for s in series {
        let prefix = format!("{}.{s}.", family);
        for i in hardware::items()
            .iter()
            .filter(|i| i.id.starts_with(&prefix))
        {
            let Ok(nl) = i.param("nominal_length") else {
                continue;
            };
            if nl + allowance <= inner_depth + 1e-9 && best.is_none_or(|(b, _)| nl > b) {
                best = Some((nl, i));
            }
        }
    }
    let Some((nl, runner_item)) = best else {
        return Err(format!(
            "no {} runner is short enough for {inner_depth:.0} mm inside depth",
            system.family
        ));
    };
    let base_recess = p("base_recess_min")?;
    let side_lift = p("base_above_datum_min")? - base_recess;
    let side_height = (space - side_lift - p("min_space_above")?).floor();
    if side_height < base_recess + 2.0 * board {
        return Err(format!(
            "no {} drawer fits in {space:.0} mm of height",
            system.family
        ));
    }
    Ok(WoodBox {
        system,
        runner: runner_item,
        locking_device: hardware::item(&format!("blum.{lock}.locking-device"))?,
        nominal_length: nl,
        inside_width: inner_width - p("inside_width_deduction")?,
        length: nl - p("length_deduction")?,
        side_height,
        side_lift,
        base_recess,
        board,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 600 mm cabinet in 16 mm board: LW 568. Base 533 × (NL − 10), back 530 × back height
    /// (Blum p.201, 203, 207).
    #[test]
    fn legrabox_cut_sizes_follow_blums_deductions() {
        let m = legrabox(568.0, 540.0, 120.0).unwrap();
        assert_eq!(m.height, "M");
        assert_eq!(m.nominal_length, 500.0);
        assert_eq!(m.base, (533.0, 490.0));
        assert_eq!(m.back, (530.0, 63.0));
        assert_eq!(m.board_thickness, 16.0);
        assert_eq!(m.sides.article, "770M5002S");
        assert_eq!(m.profile.article, "750.5001S");
        assert_eq!(m.front_fixing.article, "ZF7M70E2");
    }

    #[test]
    fn the_tallest_box_whose_space_fits_is_chosen() {
        let height = |space| legrabox(568.0, 540.0, space).map(|b| b.height);
        assert_eq!(height(80.0).as_deref(), Ok("N"));
        assert_eq!(height(105.9).as_deref(), Ok("N"));
        assert_eq!(height(106.0).as_deref(), Ok("M"));
        assert_eq!(height(144.0).as_deref(), Ok("K"));
        assert_eq!(height(400.0).as_deref(), Ok("K"));
        assert!(height(79.0).is_err());
    }

    #[test]
    fn the_nominal_length_leaves_three_mm_behind_the_box() {
        let nl = |depth| legrabox(568.0, depth, 150.0).map(|b| b.nominal_length);
        assert_eq!(nl(503.0), Ok(500.0));
        assert_eq!(nl(502.9), Ok(450.0));
        // Only the 70 kg profile comes in 650.
        let b = legrabox(568.0, 700.0, 120.0).unwrap();
        assert_eq!(
            (b.nominal_length, b.profile.article.as_str()),
            (650.0, "753.6501S")
        );
        assert!(legrabox(568.0, 250.0, 120.0).is_err());
    }

    /// Blum p.413 / p.435: SKW = LW − 42, SKL = NL − 10, NL + 3 ≤ LT.
    #[test]
    fn wood_drawers_follow_the_runner_rules() {
        let b = wood(WoodRunner::Tandem, 568.0, 560.0, 240.0, 16.0).unwrap();
        assert_eq!(b.runner.article, "560H5500B");
        assert_eq!(b.locking_device.article, "T51.1700.04");
        assert_eq!((b.inside_width, b.length), (526.0, 540.0));
        // Sides start 27.5 − 12 above the datum and stop 7 under the next drawer.
        assert_eq!(b.side_lift, 15.5);
        assert_eq!(b.side_height, (240.0_f64 - 15.5 - 7.0).floor());
        assert_eq!(b.end_height(), b.side_height - 12.0 - 16.0);
        let m = wood(WoodRunner::Movento, 568.0, 560.0, 240.0, 16.0).unwrap();
        assert_eq!(m.runner.article, "760H5500S");
        assert_eq!(m.side_lift, 16.5);
    }

    #[test]
    fn movento_uses_the_heavier_runner_only_where_the_light_one_stops() {
        let b = wood(WoodRunner::Movento, 568.0, 653.0, 240.0, 16.0).unwrap();
        assert_eq!(b.runner.article, "766H6500S");
        let b = wood(WoodRunner::Movento, 568.0, 603.0, 240.0, 16.0).unwrap();
        assert_eq!(b.runner.article, "760H6000S");
    }

    #[test]
    fn wood_drawer_sides_too_thick_or_thin_for_the_runner_are_refused() {
        assert!(wood(WoodRunner::Tandem, 568.0, 560.0, 240.0, 18.0).is_err());
        assert!(wood(WoodRunner::Tandem, 568.0, 560.0, 240.0, 10.0).is_err());
        assert!(wood(WoodRunner::Movento, 568.0, 560.0, 240.0, 12.0).is_ok());
    }
}
