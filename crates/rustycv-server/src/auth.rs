//! Accounts and sessions.
//!
//! Deliberately the smallest thing that is still a real login: an email, an
//! Argon2id password hash, and an opaque session token in an HttpOnly cookie.
//! No email verification, no password reset, no OAuth — each of those wants a
//! mail sender this app does not have.
//!
//! The cookie carries no claims, so nothing here has to be signed or rotated:
//! the token is a lookup key, and signing out deletes the row it points at.

use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};

/// The session cookie's name. `__Host-` would be the stricter choice but it
/// requires `Secure`, and this server speaks plain HTTP on loopback.
pub const SESSION_COOKIE: &str = "rustycv_session";

/// How long a session lasts without being used again. Long enough that the
/// editor is not interrupted mid-CV, short enough that a forgotten login on a
/// shared machine expires on its own.
pub const SESSION_DAYS: i64 = 30;

/// The shortest password accepted. A length floor is the only rule here:
/// composition rules push people towards `Passw0rd!` and buy nothing.
const MIN_PASSWORD: usize = 8;

/// A signed-in account, as the API talks about it. The hash never leaves this
/// module, so there is no shape of this struct that can serialize it by
/// accident.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: String,
    pub email: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Credentials {
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub password: String,
}

fn now() -> chrono::DateTime<chrono::Utc> {
    chrono::Utc::now()
}

/// The stored form of an address: one account per address, however it was
/// typed. Normalising on the way in rather than relying on a `NOCASE`
/// collation keeps "which spelling is in the table" answerable.
fn normalize_email(raw: &str) -> String {
    raw.trim().to_lowercase()
}

/// Enough of a check to catch a typo'd address, and no more. Anything stricter
/// is a losing argument with the actual grammar of an email address.
fn valid_email(email: &str) -> bool {
    match email.split_once('@') {
        Some((local, domain)) => {
            !local.is_empty()
                && domain.contains('.')
                && !domain.starts_with('.')
                && !domain.ends_with('.')
                && !email.contains(char::is_whitespace)
        }
        None => false,
    }
}

/// `n` bytes from the operating system.
///
/// A failure here is the platform refusing to produce entropy, which is not
/// something to paper over with a weaker source: both callers turn it into a
/// 500 rather than continuing with a guessable salt or token.
fn random_bytes<const N: usize>() -> ApiResult<[u8; N]> {
    let mut bytes = [0u8; N];
    getrandom::fill(&mut bytes).map_err(|e| {
        tracing::error!(error = %e, "the OS refused to provide randomness");
        ApiError::Internal
    })?;
    Ok(bytes)
}

fn hash_password(password: &str) -> ApiResult<String> {
    // 16 bytes is what `SaltString::generate` would have produced, and the
    // length the PHC format's recommendation settles on.
    let salt = SaltString::encode_b64(&random_bytes::<16>()?).map_err(|e| {
        tracing::error!(error = %e, "failed to encode a password salt");
        ApiError::Internal
    })?;
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|e| {
            tracing::error!(error = %e, "failed to hash a password");
            ApiError::Internal
        })
}

