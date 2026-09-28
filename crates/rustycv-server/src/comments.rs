//! Comments on a published CV, left by anonymous visitors to its share link.
//!
//! Visitors can post and read; only the CV's owner can delete. There is no
//! account to scope a visitor by, so the brakes are elsewhere: the per-address
//! and per-CV windows in `routes.rs`, and a hard ceiling per CV enforced inside
//! the `INSERT` so that counting and writing cannot race.
//!
//! Owner-side reads and deletes are scoped like everything else in `db.rs`:
//! the `user_id` sits in the query, and somebody else's CV is a 404.

use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};

/// How many comments one CV may hold. Refusals past it are 409, like the other
/// caps: the owner deleting one makes the identical request succeed.
pub const MAX_COMMENTS: usize = 100;

/// Characters, not bytes, like `rustycv_core::limits`.
pub const MAX_AUTHOR: usize = 80;
pub const MAX_BODY: usize = 2000;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Comment {
    pub id: String,
    pub author: String,
    pub body: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommentInput {
    /// Optional: an empty name is shown as "Anonymous" by the client.
    #[serde(default)]
    pub author: String,
    pub body: String,
}

impl CommentInput {
    fn check(&self) -> ApiResult<(String, String)> {
        let author = self.author.trim();
        let body = self.body.trim();
        if body.is_empty() {
            return Err(ApiError::BadRequest("a comment cannot be empty".into()));
        }
        if author.chars().count() > MAX_AUTHOR {
            return Err(ApiError::BadRequest(format!(
                "a name is at most {MAX_AUTHOR} characters"
            )));
        }
        if body.chars().count() > MAX_BODY {
            return Err(ApiError::BadRequest(format!(
                "a comment is at most {MAX_BODY} characters"
            )));
        }
        Ok((author.to_string(), body.to_string()))
    }
}

fn row_to_comment(row: sqlx::sqlite::SqliteRow) -> Comment {
    Comment {
        id: row.get("id"),
        author: row.get("author"),
        body: row.get("body"),
        created_at: row.get("created_at"),
    }
}

/// The comments behind a share link, oldest first. `NotFound` unless the CV is
/// currently published — an unpublished CV's comments are as hidden as it is.
pub async fn list_published(pool: &SqlitePool, public_id: &str) -> ApiResult<Vec<Comment>> {
    // With comments switched off the list is empty: the owner has hidden
    // them, not lost them, and switching back on brings them back.
    let (cv_id, enabled) = published_cv(pool, public_id).await?;
    if !enabled {
        return Ok(vec![]);
    }
    list_for(pool, &cv_id).await
}

/// The CV behind a live link, and whether it takes comments.
async fn published_cv(pool: &SqlitePool, public_id: &str) -> ApiResult<(String, bool)> {
    let row =
        sqlx::query("SELECT id, comments_enabled FROM cvs WHERE public_id = ? AND published = 1")
            .bind(public_id)
            .fetch_optional(pool)
            .await?
            .ok_or(ApiError::NotFound)?;
    Ok((row.get("id"), row.get("comments_enabled")))
}

async fn list_for(pool: &SqlitePool, cv_id: &str) -> ApiResult<Vec<Comment>> {
    let rows = sqlx::query(
        "SELECT id, author, body, created_at FROM comments
         WHERE cv_id = ? ORDER BY created_at, id",
    )
    .bind(cv_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(row_to_comment).collect())
}

/// Leave a comment on a published CV.
pub async fn create(pool: &SqlitePool, public_id: &str, input: CommentInput) -> ApiResult<Comment> {
    let (author, body) = input.check()?;
    let comment = Comment {
        id: Uuid::new_v4().to_string(),
        author,
        body,
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    // One statement finds the CV, checks it is published and open to comments
    // and checks the ceiling, so none of them can change between the check and
    // the write.
    let inserted = sqlx::query(
        "INSERT INTO comments (id, cv_id, author, body, created_at)
         SELECT ?, cvs.id, ?, ?, ?
         FROM cvs
         WHERE cvs.public_id = ? AND cvs.published = 1 AND cvs.comments_enabled = 1
           AND (SELECT COUNT(*) FROM comments WHERE cv_id = cvs.id) < ?",
    )
    .bind(&comment.id)
    .bind(&comment.author)
    .bind(&comment.body)
    .bind(&comment.created_at)
    .bind(public_id)
    .bind(MAX_COMMENTS as i64)
    .execute(pool)
    .await?
    .rows_affected();

    if inserted == 0 {
        // Nothing written: no such link (404), comments off (403), or full.
        let (_, enabled) = published_cv(pool, public_id).await?;
        if !enabled {
            return Err(ApiError::Forbidden(
                "comments are turned off for this CV".into(),
            ));
        }
        return Err(ApiError::LimitReached(format!(
            "this CV already has {MAX_COMMENTS} comments, which is as many as it can take"
        )));
    }

    Ok(comment)
}

/// The owner's view: every comment on one of their CVs, published or not.
pub async fn list_owned(pool: &SqlitePool, user_id: &str, cv_id: &str) -> ApiResult<Vec<Comment>> {
    let owned: Option<String> =
        sqlx::query_scalar("SELECT id FROM cvs WHERE id = ? AND user_id = ?")
            .bind(cv_id)
            .bind(user_id)
            .fetch_optional(pool)
            .await?;
    owned.ok_or(ApiError::NotFound)?;
    list_for(pool, cv_id).await
}

/// Only the CV's owner can remove a comment.
pub async fn delete(
    pool: &SqlitePool,
    user_id: &str,
    cv_id: &str,
    comment_id: &str,
) -> ApiResult<()> {
    let affected = sqlx::query(
        "DELETE FROM comments
         WHERE id = ? AND cv_id = (SELECT id FROM cvs WHERE id = ? AND user_id = ?)",
    )
    .bind(comment_id)
    .bind(cv_id)
    .bind(user_id)
    .execute(pool)
    .await?
    .rows_affected();

    if affected == 0 {
        return Err(ApiError::NotFound);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_comment_is_trimmed_and_bounded() {
        let input = |author: &str, body: &str| CommentInput {
            author: author.into(),
            body: body.into(),
        };

        assert_eq!(
            input("  Sam ", " nice \n").check().unwrap(),
            ("Sam".to_string(), "nice".to_string())
        );
        assert!(input("", "   ").check().is_err(), "blank is empty");
        assert!(input(&"a".repeat(MAX_AUTHOR + 1), "x").check().is_err());
        assert!(
            input("", &"é".repeat(MAX_BODY)).check().is_ok(),
            "chars, not bytes"
        );
        assert!(input("", &"a".repeat(MAX_BODY + 1)).check().is_err());
    }
}
