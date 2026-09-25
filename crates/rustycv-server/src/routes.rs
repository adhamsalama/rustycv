use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::Response;
use axum::routing::{get, post};
use axum::{Json, Router};
use rustycv_core::CvDocument;
use serde::{Deserialize, Serialize};

use crate::db;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/templates", get(list_templates))
        .route("/fonts", get(list_fonts))
        .route("/render", post(render_preview))
        .route("/cvs", get(list_cvs).post(create_cv))
        .route("/cvs/import", post(import_cv))
        .route("/cvs/{id}", get(get_cv).put(update_cv).delete(delete_cv))
        .route("/cvs/{id}/pdf", get(download_pdf))
        .route("/cvs/{id}/export", get(export_cv))
}

// ------------------------------------------------------------------ metadata

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TemplateInfo {
    id: &'static str,
    name: &'static str,
    description: &'static str,
    /// Where the editor's "reset design" control puts the spacing sliders.
    metrics: rustycv_render::Metrics,
}

async fn list_templates() -> Json<Vec<TemplateInfo>> {
    Json(
        rustycv_render::TEMPLATES
            .iter()
            .map(|t| TemplateInfo {
                id: t.id,
                name: t.name,
                description: t.description,
                metrics: t.metrics,
            })
            .collect(),
    )
}

/// The font families the renderer actually has faces for. The theme picker
/// reads this rather than hardcoding a list that could drift from the binary.
async fn list_fonts() -> Json<Vec<&'static str>> {
    Json(rustycv_render::fonts::families())
}

// -------------------------------------------------------------------- render

fn pdf_response(bytes: Vec<u8>, filename: Option<&str>) -> Response {
    let mut response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/pdf")
        .header(header::CONTENT_LENGTH, bytes.len());

    if let Some(name) = filename {
        response = response.header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{name}\""),
        );
    }
    response.body(Body::from(bytes)).unwrap()
}

/// Stateless render used by the live preview: the client posts whatever is
/// currently in the editor, saved or not, and gets the exact PDF back.
async fn render_preview(
    State(state): State<AppState>,
    Json(document): Json<CvDocument>,
) -> ApiResult<Response> {
    let pdf = state.renderer.pdf(document).await?;
    Ok(pdf_response(pdf, None))
}

async fn download_pdf(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Response> {
    let cv = db::get(&state.pool, &id).await?;
    let filename = format!("{}.pdf", slug(&cv.document.basics.full_name, &cv.title));
    let pdf = state.renderer.pdf(cv.document).await?;
    Ok(pdf_response(pdf, Some(&filename)))
}

/// A filename that survives a `Content-Disposition` header and a filesystem.
fn slug(full_name: &str, fallback: &str) -> String {
    let source = if full_name.trim().is_empty() {
        fallback
    } else {
        full_name
    };
    let slug: String = source
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
        .to_lowercase();

    if slug.is_empty() {
        "cv".to_string()
    } else {
        slug
    }
}

// ----------------------------------------------------------------------- CRUD

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateParams {
    /// Duplicate an existing CV instead of starting from a blank one.
    from: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateBody {
    title: Option<String>,
}

async fn list_cvs(State(state): State<AppState>) -> ApiResult<Json<Vec<db::CvSummary>>> {
    Ok(Json(db::list(&state.pool).await?))
}

async fn create_cv(
    State(state): State<AppState>,
    Query(params): Query<CreateParams>,
    body: Option<Json<CreateBody>>,
) -> ApiResult<(StatusCode, Json<db::Cv>)> {
    let requested_title = body.and_then(|Json(b)| b.title);

    let (title, document) = match params.from {
        Some(ref source_id) => {
            let source = db::get(&state.pool, source_id).await?;
            let mut document = source.document;
            // A duplicate must not share ids with its original, or the two would
            // fight over React keys and drag handles in the editor.
            document.reassign_ids();
            (
                requested_title.unwrap_or_else(|| format!("{} (copy)", source.title)),
                document,
            )
        }
        None => (
            requested_title.unwrap_or_else(|| "Untitled CV".to_string()),
            CvDocument::starter(),
        ),
    };

    let cv = db::create(&state.pool, &title, &document).await?;
    Ok((StatusCode::CREATED, Json(cv)))
}

async fn get_cv(State(state): State<AppState>, Path(id): Path<String>) -> ApiResult<Json<db::Cv>> {
    Ok(Json(db::get(&state.pool, &id).await?))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateBody {
    title: Option<String>,
    document: CvDocument,
}

async fn update_cv(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateBody>,
) -> ApiResult<Json<db::Cv>> {
    let cv = db::update(&state.pool, &id, body.title.as_deref(), &body.document).await?;
    Ok(Json(cv))
}

async fn delete_cv(State(state): State<AppState>, Path(id): Path<String>) -> ApiResult<StatusCode> {
    db::delete(&state.pool, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ------------------------------------------------------------- import/export

/// Download the CV *data*. This is the artifact worth keeping — the PDF can
/// always be regenerated from it, but not the other way round.
async fn export_cv(State(state): State<AppState>, Path(id): Path<String>) -> ApiResult<Response> {
    let cv = db::get(&state.pool, &id).await?;
    let json = serde_json::to_vec_pretty(&cv.document)?;
    let filename = format!("{}.json", slug(&cv.document.basics.full_name, &cv.title));

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{filename}\""),
        )
        .body(Body::from(json))
        .unwrap())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImportBody {
    title: Option<String>,
    document: CvDocument,
}

async fn import_cv(
    State(state): State<AppState>,
    Json(body): Json<ImportBody>,
) -> ApiResult<(StatusCode, Json<db::Cv>)> {
    let mut document = body.document;
    document.reassign_ids();

    if rustycv_render::templates::get(&document.template).is_none() {
        return Err(ApiError::BadRequest(format!(
            "unknown template `{}` — known templates: {}",
            document.template,
            rustycv_render::TEMPLATES
                .iter()
                .map(|t| t.id)
                .collect::<Vec<_>>()
                .join(", ")
        )));
    }

    let title = body
        .title
        .or_else(|| {
            let name = document.basics.full_name.trim();
            (!name.is_empty()).then(|| format!("{name}'s CV"))
        })
        .unwrap_or_else(|| "Imported CV".to_string());

    let cv = db::create(&state.pool, &title, &document).await?;
    Ok((StatusCode::CREATED, Json(cv)))
}
