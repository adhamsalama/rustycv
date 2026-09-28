//! Storage. One table, one JSON document per CV.
//!
//! Every read and write is scoped to an account. The scope is a `user_id`
//! argument rather than something a caller can forget: a query here that does
//! not mention it is one account reading another's CVs, so `WHERE user_id = ?`
//! sits on the lookup itself and not on a check around it.

use rustycv_core::CvDocument;
use serde::Serialize;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};

/// How many CVs one account may keep.
///
/// Enforced in the `INSERT` rather than by counting first and inserting after,
/// so two requests arriving together cannot both find room for the tenth.
pub const MAX_CVS: usize = 10;

pub async fn connect(url: &str) -> anyhow::Result<SqlitePool> {
    let options: SqliteConnectOptions = url
        .parse::<SqliteConnectOptions>()?
        .create_if_missing(true)
        // WAL keeps a long-running preview render from blocking an autosave.
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .foreign_keys(true)
        .busy_timeout(std::time::Duration::from_secs(5));

    let pool = SqlitePoolOptions::new()
        .max_connections(8)
        .connect_with(options)
        .await?;

    sqlx::migrate!("../../migrations").run(&pool).await?;
    Ok(pool)
}

/// A row as the dashboard needs it — no document body, just enough for a card.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CvSummary {
    pub id: String,
    pub title: String,
    pub template: String,
    pub full_name: String,
    pub created_at: String,
    pub updated_at: String,
    pub published: bool,
}

/// A full CV: metadata plus the document itself.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Cv {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
    pub document: CvDocument,
    /// The share link's token, once one has been assigned. Kept even while
    /// unpublished so republishing hands back the same link.
    pub public_id: Option<String>,
    pub published: bool,
    /// Whether visitors to the share link may leave comments.
    pub comments_enabled: bool,
    /// Comments left since the owner last opened them.
    pub unread_comments: i64,
}

fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

pub async fn list(pool: &SqlitePool, user_id: &str) -> ApiResult<Vec<CvSummary>> {
    // `json_extract` keeps the listing cheap: the dashboard never has to
    // deserialize a full document just to draw a card.
    let rows = sqlx::query(
        r#"
        SELECT id,
               title,
               COALESCE(json_extract(data, '$.template'), 'classic')   AS template,
               COALESCE(json_extract(data, '$.basics.fullName'), '')   AS full_name,
               created_at,
               updated_at,
               published
        FROM cvs
        WHERE user_id = ?
        ORDER BY updated_at DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| CvSummary {
            id: r.get("id"),
            title: r.get("title"),
            template: r.get("template"),
            full_name: r.get("full_name"),
            created_at: r.get("created_at"),
            updated_at: r.get("updated_at"),
            published: r.get("published"),
        })
        .collect())
}

