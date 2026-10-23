//! The visual option catalogue: every construction choice the app shows as a picker of
//! thumbnails. Choice ids are the serde names of the model enums, so the app can write a choice
//! straight back into the cabinet JSON.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Choice {
    pub id: String,
    pub label: String,
    pub description: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OptionGroup {
    /// The field of `ConstructionStandard` this group sets.
    pub field: String,
    pub title: String,
    pub choices: Vec<Choice>,
}

fn group(field: &str, title: &str, choices: &[(&str, &str, &str)]) -> OptionGroup {
    OptionGroup {
        field: field.into(),
        title: title.into(),
        choices: choices
            .iter()
            .map(|(id, label, description)| Choice {
                id: (*id).into(),
                label: (*label).into(),
                description: (*description).into(),
            })
            .collect(),
    }
}

/// The construction pickers, in the order the app shows them.
pub fn option_groups() -> Vec<OptionGroup> {
    vec![
        group(
            "top_joint",
            "Top",
            &[
                (
                    "rails",
                    "Rails",
                    "Front and back rails between the sides, for a benchtop on top",
                ),
                (
                    "between_sides",
                    "Between sides",
                    "A full top fitted between the sides",
                ),
                ("over_sides", "Over sides", "A full top across the sides"),
            ],
        ),
        group(
            "bottom_joint",
            "Bottom",
            &[
                (
                    "between_sides",
                    "Between sides",
                    "The bottom fits between the sides",
                ),
                (
                    "under_sides",
                    "Under sides",
                    "The sides stand on the bottom",
                ),
            ],
        ),
        group(
            "back",
            "Back",
            &[
                (
                    "inset_groove",
                    "Inset in groove",
                    "A thin back in grooves, set in from the rear",
                ),
                ("applied", "Applied", "A back screwed onto the rear edges"),
                (
                    "structural",
                    "Structural",
                    "A full-thickness back between the panels",
                ),
                ("none", "None", "Open back"),
            ],
        ),
        group(
            "wall_hanging",
            "Wall hanging",
            &[
                (
                    "hangers",
                    "Hangers + rail",
                    "Concealed adjustable hangers behind the back, on a wall rail",
                ),
                ("none", "None", "The installer fixes wall cabinets"),
            ],
        ),
        group(
            "toe_kick",
            "Base",
            &[
                (
                    "legs_and_plinth",
                    "Legs + plinth",
                    "Adjustable legs with a clip-on plinth",
                ),
                (
                    "plinth_box",
                    "Plinth box",
                    "The carcass sits on a separate plinth",
                ),
                (
                    "integrated_notch",
                    "Notched sides",
                    "Sides run to the floor with a toe-kick notch",
                ),
                ("none", "None", "No toe kick"),
            ],
        ),
        group(
            "overlay",
            "Door mounting",
            &[
                ("full", "Full overlay", "Fronts cover the carcass edges"),
                ("half", "Half overlay", "Fronts cover half of each side"),
                ("inset", "Inset", "Fronts sit inside the carcass opening"),
            ],
        ),
        group(
            "fixings",
            "Fixings",
            &[
                (
                    "biscuit_and_screw",
                    "Biscuit + screw",
                    "Biscuits align, screws clamp between them",
                ),
                ("biscuit_only", "Biscuits", "Glued biscuits only"),
                ("screw_only", "Screws", "Chipboard screws only"),
                ("confirmat", "Confirmat", "7 mm confirmat screws"),
                ("dowel", "Dowels", "8 mm dowels on the 32 mm grid"),
                ("cam_lock", "Cam lock", "Knock-down cam and bolt fittings"),
            ],
        ),
        group(
            "biscuit",
            "Biscuit size",
            &[
                ("no0", "#0", "47 × 15 mm"),
                ("no10", "#10", "53 × 19 mm"),
                ("no20", "#20", "56 × 23 mm"),
            ],
        ),
        group(
            "fixing_density",
            "Fixing spacing",
            &[
                ("sparse", "Sparse", "Up to 250 mm apart"),
                ("normal", "Normal", "Up to 180 mm apart"),
                ("dense", "Dense", "Up to 120 mm apart"),
            ],
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ConstructionStandard;

    /// Every choice id must deserialize into its field, or the app would write bad JSON.
    #[test]
    fn every_choice_is_a_valid_value_for_its_field() {
        let base = serde_json::to_value(ConstructionStandard::default()).unwrap();
        for g in option_groups() {
            assert!(base.get(&g.field).is_some(), "no field {}", g.field);
            for c in &g.choices {
                let mut v = base.clone();
                v[&g.field] = serde_json::Value::String(c.id.clone());
                serde_json::from_value::<ConstructionStandard>(v)
                    .unwrap_or_else(|e| panic!("{}={}: {e}", g.field, c.id));
            }
        }
    }
}
