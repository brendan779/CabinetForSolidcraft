//! Cabinet lighting: LED strips per cabinet, and the rules that wire them to drivers and
//! distributors. The rules are the owner's (BACKLOG HAF-5); the drivers and distributors come in
//! as data, so the Häfele Loox5 articles (HAF-4) plug in once they are catalogued.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::model::{Cabinet, CabinetKind, Light, LightZone, Voltage};

/// A driver may carry at most this share of its rated power.
pub const MAX_DRIVER_LOAD: f64 = 0.8;
/// Our choice: a strip stops this far short of each end of the run it lights.
pub const STRIP_END_CLEARANCE: f64 = 10.0;

/// One LED strip, cut to its cabinet.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Strip {
    pub cabinet: String,
    pub zone: LightZone,
    pub voltage: Voltage,
    pub length: f64,
    pub watts: f64,
}

/// A driver (power supply) the plan may use.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DriverSpec {
    pub id: String,
    pub voltage: Voltage,
    pub rated_watts: f64,
}

/// One driver, the distributors behind it and the strips they feed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Circuit {
    pub driver: DriverSpec,
    pub strips: Vec<Strip>,
    pub distributors: u32,
}

impl Circuit {
    pub fn load(&self) -> f64 {
        self.strips.iter().map(|s| s.watts).sum()
    }
}

/// Whether a cabinet of `kind` can carry a light in `zone`: under-cabinet lights need a cabinet
/// off the floor, plinth lights a toe kick.
pub fn zone_fits(cab: &Cabinet, zone: LightZone) -> bool {
    match zone {
        LightZone::UnderCabinet => matches!(
            cab.kind,
            CabinetKind::Wall | CabinetKind::OpenShelf | CabinetKind::CornerWall
        ),
        LightZone::InCabinet => true,
        LightZone::Plinth => cab.has_toe_kick(),
    }
}

/// The strips a cabinet's lights need: the inside width for under- and in-cabinet lights, the
/// plinth's length for plinth lights, short of each end by [`STRIP_END_CLEARANCE`].
pub fn strips(cab: &Cabinet) -> Vec<Strip> {
    let t = cab.standard.carcass_thickness;
    cab.lights
        .iter()
        .filter(|l| zone_fits(cab, l.zone))
        .map(|l: &Light| {
            let span = match l.zone {
                LightZone::Plinth => cab.width,
                _ => cab.width - 2.0 * t,
            };
            let length = (span - 2.0 * STRIP_END_CLEARANCE).max(0.0);
            Strip {
                cabinet: cab.id.clone(),
                zone: l.zone,
                voltage: l.voltage,
                length,
                watts: l.watts_per_metre * length / 1000.0,
            }
        })
        .collect()
}

/// What is wrong with a circuit: strips of another voltage than the driver, a driver loaded past
/// [`MAX_DRIVER_LOAD`], more strips than its distributors have ports.
pub fn check(circuit: &Circuit, ports_per_distributor: u32) -> Vec<String> {
    let mut problems = Vec::new();
    let d = &circuit.driver;
    for s in circuit.strips.iter().filter(|s| s.voltage != d.voltage) {
        problems.push(format!(
            "{}: a {} strip on the {} driver {}; 12 V and 24 V do not mix",
            s.cabinet,
            s.voltage.label(),
            d.voltage.label(),
            d.id
        ));
    }
    let load = circuit.load();
    if load > MAX_DRIVER_LOAD * d.rated_watts + 1e-9 {
        problems.push(format!(
            "driver {} carries {load:.1} W, over {:.0} % of its {:.0} W",
            d.id,
            MAX_DRIVER_LOAD * 100.0,
            d.rated_watts
        ));
    }
    let ports = circuit.distributors * ports_per_distributor;
    if circuit.strips.len() as u32 > ports {
        problems.push(format!(
            "driver {}: {} strips on {} distributor ports",
            d.id,
            circuit.strips.len(),
            ports
        ));
    }
    problems
}

