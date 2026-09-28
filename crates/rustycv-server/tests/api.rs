//! End-to-end tests over the real router and a real (temporary) SQLite file.

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use axum::Router;
use rustycv_core::CvDocument;
use serde_json::{json, Value};
use sqlx::Row;
use tower::ServiceExt;

const FIXTURE: &str = include_str!("../../../fixtures/adham.json");

/// Anything long enough to clear the password floor.
const PASSWORD: &str = "correct-horse-battery";

/// A router backed by a throwaway database file, signed in as one account.
///
/// Not `sqlite::memory:` — the pool opens several connections and each would
/// get its own empty in-memory database.
///
/// Every request carries the session cookie unless a test asks for otherwise,
/// so a test about CVs stays a test about CVs.
struct TestApp {
    router: Router,
    path: std::path::PathBuf,
    session: Option<String>,
}

impl TestApp {
    async fn new() -> Self {
        let mut app = Self::signed_out().await;
        app.session = Some(app.register("owner@example.com").await);
        app
    }

    /// The same app with nobody signed in, for the tests that are about what
    /// happens without a session.
    async fn signed_out() -> Self {
        let path = std::env::temp_dir().join(format!("rustycv-test-{}.db", uuid::Uuid::new_v4()));
        let router = rustycv_server::build_app(
            &format!("sqlite://{}", path.display()),
            rustycv_server::state::RenderMode::default(),
        )
        .await
        .expect("app builds");
        Self {
            router,
            path,
            session: None,
        }
    }

