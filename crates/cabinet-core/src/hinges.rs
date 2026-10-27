//! Hinge selection: a door's overlay and thickness → the Blum hinge, mounting plate and boring
//! distance, from the CLIP top data in `catalog/hardware/blum-clip-top.json`.
//!
//! Blum's planning rule (catalogue 2024/2025 p.77, p.721): front overlay FA = boring distance
//! TB + boss overlay TO − arm cranking − plate spacing MD. TB must stay inside the hinge's
//! range of 3–7 mm, so the plate spacing is chosen to bring it there.

use cabinet_catalog::HardwareItem;

use crate::hardware;
use crate::model::{DoorOverlay, HingeBoss};

/// What a door asks of its hinges.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HingeNeed {
    pub overlay: DoorOverlay,
    /// How far the door's hinged edge covers the side's front edge (FA). Negative for an inset
    /// door: its gap to the side.
    pub front_overlay: f64,
    pub door_thickness: f64,
    pub side_thickness: f64,
    /// The gap beside the door's hinged edge (to the next door or the wall), which must leave
    /// the door room to swing.
    pub gap: f64,
    /// False for doors opened by TIP-ON: those hinges have no spring.
    pub sprung: bool,
    /// How the hinge fixes in the door.
    pub boss: HingeBoss,
}

/// The hinge and plate for a door, and where they go.
#[derive(Clone, Debug, PartialEq)]
pub struct HingeChoice {
    pub hinge: &'static HardwareItem,
    pub plate: &'static HardwareItem,
    /// TB: door edge to the edge of the cup bore.
    pub boring_distance: f64,
    /// Front edge of the side to the plate's hole row.
    pub plate_setback: f64,
    /// Things the user should check (a gap too small to open, a door too thick to clip on).
    pub warnings: Vec<String>,
}

impl HingeChoice {
    /// Door edge to the cup's centre.
    pub fn cup_inset(&self) -> Result<f64, String> {
        Ok(self.boring_distance + self.hinge.param("cup_diameter")? / 2.0)
    }
}

/// The hinge row for an overlay, by the cranking that application needs, with the boss asked
/// for where the catalog has that hinge; otherwise the screw-on one and why.
fn hinge_id(overlay: DoorOverlay, sprung: bool, boss: HingeBoss) -> (String, Option<String>) {
    let app = match overlay {
        DoorOverlay::Full => "overlay",
        DoorOverlay::Half => "dual",
        DoorOverlay::Inset => "inset",
    };
    let plain = if sprung {
        format!("blum.clip-top-blumotion.110.{app}")
    } else {
        format!("blum.clip-top.110.{app}.unsprung")
    };
    let Some(suffix) = boss_suffix(boss) else {
        return (plain, None);
    };
    let id = format!("{plain}.{suffix}");
    if hardware::item(&id).is_ok() {
        (id, None)
    } else {
        let note = format!("no {suffix} hinge for this door in our data; screw-on hinges used");
        (plain, Some(note))
    }
}

fn boss_suffix(boss: HingeBoss) -> Option<&'static str> {
    match boss {
        HingeBoss::ScrewOn => None,
        HingeBoss::Expando => Some("expando"),
        HingeBoss::Inserta => Some("inserta"),
    }
}

/// The plates for a boss fixing, thinnest spacing first: Blum's EXPANDO and INSERTA plates
/// (0 and 3 mm only), then the system-screw ones for the spacings they lack.
fn plates(boss: HingeBoss) -> Vec<&'static HardwareItem> {
    let spacing = |h: &HardwareItem| h.param("plate_spacing").unwrap_or(f64::MAX);
    let of = |kind: &str| {
        let prefix = format!("blum.mounting-plate.cruciform-37-32.{kind}.");
        let mut out: Vec<_> = hardware::items()
            .iter()
            .filter(|h| h.id.starts_with(&prefix))
            .collect();
        out.sort_by(|a, b| spacing(a).total_cmp(&spacing(b)));
        out
    };
    let mut out = boss_suffix(boss).map(of).unwrap_or_default();
    out.extend(of("system-screw"));
    out
}

/// Picks the hinge and the thinnest plate that brings the boring distance into the hinge's range.
pub fn select(need: &HingeNeed) -> Result<HingeChoice, String> {
    let (id, note) = hinge_id(need.overlay, need.sprung, need.boss);
    let mut choice = select_hinge(&id, need)?;
    choice.warnings.extend(note);
    Ok(choice)
}

