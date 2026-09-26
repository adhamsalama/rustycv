//! The job tracker: the applications a CV gets sent to.
//!
//! Deliberately a table of its own rather than a section of a document. A CV
//! is a printed page and everything in it ends up in the PDF; an application
//! is private admin that never renders, so it must not be able to reach the
//! render path at all.

use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};

/// How many applications one account may track. Enforced the same way the CV
/// cap is: inside the `INSERT`, so the count cannot go stale between checking
/// it and writing the row.
pub const MAX_APPLICATIONS: usize = 10;

/// The board's columns, in the order they are drawn.
///
/// A closed set on purpose: user-defined columns would mean storing a column
/// table, ordering it, and renaming across cards, for a tracker whose whole
/// point is that it is a glance and not a project.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Status {
    #[default]
    Wishlist,
    Applied,
    Interview,
    Offer,
    Rejected,
}

impl Status {
    pub const ALL: [Status; 5] = [
        Status::Wishlist,
        Status::Applied,
        Status::Interview,
        Status::Offer,
        Status::Rejected,
    ];

    /// The stored form. Must match the `CHECK` constraint in the migration.
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Wishlist => "wishlist",
            Status::Applied => "applied",
            Status::Interview => "interview",
            Status::Offer => "offer",
            Status::Rejected => "rejected",
        }
    }

    fn parse(raw: &str) -> Option<Self> {
        Status::ALL.into_iter().find(|s| s.as_str() == raw)
    }

    /// Read a column off the wire.
    ///
    /// Parsed by hand rather than derived: `#[derive(Deserialize)]` on the enum
    /// would make an unknown column a bare 422 from the extractor, and 422 in
    /// this API already means "the template failed to compile". This gives the
    /// same 400-with-the-known-list an unknown template gets.
    pub fn from_wire(raw: &str) -> ApiResult<Self> {
        Status::parse(raw.trim()).ok_or_else(|| {
            ApiError::BadRequest(format!(
                "unknown column `{raw}` — the board has: {}",
                Status::ALL
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
        })
    }
}

/// A card. `position` is not on the wire: the order of the list *is* the order
/// of the board, so there is no second source of truth for the client to keep.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Application {
    pub id: String,
    pub company: String,
    pub role: String,
    pub url: String,
    pub notes: String,
    pub status: Status,
    pub cv_id: Option<String>,
    /// The linked CV's title, joined in so a card can name it without the
    /// board fetching every CV to look one title up.
    pub cv_title: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Everything a card owns except where it sits. Placement is the business of
/// [`create`], [`update`] and [`move_to`], not of the body.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationInput {
    #[serde(default)]
    pub company: String,
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub notes: String,
    /// Omitted means the first column — the natural place for a card someone
    /// has only just thought of.
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub cv_id: Option<String>,
}

impl ApplicationInput {
    fn status(&self) -> ApiResult<Status> {
        match self.status.as_deref() {
            None => Ok(Status::default()),
            Some(raw) => Status::from_wire(raw),
        }
    }
}

const SELECT: &str = r#"
    SELECT a.id, a.company, a.role, a.url, a.notes, a.status, a.cv_id,
           c.title AS cv_title, a.created_at, a.updated_at
    FROM applications a
    LEFT JOIN cvs c ON c.id = a.cv_id
"#;

