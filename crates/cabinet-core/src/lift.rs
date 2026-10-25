//! Lift systems: front weight, Blum's power factor and the AVENTOS parts for a lifting front,
//! from `catalog/hardware/blum-aventos-hf.json`.

use cabinet_catalog::HardwareItem;

use crate::hardware;

/// Blum's front weight: FG (kg) = FH (m) × FB (m) × FD (m) × density (kg/m³) (p.716).
pub fn front_weight(height: f64, width: f64, thickness: f64, density: f64) -> f64 {
    height / 1000.0 * width / 1000.0 * thickness / 1000.0 * density
}

/// The reference density Blum gives for a front's core (p.716): MDF or chipboard.
pub fn reference_density(mdf: bool) -> Result<f64, String> {
    hardware::item("blum.aventos.hf-top")?.param(if mdf {
        "density_mdf_kg_m3"
    } else {
        "density_chipboard_kg_m3"
    })
}

/// What an AVENTOS HF top needs to know.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BiFoldNeed {
    /// KH, or for unequal fronts TKH = 2 × the upper front's height + the gaps.
    pub cabinet_height: f64,
    pub cabinet_width: f64,
    pub inside_height: f64,
    pub inside_depth: f64,
    /// Both fronts together, with their handle.
    pub front_weight: f64,
}

/// The AVENTOS HF top parts for one cabinet.
#[derive(Clone, Debug, PartialEq)]
pub struct BiFold {
    pub power_factor: f64,
    pub mechanism: &'static HardwareItem,
    /// Mechanisms fitted: 1 for a light, narrow front, else the set's 2.
    pub mechanisms: u32,
    pub lever_arm: &'static HardwareItem,
    pub cover_caps: &'static HardwareItem,
    pub hinge: &'static HardwareItem,
    pub centre_hinge: &'static HardwareItem,
    pub plate: &'static HardwareItem,
    /// Hinges per row (front to cabinet, and between the fronts).
    pub hinges: u32,
    pub warnings: Vec<String>,
}

/// Picks the HF top lift mechanism, lever arm and hinges (catalogue p.22-25).
pub fn hf_top(need: &BiFoldNeed) -> Result<BiFold, String> {
    let sys = hardware::item("blum.aventos.hf-top")?;
    let p = |k: &str| sys.param(k);
    let kh = need.cabinet_height;
    if !(p("cabinet_height_min")?..=p("cabinet_height_max")?).contains(&kh) {
        return Err(format!(
            "AVENTOS HF top is for cabinets {}–{} mm high, not {kh:.0}",
            p("cabinet_height_min")?,
            p("cabinet_height_max")?
        ));
    }
    if need.cabinet_width > p("cabinet_width_max")? {
        return Err(format!(
            "AVENTOS HF top is for cabinets up to {} mm wide",
            p("cabinet_width_max")?
        ));
    }
    let mut warnings = Vec::new();
    if need.inside_depth < p("inside_depth_min")? {
        warnings.push(format!(
            "AVENTOS HF top needs {} mm inside depth; this cabinet has {:.0}",
            p("inside_depth_min")?,
            need.inside_depth
        ));
    }
    let lf = kh * need.front_weight;
    let mechanisms: Vec<&HardwareItem> = hardware::items()
        .iter()
        .filter(|i| {
            i.id.starts_with("blum.aventos.hf-top.mechanism.")
                && i.param("system_screws").unwrap_or(0.0) > 0.0
        })
        .collect();
    let in_range = |i: &&HardwareItem| {
        let (lo, hi) = (i.param("power_factor_min"), i.param("power_factor_max"));
        matches!((lo, hi), (Ok(lo), Ok(hi)) if (lo..=hi).contains(&lf))
    };
    // Where two ranges overlap Blum recommends the stronger mechanism.
    let strongest = |items: Vec<&'static HardwareItem>| {
        items.into_iter().max_by(|a, b| {
            let hi = |i: &HardwareItem| i.param("power_factor_max").unwrap_or(0.0);
            hi(a).total_cmp(&hi(b))
        })
    };
    let (mechanism, count) = match strongest(mechanisms.iter().copied().filter(in_range).collect())
    {
        Some(m) => (m, 2),
        None if need.cabinet_width <= p("single_mechanism_width_max")?
            && (p("single_mechanism_power_factor_min")?
                ..=p("single_mechanism_power_factor_max")?)
                .contains(&lf) =>
        {
            // One mechanism of the lighter set.
            let light = mechanisms
                .iter()
                .copied()
                .min_by(|a, b| {
                    let lo = |i: &HardwareItem| i.param("power_factor_min").unwrap_or(f64::MAX);
                    lo(a).total_cmp(&lo(b))
                })
                .ok_or("no AVENTOS HF top mechanisms in the catalog")?;
            (light, 1)
        }
        None => {
            return Err(format!(
                "no AVENTOS HF top mechanism takes power factor {lf:.0} (KH {kh:.0} × {:.1} kg); \
                 Blum allows 50 % more with a third mechanism",
                need.front_weight
            ));
        }
    };
    // The first arm whose height range holds the cabinet (the ranges overlap).
    let lever_arm = hardware::items()
        .iter()
        .filter(|i| i.id.starts_with("blum.aventos.hf-top.lever-arm."))
        .find(|i| {
            matches!(
                (i.param("cabinet_height_min"), i.param("cabinet_height_max")),
                (Ok(lo), Ok(hi)) if (lo..=hi).contains(&kh)
            )
        })
        .ok_or_else(|| format!("no AVENTOS HF top lever arm for KH {kh:.0}"))?;
    let lh = lever_arm.param("inside_height_min")?;
    if need.inside_height < lh {
        warnings.push(format!(
            "lever arm {} needs {lh} mm inside height; this cabinet has {:.0}",
            lever_arm.article, need.inside_height
        ));
    }
    let fg = need.front_weight;
    let hinges = if need.cabinet_width >= p("hinges_4_from_width")?
        || fg >= p("hinges_4_from_weight_kg")?
    {
        4
    } else if need.cabinet_width >= p("hinges_3_from_width")? || fg >= p("hinges_3_from_weight_kg")?
    {
        3
    } else {
        2
    };
    Ok(BiFold {
        power_factor: lf,
        mechanism,
        mechanisms: count,
        lever_arm,
        cover_caps: hardware::item("blum.aventos.hf-top.cover-caps")?,
        hinge: hardware::item("blum.aventos.hf-top.hinge-120")?,
        centre_hinge: hardware::item("blum.aventos.hf-top.centre-hinge")?,
        plate: hardware::item("blum.aventos.hf-top.hinge-plate")?,
        hinges,
        warnings,
    })
}

