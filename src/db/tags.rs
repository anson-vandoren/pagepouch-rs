//! Tag database operations.

use anyhow::Result;
use sqlx::SqlitePool;

/// Represents a tag with usage information.
#[derive(Clone, Debug)]
pub struct TagWithUsage {
    pub tag_id: Vec<u8>,
    pub name: String,
    pub color: Option<String>,
    pub usage_count: i64,
}

/// Simple tag information.
#[derive(Clone, Debug)]
pub struct TagInfo {
    pub name: String,
    pub color: Option<String>,
}

/// Retrieves all tags used by a user's bookmarks, ordered by usage.
///
/// # Errors
///
/// Returns an error if database query fails.
pub async fn get_user_tags(pool: &SqlitePool, user_id: &[u8]) -> Result<Vec<TagInfo>> {
    let tags = sqlx::query!(
        r#"
        select distinct t.name, t.color
        from tags t
        join bookmark_tags bt on t.tag_id = bt.tag_id
        join bookmarks b on bt.bookmark_id = b.bookmark_id
        where b.user_id = ? and b.is_archived = 0
        order by t.name
        "#,
        user_id
    )
    .fetch_all(pool)
    .await?;

    let result = tags
        .into_iter()
        .map(|tag| TagInfo {
            name: tag.name,
            color: tag.color,
        })
        .collect();

    Ok(result)
}

/// Retrieves popular tags for a user with usage counts.
///
/// # Errors
///
/// Returns an error if database query fails.
pub async fn get_popular_user_tags(pool: &SqlitePool, user_id: &[u8], limit: i64) -> Result<Vec<TagWithUsage>> {
    let tags = sqlx::query!(
        r#"
        select
            t.tag_id,
            t.name,
            t.color,
            count(*) as "usage_count: i64"
        from tags t
        join bookmark_tags bt on t.tag_id = bt.tag_id
        join bookmarks b on bt.bookmark_id = b.bookmark_id
        where b.user_id = ? and b.is_archived = 0
        group by t.tag_id, t.name, t.color
        order by 4 desc, t.name
        limit ?
        "#,
        user_id,
        limit
    )
    .fetch_all(pool)
    .await?;

    let result = tags
        .into_iter()
        .map(|tag| TagWithUsage {
            tag_id: tag.tag_id,
            name: tag.name,
            color: tag.color,
            usage_count: tag.usage_count.unwrap_or(0),
        })
        .collect();

    Ok(result)
}

/// Gets or creates a tag by name.
///
/// # Errors
///
/// Returns an error if database operations fail.
pub async fn get_or_create_tag(pool: &SqlitePool, name: &str, color: Option<&str>) -> Result<Vec<u8>> {
    // Normalize tag name (lowercase, trimmed)
    let normalized_name = name.trim().to_lowercase();

    // Try to find existing tag
    if let Some(existing) = sqlx::query!("select tag_id from tags where name = ?", normalized_name)
        .fetch_optional(pool)
        .await?
    {
        return Ok(existing.tag_id);
    }

    // Create new tag
    let result = sqlx::query!(
        "insert into tags (name, color) values (?, ?) returning tag_id",
        normalized_name,
        color
    )
    .fetch_one(pool)
    .await?;

    Ok(result.tag_id)
}

/// Renames a tag, which automatically propagates to all bookmarks.
///
/// # Errors
///
/// Returns an error if database operations fail.
pub async fn rename_tag(pool: &SqlitePool, old_name: &str, new_name: &str) -> Result<bool> {
    let normalized_old = old_name.trim().to_lowercase();
    let normalized_new = new_name.trim().to_lowercase();

    let result = sqlx::query!("update tags set name = ? where name = ?", normalized_new, normalized_old)
        .execute(pool)
        .await?;

    Ok(result.rows_affected() > 0)
}

/// Deletes unused tags (tags not associated with any bookmarks).
///
/// # Errors
///
/// Returns an error if database operations fail.
pub async fn cleanup_unused_tags(pool: &SqlitePool) -> Result<i64> {
    let result = sqlx::query!(
        r#"
        delete from tags
        where tag_id not in (
            select distinct tag_id from bookmark_tags
        )
        "#
    )
    .execute(pool)
    .await?;

    Ok(i64::try_from(result.rows_affected()).unwrap_or(0))
}
