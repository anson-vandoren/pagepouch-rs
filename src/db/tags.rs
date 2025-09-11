//! Tag database operations.

use anyhow::Result;
use sqlx::{Row, SqlitePool};

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
pub async fn get_user_tags(pool: &SqlitePool, user_id: uuid::Uuid) -> Result<Vec<TagInfo>> {
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
pub async fn get_popular_user_tags(pool: &SqlitePool, user_id: uuid::Uuid, limit: i64) -> Result<Vec<TagWithUsage>> {
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

/// Gets tags that are present in bookmarks matching the specified tag filters.
/// If no tag filters are provided, returns all user tags.
///
/// # Errors
///
/// Returns an error if database query fails.
pub async fn get_tags_for_active_filters(pool: &SqlitePool, user_id: uuid::Uuid, active_tag_filters: &[String]) -> Result<Vec<TagInfo>> {
    if active_tag_filters.is_empty() {
        // No active tags, return all user tags
        return get_user_tags(pool, user_id).await;
    }

    // Get tags from bookmarks that match the active tag filters
    if active_tag_filters.len() == 1 {
        // Single tag filter - simpler query
        let tag_pattern = format!("%{}%", active_tag_filters[0]);
        let tags = sqlx::query!(
            r#"
            select distinct t2.name, t2.color
            from tags t2
            join bookmark_tags bt2 on t2.tag_id = bt2.tag_id
            join bookmarks b2 on bt2.bookmark_id = b2.bookmark_id
            where b2.user_id = ? and b2.is_archived = 0
            and b2.bookmark_id in (
                select distinct b.bookmark_id
                from bookmarks b
                join bookmark_tags bt on b.bookmark_id = bt.bookmark_id
                join tags t on bt.tag_id = t.tag_id
                where b.user_id = ? and b.is_archived = 0 and t.name like ?
            )
            order by t2.name
            "#,
            user_id,
            user_id,
            tag_pattern
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
    } else {
        // Multiple tag filters - find bookmarks that have ALL specified tags
        let like_conditions = active_tag_filters.iter().map(|_| "t.name like ?").collect::<Vec<_>>().join(" OR ");
        let sql = format!(
            r#"
            select distinct t2.name, t2.color
            from tags t2
            join bookmark_tags bt2 on t2.tag_id = bt2.tag_id
            join bookmarks b2 on bt2.bookmark_id = b2.bookmark_id
            where b2.user_id = ? and b2.is_archived = 0
            and b2.bookmark_id in (
                select bt.bookmark_id
                from bookmark_tags bt
                join tags t on bt.tag_id = t.tag_id
                where ({})
                group by bt.bookmark_id
                having count(distinct t.tag_id) >= ?
            )
            order by t2.name
            "#,
            like_conditions
        );

        let mut query = sqlx::query(&sql).bind(user_id);
        for tag_name in active_tag_filters {
            let tag_pattern = format!("%{tag_name}%");
            query = query.bind(tag_pattern);
        }
        query = query.bind(active_tag_filters.len() as i64);

        let rows = query.fetch_all(pool).await?;
        let result = rows
            .into_iter()
            .map(|row| TagInfo {
                name: row.get("name"),
                color: row.get("color"),
            })
            .collect();

        Ok(result)
    }
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
