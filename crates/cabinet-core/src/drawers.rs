//! Drawer boxes: which drawer system box fits an opening, and the parts it needs cut, from the
//! Blum data in `catalog/hardware/`.

use cabinet_catalog::HardwareItem;

use crate::hardware;

/// Which boxes a drawer may be: each range lists (planning row, side set) keys tallest first,
/// and the box chosen is the tallest whose space requirement fits. A system without a row
/// (MERIVOBOX has no C or F) skips it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoxRange {
    /// N, M or K behind a drawer front.
    Drawers,
    /// LEGRABOX pure's high fronted pull-outs C and F as well.
    HighFronted,
    /// Inner drawers behind a door: M or K sides and a steel front piece.
    Inner,
}

impl BoxRange {
    fn heights(self) -> &'static [(&'static str, &'static str)] {
        match self {
            BoxRange::Drawers => &[("k", "k"), ("m", "m"), ("n", "n")],
            BoxRange::HighFronted => &[("f", "f"), ("c", "c"), ("k", "k"), ("m", "m"), ("n", "n")],
            BoxRange::Inner => &[("inner-k", "k"), ("inner-m", "m")],
        }
    }
}

/// Blum's steel-sided box systems, with their cabinet profile series (lighter first).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoxSystem {
    Legrabox,
    Merivobox,
    Tandembox,
}

impl BoxSystem {
    /// The system's key in catalog ids (`blum.legrabox.m`).
    pub fn key(self) -> &'static str {
        match self {
            BoxSystem::Legrabox => "legrabox",
            BoxSystem::Merivobox => "merivobox",
            BoxSystem::Tandembox => "tandembox",
        }
    }

    fn name(self) -> &'static str {
        match self {
            BoxSystem::Legrabox => "LEGRABOX",
            BoxSystem::Merivobox => "MERIVOBOX",
            BoxSystem::Tandembox => "TANDEMBOX",
        }
    }

    /// Profile series, the standard load first, then the heavy one.
    fn profiles(self) -> [&'static str; 2] {
        match self {
            BoxSystem::Legrabox => ["750", "753"],
            BoxSystem::Merivobox => ["450", "453"],
            BoxSystem::Tandembox => ["578", "576"],
        }
    }
}