/// The kinds of lift for one front.
pub use crate::model::LiftKind;

/// The AVENTOS parts for one lifting front.
#[derive(Clone, Debug, PartialEq)]
pub struct SingleLift {
    /// LF, where the family selects by it (HK top).
    pub power_factor: Option<f64>,
    /// The lift mechanism set first, then the rest of the parts, one set each.
    pub parts: Vec<&'static HardwareItem>,
    pub warnings: Vec<String>,
}

/// What a lift for one front needs to know.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LiftNeed {
    pub cabinet_height: f64,
    pub cabinet_width: f64,
    pub inside_height: f64,
    pub inside_depth: f64,
    /// The front alone, kg.
    pub front_weight: f64,
    pub handle_weight: f64,
}

/// Of the items under `prefix`, those whose cabinet height (and front weight, where given)
/// ranges hold `kh` and `fg`; the strongest where they overlap, as Blum recommends.
fn by_ranges(prefix: &str, kh: f64, fg: f64) -> Option<&'static HardwareItem> {
    let within = |i: &HardwareItem, lo: &str, hi: &str, v: f64| match (i.param(lo), i.param(hi)) {
        (Ok(lo), Ok(hi)) => (lo..=hi).contains(&v),
        _ => true,
    };
    hardware::items()
        .iter()
        .filter(|i| i.id.starts_with(prefix))
        .filter(|i| within(i, "cabinet_height_min", "cabinet_height_max", kh))
        .filter(|i| within(i, "front_weight_min_kg", "front_weight_max_kg", fg))
        .filter(|i| within(i, "power_factor_min", "power_factor_max", kh * fg))
        .max_by(|a, b| {
            let top = |i: &HardwareItem| {
                i.param("power_factor_max")
                    .or_else(|_| i.param("front_weight_max_kg"))
                    .or_else(|_| i.param("cabinet_height_max"))
                    .unwrap_or(0.0)
            };
            top(a).total_cmp(&top(b))
        })
}

