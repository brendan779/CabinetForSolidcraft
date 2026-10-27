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
            "opening",
            "Opening",
            &[
                ("handles", "Handles", "Handles or knobs on every front"),
                (
                    "tip_on",
                    "TIP-ON",
                    "Handleless: press the front and it springs open",
                ),
                (
                    "servo_drive",
                    "SERVO-DRIVE",
                    "Handleless, electric drawers; doors get TIP-ON",
                ),
            ],
        ),
        group(
            "door_style",
            "Door style",
            &[
                ("slab", "Slab", "Fronts of one board"),
                (
                    "shaker",
                    "Shaker",
                    "Five-piece fronts: a 70 mm frame round a 9 mm panel; small fronts stay slab",
                ),
                (
                    "glass_frame",
                    "Glass",
                    "The frame round 4 mm glass held in a back rebate by beads; drawers get a panel",
                ),
            ],
        ),
        group(
            "handle_size",
            "Handle",
            &[
                ("knob", "Knob", "A knob on one screw"),
                ("cc96", "96", "Handle with 96 mm hole centres"),
                ("cc128", "128", "Handle with 128 mm hole centres"),
                ("cc160", "160", "Handle with 160 mm hole centres"),
                ("cc192", "192", "Handle with 192 mm hole centres"),
                ("cc224", "224", "Handle with 224 mm hole centres"),
                ("cc256", "256", "Handle with 256 mm hole centres"),
                ("cc320", "320", "Handle with 320 mm hole centres"),
                ("cc448", "448", "Handle with 448 mm hole centres"),
            ],
        ),
        group(
            "handle_finish",
            "Handle finish",
            &[
                ("stainless", "Stainless", "Stainless steel handles"),
                ("black", "Black", "Matt black handles"),
                ("brushed_brass", "Brass", "Brushed brass handles"),
                ("brushed_nickel", "Nickel", "Brushed nickel handles"),
            ],
        ),
        group(
            "handle_place",
            "Handle place",
            &[
                (
                    "corner",
                    "Corner",
                    "Doors: by the free edge, near the top (wall: the bottom); deep drawers near the top",
                ),
                (
                    "centre",
                    "Centre",
                    "Doors: by the free edge, halfway up; drawers across the middle",
                ),
                (
                    "edge_pull",
                    "Edge pull",
                    "A pull on the top edge (wall doors: the bottom), screwed from behind",
                ),
                (
                    "finger_pull",
                    "Finger pull",
                    "Handleless: a grip routed along the back of the top edge (wall: the bottom)",
                ),
            ],
        ),
        group(
            "drawer_system",
            "Drawers",
            &[
                (
                    "legrabox",
                    "LEGRABOX",
                    "Blum steel-sided boxes; N, M or K to suit each drawer's height",
                ),
                (
                    "merivobox",
                    "MERIVOBOX",
                    "Blum MERIVOBOX steel sides, straight inside; N, M or K",
                ),
                (
                    "tandembox",
                    "TANDEMBOX",
                    "Blum TANDEMBOX antaro, curved steel sides; N, M or K",
                ),
                (
                    "movento",
                    "Wood on MOVENTO",
                    "Wooden boxes on concealed Blum MOVENTO runners, 40–70 kg",
                ),
                (
                    "tandem",
                    "Wood on TANDEM",
                    "Wooden boxes on concealed Blum TANDEM runners, 30 kg",
                ),
            ],
        ),
        group(
            "drawer_sides",
            "Drawer sides",
            &[
                (
                    "standard",
                    "N, M or K",
                    "Deep drawers are a K box behind a tall front",
                ),
                (
                    "high_fronted",
                    "High sides C/F",
                    "LEGRABOX pure high fronted pull-outs, 177 or 241 mm sides, where they fit",
                ),
            ],
        ),
        group(
            "insert_design",
            "Insert design",
            &[
                ("steel", "Steel", "AMBIA-LINE inserts and frames in steel"),
                (
                    "wood",
                    "Wood décor",
                    "AMBIA-LINE in oak or walnut décor on steel, ordered with a colour code",
                ),
            ],
        ),
        group(
            "front_grain",
            "Front grain",
            &[
                (
                    "free",
                    "Each front",
                    "Fronts are cut on their own; drawer fronts' grain runs across",
                ),
                (
                    "matched",
                    "Continuous",
                    "Stacked fronts run their grain upright, cut in sequence from one strip",
                ),
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
            "leg_fixing",
            "Leg fixing",
            &[
                (
                    "screw_on",
                    "Screw-on",
                    "Each leg's plate screws to the underside of the bottom",
                ),
                (
                    "knock_in",
                    "Knock-in",
                    "Each leg's plate knocks into holes bored in the bottom",
                ),
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
                ("dowel", "Dowels", "Fluted wooden dowels, glued"),
                ("cam_lock", "Cam lock", "Knock-down cam and bolt fittings"),
                ("domino", "DOMINO", "Festool DOMINO loose tenons, glued"),
            ],
        ),
        group(
            "dowel",
            "Dowel size",
            &[
                ("d6x30", "6 × 30", "For 12–15 mm panels"),
                ("d8x30", "8 × 30", "16 mm panels, short"),
                ("d8x35", "8 × 35", "16–18 mm panels"),
                ("d8x40", "8 × 40", "18 mm panels and up"),
                ("d10x40", "10 × 40", "Heavy panels, 22 mm and up"),
                ("d10x50", "10 × 50", "Heavy panels, 25 mm and up"),
            ],
        ),
        group(
            "dowel_spacing",
            "Dowel spacing",
            &[
                (
                    "system32",
                    "32 mm grid",
                    "On the 32 mm system from the front, like a line borer",
                ),
                (
                    "jig",
                    "Doweling jig",
                    "Evenly spread, as a hand jig sets them out",
                ),
            ],
        ),
        group(
            "domino",
            "DOMINO size",
            &[
                ("d4x20", "4 × 20", "DF 500, for thin panels"),
                ("d5x30", "5 × 30", "DF 500, 16 mm carcasses"),
                ("d6x40", "6 × 40", "DF 500, 18–19 mm panels"),
                ("d8x40", "8 × 40", "DF 500"),
                ("d8x50", "8 × 50", "DF 500"),
                ("d10x50", "10 × 50", "DF 500, heavy panels"),
                ("d8x80", "8 × 80", "DF 700 (DOMINO XL)"),
                ("d10x80", "10 × 80", "DF 700 (DOMINO XL)"),
                ("d12x100", "12 × 100", "DF 700 (DOMINO XL)"),
                ("d14x100", "14 × 100", "DF 700 (DOMINO XL)"),
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
