//! Prints the part list of a default cabinet with JSON overrides merged in (the app's cabinet
//! JSON), for comparing with a supplier's planning output.
//!
//!     cargo run -q -p cabinet-core --example part_list -- overrides.json > parts.json
//!
//! `overrides.json` holds `kind` and any cabinet fields; objects such as `standard` merge.

use serde_json::Value;

fn merge(into: &mut Value, from: &Value) {
    match (into, from) {
        (Value::Object(a), Value::Object(b)) => {
            for (k, v) in b {
                merge(a.entry(k.clone()).or_insert(Value::Null), v);
            }
        }
        (a, b) => *a = b.clone(),
    }
}

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: part_list <overrides.json>");
    let text = std::fs::read_to_string(&path).expect("read the overrides");
    let overrides: Value = serde_json::from_str(&text).expect("JSON");
    let kind = serde_json::from_value(overrides["kind"].clone()).expect("a cabinet kind");
    let mut cab = serde_json::to_value(cabinet_core::Cabinet::new("C01", kind)).unwrap();
    merge(&mut cab, &overrides);
    let cab: cabinet_core::Cabinet = serde_json::from_value(cab).expect("a cabinet");
    let list = cabinet_core::build_cabinet(&cab).expect("build");
    println!("{}", serde_json::to_string_pretty(&list).unwrap());
}
