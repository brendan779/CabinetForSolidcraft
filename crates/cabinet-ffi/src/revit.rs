//! A folder Revit can take a job from. No file format brings textures into Revit, so the folder holds
//! the geometry (`job.obj`, from `solidcraft-cli`), the manifest, every decor's image, and a Dynamo
//! graph that builds the parts and their materials with Revit's API.

use super::*;
use serde_json::{Value, json};
use std::collections::BTreeMap;

const SCRIPT: &str = include_str!("../../../revit/import_into_revit.py");
const GRAPH: &str = "Import into Revit.dyn";
const README: &str = "Cabinet Studio job for Revit

1. Copy this whole folder to the Revit machine. Keep it there: Revit links the images in textures/
   rather than storing them.
2. Open a project in Revit, then Manage > Dynamo.
3. Open \"Import into Revit.dyn\" from this folder and press Run.

Each part arrives as a Casework element at the project's origin, named after its part and grouped by
cabinet, in a material with its decor's image. Set the view to Realistic to see the images, or render.
Running it again replaces this job's parts and leaves the rest of the model alone.
These images are the board brands' own: don't pass them on.
";

/// A decor image a job needs.
#[derive(uniffi::Record)]
pub struct RevitImage {
    pub name: String,
    pub url: Option<String>,
}

fn job_images(folder: &Path) -> Result<BTreeMap<String, Option<String>>, CoreError> {
    let text = std::fs::read_to_string(folder.join("job.json")).map_err(invalid)?;
    let job: Value = serde_json::from_str(&text).map_err(invalid)?;
    let mut images = BTreeMap::new();
    if let Some(boards) = job["boards"].as_object() {
        for board in boards.values() {
            if let Some(name) = board["image"].as_str() {
                images.insert(name.to_string(), board["url"].as_str().map(str::to_string));
            }
        }
    }
    Ok(images)
}

/// The images of the job in `folder` (made by `write_blender_job`).
#[uniffi::export]
pub fn revit_images(folder: String) -> Result<Vec<RevitImage>, CoreError> {
    Ok(job_images(Path::new(&folder))?
        .into_iter()
        .map(|(name, url)| RevitImage { name, url })
        .collect())
}

/// Dynamo's file for one Python node holding the import script, in the form Revit 2026's opens.
fn dynamo_graph() -> Value {
    let node = "c0ab1e5a7d3e4f6c9a11b2c3d4e5f601";
    json!({
        "Uuid": "6f1c2d3e-4b5a-4c6d-8e7f-90a1b2c3d4e5",
        "IsCustomNode": false,
        "Description": "Brings a Cabinet Studio job into this project, with its textures.",
        "Name": "Import into Revit",
        "ElementResolver": { "ResolutionMap": {} },
        "Inputs": [],
        "Outputs": [],
        "Nodes": [{
            "ConcreteType": "PythonNodeModels.PythonNode, PythonNodeModels",
            "Code": SCRIPT,
            "Engine": "CPython3",
            "EngineName": "CPython3",
            "VariableInputPorts": true,
            "NodeType": "PythonScriptNode",
            "Id": node,
            "Inputs": [],
            "Outputs": [{
                "Id": "c0ab1e5a7d3e4f6c9a11b2c3d4e5f602",
                "Name": "OUT",
                "Description": "What it made, and anything that failed",
                "UsingDefaultValue": false,
                "Level": 2,
                "UseLevels": false,
                "KeepListStructure": false
            }],
            "Replication": "Disabled",
            "Description": "Runs the import"
        }],
        "Connectors": [],
        "Dependencies": [],
        "NodeLibraryDependencies": [],
        "Bindings": [],
        "View": {
            "Dynamo": {
                "ScaleFactor": 1.0,
                "HasRunWithoutCrash": false,
                "IsVisibleInDynamoLibrary": true,
                "Version": "3.0.0",
                "RunType": "Manual",
                "RunPeriod": "1000"
            },
            "Camera": {
                "Name": "Background Preview",
                "EyeX": -17.0, "EyeY": 24.0, "EyeZ": 50.0,
                "LookX": 12.0, "LookY": -13.0, "LookZ": -58.0,
                "UpX": 0.0, "UpY": 1.0, "UpZ": 0.0
            },
            "ConnectorPins": [],
            "NodeViews": [{
                "Id": node,
                "Name": "Import into Revit",
                "IsSetAsInput": false,
                "IsSetAsOutput": false,
                "Excluded": false,
                "ShowGeometry": true,
                "X": 200.0,
                "Y": 200.0
            }],
            "Annotations": [],
            "X": 0.0,
            "Y": 0.0,
            "Zoom": 1.0
        }
    })
}