/// A steel-sided box drawer sized for its opening.
#[derive(Clone, Debug, PartialEq)]
pub struct SteelBox {
    pub kind: BoxSystem,
    /// The planning row (`blum.legrabox.m`): side height, deductions, front drilling.
    pub system: &'static HardwareItem,
    /// The side set: N, M, K, C or F.
    pub height: String,
    /// An inner drawer: no drawer front, a steel front piece cut to `front_piece` long.
    pub front_piece: Option<(&'static HardwareItem, f64)>,
    pub nominal_length: f64,
    pub profile: &'static HardwareItem,
    /// The profile is the heavy series (70 kg LEGRABOX / MERIVOBOX, 65 kg TANDEMBOX).
    pub heavy_profile: bool,
    pub sides: &'static HardwareItem,
    pub back_fixing: &'static HardwareItem,
    /// Front fixings and how many of each a drawer takes.
    pub front_fixings: Vec<(&'static HardwareItem, u32)>,
    /// Cut sizes in 16 mm board: base width × length, back width × height.
    pub base: (f64, f64),
    pub back: (f64, f64),
    pub board_thickness: f64,
}

/// The tallest box of `kind` in `range` of the longest nominal length that fits an opening
/// `inner_width` (LW) wide, `inner_depth` (LT) deep from the back of the front, with `space` mm
/// from the drawer's datum (the surface its profiles stand on) to the next one up.
pub fn steel_box(
    kind: BoxSystem,
    range: BoxRange,
    inner_width: f64,
    inner_depth: f64,
    space: f64,
) -> Result<SteelBox, String> {
    let key = kind.key();
    let what = if range == BoxRange::Inner {
        "inner drawer"
    } else {
        "drawer"
    };
    let mut last_err = format!("no {} {what} fits in {space:.0} mm of height", kind.name());
    for &(h, sides_key) in range.heights() {
        let Ok(system) = hardware::item(&format!("blum.{key}.{h}")) else {
            continue;
        };
        if system.param("space_height")? > space + 1e-9 {
            continue;
        }
        let allowance = system.param("depth_allowance")?;
        let side_prefix = format!("blum.{key}.drawer-side.{sides_key}.");
        let mut lengths: Vec<(f64, &HardwareItem)> = hardware::items()
            .iter()
            .filter(|i| i.id.starts_with(&side_prefix))
            .filter_map(|i| Some((i.param("nominal_length").ok()?, i)))
            .filter(|(nl, _)| nl + allowance <= inner_depth + 1e-9)
            .collect();
        lengths.sort_by(|a, b| a.0.total_cmp(&b.0));
        let Some(&(nl, sides)) = lengths.last() else {
            last_err = format!(
                "no {} {} {what} is short enough for {inner_depth:.0} mm inside depth",
                kind.name(),
                sides_key.to_uppercase()
            );
            continue;
        };
        // The standard profile, or the heavy one where only it comes that long.
        let [light, heavy] = kind.profiles();
        let (profile, heavy_profile) =
            match hardware::item(&format!("blum.{key}.cabinet-profile.{light}.{nl}")) {
                Ok(p) => (p, false),
                Err(_) => (
                    hardware::item(&format!("blum.{key}.cabinet-profile.{heavy}.{nl}"))?,
                    true,
                ),
            };
        let p = |name: &str| system.param(name);
        // A bracket pair per id (F takes an M pair below a C pair): `front-fixing.f.lower`.
        let fixing = format!("blum.{key}.front-fixing.{h}");
        let front_fixings: Vec<_> = hardware::items()
            .iter()
            .filter(|i| i.id == fixing || i.id.starts_with(&format!("{fixing}.")))
            .map(|i| (i, i.param("per_drawer").map_or(2, |n| n as u32)))
            .collect();
        if front_fixings.is_empty() {
            return Err(format!("no front fixing {fixing} in the hardware catalog"));
        }
        let front_piece = match range {
            BoxRange::Inner => Some((
                hardware::item(&format!("blum.{key}.front-piece"))?,
                inner_width - p("front_piece_deduction")?,
            )),
            _ => None,
        };
        return Ok(SteelBox {
            kind,
            system,
            height: sides_key.to_uppercase(),
            front_piece,
            nominal_length: nl,
            profile,
            heavy_profile,
            sides,
            back_fixing: hardware::item(&format!("blum.{key}.back-fixing.{sides_key}"))?,
            front_fixings,
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

/// What Blum's AMBIA-LINE recommendation (p.546) puts in an M or K drawer of nominal length
/// `nl` in a cabinet `kb` wide: a cutlery insert set (`cutlery`, from NL 450) or drawer frames;
/// 242 mm frames for NL 270–400 take an adapter profile to a chipboard back from NL 350.
pub fn ambia_line(
    kb: f64,
    nl: f64,
    height: &str,
    cutlery: bool,
) -> Result<Vec<(&'static HardwareItem, u32)>, String> {
    if !matches!(height, "M" | "K") {
        return Err(format!(
            "AMBIA-LINE needs drawers of height M or K, not {height}"
        ));
    }
    let sys = hardware::item("blum.ambia-line")?;
    // The printed row at or under `kb`; nothing under the first.
    let count = |name: &str, col: f64| -> Result<u32, String> {
        let t = sys.table(name)?;
        let r = t.rows.iter().rposition(|&r| r <= kb + 1e-6);
        let c = t.cols.iter().rposition(|&c| c <= col + 1e-6).unwrap_or(0);
        Ok(r.and_then(|r| t.values[r][c]).unwrap_or(0.0) as u32)
    };
    let mut out = Vec::new();
    let mut add = |id: String, n: u32| -> Result<(), String> {
        if n > 0 {
            out.push((hardware::item(&id)?, n));
        }
        Ok(())
    };
    if nl >= 450.0 {
        let nl = nl as u32;
        let (c, f100, f200) = if cutlery {
            (
                count("cutlery_set_cutlery_inserts", 0.0)?,
                count("cutlery_set_frames_100", 0.0)?,
                count("cutlery_set_frames_200", 0.0)?,
            )
        } else {
            (
                0,
                count("long_frames_100", 0.0)?,
                count("long_frames_200", 0.0)?,
            )
        };
        add(format!("blum.ambia-line.cutlery-insert.{nl}"), c)?;
        add(format!("blum.ambia-line.frame-100.{nl}"), f100)?;
        add(format!("blum.ambia-line.frame-200.{nl}"), f200)?;
    } else if nl >= 270.0 {
        let n = count("short_frames_242", nl)?;
        add("blum.ambia-line.frame-242".into(), n)?;
        let adapter = hardware::item(&format!(
            "blum.ambia-line.adapter.{}",
            height.to_lowercase()
        ))?;
        if nl >= adapter.param("from_nominal_length")? {
            add(adapter.id.clone(), n)?;
        }
    }
    if out.is_empty() {
        return Err(format!(
            "AMBIA-LINE recommends nothing for a {kb:.0} mm cabinet at NL {nl:.0}"
        ));
    }
    Ok(out)
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

/// The TIP-ON BLUMOTION parts for one drawer on `system` (LEGRABOX, MERIVOBOX, TANDEMBOX or
/// MOVENTO): the unit set for its nominal length and load, and the synchronisation for the
/// cabinet's inside width (catalogue pp. 396, 398, 400, 504). `heavy_runner` is the 70 kg profile / 60–70 kg runner; `heavy_load`
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
    // MERIVOBOX and TANDEMBOX have their own units; LEGRABOX and MOVENTO share one range.
    let unit = hardware::item(&format!("blum.tip-on-blumotion.{system}.{unit}"))
        .or_else(|_| hardware::item(&format!("blum.tip-on-blumotion.{unit}")))?;
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

/// SERVO-DRIVE for a cabinet of drawers (any Blum box system, or MOVENTO/TANDEM, p.464) with
/// fronts `front_heights` high: a drive unit
/// and distance bumpers per drawer, and for the cabinet the bracket profile (PL = LH − 10, the
/// shortest article that cuts to it), its attachments, cabling and the power supply (p.364).
pub fn servo_drive(
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
        let m = steel_box(BoxSystem::Legrabox, BoxRange::Drawers, 568.0, 540.0, 120.0).unwrap();
        assert_eq!(m.height, "M");
        assert_eq!(m.nominal_length, 500.0);
        assert_eq!(m.base, (533.0, 490.0));
        assert_eq!(m.back, (530.0, 63.0));
        assert_eq!(m.board_thickness, 16.0);
        assert_eq!(m.sides.article, "770M5002S");
        assert_eq!(m.profile.article, "750.5001S");
        assert_eq!(m.front_fixings[0].0.article, "ZF7M70E2");
        assert_eq!(m.front_fixings[0].1, 2);
    }

    #[test]
    fn the_tallest_box_whose_space_fits_is_chosen() {
        let height = |space| {
            steel_box(BoxSystem::Legrabox, BoxRange::Drawers, 568.0, 540.0, space).map(|b| b.height)
        };
        assert_eq!(height(80.0).as_deref(), Ok("N"));
        assert_eq!(height(105.9).as_deref(), Ok("N"));
        assert_eq!(height(106.0).as_deref(), Ok("M"));
        assert_eq!(height(144.0).as_deref(), Ok("K"));
        assert_eq!(height(400.0).as_deref(), Ok("K"));
        assert!(height(79.0).is_err());
    }

    #[test]
    fn the_nominal_length_leaves_three_mm_behind_the_box() {
        let nl = |depth| {
            steel_box(BoxSystem::Legrabox, BoxRange::Drawers, 568.0, depth, 150.0)
                .map(|b| b.nominal_length)
        };
        assert_eq!(nl(503.0), Ok(500.0));
        assert_eq!(nl(502.9), Ok(450.0));
        // Only the 70 kg profile comes in 650.
        let b = steel_box(BoxSystem::Legrabox, BoxRange::Drawers, 568.0, 700.0, 120.0).unwrap();
        assert_eq!(
            (b.nominal_length, b.profile.article.as_str()),
            (650.0, "753.6501S")
        );
        assert!(steel_box(BoxSystem::Legrabox, BoxRange::Drawers, 568.0, 250.0, 120.0).is_err());
    }

    /// BLUM-4b: C (p.211) and F (p.217) take a drawer's place only where their space fits,
    /// and only in the high-fronted range.
    #[test]
    fn high_fronted_pull_outs_are_chosen_by_space_when_allowed() {
        let side = |range, space| {
            steel_box(BoxSystem::Legrabox, range, 568.0, 540.0, space).map(|b| b.height)
        };
        assert_eq!(side(BoxRange::HighFronted, 192.0).as_deref(), Ok("K"));
        assert_eq!(side(BoxRange::HighFronted, 193.0).as_deref(), Ok("C"));
        assert_eq!(side(BoxRange::HighFronted, 257.0).as_deref(), Ok("F"));
        assert_eq!(side(BoxRange::Drawers, 300.0).as_deref(), Ok("K"));
        let f = steel_box(
            BoxSystem::Legrabox,
            BoxRange::HighFronted,
            568.0,
            540.0,
            300.0,
        )
        .unwrap();
        assert_eq!((f.base, f.back), ((533.0, 490.0), (530.0, 212.0)));
        assert_eq!(f.sides.article, "770F5002S");
        let fixings: Vec<_> = f
            .front_fixings
            .iter()
            .map(|(i, n)| (i.article.as_str(), *n))
            .collect();
        assert_eq!(fixings, [("ZF7M70E2", 2), ("ZF7C70E2", 2)]);
        assert_eq!(f.system.drilling.len(), 5);
        // MERIVOBOX has no C or F: the range falls back to its own heights.
        let m = steel_box(
            BoxSystem::Merivobox,
            BoxRange::HighFronted,
            568.0,
            540.0,
            300.0,
        );
        assert_eq!(m.unwrap().height, "K");
    }

    /// BLUM-4b: inner drawers M (p.205, 104 mm) and K (p.209, 142 mm), a front piece cut to
    /// LW − 126 and one front fixing set.
    #[test]
    fn inner_drawers_take_m_or_k_sides_and_a_cut_front_piece() {
        let inner = |space| steel_box(BoxSystem::Legrabox, BoxRange::Inner, 568.0, 540.0, space);
        assert!(inner(103.0).is_err());
        let m = inner(104.0).unwrap();
        assert_eq!(
            (m.height.as_str(), m.sides.article.as_str()),
            ("M", "770M5002S")
        );
        assert_eq!(m.back, (530.0, 63.0));
        let k = inner(142.0).unwrap();
        assert_eq!(k.height, "K");
        let (piece, length) = k.front_piece.unwrap();
        assert_eq!((piece.article.as_str(), length), ("ZV7.1043C01", 442.0));
        assert_eq!(k.front_fixings.len(), 1);
        assert_eq!(
            (k.front_fixings[0].0.article.as_str(), k.front_fixings[0].1),
            ("ZI7.0KS0", 1)
        );
        assert!(k.system.drilling.is_empty());
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
        let parts = servo_drive(700.0, &[238.0, 238.0, 238.0], false).unwrap();
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
        let tall = servo_drive(700.0, &[400.0], false).unwrap();
        assert_eq!(
            tall.iter()
                .find(|(i, _)| i.article == "993.0530")
                .unwrap()
                .1,
            4
        );
    }

    /// BLUM-5: MERIVOBOX (p.257) and TANDEMBOX antaro (p.297) cut sizes and the three heights.
    #[test]
    fn merivobox_and_tandembox_follow_their_own_deductions() {
        let m = steel_box(BoxSystem::Merivobox, BoxRange::Drawers, 568.0, 540.0, 120.0).unwrap();
        assert_eq!((m.height.as_str(), m.nominal_length), ("M", 500.0));
        assert_eq!((m.base, m.back), ((517.0, 474.0), (517.0, 83.0)));
        assert_eq!(
            (m.sides.article.as_str(), m.profile.article.as_str()),
            ("470M5002S", "450.5001B")
        );
        let t = steel_box(BoxSystem::Tandembox, BoxRange::Drawers, 568.0, 540.0, 140.0).unwrap();
        assert_eq!((t.height.as_str(), t.nominal_length), ("K", 500.0));
        assert_eq!((t.base, t.back), ((493.0, 476.0), (481.0, 116.0)));
        assert_eq!(t.system.drilling.len(), 3);
        assert_eq!(t.profile.article, "578.5001M");
        let n = steel_box(BoxSystem::Tandembox, BoxRange::Drawers, 568.0, 540.0, 90.0).unwrap();
        assert_eq!(n.height, "N");
        // Only the 65 kg profile comes in 650.
        let long = steel_box(BoxSystem::Tandembox, BoxRange::Drawers, 568.0, 660.0, 120.0).unwrap();
        assert!(long.heavy_profile && long.profile.article == "576.6501M");
    }

    #[test]
    fn merivobox_and_tandembox_take_their_own_tip_on_units() {
        let unit = |sys| {
            tip_on_blumotion(sys, 500.0, false, false, 568.0).unwrap()[0]
                .0
                .article
                .clone()
        };
        assert_eq!(unit("merivobox"), "T60H4340");
        assert_eq!(unit("tandembox"), "T60B3330");
        assert_eq!(unit("legrabox"), "T60L7340");
        let narrow = tip_on_blumotion("tandembox", 500.0, false, false, 280.0).unwrap();
        assert_eq!(narrow[1].0.article, "T60.300D");
    }

    /// p.546: AMBIA-LINE by cabinet width and nominal length.
    #[test]
    fn ambia_line_follows_blums_recommendation() {
        let arts = |kb, nl, cutlery| {
            ambia_line(kb, nl, "M", cutlery)
                .unwrap()
                .iter()
                .map(|(i, n)| (i.article.clone(), *n))
                .collect::<Vec<_>>()
        };
        let a = |s: &str, n| (s.to_string(), n);
        assert_eq!(
            arts(600.0, 500.0, true),
            [a("ZC7S500BS3", 1), a("ZC7S500RS1", 1)]
        );
        assert_eq!(
            arts(900.0, 500.0, false),
            [a("ZC7S500RS1", 2), a("ZC7S500RS2", 2)]
        );
        // 700 is not printed: the 600 row.
        assert_eq!(
            arts(700.0, 550.0, false),
            [a("ZC7S550RS1", 2), a("ZC7S550RS2", 1)]
        );
        assert_eq!(arts(600.0, 300.0, false), [a("ZC7S300RSU", 2)]);
        assert_eq!(
            arts(600.0, 350.0, false),
            [a("ZC7S300RSU", 2), a("ZC7A0U0M", 2)]
        );
        assert!(ambia_line(600.0, 500.0, "N", false).is_err());
        assert!(
            ambia_line(300.0, 300.0, "M", false).is_err(),
            "nothing under 400 at NL 300"
        );
    }
}