/// As [`select`], for a given hinge (a wide-angle hinge, say) instead of the standard 110°.
pub fn select_hinge(id: &str, need: &HingeNeed) -> Result<HingeChoice, String> {
    let hinge = hardware::item(id)?;
    let (to, crank) = (hinge.param("boss_overlay")?, hinge.param("arm_cranking")?);
    let (tb_min, tb_max) = (
        hinge.param("boring_distance_min")?,
        hinge.param("boring_distance_max")?,
    );
    let mut fitted = None;
    for plate in plates(need.boss) {
        let md = plate.param("plate_spacing")?;
        let tb = need.front_overlay - to + crank + md;
        if (tb_min - 1e-9..=tb_max + 1e-9).contains(&tb) {
            fitted = Some((plate, md, tb));
            break;
        }
    }
    let Some((plate, md, tb)) = fitted else {
        return Err(format!(
            "no {} {} and plate gives a boring distance of {tb_min}–{tb_max} mm for a front \
             overlay of {} mm",
            hinge.family, hinge.variant, need.front_overlay
        ));
    };
    let mut plate_setback = plate.param("setback")?;
    if need.overlay == DoorOverlay::Inset {
        // The plate moves back with the door (p.77, note A).
        plate_setback += need.door_thickness + hinge.param("inset_plate_setback_extra")?;
    }
    let mut warnings = Vec::new();
    let fd = need.door_thickness;
    // Inset doors swing clear of the side; the gap tables are for doors in front of it.
    if need.overlay != DoorOverlay::Inset {
        let gaps = hinge.table("min_gap")?;
        let last = gaps.cols.last().copied().unwrap_or(0.0);
        // Below the table's thinnest door the gap only shrinks: read the first column.
        let fd_read = fd.max(gaps.cols.first().copied().unwrap_or(fd));
        match gaps.upper(tb, fd_read) {
            Some(f) if need.gap + 1e-9 < f => warnings.push(format!(
                "a {fd} mm door on {} hinges needs a gap of {f} mm beside its hinged edge; it \
                 has {}",
                hinge.family, need.gap
            )),
            None if fd <= last => warnings.push(format!(
                "Blum recommends a trial for {fd} mm doors at boring distance {tb} mm"
            )),
            None => warnings.push(format!(
                "a {fd} mm door is thicker than Blum's gap table for {}",
                hinge.family
            )),
            Some(_) => {}
        }
    }
    // Not every hinge page prints the clip-on limit.
    if let Ok(max_fd) = hinge.table("max_front_thickness")
        && let Some(max) = max_fd.lower(md, need.side_thickness)
        && fd > max
    {
        warnings.push(format!(
            "a {fd} mm door is too thick to clip onto {} hinges on {} mm sides (max {max} mm)",
            hinge.family, need.side_thickness
        ));
    }
    Ok(HingeChoice {
        hinge,
        plate,
        boring_distance: tb,
        plate_setback,
        warnings,
    })
}

