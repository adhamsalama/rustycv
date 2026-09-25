//! Storage. One table, one JSON document per CV.

use rustycv_core::CvDocument;
use serde::Serialize;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};

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
}

fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

pub async fn list(pool: &SqlitePool) -> ApiResult<Vec<CvSummary>> {
    // `json_extract` keeps the listing cheap: the dashboard never has to
    // deserialize a full document just to draw a card.
    let rows = sqlx::query(
        r#"
        SELECT id,
               title,
               COALESCE(json_extract(data, '$.template'), 'classic')   AS template,
               COALESCE(json_extract(data, '$.basics.fullName'), '')   AS full_name,
               created_at,
               updated_at
        FROM cvs
        ORDER BY updated_at DESC
        "#,
    )
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
        })
        .collect())
}

pub async fn get(pool: &SqlitePool, id: &str) -> ApiResult<Cv> {
    let row = sqlx::query("SELECT id, title, data, created_at, updated_at FROM cvs WHERE id = ?")
        .bind(id)
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
    })
}

pub async fn create(pool: &SqlitePool, title: &str, document: &CvDocument) -> ApiResult<Cv> {
    let id = Uuid::new_v4().to_string();
    let ts = now();
    let data = serde_json::to_string(document)?;

    sqlx::query(
        "INSERT INTO cvs (id, title, schema_version, data, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(title)
    .bind(document.schema_version as i64)
    .bind(&data)
    .bind(&ts)
    .bind(&ts)
    .execute(pool)
    .await?;

    get(pool, &id).await
}

pub async fn update(
    pool: &SqlitePool,
    id: &str,
    title: Option<&str>,
    document: &CvDocument,
) -> ApiResult<Cv> {
    let data = serde_json::to_string(document)?;
    let affected = sqlx::query(
        "UPDATE cvs
         SET data = ?, schema_version = ?, title = COALESCE(?, title), updated_at = ?
         WHERE id = ?",
    )
    .bind(&data)
    .bind(document.schema_version as i64)
    .bind(title)
    .bind(now())
    .bind(id)
    .execute(pool)
    .await?
    .rows_affected();

    if affected == 0 {
        return Err(ApiError::NotFound);
    }
    get(pool, id).await
}

pub async fn delete(pool: &SqlitePool, id: &str) -> ApiResult<()> {
    let affected = sqlx::query("DELETE FROM cvs WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?
        .rows_affected();

    if affected == 0 {
        return Err(ApiError::NotFound);
    }
    Ok(())
}
