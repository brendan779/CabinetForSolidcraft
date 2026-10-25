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
/// deep, with `space` mm from the runners' datum to the next one up, in `board` mm board;
/// `tip_on` for runners opened by TIP-ON.
pub fn wood(
    runner: WoodRunner,
    inner_width: f64,
    inner_depth: f64,
    space: f64,
    board: f64,
    tip_on: bool,
) -> Result<WoodBox, String> {
    let (system_id, series, lock) = match runner {
        WoodRunner::Movento => ("blum.movento.760h", &["760h", "766h"][..], "movento"),
        // TANDEM's TIP-ON is in the runner itself.
        WoodRunner::Tandem if tip_on => ("blum.tandem.560h", &["560h-tip-on"][..], "tandem"),
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

/// The TIP-ON BLUMOTION parts for one drawer on LEGRABOX or MOVENTO (`system`): the unit set
/// for its nominal length and load, and the synchronisation for the cabinet's inside width
/// (catalogue p.396, p.504). `heavy_runner` is the 70 kg profile / 60–70 kg runner; `heavy_load`
/// asks for the 15–40 kg unit instead of the ≤ 20 kg one.
pub fn tip_on_blumotion(
    system: &str,
    nominal_length: f64,
    heavy_runner: bool,
    heavy_load: bool,
    inner_width: f64,
) -> Result<Vec<(&'static HardwareItem, u32)>, String> {
    let plan = hardware::item(&format!("blum.tip-on-blumotion.{system}"))?;
    let p = |k: &str| plan.param(k);
    let unit = match (
        heavy_runner,
        nominal_length <= p("short_nl_max")?,
        heavy_load,
    ) {
        (true, _, _) => "l5",
        (false, true, false) => "s0",
        (false, true, true) => "s1",
        (false, false, false) => "l1",
        (false, false, true) => "l3",
    };
    let unit = hardware::item(&format!("blum.tip-on-blumotion.{unit}"))?;
    let (lo, hi) = (
        unit.param("nominal_length_min")?,
        unit.param("nominal_length_max")?,
    );
    if !(lo..=hi).contains(&nominal_length) {
        return Err(format!(
            "no TIP-ON BLUMOTION unit for nominal length {nominal_length}"
        ));
    }
    let mut out = vec![(unit, 1)];
    if inner_width < p("square_sync_lw_min")? {
        // Too narrow to synchronise: the unit works alone.
    } else if inner_width <= p("square_sync_lw_max")? {
        out.push((hardware::item("blum.tip-on-blumotion.sync-square")?, 1));
    } else if inner_width <= p("round_sync_lw_max")? {
        out.push((hardware::item("blum.tip-on-blumotion.sync-round")?, 1));
        out.push((hardware::item("blum.tip-on-blumotion.sync-adapter")?, 2));
    } else {
        return Err(format!(
            "TIP-ON BLUMOTION synchronisation reaches {} mm inside width, not {inner_width:.0}",
            p("round_sync_lw_max")?
        ));
    }
    Ok(out)
}

/// SERVO-DRIVE for a cabinet of LEGRABOX drawers with fronts `front_heights` high: a drive unit
/// and distance bumpers per drawer, and for the cabinet the bracket profile (PL = LH − 10, the
/// shortest article that cuts to it), its attachments, cabling and the power supply (p.364).
pub fn servo_drive_legrabox(
    inside_height: f64,
    front_heights: &[f64],
    heavy_runner: bool,
) -> Result<Vec<(&'static HardwareItem, u32)>, String> {
    let plan = hardware::item("blum.servo-drive.legrabox")?;
    let p = |k: &str| plan.param(k);
    let pl = inside_height - p("bracket_deduction")?;
    let mut profiles: Vec<&HardwareItem> = hardware::items()
        .iter()
        .filter(|i| i.id.starts_with("blum.servo-drive.bracket-profile."))
        .collect();
    profiles.sort_by(|a, b| {
        let l = |i: &HardwareItem| i.param("length").unwrap_or(f64::MAX);
        l(a).total_cmp(&l(b))
    });
    let profile = profiles
        .into_iter()
        .find(|i| i.param("length").is_ok_and(|l| l >= pl))
        .ok_or_else(|| format!("no SERVO-DRIVE bracket profile reaches {pl:.0} mm"))?;
    let bumpers: u32 = front_heights
        .iter()
        .map(|h| {
            if !heavy_runner && *h <= p("low_front_max").unwrap_or(0.0) {
                p("bumpers_low_front").unwrap_or(0.0) as u32
            } else {
                p("bumpers_high_front").unwrap_or(0.0) as u32
            }
        })
        .sum();
    let n = front_heights.len() as u32;
    let item = hardware::item;
    Ok(vec![
        (item("blum.servo-drive.drive-unit")?, n),
        (item("blum.servo-drive.distance-bumper")?, bumpers),
        (profile, 1),
        (item("blum.servo-drive.bracket-attachment")?, 2),
        (item("blum.servo-drive.distribution-cable")?, 1),
        (item("blum.servo-drive.connecting-node")?, 1),
        (item("blum.servo-drive.power-supply.au")?, 1),
    ])
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
        let b = wood(WoodRunner::Tandem, 568.0, 560.0, 240.0, 16.0, false).unwrap();
        assert_eq!(b.runner.article, "560H5500B");
        assert_eq!(b.locking_device.article, "T51.1700.04");
        assert_eq!((b.inside_width, b.length), (526.0, 540.0));
        // Sides start 27.5 − 12 above the datum and stop 7 under the next drawer.
        assert_eq!(b.side_lift, 15.5);
        assert_eq!(b.side_height, (240.0_f64 - 15.5 - 7.0).floor());
        assert_eq!(b.end_height(), b.side_height - 12.0 - 16.0);
        let m = wood(WoodRunner::Movento, 568.0, 560.0, 240.0, 16.0, false).unwrap();
        assert_eq!(m.runner.article, "760H5500S");
        assert_eq!(m.side_lift, 16.5);
    }

    #[test]
    fn movento_uses_the_heavier_runner_only_where_the_light_one_stops() {
        let b = wood(WoodRunner::Movento, 568.0, 653.0, 240.0, 16.0, false).unwrap();
        assert_eq!(b.runner.article, "766H6500S");
        let b = wood(WoodRunner::Movento, 568.0, 603.0, 240.0, 16.0, false).unwrap();
        assert_eq!(b.runner.article, "760H6000S");
    }

    #[test]
    fn wood_drawer_sides_too_thick_or_thin_for_the_runner_are_refused() {
        assert!(wood(WoodRunner::Tandem, 568.0, 560.0, 240.0, 18.0, false).is_err());
        assert!(wood(WoodRunner::Tandem, 568.0, 560.0, 240.0, 10.0, false).is_err());
        assert!(wood(WoodRunner::Movento, 568.0, 560.0, 240.0, 12.0, false).is_ok());
    }

    /// p.396: units by nominal length and load; square synchronisation up to LW 293, the round
    /// linkage with two adapters above.
    #[test]
    fn tip_on_blumotion_units_and_synchronisation_follow_blums_tables() {
        let parts = |nl, heavy_runner, heavy_load, lw| {
            tip_on_blumotion("legrabox", nl, heavy_runner, heavy_load, lw)
                .unwrap()
                .into_iter()
                .map(|(i, n)| (i.article.clone(), n))
                .collect::<Vec<_>>()
        };
        let a = |s: &str, n| (s.to_string(), n);
        assert_eq!(
            parts(500.0, false, false, 568.0),
            [a("T60L7340", 1), a("T60.1125W", 1), a("T60.000D", 2)]
        );
        assert_eq!(
            parts(500.0, false, true, 268.0),
            [a("T60L7540", 1), a("T60.300D", 1)]
        );
        assert_eq!(parts(300.0, false, false, 568.0)[0], a("T60L7040", 1));
        assert_eq!(parts(650.0, true, false, 568.0)[0], a("T60L7570", 1));
        // MOVENTO's short units reach 320.
        let m = tip_on_blumotion("movento", 320.0, false, false, 568.0).unwrap();
        assert_eq!(m[0].0.article, "T60L7040");
    }

    #[test]
    fn servo_drive_cuts_the_bracket_from_the_shortest_profile_that_reaches() {
        let parts = servo_drive_legrabox(700.0, &[238.0, 238.0, 238.0], false).unwrap();
        let get = |art: &str| {
            parts
                .iter()
                .find(|(i, _)| i.article == art)
                .map(|(_, n)| *n)
        };
        // PL = 700 − 10 = 690: the 700 profile.
        assert_eq!(get("Z10T700AA"), Some(1));
        assert_eq!(get("Z10A3000.04"), Some(3));
        assert_eq!(get("993.0530"), Some(3 * 2));
        assert_eq!(get("Z10NE04KI"), Some(1));
        let tall = servo_drive_legrabox(700.0, &[400.0], false).unwrap();
        assert_eq!(
            tall.iter()
                .find(|(i, _)| i.article == "993.0530")
                .unwrap()
                .1,
            4
        );
    }
}