/// How many hinges a door `height` mm high weighing `weight` kg needs: Blum's chart (p.718),
/// the fewest whose height and weight limits both hold. Past the chart, five and a warning.
pub fn hinge_count(height: f64, weight: f64) -> Result<(u32, Option<String>), String> {
    let plan = hardware::item("blum.clip-top.hinge-count")?;
    for n in 2..=5 {
        let h = plan.param(&format!("hinges_{n}_height_max"))?;
        let w = plan.param(&format!("hinges_{n}_weight_max_kg"))?;
        if height <= h && weight <= w {
            return Ok((n, None));
        }
    }
    Ok((
        5,
        Some(format!(
            "a {height:.0} mm, {weight:.1} kg door is past Blum's hinge chart: five hinges, check"
        )),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn need(overlay: DoorOverlay, fa: f64, fd: f64) -> HingeNeed {
        HingeNeed {
            overlay,
            front_overlay: fa,
            door_thickness: fd,
            side_thickness: 16.0,
            gap: 2.0,
            sprung: true,
            boss: HingeBoss::ScrewOn,
        }
    }

    /// Blum p.77: each row of the TB tables, read back through the rule.
    #[test]
    fn hinge_selection_table() {
        use DoorOverlay::*;
        // overlay, FA, FD → article, plate article, TB
        let rows = [
            (Full, 15.0, 16.0, "71B3550", "175H9100", 4.0),
            (Full, 17.0, 18.0, "71B3550", "175H9100", 6.0),
            (Full, 18.0, 18.0, "71B3550", "175H9100", 7.0),
            (Full, 13.0, 16.0, "71B3550", "175H9130", 5.0),
            (Full, 9.0, 16.0, "71B3550", "175H9160", 4.0),
            (Full, 5.0, 16.0, "71B3550", "175H9190", 3.0),
            (Half, 8.0, 16.0, "71B3650", "175H9100", 6.5),
            (Half, 2.5, 18.0, "71B3650", "175H9130", 4.0),
            (Half, -4.5, 16.0, "71B3650", "175H9190", 3.0),
            (Inset, -2.0, 16.0, "71B3750", "175H9100", 5.0),
            (Inset, -4.0, 16.0, "71B3750", "175H9100", 3.0),
            (Inset, -6.0, 16.0, "71B3750", "175H9130", 4.0),
        ];
        for (overlay, fa, fd, hinge, plate, tb) in rows {
            let c = select(&need(overlay, fa, fd)).unwrap();
            assert_eq!(
                (
                    c.hinge.article.as_str(),
                    c.plate.article.as_str(),
                    c.boring_distance
                ),
                (hinge, plate, tb),
                "{overlay:?} FA {fa}"
            );
        }
    }

    #[test]
    fn a_wide_angle_hinge_reads_its_own_boring_range_and_gap_table() {
        // 155°: TB 3–8 (p.85), so FA 19 still fits on a 0 mm plate; 18 mm doors need no gap.
        let n = need(DoorOverlay::Full, 19.0, 18.0);
        let c = select_hinge("blum.clip-top-blumotion.155.overlay", &n).unwrap();
        assert_eq!(
            (c.hinge.article.as_str(), c.boring_distance),
            ("71B7550", 8.0)
        );
        assert!(c.warnings.is_empty(), "{:?}", c.warnings);
        assert!(select(&n).is_err());
    }

    #[test]
    fn tip_on_doors_get_hinges_without_a_spring() {
        let mut n = need(DoorOverlay::Full, 15.0, 16.0);
        n.sprung = false;
        assert_eq!(select(&n).unwrap().hinge.article, "70T3550.TL");
    }

    #[test]
    fn an_overlay_no_plate_can_reach_is_refused() {
        // TB 7 at MD 0 is FA 18: more needs a cranked-out hinge Blum does not make.
        assert!(select(&need(DoorOverlay::Full, 19.0, 16.0)).is_err());
    }

    #[test]
    fn an_inset_door_moves_its_plate_back_by_its_thickness_and_a_bit() {
        let c = select(&need(DoorOverlay::Inset, -2.0, 18.0)).unwrap();
        assert_eq!(c.plate_setback, 37.0 + 18.0 + 1.5);
        let c = select(&need(DoorOverlay::Full, 15.0, 18.0)).unwrap();
        assert_eq!(c.plate_setback, 37.0);
    }

    #[test]
    fn a_thick_door_with_a_small_gap_warns() {
        // TB 4, FD 26 needs 3.8 mm.
        let c = select(&need(DoorOverlay::Full, 15.0, 26.0)).unwrap();
        assert!(
            c.warnings.iter().any(|w| w.contains("3.8 mm")),
            "{:?}",
            c.warnings
        );
        // and is too thick to clip onto 16 mm sides at MD 0 (max 20).
        assert!(
            c.warnings.iter().any(|w| w.contains("max 20")),
            "{:?}",
            c.warnings
        );
        assert!(
            select(&need(DoorOverlay::Full, 15.0, 18.0))
                .unwrap()
                .warnings
                .is_empty()
        );
    }

    #[test]
    fn the_cup_centre_is_the_boring_distance_plus_half_the_cup() {
        let c = select(&need(DoorOverlay::Full, 15.0, 16.0)).unwrap();
        assert_eq!(c.cup_inset(), Ok(4.0 + 17.5));
    }

    #[test]
    fn hinge_count_follows_blums_chart_by_height_and_weight() {
        let n = |h, w| hinge_count(h, w).unwrap().0;
        assert_eq!(n(720.0, 4.0), 2);
        assert_eq!(n(800.0, 4.0), 3);
        assert_eq!(n(720.0, 7.0), 3);
        assert_eq!(n(1900.0, 14.0), 4);
        assert_eq!(n(2200.0, 14.0), 5);
        assert!(hinge_count(2600.0, 30.0).unwrap().1.is_some());
    }
}