/// Finishes the folder: copies each image from `decors_dir` and adds the graph. Returns the images
/// that aren't there: those parts arrive in plain colour.
#[uniffi::export]
pub fn write_revit_package(folder: String, decors_dir: String) -> Result<Vec<String>, CoreError> {
    let folder = Path::new(&folder);
    let decors = Path::new(&decors_dir);
    let textures = folder.join("textures");
    std::fs::create_dir_all(&textures).map_err(invalid)?;
    let mut missing = Vec::new();
    for name in job_images(folder)?.into_keys() {
        let image = format!("{name}.jpg");
        if std::fs::copy(decors.join(&image), textures.join(&image)).is_err() {
            missing.push(name);
            continue;
        }
        // The height map Blender made, if this decor has been to Blender.
        let height = format!("{name}-h1.png");
        let _ = std::fs::copy(decors.join(&height), textures.join(&height));
    }
    let graph = serde_json::to_string_pretty(&dynamo_graph()).map_err(invalid)?;
    std::fs::write(folder.join(GRAPH), graph).map_err(invalid)?;
    std::fs::write(folder.join("import_into_revit.py"), SCRIPT).map_err(invalid)?;
    std::fs::write(folder.join("README.txt"), README).map_err(invalid)?;
    // Only SolidCraft needed it.
    let _ = std::fs::remove_file(folder.join("build.json"));
    Ok(missing)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("cabinet-revit-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn job_with(folder: &Path, images: &[&str]) {
        let boards: serde_json::Map<String, Value> = images
            .iter()
            .map(|n| {
                (
                    format!("b.{n}"),
                    json!({ "image": n, "url": format!("https://x/{n}.jpg") }),
                )
            })
            .collect();
        std::fs::write(
            folder.join("job.json"),
            json!({ "boards": boards, "parts": [] }).to_string(),
        )
        .unwrap();
    }

    #[test]
    fn a_package_holds_each_image_and_names_the_ones_it_could_not_find() {
        let (dir, decors) = (scratch("pack"), scratch("decors"));
        job_with(&dir, &["oak", "gone"]);
        std::fs::write(decors.join("oak.jpg"), b"jpg").unwrap();
        std::fs::write(decors.join("oak-h1.png"), b"png").unwrap();
        std::fs::write(decors.join("unused.jpg"), b"jpg").unwrap();
        std::fs::write(dir.join("build.json"), b"{}").unwrap();
        let missing = write_revit_package(
            dir.to_str().unwrap().into(),
            decors.to_str().unwrap().into(),
        )
        .unwrap();
        assert_eq!(missing, ["gone"]);
        assert!(dir.join("textures/oak.jpg").exists());
        assert!(dir.join("textures/oak-h1.png").exists());
        assert!(!dir.join("textures/unused.jpg").exists());
        assert!(!dir.join("build.json").exists());
        assert!(dir.join("README.txt").exists());
    }

    #[test]
    fn the_graph_carries_the_whole_import_script() {
        let (dir, decors) = (scratch("graph"), scratch("empty"));
        job_with(&dir, &[]);
        write_revit_package(
            dir.to_str().unwrap().into(),
            decors.to_str().unwrap().into(),
        )
        .unwrap();
        let text = std::fs::read_to_string(dir.join(GRAPH)).unwrap();
        let graph: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(graph["Nodes"][0]["Code"], SCRIPT);
        assert_eq!(graph["Nodes"][0]["Id"], graph["View"]["NodeViews"][0]["Id"]);
    }

    #[test]
    fn revit_images_lists_a_shared_decor_once_with_its_address() {
        let dir = scratch("list");
        std::fs::write(
            dir.join("job.json"),
            json!({ "boards": {
                "a.16": { "image": "oak", "url": "https://x/oak.jpg" },
                "a.18": { "image": "oak", "url": "https://x/oak.jpg" },
                "none": { "image": null, "url": null },
            }})
            .to_string(),
        )
        .unwrap();
        let images = revit_images(dir.to_str().unwrap().into()).unwrap();
        assert_eq!(images.len(), 1);
        assert_eq!(images[0].url.as_deref(), Some("https://x/oak.jpg"));
    }
}
