use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post, put};
use axum::{Extension, Json, Router};
use rustycv_core::CvDocument;
use serde::{Deserialize, Serialize};

use crate::auth::{self, Credentials, User};
use crate::comments;
use crate::db;
use crate::error::{ApiError, ApiResult};
use crate::jobs::{self, Application, ApplicationInput, Status};
use crate::middleware::{require_auth, CurrentUser, PeerAddr, SessionToken};
use crate::state::{AppState, RenderMode};

pub fn router() -> Router<AppState> {
    // Signing up and signing in are the only things that can be done without
    // an account; everything else goes behind `require_auth`, including the
    // template and font listings — they are only ever read by an editor that
    // is already open, and leaving them out would be a list of endpoints to
    // keep in step by hand.
    let public = Router::new()
        .route("/auth/signup", post(signup))
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout))
        .route("/auth/me", get(me))
        // Anonymous, by design: this is the whole point of a share link. Kept
        // out from behind `require_auth`, but not out from under rate limits —
        // see `check_public_rate_limit`, which each of these calls itself
        // since the limit is keyed on the CV in the path, not just the caller.
        .route("/public/cvs/{public_id}", get(get_published_cv))
        .route("/public/cvs/{public_id}/pdf", get(download_published_pdf))
        .route(
            "/public/cvs/{public_id}/comments",
            get(list_published_comments).post(post_comment),
        );

    let protected = Router::new()
        .route("/config", get(app_config))
        .route("/templates", get(list_templates))
        .route("/fonts", get(list_fonts))
        .route("/render", post(render_preview))
        .route("/cvs", get(list_cvs).post(create_cv))
        .route("/cvs/import", post(import_cv))
        .route("/cvs/{id}", get(get_cv).put(update_cv).delete(delete_cv))
        .route("/cvs/{id}/pdf", get(download_pdf))
        .route("/cvs/{id}/export", get(export_cv))
        .route("/cvs/{id}/publish", post(publish_cv))
        .route("/cvs/{id}/unpublish", post(unpublish_cv))
        .route("/cvs/{id}/comments", get(list_cv_comments))
        .route("/cvs/{id}/comment-settings", put(set_comment_settings))
        .route(
            "/cvs/{id}/comments/{comment_id}",
            axum::routing::delete(delete_cv_comment),
        )
        .route(
            "/applications",
            get(list_applications).post(create_application),
        )
        .route(
            "/applications/{id}",
            put(update_application).delete(delete_application),
        )
        .route("/applications/{id}/move", post(move_application))
        // Behind `require_auth` rather than beside the other auth routes: you
        // change your own password, so there has to be a session saying whose.
        .route("/auth/password", post(change_password))
        .layer(axum::middleware::from_fn(require_auth));

    public.merge(protected)
}

// ---------------------------------------------------------------- accounts

/// Signing up signs you in: there is no verification step to wait for, so
/// leaving the user on the form to type the same details again would be
/// ceremony with nothing behind it.
async fn signup(
    State(state): State<AppState>,
    Json(credentials): Json<Credentials>,
) -> ApiResult<Response> {
    let user = auth::signup(&state.pool, &credentials).await?;
    let token = auth::create_session(&state.pool, &user.id).await?;
    Ok(session_response(StatusCode::CREATED, &token, user))
}

async fn login(
    State(state): State<AppState>,
    Json(credentials): Json<Credentials>,
) -> ApiResult<Response> {
    let user = auth::login(&state.pool, &credentials).await?;
    let token = auth::create_session(&state.pool, &user.id).await?;
    Ok(session_response(StatusCode::OK, &token, user))
}

/// Signing out deletes the session rather than only clearing the cookie, so a
/// token that was copied somewhere stops working too.
///
/// Public, and quiet about it: signing out without a session is the state the
/// caller wanted, not an error.
async fn logout(
    State(state): State<AppState>,
    token: Option<Extension<SessionToken>>,
) -> ApiResult<Response> {
    if let Some(Extension(SessionToken(token))) = token {
        auth::delete_session(&state.pool, &token).await?;
    }
    Ok(clear_session_response())
}

