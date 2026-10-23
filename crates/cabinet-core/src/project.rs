//! A project: the shop's construction standard once, and each cabinet with only the standard
//! fields it overrides. Changing the project standard changes every cabinet that does not
//! override that field.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::model::{Cabinet, ConstructionStandard};
use crate::{Error, Result};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub name: String,
    pub standard: ConstructionStandard,
    pub cabinets: Vec<StoredCabinet>,
}

/// A cabinet as the project stores it: everything but the standard, plus the standard's fields
/// that differ from the project's.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StoredCabinet {
    /// The cabinet's own fields (id, kind, size, fronts...), without `standard`.
    #[serde(flatten)]
    pub fields: Map<String, Value>,
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub overrides: Map<String, Value>,
}

impl Project {
    /// A project holding `cabinets` under `standard`: each keeps only its differences from it.
    pub fn new(
        name: impl Into<String>,
        standard: ConstructionStandard,
        cabinets: &[Cabinet],
    ) -> Result<Self> {
        let base = to_map(&standard)?;
        let cabinets = cabinets
            .iter()
            .map(|c| {
                let mut fields = to_map(c)?;
                let own = match fields.remove("standard") {
                    Some(Value::Object(m)) => m,
                    _ => Map::new(),
                };
                let overrides = own
                    .into_iter()
                    .filter(|(k, v)| base.get(k) != Some(v))
                    .collect();
                Ok(StoredCabinet { fields, overrides })
            })
            .collect::<Result<_>>()?;
        Ok(Self {
            name: name.into(),
            standard,
            cabinets,
        })
    }

    /// The project's cabinets with their standards resolved.
    pub fn resolved(&self) -> Result<Vec<Cabinet>> {
        let base = to_map(&self.standard)?;
        self.cabinets
            .iter()
            .map(|s| {
                let mut standard = base.clone();
                standard.extend(s.overrides.clone());
                let mut fields = s.fields.clone();
                fields.insert("standard".into(), Value::Object(standard));
                serde_json::from_value(Value::Object(fields))
                    .map_err(|e| Error::Invalid(e.to_string()))
            })
            .collect()
    }
}

fn to_map(v: &impl Serialize) -> Result<Map<String, Value>> {
    match serde_json::to_value(v).map_err(|e| Error::Invalid(e.to_string()))? {
        Value::Object(m) => Ok(m),
        _ => Err(Error::Invalid("expected an object".into())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{BackStyle, CabinetKind, ToeKick};

    fn kitchen() -> Vec<Cabinet> {
        let mut wall = Cabinet::new("W01", CabinetKind::Wall);
        wall.standard.back = BackStyle::Applied;
        vec![Cabinet::new("B01", CabinetKind::Base), wall]
    }

    #[test]
    fn the_project_stores_one_standard_and_only_the_overrides() {
        let p = Project::new("Kitchen", ConstructionStandard::default(), &kitchen()).unwrap();
        let json = serde_json::to_value(&p).unwrap();
        assert!(json["standard"].is_object());
        assert!(json["cabinets"][0].get("standard").is_none());
        assert!(
            json["cabinets"][0].get("overrides").is_none(),
            "B01 follows the standard"
        );
        assert_eq!(
            json["cabinets"][1]["overrides"],
            serde_json::json!({ "back": "applied" })
        );
    }

    #[test]
    fn cabinets_round_trip_through_the_project() {
        let cabs = kitchen();
        let p = Project::new("Kitchen", ConstructionStandard::default(), &cabs).unwrap();
        let text = serde_json::to_string(&p).unwrap();
        let back: Project = serde_json::from_str(&text).unwrap();
        assert_eq!(back.resolved().unwrap(), cabs);
    }

    #[test]
    fn changing_the_project_standard_reaches_cabinets_that_do_not_override_it() {
        let mut p = Project::new("Kitchen", ConstructionStandard::default(), &kitchen()).unwrap();
        p.standard.toe_kick = ToeKick::PlinthBox;
        p.standard.back = BackStyle::Structural;
        let cabs = p.resolved().unwrap();
        assert_eq!(cabs[0].standard.toe_kick, ToeKick::PlinthBox);
        assert_eq!(cabs[0].standard.back, BackStyle::Structural);
        assert_eq!(
            cabs[1].standard.back,
            BackStyle::Applied,
            "W01 keeps its override"
        );
    }
}