/// One CV, if it belongs to this account.
///
/// Somebody else's id is `NotFound` rather than a 403: whether an id exists at
/// all is not this account's business.
pub async fn get(pool: &SqlitePool, user_id: &str, id: &str) -> ApiResult<Cv> {
    let row = sqlx::query(
        "SELECT id, title, data, created_at, updated_at, public_id, published,
                comments_enabled,
                (SELECT COUNT(*) FROM comments
                 WHERE comments.cv_id = cvs.id
                   AND (cvs.comments_read_at IS NULL
                        OR comments.created_at > cvs.comments_read_at)) AS unread_comments
         FROM cvs WHERE id = ? AND user_id = ?",
    )
    .bind(id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?
    .ok_or(ApiError::NotFound)?;

    let data: String = row.get("data");
    Ok(Cv {
        id: row.get("id"),
        title: row.get("title"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
        document: serde_json::from_str(&data)?,
        public_id: row.get("public_id"),
        published: row.get("published"),
        comments_enabled: row.get("comments_enabled"),
        unread_comments: row.get("unread_comments"),
    })
}

/// The owner has seen every comment there is so far.
///
/// Marks up to the newest comment's own timestamp rather than the clock's, so
/// the comparison is between two values this table wrote and a comment
/// arriving mid-request is still unread afterwards.
pub async fn mark_comments_read(pool: &SqlitePool, user_id: &str, id: &str) -> ApiResult<Cv> {
    let affected = sqlx::query(
        "UPDATE cvs
         SET comments_read_at = COALESCE(
             (SELECT MAX(created_at) FROM comments WHERE cv_id = cvs.id),
             comments_read_at)
         WHERE id = ? AND user_id = ?",
    )
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await?
    .rows_affected();
    if affected == 0 {
        return Err(ApiError::NotFound);
    }
    get(pool, user_id, id).await
}

/// Turn comments on the share link on or off. Existing comments are kept
/// either way; switching off only hides them and stops new ones.
pub async fn set_comments_enabled(
    pool: &SqlitePool,
    user_id: &str,
    id: &str,
    enabled: bool,
) -> ApiResult<Cv> {
    let affected = sqlx::query("UPDATE cvs SET comments_enabled = ? WHERE id = ? AND user_id = ?")
        .bind(enabled)
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await?
        .rows_affected();
    if affected == 0 {
        return Err(ApiError::NotFound);
    }
    get(pool, user_id, id).await
}

pub async fn create(
    pool: &SqlitePool,
    user_id: &str,
    title: &str,
    document: &CvDocument,
) -> ApiResult<Cv> {
    let id = Uuid::new_v4().to_string();
    let ts = now();
    let data = serde_json::to_string(document)?;

    // `INSERT ... SELECT ... WHERE` so the count and the write are one
    // statement: the row appears only if there was room for it at the moment
    // it was written, and no row written means the account is full.
    let inserted = sqlx::query(
        "INSERT INTO cvs (id, user_id, title, schema_version, data, created_at, updated_at)
         SELECT ?, ?, ?, ?, ?, ?, ?
         WHERE (SELECT COUNT(*) FROM cvs WHERE user_id = ?) < ?",
    )
    .bind(&id)
    .bind(user_id)
    .bind(title)
    .bind(document.schema_version as i64)
    .bind(&data)
    .bind(&ts)
    .bind(&ts)
    .bind(user_id)
    .bind(MAX_CVS as i64)
    .execute(pool)
    .await?
    .rows_affected();

    if inserted == 0 {
        return Err(ApiError::LimitReached(format!(
            "you already have {MAX_CVS} CVs — delete one to make room"
        )));
    }

    get(pool, user_id, &id).await
}

pub async fn update(
    pool: &SqlitePool,
    user_id: &str,
    id: &str,
    title: Option<&str>,
    document: &CvDocument,
) -> ApiResult<Cv> {
    let data = serde_json::to_string(document)?;
    let affected = sqlx::query(
        "UPDATE cvs
         SET data = ?, schema_version = ?, title = COALESCE(?, title), updated_at = ?
         WHERE id = ? AND user_id = ?",
    )
    .bind(&data)
    .bind(document.schema_version as i64)
    .bind(title)
    .bind(now())
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await?
    .rows_affected();

    if affected == 0 {
        return Err(ApiError::NotFound);
    }
    get(pool, user_id, id).await
}

pub async fn delete(pool: &SqlitePool, user_id: &str, id: &str) -> ApiResult<()> {
    let affected = sqlx::query("DELETE FROM cvs WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await?
        .rows_affected();

    if affected == 0 {
        return Err(ApiError::NotFound);
    }
    Ok(())
}

// ------------------------------------------------------------- publishing

/// Turn the share link on, assigning it a token the first time this is
/// called. Later calls with the link already off just flip it back on,
/// handing back the same token — a link once shared should not silently stop
/// working just because it was toggled off and on again.
pub async fn publish(pool: &SqlitePool, user_id: &str, id: &str) -> ApiResult<Cv> {
    let existing = get(pool, user_id, id).await?;
    let public_id = existing
        .public_id
        .clone()
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    sqlx::query("UPDATE cvs SET public_id = ?, published = 1 WHERE id = ? AND user_id = ?")
        .bind(&public_id)
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await?;

    get(pool, user_id, id).await
}

pub async fn unpublish(pool: &SqlitePool, user_id: &str, id: &str) -> ApiResult<Cv> {
    // The cache is cleared, not just gated off: `published = 0` already keeps
    // `get_published` from ever handing these bytes to a visitor, but leaving
    // a rendered PDF sitting in the row after the link is switched off is the
    // one thing this cache was never supposed to be — the CV's data, rather
    // than a disposable memo of it.
    let affected = sqlx::query(
        "UPDATE cvs
         SET published = 0, public_pdf = NULL, public_pdf_cached_for = NULL
         WHERE id = ? AND user_id = ?",
    )
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await?
    .rows_affected();

    if affected == 0 {
        return Err(ApiError::NotFound);
    }
    get(pool, user_id, id).await
}

/// A published CV, as the public link needs it: the document to render, plus
/// whatever the last render for this content was, so the caller can decide
/// whether it is still good.
pub struct PublishedCv {
    pub id: String,
    pub title: String,
    pub document: CvDocument,
    /// The content's own version. `cached_pdf` is only good while this
    /// matches — the moment an edit lands, `updated_at` moves on and the
    /// cache is stale.
    pub updated_at: String,
    pub cached_pdf: Option<Vec<u8>>,
    pub cached_for: Option<String>,
    pub comments_enabled: bool,
}

/// Somebody following a share link. `NotFound` unless the CV both exists and
/// is currently published — a toggled-off link reads exactly like one that
/// never existed, which is the point of the switch.
pub async fn get_published(pool: &SqlitePool, public_id: &str) -> ApiResult<PublishedCv> {
    let row = sqlx::query(
        "SELECT id, title, data, updated_at, public_pdf, public_pdf_cached_for,
                comments_enabled
         FROM cvs WHERE public_id = ? AND published = 1",
    )
    .bind(public_id)
    .fetch_optional(pool)
    .await?
    .ok_or(ApiError::NotFound)?;

    let data: String = row.get("data");
    Ok(PublishedCv {
        id: row.get("id"),
        title: row.get("title"),
        document: serde_json::from_str(&data)?,
        updated_at: row.get("updated_at"),
        cached_pdf: row.get("public_pdf"),
        cached_for: row.get("public_pdf_cached_for"),
        comments_enabled: row.get("comments_enabled"),
    })
}

/// Save a freshly rendered PDF as the cache for this content version.
///
/// `rendered_for` is the `updated_at` that was current when rendering
/// started, not when it finished — if an edit lands in between, the row's
/// `updated_at` has already moved past what we are about to write, so the
/// next visit's version check fails and it renders again instead of serving
/// what just went stale.
pub async fn cache_published_pdf(pool: &SqlitePool, id: &str, rendered_for: &str, pdf: &[u8]) {
    // Best-effort: a failure to cache means the next visit renders again,
    // which is correct behaviour, not a degraded one — so it is logged rather
    // than surfaced as a request failure.
    if let Err(error) =
        sqlx::query("UPDATE cvs SET public_pdf = ?, public_pdf_cached_for = ? WHERE id = ?")
            .bind(pdf)
            .bind(rendered_for)
            .bind(id)
            .execute(pool)
            .await
    {
        tracing::warn!(%error, cv_id = %id, "failed to cache published pdf");
    }
}
