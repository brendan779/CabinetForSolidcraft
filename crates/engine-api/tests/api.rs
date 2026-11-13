use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use engine_api::{AppState, router, sample_run};
use http_body_util::BodyExt;
use tower::ServiceExt;

fn app() -> axum::Router {
    router(Arc::new(AppState::default()))
}

async fn post(path: &str, body: String) -> (StatusCode, Vec<u8>) {
    let res = app()
        .oneshot(Request::post(path).body(Body::from(body)).unwrap())
        .await
        .unwrap();
    let status = res.status();
    (
        status,
        res.into_body().collect().await.unwrap().to_bytes().to_vec(),
    )
}

#[tokio::test]
async fn sample_kitchen_builds_with_parts_hardware_and_no_clashes() {
    let run = serde_json::to_string(&sample_run()).unwrap();
    let (status, body) = post("/v1/build", run).await;
    assert_eq!(status, StatusCode::OK);
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["cabinets"], 5);
    assert!(v["parts"].as_u64().unwrap() > 40);
    assert!(v["operations"].as_u64().unwrap() > 100);
    assert_eq!(v["clashes"].as_array().unwrap().len(), 0);
    assert!(!v["hardware"].as_array().unwrap().is_empty());
    assert!(!v["sheets"].as_array().unwrap().is_empty());
    assert_eq!(v["unplaced"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn files_come_back_as_a_zip_with_pdfs_and_dxfs() {
    let run = serde_json::to_string(&sample_run()).unwrap();
    let (status, body) = post("/v1/build/files?job=Test", run).await;
    assert_eq!(status, StatusCode::OK);
    let mut z = zip::ZipArchive::new(std::io::Cursor::new(body)).unwrap();
    let names: Vec<String> = (0..z.len())
        .map(|i| z.by_index(i).unwrap().name().to_string())
        .collect();
    for want in [
        "cutlist.csv",
        "hardware_bom.csv",
        "cut_sheets.pdf",
        "labels.pdf",
        "nesting.pdf",
    ] {
        assert!(names.iter().any(|n| n == want), "missing {want}");
    }
    assert!(
        names
            .iter()
            .any(|n| n.starts_with("dxf/") && n.ends_with(".dxf"))
    );
}

#[tokio::test]
async fn a_body_that_is_not_a_run_is_refused_with_a_reason() {
    let (status, body) = post("/v1/build", "{\"items\": 3}".into()).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(v["error"].as_str().unwrap().starts_with("not a valid run"));
}

#[tokio::test]
async fn health_answers_ok() {
    let res = app()
        .oneshot(Request::get("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

fn sample_value() -> serde_json::Value {
    serde_json::to_value(sample_run()).unwrap()
}

async fn error_of(path: &str, run: serde_json::Value) -> (StatusCode, String) {
    let (status, body) = post(path, run.to_string()).await;
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    (status, v["error"].as_str().unwrap_or("").to_string())
}

#[tokio::test]
async fn a_huge_cabinet_is_refused_before_the_engine_runs() {
    let mut run = sample_value();
    run["items"][0]["width"] = serde_json::json!(1e300);
    let (status, msg) = error_of("/v1/build", run).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(msg.contains("width"), "{msg}");
}

#[tokio::test]
async fn a_cabinet_wider_than_five_metres_is_refused() {
    let mut run = sample_value();
    run["items"][0]["width"] = serde_json::json!(6000.0);
    let (status, msg) = error_of("/v1/build/files", run).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(msg.contains("at most 5000"), "{msg}");
}

#[tokio::test]
async fn ids_that_could_escape_the_zip_folder_are_refused() {
    let mut run = sample_value();
    run["items"][0]["id"] = serde_json::json!("../../evil");
    let (status, msg) = error_of("/v1/build/files", run).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(msg.contains("letters, digits"), "{msg}");
}

#[tokio::test]
async fn duplicate_ids_are_refused_on_both_routes() {
    for path in ["/v1/build", "/v1/build/files"] {
        let mut run = sample_value();
        run["items"][1]["id"] = serde_json::json!("B01");
        let (status, msg) = error_of(path, run).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{path}");
        assert!(msg.contains("used twice"), "{path}: {msg}");
    }
}

#[tokio::test]
async fn options_out_of_range_are_refused_as_json() {
    let run = serde_json::to_string(&sample_run()).unwrap();
    for q in ["tape=1e9", "kerf=NaN", "trim=-1", "kerf=abc"] {
        let (status, body) = post(&format!("/v1/build?{q}"), run.clone()).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{q}");
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(v["error"].is_string(), "{q}");
    }
}

#[tokio::test]
async fn a_body_over_the_limit_gets_a_json_413() {
    let (status, body) = post("/v1/build", " ".repeat(3 * 1024 * 1024)).await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(v["error"].is_string());
}

#[tokio::test]
async fn with_a_token_set_v1_needs_it_and_health_does_not() {
    let app = router(Arc::new(AppState::new(None, Some("s3cret".into()))));
    let run = serde_json::to_string(&sample_run()).unwrap();
    let denied = app
        .clone()
        .oneshot(
            Request::post("/v1/build")
                .body(Body::from(run.clone()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(denied.status(), StatusCode::UNAUTHORIZED);
    let allowed = app
        .clone()
        .oneshot(
            Request::post("/v1/build")
                .header("authorization", "Bearer s3cret")
                .body(Body::from(run))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(allowed.status(), StatusCode::OK);
    let health = app
        .oneshot(Request::get("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(health.status(), StatusCode::OK);
}

#[tokio::test]
async fn a_cabinet_with_thousands_of_shelves_is_refused() {
    let mut run = sample_value();
    run["items"][0]["adjustable_shelves"] = serde_json::json!(20000);
    let (status, msg) = error_of("/v1/build/files", run).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(msg.contains("adjustable_shelves"), "{msg}");
}

#[tokio::test]
async fn tiles_too_small_to_lay_one_by_one_are_refused() {
    let mut run = sample_value();
    run["items"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({
            "type": "splashback", "id": "S1", "tile_width": 0.05, "tile_height": 0.05,
            "grout": 0.0, "pattern": "stack", "colour": "#ffffff", "grout_colour": "#888888",
            "thickness": 10.0
        }));
    let (status, msg) = error_of("/v1/build", run).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(msg.contains("at least 50 mm"), "{msg}");
}

#[tokio::test]
async fn every_kind_of_cabinet_with_its_defaults_passes_the_limits() {
    let items: Vec<_> = cabinet_core::CabinetKind::ALL
        .iter()
        .enumerate()
        .map(|(i, k)| {
            cabinet_core::RunItem::Cabinet(Box::new(cabinet_core::Cabinet::new(
                format!("C{i:02}"),
                *k,
            )))
        })
        .collect();
    let mut run = sample_run();
    run.items = items;
    let (status, body) = post("/v1/build", serde_json::to_string(&run).unwrap()).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
}
