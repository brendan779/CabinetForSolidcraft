//! Limits on what a request may ask for. The engine trusts its input, so the service checks it
//! first: a cabinet 1e300 mm wide would make the builder allocate until the process dies.

use std::collections::HashSet;

use cabinet_core::Run;
use serde_json::Value;

use crate::Options;

/// The largest any number in a run may be (mm, or a count).
pub const MAX_LENGTH: f64 = 20_000.0;
/// Largest outside width, depth or height of one cabinet (mm).
pub const MAX_CABINET: f64 = 5_000.0;
pub const MAX_ITEMS: usize = 100;
pub const MAX_DRAWERS: usize = 12;
pub const MAX_ID: usize = 32;
pub const MAX_JOB: usize = 120;
/// Longest list anywhere in a run (fixed shelves, split widths, fronts...).
pub const MAX_LIST: usize = 100;
/// Most parts one build may make, checked before any file is drawn: a big kitchen is a few
/// hundred.
pub const MAX_PARTS: usize = 2_000;
/// Smallest tile laid one by one (mm). Mosaic comes on sheets: give it as one sheet-sized tile.
pub const MIN_TILE: f64 = 50.0;
pub const MAX_GROUT: f64 = 20.0;

/// Counts the builder makes that many parts or holes from. Every number is already within
/// ±MAX_LENGTH, which is far too many shelves.
const COUNT_CAPS: &[(&str, f64)] = &[
    ("adjustable_shelves", 20.0),
    ("shelves", 20.0),
    ("inner_drawers", 12.0),
    ("count", 50.0),
    ("mechanisms", 20.0),
    ("hinges", 20.0),
    ("distributors", 20.0),
];

/// Ids become part names and file names: keep them to letters, digits, `.`, `_` and `-`.
fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= MAX_ID
        && !id.starts_with('.')
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

/// Every number anywhere in the run is finite and within ±MAX_LENGTH, every list is short,
/// counts are small, and tiles are big enough to lay one by one: whatever the builder makes in
/// proportion to a number stays bounded.
fn check_numbers(v: &Value, path: &str) -> Result<(), String> {
    match v {
        Value::Number(n) => match n.as_f64() {
            Some(x) if x.is_finite() && x.abs() <= MAX_LENGTH => Ok(()),
            _ => Err(format!(
                "{path} is {n}: numbers must be within ±{MAX_LENGTH}"
            )),
        },
        Value::Array(a) if a.len() > MAX_LIST => Err(format!(
            "{path} has {} entries: at most {MAX_LIST}",
            a.len()
        )),
        Value::Array(a) => a
            .iter()
            .enumerate()
            .try_for_each(|(i, x)| check_numbers(x, &format!("{path}[{i}]"))),
        Value::Object(o) => o.iter().try_for_each(|(k, x)| {
            let at = format!("{path}.{k}");
            check_numbers(x, &at)?;
            let Some(n) = x.as_f64() else { return Ok(()) };
            if let Some((_, cap)) = COUNT_CAPS.iter().find(|(name, _)| name == k)
                && n > *cap
            {
                return Err(format!("{at} is {n}: at most {cap}"));
            }
            match k.as_str() {
                "tile_width" | "tile_height" if n > 0.0 && n < MIN_TILE => Err(format!(
                    "{at} is {n}: tiles laid one by one are at least {MIN_TILE} mm (give mosaic as one sheet-sized tile)"
                )),
                "grout" if !(0.0..=MAX_GROUT).contains(&n) => Err(format!(
                    "{at} is {n}: grout is from 0 to {MAX_GROUT} mm"
                )),
                _ => Ok(()),
            }
        }),
        _ => Ok(()),
    }
}

/// The built run is small enough to draw: refused before any PDF or DXF is made.
pub fn parts(count: usize) -> Result<(), String> {
    if count > MAX_PARTS {
        return Err(format!(
            "the run makes {count} parts: at most {MAX_PARTS} in one build"
        ));
    }
    Ok(())
}

pub fn run(run: &Run) -> Result<(), String> {
    if run.items.is_empty() {
        return Err("the run has no items".into());
    }
    if run.items.len() > MAX_ITEMS {
        return Err(format!(
            "the run has {} items: at most {MAX_ITEMS}",
            run.items.len()
        ));
    }
    let value = serde_json::to_value(run).map_err(|e| e.to_string())?;
    check_numbers(&value, "run")?;
    let mut seen = HashSet::new();
    for (i, item) in value["items"].as_array().into_iter().flatten().enumerate() {
        if let Some(id) = item.get("id").and_then(Value::as_str) {
            if !valid_id(id) {
                return Err(format!(
                    "items[{i}].id {id:?}: use 1–{MAX_ID} letters, digits, '.', '_' or '-'"
                ));
            }
            if !seen.insert(id.to_string()) {
                return Err(format!("items[{i}].id {id:?} is used twice"));
            }
        }
        if item.get("type").and_then(Value::as_str) == Some("cabinet") {
            for dim in ["width", "depth", "height"] {
                let x = item.get(dim).and_then(Value::as_f64).unwrap_or(0.0);
                if x <= 0.0 || x > MAX_CABINET {
                    return Err(format!(
                        "items[{i}] ({}) {dim} is {x}: a cabinet must be over 0 and at most {MAX_CABINET} mm",
                        item["id"].as_str().unwrap_or("?")
                    ));
                }
            }
            let drawers = item
                .pointer("/fronts/ratios")
                .and_then(Value::as_array)
                .map_or(0, Vec::len);
            if drawers > MAX_DRAWERS {
                return Err(format!(
                    "items[{i}] has {drawers} drawers: at most {MAX_DRAWERS}"
                ));
            }
        }
    }
    Ok(())
}

pub fn options(o: &Options) -> Result<(), String> {
    let range = |name: &str, x: f64, max: f64| {
        if x.is_finite() && (0.0..=max).contains(&x) {
            Ok(())
        } else {
            Err(format!("{name} is {x}: it must be from 0 to {max} mm"))
        }
    };
    range("tape", o.tape, 5.0)?;
    range("kerf", o.kerf, 10.0)?;
    range("trim", o.trim, 50.0)?;
    if o.job.chars().count() > MAX_JOB {
        return Err(format!("job name is longer than {MAX_JOB} characters"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_that_could_escape_a_folder_are_refused() {
        assert!(valid_id("B01"));
        assert!(valid_id("T-01_a.2"));
        assert!(!valid_id("../evil"));
        assert!(!valid_id("a/b"));
        assert!(!valid_id(".hidden"));
        assert!(!valid_id(""));
    }
}