/// Picks the AVENTOS parts for one front: HK top (stay lift, p.46), HS top (up and over, p.34)
/// or HL top (lift up, p.40).
pub fn single_front(kind: LiftKind, need: &LiftNeed) -> Result<SingleLift, String> {
    let family = match kind {
        LiftKind::StayLift => "hk-top",
        LiftKind::UpAndOver => "hs-top",
        LiftKind::LiftUp => "hl-top",
    };
    let sys = hardware::item(&format!("blum.aventos.{family}"))?;
    let p = |k: &str| sys.param(k);
    let kh = need.cabinet_height;
    if !(p("cabinet_height_min")?..=p("cabinet_height_max")?).contains(&kh) {
        return Err(format!(
            "{} is for cabinets {}–{} mm high, not {kh:.0}",
            sys.family,
            p("cabinet_height_min")?,
            p("cabinet_height_max")?
        ));
    }
    if need.cabinet_width > p("cabinet_width_max")? {
        return Err(format!(
            "{} is for cabinets up to {} mm wide",
            sys.family,
            p("cabinet_width_max")?
        ));
    }
    let fg = need.front_weight + p("handle_weight_factor")? * need.handle_weight;
    if let Ok(max) = p("front_weight_max_kg")
        && fg > max
    {
        return Err(format!(
            "a {fg:.1} kg front is over {}'s {max} kg",
            sys.family
        ));
    }
    let mut warnings = Vec::new();
    for (key, have) in [
        ("inside_height_min", need.inside_height),
        ("inside_depth_min", need.inside_depth),
    ] {
        if let Ok(min) = p(key)
            && have < min
        {
            warnings.push(format!(
                "{} needs {min} mm {}; this cabinet has {have:.0}",
                sys.family,
                if key.contains("height") {
                    "inside height"
                } else {
                    "inside depth"
                }
            ));
        }
    }
    let mechanism =
        by_ranges(&format!("blum.aventos.{family}.mechanism."), kh, fg).ok_or_else(|| {
            format!(
                "no {} mechanism for KH {kh:.0} and a {fg:.1} kg front",
                sys.family
            )
        })?;
    let item = hardware::item;
    let mut parts = vec![mechanism];
    match kind {
        LiftKind::StayLift => parts.push(item("blum.aventos.hk-top.cover-caps")?),
        LiftKind::UpAndOver => {
            parts.push(item("blum.aventos.hs-top.lever-arm")?);
            parts.push(item("blum.aventos.cover-caps")?);
        }
        LiftKind::LiftUp => {
            parts.push(
                by_ranges("blum.aventos.hl-top.lever-arm.", kh, fg).ok_or_else(|| {
                    format!("no AVENTOS HL top lever arm for KH {kh:.0} and {fg:.1} kg")
                })?,
            );
            parts.push(item("blum.aventos.hl-top.stabiliser")?);
            parts.push(item("blum.aventos.cover-caps")?);
        }
    }
    parts.push(item("blum.aventos.hk-top.front-fixing")?);
    Ok(SingleLift {
        power_factor: (kind == LiftKind::StayLift).then_some(kh * fg),
        parts,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn need(kh: f64, kb: f64, fg: f64) -> BiFoldNeed {
        BiFoldNeed {
            cabinet_height: kh,
            cabinet_width: kb,
            inside_height: kh - 32.0,
            inside_depth: 300.0,
            front_weight: fg,
        }
    }

    /// BLUM-7a's check: a 600 × 360 front of 19 mm chipboard weighs 2.8 kg (p.716 lists 2.7 for
    /// 350 and 3.1 for 400 high at 600 wide), so a 720 mm cabinet with two such fronts has power
    /// factor 720 × 5.6.
    #[test]
    fn front_weight_and_power_factor_for_a_600_by_360_front() {
        let fg = front_weight(360.0, 600.0, 19.0, reference_density(false).unwrap());
        assert!((fg - 2.79072).abs() < 1e-9, "{fg}");
        let hf = hf_top(&need(720.0, 600.0, 2.0 * fg)).unwrap();
        assert!((hf.power_factor - 720.0 * 2.0 * fg).abs() < 1e-9);
        assert_eq!(hf.mechanism.article, "22F2510");
        assert_eq!(hf.mechanisms, 2);
        assert_eq!(hf.lever_arm.article, "22F3500");
        assert_eq!(hf.hinges, 2);
    }

    #[test]
    fn blums_reference_table_matches_the_formula() {
        // p.716: chipboard 19 mm, FH 500 × FB 800 → 5.2 kg; MDF FH 1000 × FB 1200 → 17.3 kg.
        let chip = reference_density(false).unwrap();
        let mdf = reference_density(true).unwrap();
        assert!((front_weight(500.0, 800.0, 19.0, chip) - 5.2).abs() < 0.05);
        assert!((front_weight(1000.0, 1200.0, 19.0, mdf) - 17.3).abs() < 0.05);
    }

    #[test]
    fn the_stronger_mechanism_is_chosen_where_the_ranges_overlap() {
        let pick = |lf: f64| {
            hf_top(&need(1000.0, 900.0, lf / 1000.0)).map(|h| h.mechanism.article.clone())
        };
        assert_eq!(pick(9000.0).as_deref(), Ok("22F2510"));
        assert_eq!(pick(12000.0).as_deref(), Ok("22F2810"));
        assert_eq!(pick(19000.0).as_deref(), Ok("22F2810"));
        assert!(pick(20000.0).is_err());
    }

    #[test]
    fn a_light_narrow_front_gets_one_mechanism() {
        let hf = hf_top(&need(500.0, 500.0, 4.0)).unwrap();
        assert_eq!(
            (hf.mechanism.article.as_str(), hf.mechanisms),
            ("22F2510", 1)
        );
        // The same front on a wider cabinet has no mechanism light enough.
        assert!(hf_top(&need(500.0, 700.0, 4.0)).is_err());
    }

    #[test]
    fn wide_or_heavy_fronts_get_more_hinges_and_the_lever_arm_follows_the_height() {
        assert_eq!(hf_top(&need(720.0, 1200.0, 6.0)).unwrap().hinges, 3);
        assert_eq!(hf_top(&need(720.0, 900.0, 12.0)).unwrap().hinges, 3);
        assert_eq!(hf_top(&need(1100.0, 1800.0, 15.0)).unwrap().hinges, 4);
        assert_eq!(
            hf_top(&need(500.0, 900.0, 8.0)).unwrap().lever_arm.article,
            "22F3200"
        );
        assert_eq!(
            hf_top(&need(1000.0, 900.0, 10.0))
                .unwrap()
                .lever_arm
                .article,
            "22F3900"
        );
        assert!(hf_top(&need(400.0, 900.0, 8.0)).is_err());
    }

    fn lift(kind: LiftKind, kh: f64, fg: f64, handle: f64) -> Result<SingleLift, String> {
        single_front(
            kind,
            &LiftNeed {
                cabinet_height: kh,
                cabinet_width: 600.0,
                inside_height: kh - 32.0,
                inside_depth: 300.0,
                front_weight: fg,
                handle_weight: handle,
            },
        )
    }

    /// p.46: LF = KH × (FG + 2 × handle); overlapping ranges take the stronger set.
    #[test]
    fn hk_top_picks_the_mechanism_by_power_factor_with_double_handle_weight() {
        let pick = |kh: f64, fg: f64, handle: f64| {
            lift(LiftKind::StayLift, kh, fg, handle)
                .map(|h| (h.parts[0].article.clone(), h.power_factor.unwrap()))
        };
        // 360 × (3 + 2 × 0.3) = 1296: in 420–1610 and 930–2800, so the stronger.
        assert_eq!(pick(360.0, 3.0, 0.3), Ok(("22K2510".to_string(), 1296.0)));
        assert_eq!(pick(400.0, 1.5, 0.0).unwrap().0, "22K2310");
        assert_eq!(pick(600.0, 14.0, 0.5).unwrap().0, "22K2910");
        assert!(pick(600.0, 17.0, 1.0).is_err(), "19 kg is over 18");
        assert!(pick(720.0, 5.0, 0.0).is_err(), "too tall for HK top");
    }

    /// p.34: HS top by cabinet height and front weight (twice the handle); Blum's example,
    /// KH 600 and FG 10 kg, takes 22S25x0.
    #[test]
    fn hs_top_follows_blums_example_and_its_overlaps() {
        let mech = |kh: f64, fg: f64| {
            lift(LiftKind::UpAndOver, kh, fg, 0.0).map(|l| l.parts[0].article.clone())
        };
        assert_eq!(mech(600.0, 10.0).as_deref(), Ok("22S2510"));
        assert_eq!(mech(400.0, 5.0).as_deref(), Ok("22S2210"));
        // 500 high, 12 kg: both 22S2210 (to 12.5) and 22S2510 (to 15.25) fit; the stronger.
        assert_eq!(mech(500.0, 12.0).as_deref(), Ok("22S2510"));
        assert_eq!(mech(700.0, 18.0).as_deref(), Ok("22S2810"));
        assert!(mech(400.0, 1.0).is_err());
    }

    /// p.40: HL top's mechanism by height, its lever arm by height and weight with the handle.
    #[test]
    fn hl_top_picks_mechanism_by_height_and_arm_by_height_and_weight() {
        let parts = |kh: f64, fg: f64, handle: f64| {
            lift(LiftKind::LiftUp, kh, fg, handle)
                .unwrap()
                .parts
                .iter()
                .map(|i| i.article.clone())
                .collect::<Vec<_>>()
        };
        let p = parts(360.0, 6.0, 0.5);
        assert_eq!(&p[..3], ["22L2210", "22L3500", "22Q1076U"]);
        let p = parts(500.0, 11.0, 0.5);
        assert_eq!(&p[..2], ["22L2510", "22L3900"]);
        assert!(lift(LiftKind::LiftUp, 600.0, 5.0, 0.0).is_err());
    }
}
