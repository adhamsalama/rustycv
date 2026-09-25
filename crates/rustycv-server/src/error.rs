use axum::http::StatusCode;
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
        let (status, message, diagnostics) = match self {
            ApiError::NotFound => (StatusCode::NOT_FOUND, self.to_string(), vec![]),
            ApiError::BadRequest(ref m) => (StatusCode::BAD_REQUEST, m.clone(), vec![]),
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

        (
            status,
            Json(ErrorBody {
                error: message,
                diagnostics,
            }),
        )
            .into_response()
    }
}

pub type ApiResult<T> = Result<T, ApiError>;
