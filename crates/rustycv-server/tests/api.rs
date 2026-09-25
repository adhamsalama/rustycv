//! End-to-end tests over the real router and a real (temporary) SQLite file.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use rustycv_core::CvDocument;
use serde_json::{json, Value};
use tower::ServiceExt;

const FIXTURE: &str = include_str!("../../../fixtures/adham.json");

/// A router backed by a throwaway database file.
///
/// Not `sqlite::memory:` — the pool opens several connections and each would
/// get its own empty in-memory database.
struct TestApp {
    router: Router,
    path: std::path::PathBuf,
}

impl TestApp {
    async fn new() -> Self {
        let path = std::env::temp_dir().join(format!("rustycv-test-{}.db", uuid::Uuid::new_v4()));
        let router = rustycv_server::build_app(&format!("sqlite://{}", path.display()))
            .await
            .expect("app builds");
        Self { router, path }
    }

    async fn send(&self, request: Request<Body>) -> (StatusCode, Vec<u8>) {
        let response = self.router.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), 32 * 1024 * 1024)
            .await
            .unwrap()
            .to_vec();
        (status, body)
    }

    async fn json(&self, method: &str, uri: &str, body: Value) -> (StatusCode, Value) {
        let (status, bytes) = self
            .send(
                Request::builder()
                    .method(method)
                    .uri(uri)
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await;
        let value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        (status, value)
    }

    async fn get(&self, uri: &str) -> (StatusCode, Vec<u8>) {
        self.send(Request::builder().uri(uri).body(Body::empty()).unwrap())
            .await
    }
}

impl Drop for TestApp {
    fn drop(&mut self) {
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{suffix}", self.path.display()));
        }
    }
}

fn fixture() -> Value {
    serde_json::from_str(FIXTURE).unwrap()
}

#[tokio::test]
async fn a_cv_survives_create_update_and_reload() {
    let app = TestApp::new().await;

    let (status, created) = app
        .json("POST", "/api/cvs", json!({"title": "My CV"}))
        .await;
    assert_eq!(status, StatusCode::CREATED);
    let id = created["id"].as_str().unwrap().to_string();

    let (status, updated) = app
        .json(
            "PUT",
            &format!("/api/cvs/{id}"),
            json!({"title": "Backend roles", "document": fixture()}),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(updated["title"], "Backend roles");
    assert_eq!(
        updated["document"]["basics"]["fullName"],
        "Adham Salama Mustafa"
    );

    // Read it back through a fresh request: this is the "data, not PDFs" claim.
    let (status, bytes) = app.get(&format!("/api/cvs/{id}")).await;
    assert_eq!(status, StatusCode::OK);
    let reloaded: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        reloaded["document"]["sections"].as_array().unwrap().len(),
        5
    );
}

#[tokio::test]
async fn the_preview_endpoint_returns_a_pdf() {
    let app = TestApp::new().await;

    let (status, bytes) = app
        .send(
            Request::builder()
                .method("POST")
                .uri("/api/render")
                .header("content-type", "application/json")
                .body(Body::from(FIXTURE))
                .unwrap(),
        )
        .await;

    assert_eq!(status, StatusCode::OK);
    assert!(bytes.starts_with(b"%PDF"));
}

#[tokio::test]
async fn downloading_a_pdf_names_the_file_after_the_person() {
    let app = TestApp::new().await;
    let (_, created) = app.json("POST", "/api/cvs", json!({"title": "X"})).await;
    let id = created["id"].as_str().unwrap();
    app.json(
        "PUT",
        &format!("/api/cvs/{id}"),
        json!({"document": fixture()}),
    )
    .await;

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/cvs/{id}/pdf"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["content-type"], "application/pdf");
    assert_eq!(
        response.headers()["content-disposition"],
        "attachment; filename=\"adham-salama-mustafa.pdf\""
    );
}

#[tokio::test]
async fn export_and_import_round_trip() {
    let app = TestApp::new().await;
    let (_, created) = app
        .json("POST", "/api/cvs", json!({"title": "Original"}))
        .await;
    let id = created["id"].as_str().unwrap();
    app.json(
        "PUT",
        &format!("/api/cvs/{id}"),
        json!({"document": fixture()}),
    )
    .await;

    let (status, exported) = app.get(&format!("/api/cvs/{id}/export")).await;
    assert_eq!(status, StatusCode::OK);
    let document: Value = serde_json::from_slice(&exported).unwrap();

    let (status, imported) = app
        .json(
            "POST",
            "/api/cvs/import",
            json!({"title": "Reimported", "document": document}),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(
        imported["document"]["basics"]["fullName"],
        "Adham Salama Mustafa"
    );

    // The import must not reuse the original's ids.
    let original_section_id = fixture()["sections"][0]["id"].clone();
    assert_ne!(
        imported["document"]["sections"][0]["id"],
        original_section_id
    );
}

#[tokio::test]
async fn duplicating_copies_content_but_not_ids() {
    let app = TestApp::new().await;
    let (_, created) = app
        .json("POST", "/api/cvs", json!({"title": "Original"}))
        .await;
    let id = created["id"].as_str().unwrap();
    app.json(
        "PUT",
        &format!("/api/cvs/{id}"),
        json!({"document": fixture()}),
    )
    .await;

    let (status, copy) = app
        .json("POST", &format!("/api/cvs?from={id}"), json!({}))
        .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(copy["title"], "Original (copy)");
    assert_ne!(copy["id"], created["id"]);
    assert_eq!(copy["document"]["sections"].as_array().unwrap().len(), 5);

    let (_, listed) = app.json("GET", "/api/cvs", json!({})).await;
    assert_eq!(listed.as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn deleting_removes_the_cv() {
    let app = TestApp::new().await;
    let (_, created) = app.json("POST", "/api/cvs", json!({"title": "Temp"})).await;
    let id = created["id"].as_str().unwrap();

    let (status, _) = app
        .json("DELETE", &format!("/api/cvs/{id}"), json!({}))
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, _) = app.get(&format!("/api/cvs/{id}")).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn an_unknown_template_is_rejected_with_a_useful_message() {
    let app = TestApp::new().await;
    let mut document: CvDocument = serde_json::from_str(FIXTURE).unwrap();
    document.template = "nope".into();

    let (status, body) = app
        .json(
            "POST",
            "/api/render",
            serde_json::to_value(&document).unwrap(),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"].as_str().unwrap().contains("nope"));
}

#[tokio::test]
async fn a_brand_new_cv_renders_before_anything_is_typed() {
    let app = TestApp::new().await;
    let (_, created) = app.json("POST", "/api/cvs", json!({})).await;
    let id = created["id"].as_str().unwrap();

    let (status, bytes) = app.get(&format!("/api/cvs/{id}/pdf")).await;
    assert_eq!(status, StatusCode::OK);
    assert!(bytes.starts_with(b"%PDF"));
}

#[tokio::test]
async fn hostile_theme_values_do_not_break_rendering() {
    let app = TestApp::new().await;
    let mut document: Value = fixture();
    document["theme"]["fontSizePt"] = json!(10_000);
    document["theme"]["marginMm"] = json!(-50);
    document["theme"]["accent"] = json!("not-a-colour");

    let (status, bytes) = app
        .send(
            Request::builder()
                .method("POST")
                .uri("/api/render")
                .header("content-type", "application/json")
                .body(Body::from(document.to_string()))
                .unwrap(),
        )
        .await;

    assert_eq!(status, StatusCode::OK, "clamped theme should still render");
    assert!(bytes.starts_with(b"%PDF"));
}