    /// Create an account and return the session cookie it was issued, without
    /// changing who this app is signed in as.
    async fn register(&self, email: &str) -> String {
        let response = self
            .response_as(
                None,
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/signup")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({"email": email, "password": PASSWORD}).to_string(),
                    ))
                    .unwrap(),
            )
            .await;

        assert_eq!(
            response.status(),
            StatusCode::CREATED,
            "signing up {email} should succeed"
        );
        session_cookie(&response)
            .unwrap_or_else(|| panic!("signing up {email} should set a session cookie"))
    }

    async fn send(&self, request: Request<Body>) -> (StatusCode, Vec<u8>) {
        self.send_as(self.session.as_deref(), request).await
    }

    /// The whole response, for the tests that assert on headers.
    async fn response(&self, request: Request<Body>) -> axum::response::Response {
        self.response_as(self.session.as_deref(), request).await
    }

    async fn response_as(
        &self,
        session: Option<&str>,
        request: Request<Body>,
    ) -> axum::response::Response {
        let mut request = request;
        if let Some(cookie) = session {
            request
                .headers_mut()
                .insert(header::COOKIE, cookie.parse().unwrap());
        }
        self.router.clone().oneshot(request).await.unwrap()
    }

    /// Send with an explicit session — `None` for an anonymous request.
    async fn send_as(
        &self,
        session: Option<&str>,
        request: Request<Body>,
    ) -> (StatusCode, Vec<u8>) {
        let response = self.response_as(session, request).await;
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), 32 * 1024 * 1024)
            .await
            .unwrap()
            .to_vec();
        (status, body)
    }

    async fn json(&self, method: &str, uri: &str, body: Value) -> (StatusCode, Value) {
        self.json_as(self.session.as_deref(), method, uri, body)
            .await
    }

    async fn json_as(
        &self,
        session: Option<&str>,
        method: &str,
        uri: &str,
        body: Value,
    ) -> (StatusCode, Value) {
        let (status, bytes) = self
            .send_as(
                session,
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

/// The `name=value` pair from a response's `Set-Cookie`, ready to be sent back
/// as a `Cookie` header. An expiry-clearing cookie yields an empty value.
fn session_cookie(response: &axum::response::Response) -> Option<String> {
    let header = response.headers().get(header::SET_COOKIE)?.to_str().ok()?;
    let pair = header.split(';').next()?.trim();
    pair.split_once('=')
        .filter(|(_, value)| !value.is_empty())
        .map(|_| pair.to_string())
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
        .response(
            Request::builder()
                .uri(format!("/api/cvs/{id}/pdf"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;

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

// ---------------------------------------------------------------- job board

/// Company names in board order, per column.
fn column(board: &Value, status: &str) -> Vec<String> {
    board
        .as_array()
        .unwrap()
        .iter()
        .filter(|a| a["status"] == status)
        .map(|a| a["company"].as_str().unwrap().to_string())
        .collect()
}

async fn add(app: &TestApp, company: &str, status: &str) -> String {
    let (status_code, created) = app
        .json(
            "POST",
            "/api/applications",
            json!({"company": company, "role": "Engineer", "status": status}),
        )
        .await;
    assert_eq!(status_code, StatusCode::CREATED);
    created["id"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn cards_stack_up_in_the_column_they_were_added_to() {
    let app = TestApp::new().await;
    add(&app, "Acme", "wishlist").await;
    add(&app, "Globex", "wishlist").await;
    add(&app, "Initech", "applied").await;

    let (status, board) = app.json("GET", "/api/applications", json!({})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(column(&board, "wishlist"), ["Acme", "Globex"]);
    assert_eq!(column(&board, "applied"), ["Initech"]);
}

#[tokio::test]
async fn a_dragged_card_lands_where_it_was_dropped() {
    let app = TestApp::new().await;
    add(&app, "Acme", "applied").await;
    add(&app, "Globex", "applied").await;
    let initech = add(&app, "Initech", "wishlist").await;

    // Dropped onto the *first* card of another column, not appended to it.
    let (status, moved) = app
        .json(
            "POST",
            &format!("/api/applications/{initech}/move"),
            json!({"status": "applied", "index": 0}),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(moved["status"], "applied");

    let (_, board) = app.json("GET", "/api/applications", json!({})).await;
    assert_eq!(column(&board, "applied"), ["Initech", "Acme", "Globex"]);
    assert!(column(&board, "wishlist").is_empty());

    // Reordering inside one column, and an index past the end meaning "last".
    app.json(
        "POST",
        &format!("/api/applications/{initech}/move"),
        json!({"status": "applied", "index": 99}),
    )
    .await;

    let (_, board) = app.json("GET", "/api/applications", json!({})).await;
    assert_eq!(column(&board, "applied"), ["Acme", "Globex", "Initech"]);
}

#[tokio::test]
async fn editing_the_status_moves_the_card_to_the_end_of_its_new_column() {
    let app = TestApp::new().await;
    add(&app, "Acme", "interview").await;
    let globex = add(&app, "Globex", "wishlist").await;

    let (status, updated) = app
        .json(
            "PUT",
            &format!("/api/applications/{globex}"),
            json!({"company": "Globex", "role": "Staff Engineer", "status": "interview"}),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(updated["role"], "Staff Engineer");

    let (_, board) = app.json("GET", "/api/applications", json!({})).await;
    assert_eq!(column(&board, "interview"), ["Acme", "Globex"]);
}

#[tokio::test]
async fn a_card_names_the_cv_it_was_sent_with_and_outlives_it() {
    let app = TestApp::new().await;
    let (_, cv) = app
        .json("POST", "/api/cvs", json!({"title": "Backend roles"}))
        .await;
    let cv_id = cv["id"].as_str().unwrap().to_string();

    let (status, created) = app
        .json(
            "POST",
            "/api/applications",
            json!({"company": "Acme", "role": "Engineer", "status": "applied", "cvId": cv_id}),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(created["cvTitle"], "Backend roles");

    // Deleting the CV must not delete the history of having applied with it.
    app.json("DELETE", &format!("/api/cvs/{cv_id}"), json!({}))
        .await;

    let (_, board) = app.json("GET", "/api/applications", json!({})).await;
    assert_eq!(board.as_array().unwrap().len(), 1);
    assert_eq!(board[0]["cvId"], Value::Null);
    assert_eq!(board[0]["cvTitle"], Value::Null);
}

#[tokio::test]
async fn an_unknown_column_or_cv_is_rejected_rather_than_stored() {
    let app = TestApp::new().await;

    let (status, _) = app
        .json(
            "POST",
            "/api/applications",
            json!({"company": "Acme", "role": "Engineer", "status": "ghosted"}),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let (status, body) = app
        .json(
            "POST",
            "/api/applications",
            json!({"company": "Acme", "role": "Engineer", "cvId": "not-a-cv"}),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"].as_str().unwrap().contains("not-a-cv"));
}

#[tokio::test]
async fn deleting_a_card_removes_it_from_the_board() {
    let app = TestApp::new().await;
    let id = add(&app, "Acme", "rejected").await;

    let (status, _) = app
        .json("DELETE", &format!("/api/applications/{id}"), json!({}))
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (_, board) = app.json("GET", "/api/applications", json!({})).await;
    assert!(board.as_array().unwrap().is_empty());

    let (status, _) = app
        .json("DELETE", &format!("/api/applications/{id}"), json!({}))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

/// The editor has no say in where it renders, so the only way it can find out
/// is to be told. An instance that answered the wrong thing here would send
/// every render to the wrong machine and look, from the outside, like the
/// setting being ignored.
#[tokio::test]
async fn the_editor_is_told_where_this_instance_renders() {
    let app = TestApp::new().await;

    let (status, config) = app.json("GET", "/api/config", json!({})).await;
    assert_eq!(status, StatusCode::OK);
    // What `build_app` was handed in `signed_out`, which is the default.
    assert_eq!(config["renderMode"], "browser");
}

#[tokio::test]
async fn an_instance_can_be_told_to_render_on_the_server() {
    let path = std::env::temp_dir().join(format!("rustycv-test-{}.db", uuid::Uuid::new_v4()));
    let router = rustycv_server::build_app(
        &format!("sqlite://{}", path.display()),
        rustycv_server::state::RenderMode::Server,
    )
    .await
    .expect("app builds");
    let mut app = TestApp {
        router,
        path,
        session: None,
    };
    app.session = Some(app.register("owner@example.com").await);

    let (_, config) = app.json("GET", "/api/config", json!({})).await;
    assert_eq!(config["renderMode"], "server");
}

// ----------------------------------------------------------------- accounts

#[tokio::test]
async fn nothing_but_signing_in_works_without_a_session() {
    let app = TestApp::signed_out().await;

    // One of each shape: a listing, a write, a render and a metadata read.
    for (method, uri) in [
        ("GET", "/api/cvs"),
        ("POST", "/api/cvs"),
        ("GET", "/api/applications"),
        ("POST", "/api/render"),
        ("GET", "/api/templates"),
        ("GET", "/api/config"),
        ("GET", "/api/auth/me"),
    ] {
        let (status, _) = app.json_as(None, method, uri, json!({})).await;
        assert_eq!(
            status,
            StatusCode::UNAUTHORIZED,
            "{method} {uri} should need a session"
        );
    }
}

#[tokio::test]
async fn signing_up_signs_you_in_and_signing_out_revokes_the_session() {
    let app = TestApp::signed_out().await;
    let session = app.register("someone@example.com").await;

    let (status, me) = app
        .json_as(Some(&session), "GET", "/api/auth/me", json!({}))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(me["email"], "someone@example.com");
    assert!(
        me.get("passwordHash").is_none() && me.get("password_hash").is_none(),
        "the hash must not be on the wire: {me}"
    );

    let (status, _) = app
        .json_as(Some(&session), "POST", "/api/auth/logout", json!({}))
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // The cookie is gone from the browser, but the token is also dead: a copy
    // of it kept anywhere else stops working too.
    let (status, _) = app
        .json_as(Some(&session), "GET", "/api/auth/me", json!({}))
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn an_address_is_one_account_and_a_wrong_password_is_refused() {
    let app = TestApp::signed_out().await;
    app.register("taken@example.com").await;

    let (status, body) = app
        .json_as(
            None,
            "POST",
            "/api/auth/signup",
            json!({"email": "TAKEN@example.com", "password": PASSWORD}),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");

    let (status, _) = app
        .json_as(
            None,
            "POST",
            "/api/auth/login",
            json!({"email": "taken@example.com", "password": "not-the-password"}),
        )
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // A case-different address is the same account, and still signs in.
    let (status, _) = app
        .json_as(
            None,
            "POST",
            "/api/auth/login",
            json!({"email": " Taken@Example.com ", "password": PASSWORD}),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn a_short_password_or_a_bad_address_is_refused_before_an_account_exists() {
    let app = TestApp::signed_out().await;

    for body in [
        json!({"email": "fine@example.com", "password": "short"}),
        json!({"email": "not-an-address", "password": PASSWORD}),
    ] {
        let (status, _) = app.json_as(None, "POST", "/api/auth/signup", body).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    let (status, _) = app
        .json_as(
            None,
            "POST",
            "/api/auth/login",
            json!({"email": "fine@example.com", "password": "short"}),
        )
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "nothing was created");
}

#[tokio::test]
async fn one_account_cannot_reach_anothers_cv() {
    let app = TestApp::new().await;
    let stranger = app.register("stranger@example.com").await;

    let (_, mine) = app.json("POST", "/api/cvs", json!({"title": "Mine"})).await;
    let id = mine["id"].as_str().unwrap();

    // 404 rather than 403 throughout: whether the id exists is not a stranger's
    // business, and a 403 would confirm it.
    for (method, uri) in [
        ("GET", format!("/api/cvs/{id}")),
        ("DELETE", format!("/api/cvs/{id}")),
        ("GET", format!("/api/cvs/{id}/pdf")),
        ("GET", format!("/api/cvs/{id}/export")),
    ] {
        let (status, _) = app.json_as(Some(&stranger), method, &uri, json!({})).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{method} {uri}");
    }

    let (status, _) = app
        .json_as(
            Some(&stranger),
            "PUT",
            &format!("/api/cvs/{id}"),
            json!({"title": "Hijacked", "document": fixture()}),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (_, listed) = app
        .json_as(Some(&stranger), "GET", "/api/cvs", json!({}))
        .await;
    assert_eq!(listed.as_array().unwrap().len(), 0, "{listed}");

    // And the CV is untouched.
    let (_, reloaded) = app.json("GET", &format!("/api/cvs/{id}"), json!({})).await;
    assert_eq!(reloaded["title"], "Mine");
}

#[tokio::test]
async fn one_account_cannot_reach_anothers_board() {
    let app = TestApp::new().await;
    let stranger = app.register("stranger@example.com").await;

    let acme = add(&app, "Acme", "applied").await;

    let (_, board) = app
        .json_as(Some(&stranger), "GET", "/api/applications", json!({}))
        .await;
    assert_eq!(board.as_array().unwrap().len(), 0, "{board}");

    for (method, uri, body) in [
        (
            "PUT",
            format!("/api/applications/{acme}"),
            json!({"company": "Hijacked", "role": "None"}),
        ),
        (
            "POST",
            format!("/api/applications/{acme}/move"),
            json!({"status": "offer", "index": 0}),
        ),
        ("DELETE", format!("/api/applications/{acme}"), json!({})),
    ] {
        let (status, _) = app.json_as(Some(&stranger), method, &uri, body).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{method} {uri}");
    }

    let (_, board) = app.json("GET", "/api/applications", json!({})).await;
    assert_eq!(column(&board, "applied"), ["Acme"]);
}

#[tokio::test]
async fn a_card_cannot_be_linked_to_someone_elses_cv() {
    // The board joins the CV's title onto the card, so a link across accounts
    // would print a stranger's CV title on this board.
    let app = TestApp::new().await;
    let stranger = app.register("stranger@example.com").await;

    let (_, mine) = app
        .json("POST", "/api/cvs", json!({"title": "Private"}))
        .await;
    let cv_id = mine["id"].as_str().unwrap();

    let (status, body) = app
        .json_as(
            Some(&stranger),
            "POST",
            "/api/applications",
            json!({"company": "Acme", "role": "Engineer", "cvId": cv_id}),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
}

// ------------------------------------------------------------------- limits

#[tokio::test]
async fn an_account_stops_at_ten_cvs() {
    let app = TestApp::new().await;

    for n in 0..rustycv_server::db::MAX_CVS {
        let (status, _) = app
            .json("POST", "/api/cvs", json!({"title": format!("CV {n}")}))
            .await;
        assert_eq!(status, StatusCode::CREATED, "CV {n} should fit");
    }

    let (status, body) = app
        .json("POST", "/api/cvs", json!({"title": "One too many"}))
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert!(
        body["error"].as_str().unwrap().contains("10"),
        "the refusal should name the ceiling: {body}"
    );

    // Every other way of making one is the same ceiling, not a way around it.
    let existing = {
        let (_, listed) = app.json("GET", "/api/cvs", json!({})).await;
        listed[0]["id"].as_str().unwrap().to_string()
    };
    let (status, _) = app
        .json("POST", &format!("/api/cvs?from={existing}"), json!({}))
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "duplicating is still a CV");

    let (status, _) = app
        .json("POST", "/api/cvs/import", json!({"document": fixture()}))
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "importing is still a CV");

    // Deleting one makes room again: the cap is a ceiling, not a quota spent.
    app.json("DELETE", &format!("/api/cvs/{existing}"), json!({}))
        .await;
    let (status, _) = app.json("POST", "/api/cvs", json!({"title": "Room"})).await;
    assert_eq!(status, StatusCode::CREATED);
}

#[tokio::test]
async fn an_account_stops_at_ten_applications() {
    let app = TestApp::new().await;

    for n in 0..rustycv_server::jobs::MAX_APPLICATIONS {
        add(&app, &format!("Company {n}"), "wishlist").await;
    }

    let (status, body) = app
        .json(
            "POST",
            "/api/applications",
            json!({"company": "One too many", "role": "Engineer"}),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert!(
        body["error"].as_str().unwrap().contains("10"),
        "the refusal should name the ceiling: {body}"
    );
}

#[tokio::test]
async fn the_limits_are_per_account() {
    // A shared ceiling would mean one busy account could fill everyone else's.
    let app = TestApp::new().await;
    let other = app.register("other@example.com").await;

    for n in 0..rustycv_server::db::MAX_CVS {
        app.json("POST", "/api/cvs", json!({"title": format!("CV {n}")}))
            .await;
    }
    assert_eq!(
        app.json("POST", "/api/cvs", json!({})).await.0,
        StatusCode::CONFLICT
    );

    let (status, _) = app
        .json_as(Some(&other), "POST", "/api/cvs", json!({"title": "Theirs"}))
        .await;
    assert_eq!(
        status,
        StatusCode::CREATED,
        "another account still has room"
    );
}

// --------------------------------------------------------------- rate limit

/// A signed-out app whose limiter is small enough to reach the end of.
async fn signed_out_limited_to(limit: u32) -> TestApp {
    let path = std::env::temp_dir().join(format!("rustycv-test-{}.db", uuid::Uuid::new_v4()));
    let pool = rustycv_server::db::connect(&format!("sqlite://{}", path.display()))
        .await
        .expect("database opens");
    let router = rustycv_server::app_with_state(rustycv_server::state::AppState {
        pool,
        renderer: rustycv_server::render::Renderer::new(),
        limiter: rustycv_server::ratelimit::RateLimiter::with_limit(limit),
        public_ip_limiter: rustycv_server::ratelimit::RateLimiter::new(),
        public_cv_limiter: rustycv_server::ratelimit::RateLimiter::new(),
        render_mode: rustycv_server::state::RenderMode::default(),
    });

    TestApp {
        router,
        path,
        session: None,
    }
}

async fn app_limited_to(limit: u32) -> TestApp {
    let mut app = signed_out_limited_to(limit).await;
    app.session = Some(app.register("owner@example.com").await);
    app
}

#[tokio::test]
async fn requests_past_the_limit_are_refused_with_a_retry_after() {
    let app = app_limited_to(4).await;

    // The signup was counted against the address it came from, not against the
    // account — which did not exist yet. So the account starts with all four.
    for n in 0..4 {
        let (status, _) = app.json("GET", "/api/auth/me", json!({})).await;
        assert_eq!(status, StatusCode::OK, "request {n} is within the limit");
    }

    let response = app
        .response(
            Request::builder()
                .uri("/api/auth/me")
                .body(Body::empty())
                .unwrap(),
        )
        .await;

    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    let retry_after: u64 = response.headers()[header::RETRY_AFTER]
        .to_str()
        .unwrap()
        .parse()
        .expect("Retry-After is a number of seconds");
    assert!(
        retry_after > 0 && retry_after <= rustycv_server::ratelimit::WINDOW.as_secs(),
        "a refusal has to say when to come back: {retry_after}s"
    );
}

#[tokio::test]
async fn two_sessions_for_one_account_share_its_allowance() {
    // Otherwise signing in again would be the way around the limit.
    let app = app_limited_to(4).await;
    let first = app.session.clone().unwrap();

    let (status, _) = app
        .json_as(
            None,
            "POST",
            "/api/auth/login",
            json!({"email": "owner@example.com", "password": PASSWORD}),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let second = app.session.clone().unwrap();

    // Four between the two of them, in any order.
    for session in [&first, &second, &first, &second] {
        let (status, _) = app
            .json_as(Some(session), "GET", "/api/auth/me", json!({}))
            .await;
        assert_eq!(status, StatusCode::OK);
    }

    let (status, _) = app
        .json_as(Some(&second), "GET", "/api/auth/me", json!({}))
        .await;
    assert_eq!(
        status,
        StatusCode::TOO_MANY_REQUESTS,
        "the fresh session should not come with a fresh allowance"
    );
}

#[tokio::test]
async fn one_account_being_throttled_does_not_throttle_another() {
    let app = app_limited_to(4).await;
    let other = app.register("other@example.com").await;

    for _ in 0..4 {
        app.json("GET", "/api/auth/me", json!({})).await;
    }
    assert_eq!(
        app.json("GET", "/api/auth/me", json!({})).await.0,
        StatusCode::TOO_MANY_REQUESTS
    );

    let (status, _) = app
        .json_as(Some(&other), "GET", "/api/auth/me", json!({}))
        .await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn failed_sign_ins_are_counted_too() {
    // The endpoint that takes a password is the one most worth flooding, and
    // it is reached without a session — so it is counted by address instead.
    // A limit that only applied once you were through the door would leave the
    // door itself unguarded.
    let app = signed_out_limited_to(3).await;
    let wrong = json!({"email": "nobody@example.com", "password": "wrong"});

    for n in 0..3 {
        let (status, _) = app
            .json_as(None, "POST", "/api/auth/login", wrong.clone())
            .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "attempt {n}");
    }

    let (status, _) = app.json_as(None, "POST", "/api/auth/login", wrong).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
}

// ---------------------------------------------------------------- migration

#[tokio::test]
async fn the_first_account_adopts_cvs_that_predate_accounts() {
    // A database in use before this feature has rows nobody owns. Leaving them
    // unowned would be indistinguishable, from the dashboard, from having lost
    // them — so the first account created on such a database takes them on.
    let path = std::env::temp_dir().join(format!("rustycv-test-{}.db", uuid::Uuid::new_v4()));
    let pool = rustycv_server::db::connect(&format!("sqlite://{}", path.display()))
        .await
        .expect("database opens");

    // What the pre-accounts server would have written: no `user_id`.
    sqlx::query(
        "INSERT INTO cvs (id, title, schema_version, data, created_at, updated_at)
         VALUES ('legacy-cv', 'From before', 1, ?, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
    )
    .bind(FIXTURE)
    .execute(&pool)
    .await
    .expect("legacy row inserts");
    sqlx::query(
        "INSERT INTO applications
             (id, company, role, status, position, created_at, updated_at)
         VALUES ('legacy-app', 'Acme', 'Engineer', 'applied', 0,
                 '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
    )
    .execute(&pool)
    .await
    .expect("legacy card inserts");

    let router = rustycv_server::app_with_state(rustycv_server::state::AppState {
        pool,
        renderer: rustycv_server::render::Renderer::new(),
        limiter: rustycv_server::ratelimit::RateLimiter::new(),
        public_ip_limiter: rustycv_server::ratelimit::RateLimiter::new(),
        public_cv_limiter: rustycv_server::ratelimit::RateLimiter::new(),
        render_mode: rustycv_server::state::RenderMode::default(),
    });
    let mut app = TestApp {
        router,
        path,
        session: None,
    };
    app.session = Some(app.register("first@example.com").await);

    let (status, cvs) = app.json("GET", "/api/cvs", json!({})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(cvs.as_array().unwrap().len(), 1, "{cvs}");
    assert_eq!(cvs[0]["title"], "From before");

    let (_, board) = app.json("GET", "/api/applications", json!({})).await;
    assert_eq!(column(&board, "applied"), ["Acme"]);

    // Only the first: a second account starts empty, or every later signup
    // would be handed the same CVs.
    let second = app.register("second@example.com").await;
    let (_, cvs) = app
        .json_as(Some(&second), "GET", "/api/cvs", json!({}))
        .await;
    assert_eq!(cvs.as_array().unwrap().len(), 0, "{cvs}");
}

// --------------------------------------------------------- password change

/// Sign in as `email` and return the session cookie, without disturbing
/// whoever the app is currently signed in as.
async fn sign_in(app: &TestApp, email: &str, password: &str) -> Option<String> {
    let response = app
        .response_as(
            None,
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"email": email, "password": password}).to_string(),
                ))
                .unwrap(),
        )
        .await;
    (response.status() == StatusCode::OK)
        .then(|| session_cookie(&response))
        .flatten()
}

#[tokio::test]
async fn a_changed_password_is_the_only_one_that_works_afterwards() {
    let app = TestApp::new().await;

    let (status, _) = app
        .json(
            "POST",
            "/api/auth/password",
            json!({"currentPassword": PASSWORD, "newPassword": "a-whole-new-password"}),
        )
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    assert!(
        sign_in(&app, "owner@example.com", PASSWORD).await.is_none(),
        "the old password should stop working"
    );
    assert!(
        sign_in(&app, "owner@example.com", "a-whole-new-password")
            .await
            .is_some(),
        "the new password should work"
    );
}

#[tokio::test]
async fn changing_a_password_evicts_every_other_session() {
    // The reason someone changes a password is often that another machine has
    // one of these. A change that left them working would look like it had
    // fixed something it had not.
    let app = TestApp::new().await;
    let elsewhere = sign_in(&app, "owner@example.com", PASSWORD)
        .await
        .expect("a second session");

    let (status, _) = app
        .json(
            "POST",
            "/api/auth/password",
            json!({"currentPassword": PASSWORD, "newPassword": "a-whole-new-password"}),
        )
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, _) = app
        .json_as(Some(&elsewhere), "GET", "/api/auth/me", json!({}))
        .await;
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "the other session is gone"
    );

    // But not the one that asked: changing your password should not sign you
    // out of the tab you changed it in.
    let (status, _) = app.json("GET", "/api/auth/me", json!({})).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn a_wrong_current_password_is_a_400_and_changes_nothing() {
    let app = TestApp::new().await;

    let (status, body) = app
        .json(
            "POST",
            "/api/auth/password",
            json!({"currentPassword": "not-it", "newPassword": "a-whole-new-password"}),
        )
        .await;

    // 400 rather than 401 on purpose: the editor reads any 401 as "the session
    // is gone" and drops to the landing page, so a typo in a form field would
    // throw the user out of the app mid-change.
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");

    assert!(
        sign_in(&app, "owner@example.com", PASSWORD).await.is_some(),
        "the password should be untouched"
    );
    assert!(
        sign_in(&app, "owner@example.com", "a-whole-new-password")
            .await
            .is_none(),
        "the rejected password should not have been written"
    );
}

#[tokio::test]
async fn a_new_password_still_has_to_clear_the_floor() {
    let app = TestApp::new().await;

    let (status, _) = app
        .json(
            "POST",
            "/api/auth/password",
            json!({"currentPassword": PASSWORD, "newPassword": "short"}),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    assert!(
        sign_in(&app, "owner@example.com", PASSWORD).await.is_some(),
        "the old password should still be the password"
    );
}

#[tokio::test]
async fn changing_a_password_needs_a_session() {
    let app = TestApp::signed_out().await;
    let (status, _) = app
        .json_as(
            None,
            "POST",
            "/api/auth/password",
            json!({"currentPassword": PASSWORD, "newPassword": "a-whole-new-password"}),
        )
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

// -------------------------------------------------------------- publishing

/// A CV, plus a place to publish it from.
async fn create_cv(app: &TestApp) -> String {
    let (status, created) = app
        .json("POST", "/api/cvs", json!({"title": "Shareable"}))
        .await;
    assert_eq!(status, StatusCode::CREATED);
    created["id"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn publishing_hands_back_a_link_and_unpublishing_keeps_it_but_hides_it() {
    let app = TestApp::new().await;
    let id = create_cv(&app).await;

    let (status, published) = app
        .json("POST", &format!("/api/cvs/{id}/publish"), json!({}))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(published["published"], true);
    let public_id = published["publicId"].as_str().unwrap().to_string();
    assert!(!public_id.is_empty());

    // A visitor with no session can reach it.
    let (status, _) = app
        .json_as(
            None,
            "GET",
            &format!("/api/public/cvs/{public_id}"),
            json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    let (status, unpublished) = app
        .json("POST", &format!("/api/cvs/{id}/unpublish"), json!({}))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(unpublished["published"], false);
    assert_eq!(
        unpublished["publicId"], public_id,
        "the link survives being switched off, so it works again if switched back on"
    );

    let (status, _) = app
        .json_as(
            None,
            "GET",
            &format!("/api/public/cvs/{public_id}"),
            json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "an off link reads as gone");

    // Republishing hands back the very same link.
    let (status, republished) = app
        .json("POST", &format!("/api/cvs/{id}/publish"), json!({}))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(republished["publicId"], public_id);
}

#[tokio::test]
async fn an_unknown_public_link_is_not_found() {
    let app = TestApp::new().await;
    let (status, _) = app
        .json_as(None, "GET", "/api/public/cvs/does-not-exist", json!({}))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let response = app
        .response_as(
            None,
            Request::builder()
                .uri("/api/public/cvs/does-not-exist/pdf")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn only_the_owner_can_publish_or_unpublish() {
    let app = TestApp::new().await;
    let id = create_cv(&app).await;
    let other = app.register("someone-else@example.com").await;

    let (status, _) = app
        .json_as(
            Some(&other),
            "POST",
            &format!("/api/cvs/{id}/publish"),
            json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, _) = app
        .json_as(None, "POST", &format!("/api/cvs/{id}/publish"), json!({}))
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn a_visitor_can_download_the_published_pdf_without_a_session() {
    let app = TestApp::new().await;
    let id = create_cv(&app).await;
    let (_, published) = app
        .json("POST", &format!("/api/cvs/{id}/publish"), json!({}))
        .await;
    let public_id = published["publicId"].as_str().unwrap();

    let response = app
        .response_as(
            None,
            Request::builder()
                .uri(format!("/api/public/cvs/{public_id}/pdf"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[header::CONTENT_TYPE], "application/pdf");
}

#[tokio::test]
async fn the_published_pdf_is_cached_and_invalidated_by_an_edit() {
    let app = TestApp::new().await;
    let id = create_cv(&app).await;
    let (_, published) = app
        .json("POST", &format!("/api/cvs/{id}/publish"), json!({}))
        .await;
    let public_id = published["publicId"].as_str().unwrap().to_string();

    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .connect(&format!("sqlite://{}", app.path.display()))
        .await
        .unwrap();

    async fn cache_row(
        pool: &sqlx::SqlitePool,
        id: &str,
    ) -> (Option<Vec<u8>>, Option<String>, String) {
        let row = sqlx::query(
            "SELECT public_pdf, public_pdf_cached_for, updated_at FROM cvs WHERE id = ?",
        )
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap();
        (
            row.get("public_pdf"),
            row.get("public_pdf_cached_for"),
            row.get("updated_at"),
        )
    }

    let (pdf, cached_for, _) = cache_row(&pool, &id).await;
    assert!(pdf.is_none(), "nothing has been rendered yet");
    assert!(cached_for.is_none());

    let (status, _) = app.get(&format!("/api/public/cvs/{public_id}/pdf")).await;
    assert_eq!(status, StatusCode::OK);

    let (pdf, cached_for, updated_at) = cache_row(&pool, &id).await;
    assert!(pdf.is_some(), "the first visit renders and caches");
    assert_eq!(cached_for.as_deref(), Some(updated_at.as_str()));
    let first_pdf = pdf.unwrap();

    let (status, _) = app.get(&format!("/api/public/cvs/{public_id}/pdf")).await;
    assert_eq!(status, StatusCode::OK);
    let (pdf_after_second_visit, cached_for_after, updated_at_after) = cache_row(&pool, &id).await;
    assert_eq!(
        pdf_after_second_visit.as_deref(),
        Some(first_pdf.as_slice()),
        "a second visit with nothing changed serves the same cached bytes"
    );
    assert_eq!(updated_at_after, updated_at);
    assert_eq!(cached_for_after, cached_for);

    // Edit the CV: the document changes, so `updated_at` moves on.
    let mut document = fixture();
    document["basics"]["fullName"] = json!("A New Name");
    let document: CvDocument = serde_json::from_value(document).unwrap();
    let (status, _) = app
        .json(
            "PUT",
            &format!("/api/cvs/{id}"),
            json!({"document": document}),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    let (_, cached_for_stale, updated_at_new) = cache_row(&pool, &id).await;
    assert_ne!(updated_at_new, updated_at, "the edit bumped updated_at");
    assert_eq!(
        cached_for_stale, cached_for,
        "the cache is not touched by the write itself, only by the next visit"
    );

    let (status, _) = app.get(&format!("/api/public/cvs/{public_id}/pdf")).await;
    assert_eq!(status, StatusCode::OK);

    let (_, cached_for_fresh, _) = cache_row(&pool, &id).await;
    assert_eq!(
        cached_for_fresh.as_deref(),
        Some(updated_at_new.as_str()),
        "the next visit after an edit re-renders and re-caches"
    );
}

#[tokio::test]
async fn unpublishing_clears_the_cached_pdf_rather_than_just_hiding_it() {
    let app = TestApp::new().await;
    let id = create_cv(&app).await;
    let (_, published) = app
        .json("POST", &format!("/api/cvs/{id}/publish"), json!({}))
        .await;
    let public_id = published["publicId"].as_str().unwrap().to_string();

    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .connect(&format!("sqlite://{}", app.path.display()))
        .await
        .unwrap();

    async fn cache_columns(pool: &sqlx::SqlitePool, id: &str) -> (Option<Vec<u8>>, Option<String>) {
        let row = sqlx::query("SELECT public_pdf, public_pdf_cached_for FROM cvs WHERE id = ?")
            .bind(id)
            .fetch_one(pool)
            .await
            .unwrap();
        (row.get("public_pdf"), row.get("public_pdf_cached_for"))
    }

    let (status, _) = app.get(&format!("/api/public/cvs/{public_id}/pdf")).await;
    assert_eq!(status, StatusCode::OK);
    let (pdf, cached_for) = cache_columns(&pool, &id).await;
    assert!(pdf.is_some(), "the visit above should have cached one");
    assert!(cached_for.is_some());

    let (status, _) = app
        .json("POST", &format!("/api/cvs/{id}/unpublish"), json!({}))
        .await;
    assert_eq!(status, StatusCode::OK);

    let (pdf, cached_for) = cache_columns(&pool, &id).await;
    assert!(
        pdf.is_none(),
        "unpublishing must not leave a rendered PDF sitting in the row"
    );
    assert!(cached_for.is_none());
}

/// A public app whose two share-link limiters are small enough to reach the
/// end of, with the general limiter left generous so it never interferes.
async fn app_with_public_limits(ip_limit: u32, cv_limit: u32) -> TestApp {
    let path = std::env::temp_dir().join(format!("rustycv-test-{}.db", uuid::Uuid::new_v4()));
    let pool = rustycv_server::db::connect(&format!("sqlite://{}", path.display()))
        .await
        .expect("database opens");
    let router = rustycv_server::app_with_state(rustycv_server::state::AppState {
        pool,
        renderer: rustycv_server::render::Renderer::new(),
        limiter: rustycv_server::ratelimit::RateLimiter::new(),
        public_ip_limiter: rustycv_server::ratelimit::RateLimiter::with_limit(ip_limit),
        public_cv_limiter: rustycv_server::ratelimit::RateLimiter::with_limit(cv_limit),
        render_mode: rustycv_server::state::RenderMode::default(),
    });
    let mut app = TestApp {
        router,
        path,
        session: None,
    };
    app.session = Some(app.register("owner@example.com").await);
    app
}

#[tokio::test]
async fn a_flooded_share_link_is_refused_without_touching_others() {
    let app = app_with_public_limits(1000, 3).await;

    let id_a = create_cv(&app).await;
    let (_, published_a) = app
        .json("POST", &format!("/api/cvs/{id_a}/publish"), json!({}))
        .await;
    let public_a = published_a["publicId"].as_str().unwrap().to_string();

    let id_b = create_cv(&app).await;
    let (_, published_b) = app
        .json("POST", &format!("/api/cvs/{id_b}/publish"), json!({}))
        .await;
    let public_b = published_b["publicId"].as_str().unwrap().to_string();

    for n in 0..3 {
        let (status, _) = app
            .json_as(
                None,
                "GET",
                &format!("/api/public/cvs/{public_a}"),
                json!({}),
            )
            .await;
        assert_eq!(status, StatusCode::OK, "visit {n} to A is within its limit");
    }

    let (status, _) = app
        .json_as(
            None,
            "GET",
            &format!("/api/public/cvs/{public_a}"),
            json!({}),
        )
        .await;
    assert_eq!(
        status,
        StatusCode::TOO_MANY_REQUESTS,
        "A's own link is flooded"
    );

    let (status, _) = app
        .json_as(
            None,
            "GET",
            &format!("/api/public/cvs/{public_b}"),
            json!({}),
        )
        .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "B's link is a different key and must not be caught up in A's flood"
    );
}

#[tokio::test]
async fn one_address_hammering_a_share_link_is_refused() {
    let app = app_with_public_limits(2, 1000).await;
    let id = create_cv(&app).await;
    let (_, published) = app
        .json("POST", &format!("/api/cvs/{id}/publish"), json!({}))
        .await;
    let public_id = published["publicId"].as_str().unwrap().to_string();

    for n in 0..2 {
        let (status, _) = app
            .json_as(
                None,
                "GET",
                &format!("/api/public/cvs/{public_id}"),
                json!({}),
            )
            .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "visit {n} is within the per-address limit"
        );
    }

    let (status, _) = app
        .json_as(
            None,
            "GET",
            &format!("/api/public/cvs/{public_id}"),
            json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn an_oversized_cv_is_neither_stored_nor_rendered() {
    let app = TestApp::new().await;
    let (_, created) = app.json("POST", "/api/cvs", json!({})).await;
    let id = created["id"].as_str().unwrap();

    let mut document: CvDocument = serde_json::from_str(FIXTURE).unwrap();
    document.basics.headline = "a".repeat(rustycv_core::limits::MAX_STRING_CHARS + 1);
    let document = serde_json::to_value(&document).unwrap();

    let (status, _) = app
        .json(
            "PUT",
            &format!("/api/cvs/{id}"),
            json!({ "document": document }),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let (status, _) = app
        .json(
            "PUT",
            &format!("/api/cvs/{id}"),
            json!({ "title": "t".repeat(201), "document": {} }),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let (status, _) = app.json("POST", "/api/render", document).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}