/// Wires `strips` to drivers from `drivers`: one voltage per driver, strips added in order while
/// the largest driver of that voltage stays within [`MAX_DRIVER_LOAD`], then the smallest driver
/// that carries the circuit, and enough distributors for its strips. Returns the circuits and
/// what could not be wired.
pub fn plan(
    strips: &[Strip],
    drivers: &[DriverSpec],
    ports_per_distributor: u32,
) -> (Vec<Circuit>, Vec<String>) {
    let mut by_voltage: BTreeMap<Voltage, Vec<&Strip>> = BTreeMap::new();
    for s in strips {
        by_voltage.entry(s.voltage).or_default().push(s);
    }
    let (mut circuits, mut problems) = (Vec::new(), Vec::new());
    for (voltage, group) in by_voltage {
        let mut fitting: Vec<&DriverSpec> =
            drivers.iter().filter(|d| d.voltage == voltage).collect();
        fitting.sort_by(|a, b| a.rated_watts.total_cmp(&b.rated_watts));
        let Some(largest) = fitting.last() else {
            problems.push(format!("no {} driver to choose from", voltage.label()));
            continue;
        };
        let limit = MAX_DRIVER_LOAD * largest.rated_watts;
        let mut loads: Vec<Vec<Strip>> = Vec::new();
        for s in group {
            if s.watts > limit + 1e-9 {
                problems.push(format!(
                    "{}: its {:.1} W strip is more than any {} driver may carry",
                    s.cabinet,
                    s.watts,
                    voltage.label()
                ));
                continue;
            }
            let load = |c: &Vec<Strip>| c.iter().map(|s| s.watts).sum::<f64>();
            match loads.last_mut() {
                Some(c) if load(c) + s.watts <= limit + 1e-9 => c.push(s.clone()),
                _ => loads.push(vec![s.clone()]),
            }
        }
        for c in loads {
            let watts: f64 = c.iter().map(|s| s.watts).sum();
            let driver = fitting
                .iter()
                .find(|d| watts <= MAX_DRIVER_LOAD * d.rated_watts + 1e-9)
                .unwrap_or(largest);
            let distributors = (c.len() as u32).div_ceil(ports_per_distributor.max(1));
            circuits.push(Circuit {
                driver: (*driver).clone(),
                strips: c,
                distributors,
            });
        }
    }
    for c in &circuits {
        problems.extend(check(c, ports_per_distributor));
    }
    (circuits, problems)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strip(id: &str, voltage: Voltage, watts: f64) -> Strip {
        Strip {
            cabinet: id.into(),
            zone: LightZone::UnderCabinet,
            voltage,
            length: 1000.0,
            watts,
        }
    }

    fn driver(id: &str, voltage: Voltage, rated_watts: f64) -> DriverSpec {
        DriverSpec {
            id: id.into(),
            voltage,
            rated_watts,
        }
    }

    #[test]
    fn a_strip_of_the_other_voltage_on_a_driver_is_refused() {
        let c = Circuit {
            driver: driver("d24", Voltage::V24, 60.0),
            strips: vec![strip("W01", Voltage::V12, 5.0)],
            distributors: 1,
        };
        let p = check(&c, 6);
        assert_eq!(p.len(), 1);
        assert!(p[0].contains("do not mix"), "{p:?}");
    }

    #[test]
    fn a_driver_may_carry_eighty_percent_of_its_rating() {
        let at = |w| Circuit {
            driver: driver("d", Voltage::V24, 30.0),
            strips: vec![strip("W01", Voltage::V24, w)],
            distributors: 1,
        };
        assert!(check(&at(24.0), 6).is_empty());
        assert!(check(&at(24.1), 6)[0].contains("over 80 %"));
    }

    #[test]
    fn strips_beyond_the_distributor_ports_are_reported() {
        let c = Circuit {
            driver: driver("d", Voltage::V12, 100.0),
            strips: (0..7)
                .map(|i| strip(&format!("W0{i}"), Voltage::V12, 1.0))
                .collect(),
            distributors: 1,
        };
        assert!(check(&c, 6)[0].contains("7 strips on 6"));
    }

    #[test]
    fn the_plan_splits_voltages_and_loads_and_takes_the_smallest_driver() {
        let drivers = [
            driver("d12-15", Voltage::V12, 15.0),
            driver("d24-30", Voltage::V24, 30.0),
            driver("d24-60", Voltage::V24, 60.0),
        ];
        let strips = [
            strip("W01", Voltage::V24, 20.0),
            strip("W02", Voltage::V24, 20.0),
            strip("W03", Voltage::V24, 20.0),
            strip("W04", Voltage::V12, 4.0),
        ];
        let (circuits, problems) = plan(&strips, &drivers, 6);
        assert_eq!(problems, Vec::<String>::new());
        let summary: Vec<_> = circuits
            .iter()
            .map(|c| (c.driver.id.as_str(), c.strips.len(), c.distributors))
            .collect();
        // 48 W fills a 60 W driver to 80 %; the third 20 W strip takes a 30 W one.
        assert_eq!(
            summary,
            [("d12-15", 1, 1), ("d24-60", 2, 1), ("d24-30", 1, 1)]
        );
        for c in &circuits {
            assert!(c.strips.iter().all(|s| s.voltage == c.driver.voltage));
            assert!(c.load() <= MAX_DRIVER_LOAD * c.driver.rated_watts);
        }
    }

    #[test]
    fn the_plan_adds_distributors_for_more_strips_than_ports() {
        let strips: Vec<_> = (0..9)
            .map(|i| strip(&format!("W{i}"), Voltage::V12, 1.0))
            .collect();
        let (circuits, problems) = plan(&strips, &[driver("d", Voltage::V12, 60.0)], 6);
        assert!(problems.is_empty());
        assert_eq!(circuits.len(), 1);
        assert_eq!(circuits[0].distributors, 2);
    }

    #[test]
    fn a_strip_no_driver_can_carry_is_reported() {
        let (circuits, problems) = plan(
            &[strip("W01", Voltage::V12, 50.0)],
            &[driver("d", Voltage::V12, 30.0)],
            6,
        );
        assert!(circuits.is_empty());
        assert!(problems[0].contains("more than any 12 V driver"));
        let (_, problems) = plan(&[strip("W01", Voltage::V24, 5.0)], &[], 6);
        assert!(problems[0].contains("no 24 V driver"));
    }

    #[test]
    fn strips_run_the_inside_width_and_only_where_their_zone_fits() {
        let mut wall = Cabinet::new("W01", CabinetKind::Wall);
        wall.lights = vec![
            Light {
                zone: LightZone::UnderCabinet,
                voltage: Voltage::V24,
                watts_per_metre: 10.0,
            },
            Light {
                zone: LightZone::Plinth,
                voltage: Voltage::V24,
                watts_per_metre: 10.0,
            },
        ];
        let s = strips(&wall);
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].length, 600.0 - 32.0 - 20.0);
        assert!((s[0].watts - 5.48).abs() < 1e-9);
    }

    /// HAF-6: the app writes lights as `{zone, voltage: "24v", watts_per_metre}`.
    #[test]
    fn lights_read_from_the_apps_json() {
        let cab: Cabinet = serde_json::from_value(serde_json::json!({
            "id": "W01", "kind": "wall", "width": 600.0, "depth": 330.0, "height": 720.0,
            "lights": [{"zone": "under_cabinet", "voltage": "24v", "watts_per_metre": 9.6}]
        }))
        .unwrap();
        assert_eq!(cab.lights[0].voltage, Voltage::V24);
        assert_eq!(strips(&cab).len(), 1);
    }
}
