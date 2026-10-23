//! Recognising the plain boxes a user drew in SolidCraft, to turn them into cabinets (SC-2).

use cabinet_core::geom::{Aabb, Axis, Vec3};
use serde_json::Value;

/// Names of the bodies in a `document.inspect` selection: selected bodies, and the bodies of
/// selected faces and edges, each once, in selection order.
pub fn selected_bodies(inspect: &Value) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for item in inspect["selection"].as_array().into_iter().flatten() {
        let name = match item["type"].as_str() {
            Some("body") => item["name"].as_str(),
            Some("face") | Some("edge") => item["body"].as_str(),
            _ => None,
        };
        if let Some(n) = name
            && !names.iter().any(|x| x == n)
        {
            names.push(n.to_string());
        }
    }
    names
}

/// The box a body is, from its `model.faces` result, when it is an axis-aligned cuboid: six flat
/// faces, one facing each way along X, Y and Z, each a full rectangle of the box.
pub fn cuboid(faces: &Value) -> Result<Aabb, String> {
    let faces = faces["faces"].as_array().ok_or("no faces")?;
    if faces.len() != 6 {
        return Err(format!("has {} faces, a box has 6", faces.len()));
    }
    // Per axis: the face positions at the low and high end, and the faces' areas.
    let mut low = [None; 3];
    let mut high = [None; 3];
    let mut areas = [[0.0; 2]; 3];
    for f in faces {
        let n = vec3(&f["plane_normal"]).ok_or("has a curved face")?;
        let c = vec3(&f["centroid"]).ok_or("a face has no centroid")?;
        let (i, axis) = Axis::ALL
            .iter()
            .enumerate()
            .find(|(_, a)| (n.get(**a).abs() - 1.0).abs() < 1e-6)
            .ok_or("is turned off the X, Y and Z axes")?;
        let up = n.get(*axis) > 0.0;
        let side = if up { &mut high } else { &mut low };
        if side[i].replace(c.get(*axis)).is_some() {
            return Err("has two faces facing the same way".into());
        }
        areas[i][usize::from(up)] = f["area"].as_f64().ok_or("a face has no area")?;
    }
    let mut min = Vec3::default();
    let mut size = Vec3::default();
    for (i, axis) in Axis::ALL.into_iter().enumerate() {
        let (lo, hi) = (low[i].ok_or("is open")?, high[i].ok_or("is open")?);
        min = min.with(axis, lo);
        size = size.with(axis, hi - lo);
    }
    // Each face must cover the whole side: an L-shape or a notched box has smaller ones.
    for (i, axis) in Axis::ALL.into_iter().enumerate() {
        let [a, b] = axis.others();
        let full = size.get(a) * size.get(b);
        if areas[i]
            .iter()
            .any(|a| (a - full).abs() > 1e-6 * full.max(1.0))
        {
            return Err("is not a plain box".into());
        }
    }
    Ok(Aabb::new(min, size))
}

fn vec3(v: &Value) -> Option<Vec3> {
    let a = v.as_array()?;
    Some(Vec3::new(
        a.first()?.as_f64()?,
        a.get(1)?.as_f64()?,
        a.get(2)?.as_f64()?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::f64::consts::FRAC_1_SQRT_2;

    /// `model.faces` of a box, as SolidCraft 0.20 answers it.
    fn box_faces(min: [f64; 3], size: [f64; 3]) -> Value {
        let mut faces = Vec::new();
        for (i, n) in [
            [0, 0, -1],
            [0, -1, 0],
            [1, 0, 0],
            [0, 1, 0],
            [-1, 0, 0],
            [0, 0, 1],
        ]
        .into_iter()
        .enumerate()
        {
            let mut c = [0.0; 3];
            let mut area = 1.0;
            for k in 0..3 {
                c[k] = match n[k] {
                    -1 => min[k],
                    1 => min[k] + size[k],
                    _ => {
                        area *= size[k];
                        min[k] + size[k] / 2.0
                    }
                };
            }
            faces.push(json!({ "index": i, "area": area, "centroid": c, "plane_normal": n }));
        }
        json!({ "body": "Box1", "faces": faces })
    }

    #[test]
    fn a_box_is_recognised_with_its_size_and_position() {
        let b = cuboid(&box_faces([100.0, 0.0, 0.0], [600.0, 560.0, 870.0])).unwrap();
        assert_eq!(b.min, Vec3::new(100.0, 0.0, 0.0));
        assert_eq!(b.size, Vec3::new(600.0, 560.0, 870.0));
    }

    #[test]
    fn a_cylinder_is_not_a_box() {
        let f = json!({ "faces": [
            { "area": 1.0, "centroid": [0, 0, 0], "plane_normal": [0, 0, -1] },
            { "area": 1.0, "centroid": [0, 0, 1], "plane_normal": [0, 0, 1] },
            { "area": 1.0, "centroid": [0, 0, 0.5], "cylinder": {} },
        ] });
        assert!(cuboid(&f).is_err());
    }

    #[test]
    fn a_turned_box_is_not_taken() {
        let mut f = box_faces([0.0; 3], [600.0, 560.0, 870.0]);
        f["faces"][2]["plane_normal"] = json!([FRAC_1_SQRT_2, FRAC_1_SQRT_2, 0.0]);
        assert_eq!(cuboid(&f).unwrap_err(), "is turned off the X, Y and Z axes");
    }

    #[test]
    fn a_notched_box_with_six_faces_is_not_taken() {
        let mut f = box_faces([0.0; 3], [600.0, 560.0, 870.0]);
        f["faces"][0]["area"] = json!(1000.0);
        assert_eq!(cuboid(&f).unwrap_err(), "is not a plain box");
    }

    #[test]
    fn selected_faces_count_as_their_body_once() {
        let inspect = json!({ "selection": [
            { "type": "body", "name": "Box1" },
            { "type": "face", "body": "Box2", "index": 3, "point": [0, 0, 0] },
            { "type": "face", "body": "Box1", "index": 1, "point": [0, 0, 0] },
            { "type": "feature", "id": 7 },
        ] });
        assert_eq!(selected_bodies(&inspect), ["Box1", "Box2"]);
    }
}