/// A 256-bit token, hex so it survives a cookie header unescaped.
fn new_token() -> ApiResult<String> {
    Ok(random_bytes::<32>()?
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}

/// Create an account and return it.
///
/// The first account on a database created before accounts existed adopts the
/// rows nobody owns. That is the whole migration story for an installation
/// that has been in use: without it a local instance would come back up
/// looking empty, which is indistinguishable from having lost the CVs.
pub async fn signup(pool: &SqlitePool, credentials: &Credentials) -> ApiResult<User> {
    let email = normalize_email(&credentials.email);
    if !valid_email(&email) {
        return Err(ApiError::BadRequest(
            "that does not look like an email address".into(),
        ));
    }
    if credentials.password.chars().count() < MIN_PASSWORD {
        return Err(ApiError::BadRequest(format!(
            "a password needs at least {MIN_PASSWORD} characters"
        )));
    }

    let hash = hash_password(&credentials.password)?;
    let id = Uuid::new_v4().to_string();
    let ts = now().to_rfc3339();

    let mut tx = pool.begin().await?;

    // Whether this is the first account has to be read inside the transaction
    // that writes it, or two simultaneous signups could both decide they were
    // first and both adopt the orphans.
    let existing: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&mut *tx)
        .await?;

    let inserted = sqlx::query(
        "INSERT INTO users (id, email, password_hash, created_at)
         VALUES (?, ?, ?, ?)
         ON CONFLICT (email) DO NOTHING",
    )
    .bind(&id)
    .bind(&email)
    .bind(&hash)
    .bind(&ts)
    .execute(&mut *tx)
    .await?
    .rows_affected();

    if inserted == 0 {
        // Deliberately the same wording the editor shows for a wrong password
        // on the sign-in form would not be — an address either has an account
        // or it does not, and the signup form has to say which.
        return Err(ApiError::BadRequest(
            "an account already exists for that address".into(),
        ));
    }

    if existing == 0 {
        sqlx::query("UPDATE cvs SET user_id = ? WHERE user_id IS NULL")
            .bind(&id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE applications SET user_id = ? WHERE user_id IS NULL")
            .bind(&id)
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await?;

    Ok(User {
        id,
        email,
        created_at: ts,
    })
}

/// Check a password and return the account it belongs to.
///
/// A missing account and a wrong password are the same error on purpose: the
/// difference is exactly the "is this address registered here?" oracle that a
/// login form should not be.
pub async fn login(pool: &SqlitePool, credentials: &Credentials) -> ApiResult<User> {
    let email = normalize_email(&credentials.email);

    let row = sqlx::query("SELECT id, email, password_hash, created_at FROM users WHERE email = ?")
        .bind(&email)
        .fetch_optional(pool)
        .await?;

    let Some(row) = row else {
        // Still spend the time an Argon2 verification costs. Returning early
        // makes "no such account" measurably faster than "wrong password",
        // which hands back the oracle the shared message is hiding.
        let _ = hash_password(&credentials.password);
        return Err(ApiError::InvalidCredentials);
    };

    let stored: String = row.get("password_hash");
    let parsed = PasswordHash::new(&stored).map_err(|e| {
        tracing::error!(error = %e, "a stored password hash did not parse");
        ApiError::Internal
    })?;

    Argon2::default()
        .verify_password(credentials.password.as_bytes(), &parsed)
        .map_err(|_| ApiError::InvalidCredentials)?;

    Ok(User {
        id: row.get("id"),
        email: row.get("email"),
        created_at: row.get("created_at"),
    })
}

/// Issue a session token for `user_id`.
pub async fn create_session(pool: &SqlitePool, user_id: &str) -> ApiResult<String> {
    let token = new_token()?;
    let created = now();
    let expires = created + chrono::Duration::days(SESSION_DAYS);

    sqlx::query(
        "INSERT INTO sessions (token, user_id, created_at, expires_at) VALUES (?, ?, ?, ?)",
    )
    .bind(&token)
    .bind(user_id)
    .bind(created.to_rfc3339())
    .bind(expires.to_rfc3339())
    .execute(pool)
    .await?;

    Ok(token)
}

/// The account a token belongs to, or `None` if the token is unknown or
/// expired. An expired row is deleted as it is found, which is all the session
/// cleanup this needs — a session nobody presents costs one row.
pub async fn user_for_token(pool: &SqlitePool, token: &str) -> ApiResult<Option<User>> {
    let row = sqlx::query(
        "SELECT u.id, u.email, u.created_at, s.expires_at
         FROM sessions s
         JOIN users u ON u.id = s.user_id
         WHERE s.token = ?",
    )
    .bind(token)
    .fetch_optional(pool)
    .await?;

    let Some(row) = row else {
        return Ok(None);
    };

    let expires_at: String = row.get("expires_at");
    let expired = chrono::DateTime::parse_from_rfc3339(&expires_at)
        .map(|t| t < now())
        // An unparseable timestamp is a row this server did not write the way
        // it thinks it did; treating it as expired fails closed.
        .unwrap_or(true);

    if expired {
        delete_session(pool, token).await?;
        return Ok(None);
    }

    Ok(Some(User {
        id: row.get("id"),
        email: row.get("email"),
        created_at: row.get("created_at"),
    }))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PasswordChange {
    #[serde(default)]
    pub current_password: String,
    #[serde(default)]
    pub new_password: String,
}

/// Replace a password, and evict every session but the one asking.
///
/// Evicting the others is most of the point: someone changing a password
/// because a machine was lost or borrowed needs the sessions on it to stop
/// working, and a password change that left them alive would look like it had
/// fixed something it had not. The caller's own session survives, so changing
/// your password does not sign you out of the tab you did it in.
///
/// A wrong current password is a **400, not a 401** — deliberately. The editor
/// treats any 401 as "the session is gone" and drops to the landing page, so
/// answering 401 here would throw someone out of the app for a typo in a form
/// field.
pub async fn change_password(
    pool: &SqlitePool,
    user_id: &str,
    keep_token: &str,
    change: &PasswordChange,
) -> ApiResult<()> {
    let stored: String = sqlx::query_scalar("SELECT password_hash FROM users WHERE id = ?")
        .bind(user_id)
        .fetch_optional(pool)
        .await?
        .ok_or(ApiError::NotFound)?;

    let parsed = PasswordHash::new(&stored).map_err(|e| {
        tracing::error!(error = %e, "a stored password hash did not parse");
        ApiError::Internal
    })?;

    Argon2::default()
        .verify_password(change.current_password.as_bytes(), &parsed)
        .map_err(|_| ApiError::BadRequest("your current password is not right".into()))?;

    if change.new_password.chars().count() < MIN_PASSWORD {
        return Err(ApiError::BadRequest(format!(
            "a password needs at least {MIN_PASSWORD} characters"
        )));
    }

    let hash = hash_password(&change.new_password)?;

    // One transaction: a new password that had not yet evicted the old
    // sessions would be a window where the change had not taken effect.
    let mut tx = pool.begin().await?;

    sqlx::query("UPDATE users SET password_hash = ? WHERE id = ?")
        .bind(&hash)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query("DELETE FROM sessions WHERE user_id = ? AND token <> ?")
        .bind(user_id)
        .bind(keep_token)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(())
}

pub async fn delete_session(pool: &SqlitePool, token: &str) -> ApiResult<()> {
    sqlx::query("DELETE FROM sessions WHERE token = ?")
        .bind(token)
        .execute(pool)
        .await?;
    Ok(())
}

/// The `Set-Cookie` value that puts `token` in the browser.
///
/// `HttpOnly` so a script cannot read it, `SameSite=Lax` so a cross-site POST
/// cannot ride it while a normal link still works. No `Secure`: the server
/// speaks HTTP on loopback and the flag would stop the cookie being stored at
/// all. Behind TLS it belongs here — see the README.
pub fn session_cookie(token: &str) -> String {
    format!(
        "{SESSION_COOKIE}={token}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}",
        SESSION_DAYS * 24 * 60 * 60
    )
}

/// The `Set-Cookie` value that clears it.
pub fn expired_cookie() -> String {
    format!("{SESSION_COOKIE}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0")
}

/// Pull one cookie out of a `Cookie` header.
///
/// Hand-parsed rather than pulling in a cookie crate: this is the only cookie
/// the server reads, and the header is a `;`-separated list of `name=value`.
pub fn cookie_value<'a>(header: &'a str, name: &str) -> Option<&'a str> {
    header.split(';').find_map(|part| {
        let (key, value) = part.split_once('=')?;
        (key.trim() == name).then(|| value.trim())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cookie_is_found_wherever_it_sits_in_the_header() {
        let header = "other=1; rustycv_session=abc123; last=2";
        assert_eq!(cookie_value(header, SESSION_COOKIE), Some("abc123"));
        assert_eq!(
            cookie_value("rustycv_session=solo", SESSION_COOKIE),
            Some("solo")
        );
        assert_eq!(cookie_value("nothing=here", SESSION_COOKIE), None);
        // A prefix match would find this one, and it is a different cookie.
        assert_eq!(cookie_value("not_rustycv_session=x", SESSION_COOKIE), None);
    }

    #[test]
    fn an_address_is_one_account_however_it_is_typed() {
        assert_eq!(normalize_email("  Adham@Example.COM "), "adham@example.com");
    }

    #[test]
    fn obvious_non_addresses_are_rejected() {
        assert!(valid_email("a@b.co"));
        assert!(!valid_email("no-at-sign.com"));
        assert!(!valid_email("@example.com"));
        assert!(!valid_email("a@nodot"));
        assert!(!valid_email("a b@example.com"));
    }
}
