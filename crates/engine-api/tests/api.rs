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
