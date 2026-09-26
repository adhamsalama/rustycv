//! What runs around every API request: who is calling, and how often.
//!
//! Order matters and is set in [`crate::app_with_state`]. The session is
//! resolved first so the rate limiter can key on the account rather than the
//! address it happens to be dialling from; `require_auth` runs last, on the
//! routes that need somebody signed in, so an unauthenticated flood is still
//! counted rather than being rejected for free.

use std::net::SocketAddr;

use axum::extract::{ConnectInfo, Request, State};
use axum::http::header;
use axum::middleware::Next;
use axum::response::Response;
use axum::RequestPartsExt;

use crate::auth::{self, User, SESSION_COOKIE};
use crate::error::ApiError;
use crate::state::AppState;

/// Attach the signed-in account to the request, if the cookie names one.
///
/// Never fails: an absent, unknown or expired cookie simply leaves no user
/// attached, and it is [`require_auth`] that decides whether that is allowed.
/// A database error while looking a session up is the one exception — failing
/// open there would silently downgrade every protected route to public.
pub async fn resolve_session(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let token = request
        .headers()
        .get(header::COOKIE)
        .and_then(|value| value.to_str().ok())
        .and_then(|header| auth::cookie_value(header, SESSION_COOKIE))
        .map(str::to_string);

    if let Some(token) = token {
        if let Some(user) = auth::user_for_token(&state.pool, &token).await? {
            request.extensions_mut().insert(user);
            // The token itself, so signing out can delete the row rather than
            // only clearing the cookie. Kept apart from the user so that only
            // the handler that revokes it has to know it exists.
            request.extensions_mut().insert(SessionToken(token));
        }
    }

    Ok(next.run(request).await)
}

/// Count this request against whoever is making it.
///
/// A signed-in caller is counted by account, so the limit follows them across
/// networks and cannot be shed by changing address. Everyone else is counted
/// by peer address — `X-Forwarded-For` is deliberately not read, because a
/// header the client writes is a limit the client can opt out of.
pub async fn rate_limit(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let key = match request.extensions().get::<User>() {
        Some(user) => format!("user:{}", user.id),
        None => {
            let address = request
                .extensions()
                .get::<ConnectInfo<SocketAddr>>()
                // No connection info means the router is being driven directly
                // rather than served over TCP — a test, or an in-process call.
                // One shared bucket is the safe reading of "an unknown caller".
                .map_or_else(|| "unknown".to_string(), |info| info.0.ip().to_string());
            format!("ip:{address}")
        }
    };

    match state.limiter.check(&key) {
        Ok(_) => Ok(next.run(request).await),
        Err(allowance) => {
            tracing::warn!(key = %key, "rate limit reached");
            Err(ApiError::RateLimited {
                retry_after: allowance.reset_in,
            })
        }
    }
}

/// Refuse anything that reached a protected route without a session.
pub async fn require_auth(request: Request, next: Next) -> Result<Response, ApiError> {
    if request.extensions().get::<User>().is_none() {
        return Err(ApiError::Unauthorized);
    }
    Ok(next.run(request).await)
}

/// The session token this request arrived with, once it is known to be valid.
#[derive(Debug, Clone)]
pub struct SessionToken(pub String);

/// The signed-in account, for handlers that need it.
///
/// Reads what [`resolve_session`] attached rather than looking the session up
/// again, so a request costs one session query however many handlers want to
/// know who is calling.
#[derive(Debug, Clone)]
pub struct CurrentUser(pub User);

impl CurrentUser {
    pub fn id(&self) -> &str {
        &self.0.id
    }
}

impl<S> axum::extract::FromRequestParts<S> for CurrentUser
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        parts
            .extract::<axum::Extension<User>>()
            .await
            .map(|axum::Extension(user)| CurrentUser(user))
            .map_err(|_| ApiError::Unauthorized)
    }
}