fn row_to_application(row: sqlx::sqlite::SqliteRow) -> Application {
    let status: String = row.get("status");
    Application {
        id: row.get("id"),
        company: row.get("company"),
        role: row.get("role"),
        url: row.get("url"),
        notes: row.get("notes"),
        // The CHECK constraint makes an unknown value unreachable; landing in
        // the first column beats failing the whole board if one ever appears.
        status: Status::parse(&status).unwrap_or_default(),
        cv_id: row.get("cv_id"),
        cv_title: row.get("cv_title"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// An empty selection in the CV picker arrives as `""`, which is not a CV id.
///
/// Scoped to the account: linking to somebody else's CV would put their title
/// on this board through the join in `SELECT`.
async fn linked_cv(
    pool: &SqlitePool,
    user_id: &str,
    cv_id: Option<String>,
) -> ApiResult<Option<String>> {
    let Some(id) = cv_id.filter(|id| !id.trim().is_empty()) else {
        return Ok(None);
    };

    // Checked here rather than left to the foreign key, so a stale id is a 400
    // naming the problem instead of a 500 from a constraint violation.
    let exists = sqlx::query("SELECT 1 FROM cvs WHERE id = ? AND user_id = ?")
        .bind(&id)
        .bind(user_id)
        .fetch_optional(pool)
        .await?
        .is_some();

    if !exists {
        return Err(ApiError::BadRequest(format!("no such CV: {id}")));
    }
    Ok(Some(id))
}

/// The whole board, in board order. Columns are grouped by the client.
pub async fn list(pool: &SqlitePool, user_id: &str) -> ApiResult<Vec<Application>> {
    let rows = sqlx::query(&format!(
        "{SELECT} WHERE a.user_id = ? ORDER BY a.position, a.created_at"
    ))
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(row_to_application).collect())
}

pub async fn get(pool: &SqlitePool, user_id: &str, id: &str) -> ApiResult<Application> {
    let row = sqlx::query(&format!("{SELECT} WHERE a.id = ? AND a.user_id = ?"))
        .bind(id)
        .bind(user_id)
        .fetch_optional(pool)
        .await?
        .ok_or(ApiError::NotFound)?;
    Ok(row_to_application(row))
}

/// New cards land at the foot of their column, where the "Add" control is.
pub async fn create(
    pool: &SqlitePool,
    user_id: &str,
    input: ApplicationInput,
) -> ApiResult<Application> {
    let id = Uuid::new_v4().to_string();
    let ts = now();
    let status = input.status()?;
    let cv_id = linked_cv(pool, user_id, input.cv_id).await?;

    let inserted = sqlx::query(
        "INSERT INTO applications
             (id, user_id, company, role, url, notes, status, position, cv_id,
              created_at, updated_at)
         SELECT ?, ?, ?, ?, ?, ?, ?,
                (SELECT COALESCE(MAX(position), -1) + 1
                 FROM applications WHERE user_id = ? AND status = ?),
                ?, ?, ?
         WHERE (SELECT COUNT(*) FROM applications WHERE user_id = ?) < ?",
    )
    .bind(&id)
    .bind(user_id)
    .bind(input.company.trim())
    .bind(input.role.trim())
    .bind(input.url.trim())
    .bind(&input.notes)
    .bind(status.as_str())
    .bind(user_id)
    .bind(status.as_str())
    .bind(&cv_id)
    .bind(&ts)
    .bind(&ts)
    .bind(user_id)
    .bind(MAX_APPLICATIONS as i64)
    .execute(pool)
    .await?
    .rows_affected();

    if inserted == 0 {
        return Err(ApiError::LimitReached(format!(
            "you are already tracking {MAX_APPLICATIONS} applications — delete one to make room"
        )));
    }

    get(pool, user_id, &id).await
}

/// Edit a card's contents. Changing the status here is the keyboard route
/// between columns, so it has to place the card as well: it lands at the foot
/// of the column it moved to, exactly where a new card would.
pub async fn update(
    pool: &SqlitePool,
    user_id: &str,
    id: &str,
    input: ApplicationInput,
) -> ApiResult<Application> {
    let current = get(pool, user_id, id).await?;
    let status = input.status()?;
    let cv_id = linked_cv(pool, user_id, input.cv_id).await?;

    sqlx::query(
        "UPDATE applications
         SET company = ?, role = ?, url = ?, notes = ?, status = ?, cv_id = ?, updated_at = ?
         WHERE id = ? AND user_id = ?",
    )
    .bind(input.company.trim())
    .bind(input.role.trim())
    .bind(input.url.trim())
    .bind(&input.notes)
    .bind(status.as_str())
    .bind(&cv_id)
    .bind(now())
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await?;

    if current.status != status {
        // `id <> ?` so the row's own just-written position is not the maximum
        // it is being placed after.
        sqlx::query(
            "UPDATE applications
             SET position = (SELECT COALESCE(MAX(position), -1) + 1
                             FROM applications
                             WHERE user_id = ? AND status = ? AND id <> ?)
             WHERE id = ?",
        )
        .bind(user_id)
        .bind(status.as_str())
        .bind(id)
        .bind(id)
        .execute(pool)
        .await?;
    }

    get(pool, user_id, id).await
}

/// Drop a card into `status` at `index`, the way a drag leaves it.
///
/// Only the destination column is renumbered. The column it left keeps its
/// gaps, which costs nothing: `position` is read by `ORDER BY` alone and never
/// leaves the server.
pub async fn move_to(
    pool: &SqlitePool,
    user_id: &str,
    id: &str,
    status: Status,
    index: usize,
) -> ApiResult<Application> {
    // Fail before opening a transaction if the card is gone.
    get(pool, user_id, id).await?;

    let mut tx = pool.begin().await?;

    sqlx::query("UPDATE applications SET status = ?, updated_at = ? WHERE id = ? AND user_id = ?")
        .bind(status.as_str())
        .bind(now())
        .bind(id)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    let mut ids: Vec<String> = sqlx::query_scalar(
        "SELECT id FROM applications
         WHERE user_id = ? AND status = ? AND id <> ?
         ORDER BY position, created_at",
    )
    .bind(user_id)
    .bind(status.as_str())
    .bind(id)
    .fetch_all(&mut *tx)
    .await?;

    // An index past the end means "last"; the client should not have to know
    // how long the column was when the drag started.
    ids.insert(index.min(ids.len()), id.to_string());

    for (position, row_id) in ids.iter().enumerate() {
        sqlx::query("UPDATE applications SET position = ? WHERE id = ?")
            .bind(position as i64)
            .bind(row_id)
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await?;
    get(pool, user_id, id).await
}

pub async fn delete(pool: &SqlitePool, user_id: &str, id: &str) -> ApiResult<()> {
    let affected = sqlx::query("DELETE FROM applications WHERE id = ? AND user_id = ?")
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
