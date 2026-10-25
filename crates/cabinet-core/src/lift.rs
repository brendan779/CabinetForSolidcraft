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
}
