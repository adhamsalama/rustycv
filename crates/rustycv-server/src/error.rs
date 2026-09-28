use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use rustycv_render::{Diagnostic, RenderError};
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("cv not found")]
    NotFound,
    #[error("{0}")]
    BadRequest(String),
    #[error("sign in to continue")]
    Unauthorized,
    #[error("that email and password do not match an account")]
    InvalidCredentials,
    /// One of the per-account caps. Carries its own sentence because "you have
    /// 10 CVs" and "you have 10 applications" are different ceilings.
    #[error("{0}")]
    LimitReached(String),
    #[error("too many requests — try again in {retry_after}s")]
    RateLimited { retry_after: u64 },
    #[error("internal error")]
    Internal,
    #[error(transparent)]
    Render(#[from] RenderError),
    #[error(transparent)]
    Db(#[from] sqlx::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ErrorBody {
    error: String,
    /// Present only for template compile failures, so the editor can show the
    /// user *where* the render broke instead of an empty preview.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    diagnostics: Vec<Diagnostic>,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        // A refusal has to say how long for, or the client can only guess and
        // poll. Carried alongside because the body is JSON like every other
        // error and this belongs in a header.
        let retry_after = match self {
            ApiError::RateLimited { retry_after } => Some(retry_after),
            _ => None,
        };

        let (status, message, diagnostics) = match self {
            ApiError::NotFound => (StatusCode::NOT_FOUND, self.to_string(), vec![]),
            ApiError::BadRequest(ref m) => (StatusCode::BAD_REQUEST, m.clone(), vec![]),
            ApiError::Unauthorized | ApiError::InvalidCredentials => {
                (StatusCode::UNAUTHORIZED, self.to_string(), vec![])
            }
            // 409 rather than 403: nothing is forbidden about the request, the
            // account is simply already at the ceiling, and deleting something
            // makes the identical request succeed.
            ApiError::LimitReached(ref m) => (StatusCode::CONFLICT, m.clone(), vec![]),
            ApiError::RateLimited { .. } => {
                (StatusCode::TOO_MANY_REQUESTS, self.to_string(), vec![])
            }
            ApiError::Internal => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string(), vec![]),
            ApiError::Json(ref e) => (StatusCode::BAD_REQUEST, e.to_string(), vec![]),
            ApiError::Render(RenderError::UnknownTemplate(ref t)) => (
                StatusCode::BAD_REQUEST,
                format!("unknown template: {t}"),
                vec![],
            ),
            // A template that fails to compile is the *document's* problem, not
            // the server's — 422 keeps it out of the 5xx error budget and lets
            // the client render the diagnostics inline.
            ApiError::Render(RenderError::Typst(diagnostics)) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "the template failed to compile".to_string(),
                diagnostics,
            ),
            // Like a compile failure, an oversized CV is the document's fault.
            ApiError::Render(ref e @ (RenderError::TooLarge(_) | RenderError::TooManyPages(_))) => {
                (StatusCode::UNPROCESSABLE_ENTITY, e.to_string(), vec![])
            }
            ApiError::Render(ref e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string(), vec![]),
            ApiError::Db(ref e) => {
                tracing::error!(error = %e, "database error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "database error".to_string(),
                    vec![],
                )
            }
        };

        let mut response = (
            status,
            Json(ErrorBody {
                error: message,
                diagnostics,
            }),
        )
            .into_response();

        if let Some(seconds) = retry_after {
            if let Ok(value) = seconds.to_string().parse() {
                response.headers_mut().insert(header::RETRY_AFTER, value);
            }
        }

        response
    }
}

impl From<rustycv_core::LimitError> for ApiError {
    fn from(error: rustycv_core::LimitError) -> Self {
        ApiError::BadRequest(error.to_string())
    }
}

/// Refuse a title or document over the limits before it is stored.
pub fn check_cv(title: Option<&str>, document: &rustycv_core::CvDocument) -> ApiResult<()> {
    if let Some(title) = title {
        rustycv_core::limits::check_title(title).map_err(ApiError::BadRequest)?;
    }
    document.check_limits()?;
    Ok(())
}

pub type ApiResult<T> = Result<T, ApiError>;