/// Who is signed in. The editor's first call: a 401 here is what puts the
/// login form on screen.
async fn me(user: CurrentUser) -> Json<User> {
    Json(user.0)
}

/// Change your own password, keeping this session and dropping the rest.
async fn change_password(
    State(state): State<AppState>,
    user: CurrentUser,
    Extension(SessionToken(token)): Extension<SessionToken>,
    Json(change): Json<auth::PasswordChange>,
) -> ApiResult<StatusCode> {
    auth::change_password(&state.pool, user.id(), &token, &change).await?;
    Ok(StatusCode::NO_CONTENT)
}

fn session_response(status: StatusCode, token: &str, user: User) -> Response {
    (
        status,
        [(header::SET_COOKIE, auth::session_cookie(token))],
        Json(user),
    )
        .into_response()
}

fn clear_session_response() -> Response {
    (
        StatusCode::NO_CONTENT,
        [(header::SET_COOKIE, auth::expired_cookie())],
    )
        .into_response()
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AppConfig {
    render_mode: RenderMode,
}

/// What the editor has to be told about how this instance is set up.
///
/// Only one thing so far, and it is fixed for the life of the process — the
/// editor reads it once on arrival and caches it, the way it does the template
/// and font lists beside it.
async fn app_config(State(state): State<AppState>) -> Json<AppConfig> {
    Json(AppConfig {
        render_mode: state.render_mode,
    })
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
    user: CurrentUser,
    Path(id): Path<String>,
) -> ApiResult<Response> {
    let cv = db::get(&state.pool, user.id(), &id).await?;
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

async fn list_cvs(
    State(state): State<AppState>,
    user: CurrentUser,
) -> ApiResult<Json<Vec<db::CvSummary>>> {
    Ok(Json(db::list(&state.pool, user.id()).await?))
}

async fn create_cv(
    State(state): State<AppState>,
    user: CurrentUser,
    Query(params): Query<CreateParams>,
    body: Option<Json<CreateBody>>,
) -> ApiResult<(StatusCode, Json<db::Cv>)> {
    let requested_title = body.and_then(|Json(b)| b.title);

    let (title, document) = match params.from {
        Some(ref source_id) => {
            let source = db::get(&state.pool, user.id(), source_id).await?;
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

    crate::error::check_cv(Some(&title), &document)?;
    let cv = db::create(&state.pool, user.id(), &title, &document).await?;
    Ok((StatusCode::CREATED, Json(cv)))
}

async fn get_cv(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<String>,
) -> ApiResult<Json<db::Cv>> {
    Ok(Json(db::get(&state.pool, user.id(), &id).await?))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateBody {
    title: Option<String>,
    document: CvDocument,
}

async fn update_cv(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<String>,
    Json(body): Json<UpdateBody>,
) -> ApiResult<Json<db::Cv>> {
    crate::error::check_cv(body.title.as_deref(), &body.document)?;
    let cv = db::update(
        &state.pool,
        user.id(),
        &id,
        body.title.as_deref(),
        &body.document,
    )
    .await?;
    Ok(Json(cv))
}

async fn delete_cv(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    db::delete(&state.pool, user.id(), &id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// -------------------------------------------------------------- publishing

async fn publish_cv(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<String>,
) -> ApiResult<Json<db::Cv>> {
    Ok(Json(db::publish(&state.pool, user.id(), &id).await?))
}

async fn unpublish_cv(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<String>,
) -> ApiResult<Json<db::Cv>> {
    Ok(Json(db::unpublish(&state.pool, user.id(), &id).await?))
}

/// What a share link is limited to, apart from the general per-caller limit
/// every other route already sits behind (`middleware::rate_limit`).
///
/// Both keys are checked, and either one being over is enough to refuse: a
/// single address must not be able to hammer one link into the ground, and a
/// single link must not be able to soak up load from many addresses at once.
fn check_public_rate_limit(state: &AppState, addr: &str, public_id: &str) -> ApiResult<()> {
    check_pair(
        &state.public_ip_limiter,
        &state.public_cv_limiter,
        addr,
        public_id,
        "public cv rate limit reached",
    )
}

fn check_pair(
    ip_limiter: &crate::ratelimit::RateLimiter,
    cv_limiter: &crate::ratelimit::RateLimiter,
    addr: &str,
    public_id: &str,
    what: &str,
) -> ApiResult<()> {
    let ip_key = format!("ip:{addr}");
    if let Err(allowance) = ip_limiter.check(&ip_key) {
        tracing::warn!(key = %ip_key, "{what}");
        return Err(ApiError::RateLimited {
            retry_after: allowance.reset_in,
        });
    }

    let cv_key = format!("cv:{public_id}");
    if let Err(allowance) = cv_limiter.check(&cv_key) {
        tracing::warn!(key = %cv_key, "{what}");
        return Err(ApiError::RateLimited {
            retry_after: allowance.reset_in,
        });
    }

    Ok(())
}

/// What a visitor to a share link gets before the PDF itself: enough to put a
/// name on the page, without shipping the whole document to a caller who is
/// never going to see anything but its rendering anyway.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PublishedCvInfo {
    title: String,
    full_name: String,
    comments_enabled: bool,
}

async fn get_published_cv(
    State(state): State<AppState>,
    PeerAddr(addr): PeerAddr,
    Path(public_id): Path<String>,
) -> ApiResult<Json<PublishedCvInfo>> {
    check_public_rate_limit(&state, &addr, &public_id)?;
    let cv = db::get_published(&state.pool, &public_id).await?;
    Ok(Json(PublishedCvInfo {
        title: cv.title,
        full_name: cv.document.basics.full_name,
        comments_enabled: cv.comments_enabled,
    }))
}

/// The PDF behind a share link.
///
/// Goes through the same [`crate::render::Renderer`] as the preview and the
/// owner's own download — a share link shows exactly the same pure function
/// of `(document, template, theme)`, just cached, because unlike the editor
/// it can be visited by anyone at any time and does not get to assume its
/// caller is the one CV's owner about to leave the page.
async fn download_published_pdf(
    State(state): State<AppState>,
    PeerAddr(addr): PeerAddr,
    Path(public_id): Path<String>,
) -> ApiResult<Response> {
    check_public_rate_limit(&state, &addr, &public_id)?;
    let cv = db::get_published(&state.pool, &public_id).await?;
    let filename = format!("{}.pdf", slug(&cv.document.basics.full_name, &cv.title));

    let fresh = cv.cached_for.as_deref() == Some(cv.updated_at.as_str());
    let pdf = match cv.cached_pdf.filter(|_| fresh) {
        Some(bytes) => bytes,
        None => {
            let id = cv.id.clone();
            let rendered_for = cv.updated_at.clone();
            let bytes = state.renderer.pdf(cv.document).await?;
            db::cache_published_pdf(&state.pool, &id, &rendered_for, &bytes).await;
            bytes
        }
    };

    Ok(pdf_response(pdf, Some(&filename)))
}

// ------------------------------------------------------------------ comments

/// Reading is one indexed query over at most `MAX_COMMENTS` rows, so it sits
/// behind the general per-caller limit only; spending the share link's render
/// allowance on it would cost a visitor PDF loads for reading text.
async fn list_published_comments(
    State(state): State<AppState>,
    Path(public_id): Path<String>,
) -> ApiResult<Json<Vec<comments::Comment>>> {
    Ok(Json(
        comments::list_published(&state.pool, &public_id).await?,
    ))
}

/// Anonymous, and shown to every later visitor — so it has its own windows,
/// per address and per CV, far tighter than reading's, on top of the cap on
/// how many one CV can hold at all.
async fn post_comment(
    State(state): State<AppState>,
    PeerAddr(addr): PeerAddr,
    Path(public_id): Path<String>,
    Json(input): Json<comments::CommentInput>,
) -> ApiResult<(StatusCode, Json<comments::Comment>)> {
    check_pair(
        &state.comment_ip_limiter,
        &state.comment_cv_limiter,
        &addr,
        &public_id,
        "comment rate limit reached",
    )?;
    let comment = comments::create(&state.pool, &public_id, input).await?;
    Ok((StatusCode::CREATED, Json(comment)))
}

async fn list_cv_comments(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<String>,
) -> ApiResult<Json<Vec<comments::Comment>>> {
    Ok(Json(
        comments::list_owned(&state.pool, user.id(), &id).await?,
    ))
}

#[derive(Deserialize)]
struct CommentSettings {
    enabled: bool,
}

async fn set_comment_settings(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<String>,
    Json(settings): Json<CommentSettings>,
) -> ApiResult<Json<db::Cv>> {
    Ok(Json(
        db::set_comments_enabled(&state.pool, user.id(), &id, settings.enabled).await?,
    ))
}

async fn delete_cv_comment(
    State(state): State<AppState>,
    user: CurrentUser,
    Path((id, comment_id)): Path<(String, String)>,
) -> ApiResult<StatusCode> {
    comments::delete(&state.pool, user.id(), &id, &comment_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ------------------------------------------------------------- import/export

/// Download the CV *data*. This is the artifact worth keeping — the PDF can
/// always be regenerated from it, but not the other way round.
async fn export_cv(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<String>,
) -> ApiResult<Response> {
    let cv = db::get(&state.pool, user.id(), &id).await?;
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
    user: CurrentUser,
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

    crate::error::check_cv(Some(&title), &document)?;
    let cv = db::create(&state.pool, user.id(), &title, &document).await?;
    Ok((StatusCode::CREATED, Json(cv)))
}

// -------------------------------------------------------------- job tracker

async fn list_applications(
    State(state): State<AppState>,
    user: CurrentUser,
) -> ApiResult<Json<Vec<Application>>> {
    Ok(Json(jobs::list(&state.pool, user.id()).await?))
}

async fn create_application(
    State(state): State<AppState>,
    user: CurrentUser,
    Json(input): Json<ApplicationInput>,
) -> ApiResult<(StatusCode, Json<Application>)> {
    let application = jobs::create(&state.pool, user.id(), input).await?;
    Ok((StatusCode::CREATED, Json(application)))
}

async fn update_application(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<String>,
    Json(input): Json<ApplicationInput>,
) -> ApiResult<Json<Application>> {
    Ok(Json(
        jobs::update(&state.pool, user.id(), &id, input).await?,
    ))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MoveBody {
    status: String,
    /// Where in the destination column the card was dropped. Past the end of
    /// the column means last.
    index: usize,
}

/// Where a drag ends. Separate from the update above because it is the only
/// call that reorders a column, and because a drag carries no field edits — so
/// a card being dragged while its form is open cannot write stale text back.
async fn move_application(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<String>,
    Json(body): Json<MoveBody>,
) -> ApiResult<Json<Application>> {
    let status = Status::from_wire(&body.status)?;
    Ok(Json(
        jobs::move_to(&state.pool, user.id(), &id, status, body.index).await?,
    ))
}

async fn delete_application(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    jobs::delete(&state.pool, user.id(), &id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::slug;

    /// The browser render path has no response to put a `Content-Disposition`
    /// on, so it builds the same name itself in `pdfFilename` — see
    /// `web/src/renderer.ts`. These cases and the ones in
    /// `web/src/renderer.test.ts` are the same list, and are meant to be read
    /// and changed together.
    #[test]
    fn the_download_name_matches_the_editors() {
        assert_eq!(slug("Adham Salama", "Backend CV"), "adham-salama");
        assert_eq!(slug("R&D Lead", "x"), "r-d-lead");
        assert_eq!(slug("  Ada   Lovelace  ", "x"), "ada-lovelace");
        // Unicode letters and digits survive; everything else is a separator.
        assert_eq!(slug("José Ñuñez 3", "x"), "josé-ñuñez-3");

        assert_eq!(slug("", "My Résumé"), "my-résumé");
        assert_eq!(slug("   ", "My Résumé"), "my-résumé");
        // A name with nothing alphanumeric in it leaves no slug to use.
        assert_eq!(slug("!!!", "ignored"), "cv");
        assert_eq!(slug("", ""), "cv");
    }
}
