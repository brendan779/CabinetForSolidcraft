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
            "handle_range",
            "Handle range",
            &[
                (
                    "generic",
                    "Generic",
                    "Our bar handles and knobs at the standard centres",
                ),
                (
                    "abi",
                    "ABI Interiors",
                    "ABI's pulls and knobs; the style picker chooses family, size and finish",
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
                    "Five-piece fronts: a frame round a thin grooved panel; small fronts stay slab",
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
                    "sharknose",
                    "Sharknose",
                    "No handle: each front's top edge (wall: the bottom) bevelled at the back to pull on",
                ),
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
                    "A pull on the top edge (wall doors: the bottom), screwed from behind. For ABI's own edge pulls, choose ABI Interiors under Handle range",
                ),
                (
                    "finger_pull",
                    "Finger pull",
                    "Handleless: a grip routed along the back of the top edge (wall: the bottom)",
                ),
            ],
        ),
        group(
            "sharknose_angle",
            "Sharknose bevel",
            &[
                (
                    "bevel_30",
                    "30°",
                    "A long slope down the back of the edge: the deepest grip",
                ),
                ("bevel_45", "45°", "The usual reverse bevel"),
                ("bevel_60", "60°", "A short, steep slope"),
            ],
        ),
        group(
            "sharknose_door_edge",
            "Sharknose on doors",
            &[
                ("auto", "Usual", "Top of a base door, bottom of a wall door"),
                ("top", "Top", "The door's top edge"),
                ("bottom", "Bottom", "The door's bottom edge"),
                ("opening", "Opening side", "The side opposite the hinges"),
            ],
        ),
        group(
            "drawer_frame",
            "Drawer frame",
            &[
                ("none", "No frame", "The carcass shows between the fronts"),
                (
                    "board",
                    "Frame",
                    "One board behind the fronts, cut out for each drawer",
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
                    "Usual",
                    "Doors upright, drawer fronts across, each cut on its own",
                ),
                (
                    "upright",
                    "Upright",
                    "Every front's grain runs upright, each cut on its own",
                ),
                (
                    "across",
                    "Across",
                    "Every front's grain runs across, each cut on its own",
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
                    "integrated_notch",
                    "Notched sides",
                    "Sides run to the floor with a toe-kick notch; the kick board screws to a hidden block",
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
            "hinge_boss",
            "Hinge fixing",
            &[
                (
                    "screw_on",
                    "Screw-on",
                    "Two screws through the hinge into the door",
                ),
                (
                    "expando",
                    "EXPANDO",
                    "Pre-mounted screws with split dowels in two 8 mm holes",
                ),
                (
                    "inserta",
                    "INSERTA",
                    "Dowels pressed into two 8 mm holes, no tools",
                ),
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

/// Whether the picker for `field` means anything under `s`: the size of a fixing only with that
/// fixing, handle pickers only with handles, leg fixing only on legs, drawer side height only for
/// steel-sided drawers. The app hides the others.
pub fn applies(field: &str, s: &crate::model::ConstructionStandard) -> bool {
    use crate::model::{DrawerSystem, FixingRecipe, HandleRange, Opening, ToeKick};
    match field {
        "biscuit" => matches!(
            s.fixings,
            FixingRecipe::BiscuitAndScrew | FixingRecipe::BiscuitOnly
        ),
        "domino" => s.fixings == FixingRecipe::Domino,
        "dowel" | "dowel_spacing" => s.fixings == FixingRecipe::Dowel,
        "handle_range" | "handle_place" => s.opening == Opening::Handles,
        "sharknose_angle" | "sharknose_door_edge" => s.is_sharknose(),
        // The generic size and finish pickers; an ABI job picks family, size and finish together.
        "handle_size" | "handle_finish" => {
            s.opening == Opening::Handles
                && s.handle_range == HandleRange::Generic
                && !s.is_sharknose()
        }
        "leg_fixing" => s.toe_kick == ToeKick::LegsAndPlinth,
        "drawer_sides" => matches!(
            s.drawer_system,
            DrawerSystem::Legrabox | DrawerSystem::Merivobox | DrawerSystem::Tandembox
        ),
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ConstructionStandard;

    #[test]
    fn a_sharknose_front_asks_for_its_bevel_and_no_handle_size() {
        let mut s = ConstructionStandard::default();
        assert!(!applies("sharknose_angle", &s));
        assert!(applies("handle_size", &s));
        s.handle_place = crate::model::HandlePlace::Sharknose;
        assert!(applies("sharknose_angle", &s));
        assert!(!applies("handle_size", &s));
        assert!(!applies("handle_finish", &s));
    }

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

    #[test]
    fn only_the_chosen_fixing_shows_its_size() {
        use crate::model::{ConstructionStandard, FixingRecipe, HandleRange, Opening};
        let mut s = ConstructionStandard {
            fixings: FixingRecipe::ScrewOnly,
            ..Default::default()
        };
        for f in ["biscuit", "domino", "dowel", "dowel_spacing"] {
            assert!(!applies(f, &s), "{f}");
        }
        s.fixings = FixingRecipe::Domino;
        assert!(applies("domino", &s) && !applies("biscuit", &s));
        s.fixings = FixingRecipe::Dowel;
        assert!(applies("dowel", &s) && applies("dowel_spacing", &s));
        s.opening = Opening::TipOn;
        assert!(!applies("handle_size", &s));
        assert!(applies("fixings", &s));
        // ABI-3: the generic size and finish give way to the ABI style picker.
        s.opening = Opening::Handles;
        s.handle_range = HandleRange::Abi;
        assert!(applies("handle_range", &s));
        assert!(!applies("handle_size", &s) && !applies("handle_finish", &s));
        s.handle_range = HandleRange::Generic;
        assert!(applies("handle_size", &s) && applies("handle_finish", &s));
    }
}
