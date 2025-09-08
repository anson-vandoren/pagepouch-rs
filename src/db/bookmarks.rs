//! Bookmark database operations.

use anyhow::Result;
use sqlx::SqlitePool;

/// Represents a bookmark with its associated tags for display.
#[derive(Clone, Debug)]
pub struct BookmarkWithTags {
    pub bookmark_id: Vec<u8>,
    pub url: String,
    pub title: String,
    pub description: Option<String>,
    pub created_at: i64,
    pub formatted_date: String,
    pub created_by: String,
    pub tags: Vec<TagInfo>,
}

/// Tag information for bookmarks.
#[derive(Clone, Debug)]
pub struct TagInfo {
    pub name: String,
}

/// Query parameters for filtering bookmarks.
#[derive(Debug, Default)]
pub struct BookmarkFilters {
    pub search_query: Option<String>,
    pub tag_filter: Option<String>,
    pub page: Option<i64>,
    pub limit: i64,
}

/// Retrieves bookmarks for a user with basic filtering.
///
/// # Errors
///
/// Returns an error if database query fails.
pub async fn get_user_bookmarks(pool: &SqlitePool, user_id: &[u8], limit: i64, offset: i64) -> Result<Vec<BookmarkWithTags>> {
    let bookmarks = sqlx::query!(
        r#"
        select 
            b.bookmark_id,
            b.url,
            b.title,
            b.description,
            b.created_at,
            u.username as created_by
        from bookmarks b
        join users u on b.user_id = u.user_id
        where b.user_id = ? and b.is_archived = 0
        order by b.created_at desc
        limit ? offset ?
        "#,
        user_id,
        limit,
        offset
    )
    .fetch_all(pool)
    .await?;

    let mut result = Vec::new();
    for bookmark in bookmarks {
        // Get tags for this bookmark
        let tags = sqlx::query!(
            r#"
            select t.name
            from bookmark_tags bt
            join tags t on bt.tag_id = t.tag_id
            where bt.bookmark_id = ?
            order by t.name
            "#,
            bookmark.bookmark_id
        )
        .fetch_all(pool)
        .await?;

        let tag_infos: Vec<TagInfo> = tags.into_iter().map(|tag| TagInfo { name: tag.name }).collect();

        let formatted_date = format_timestamp(bookmark.created_at);

        result.push(BookmarkWithTags {
            bookmark_id: bookmark.bookmark_id,
            url: bookmark.url,
            title: bookmark.title,
            description: bookmark.description,
            created_at: bookmark.created_at,
            formatted_date,
            created_by: bookmark.created_by,
            tags: tag_infos,
        });
    }

    Ok(result)
}

/// Retrieves bookmarks filtered by tag.
///
/// # Errors
///
/// Returns an error if database query fails.
pub async fn get_user_bookmarks_by_tag(
    pool: &SqlitePool,
    user_id: &[u8],
    tag_name: &str,
    limit: i64,
    offset: i64,
) -> Result<Vec<BookmarkWithTags>> {
    let bookmarks = sqlx::query!(
        r#"
        select 
            b.bookmark_id,
            b.url,
            b.title,
            b.description,
            b.created_at,
            u.username as created_by
        from bookmarks b
        join users u on b.user_id = u.user_id
        join bookmark_tags bt on b.bookmark_id = bt.bookmark_id
        join tags t on bt.tag_id = t.tag_id
        where b.user_id = ? and b.is_archived = 0 and t.name = ?
        order by b.created_at desc
        limit ? offset ?
        "#,
        user_id,
        tag_name,
        limit,
        offset
    )
    .fetch_all(pool)
    .await?;

    let mut result = Vec::new();
    for bookmark in bookmarks {
        // Get all tags for this bookmark
        let tags = sqlx::query!(
            r#"
            select t.name
            from bookmark_tags bt
            join tags t on bt.tag_id = t.tag_id
            where bt.bookmark_id = ?
            order by t.name
            "#,
            bookmark.bookmark_id
        )
        .fetch_all(pool)
        .await?;

        let tag_infos: Vec<TagInfo> = tags.into_iter().map(|tag| TagInfo { name: tag.name }).collect();

        let formatted_date = format_timestamp(bookmark.created_at);

        result.push(BookmarkWithTags {
            bookmark_id: bookmark.bookmark_id,
            url: bookmark.url,
            title: bookmark.title,
            description: bookmark.description,
            created_at: bookmark.created_at,
            formatted_date,
            created_by: bookmark.created_by,
            tags: tag_infos,
        });
    }

    Ok(result)
}

/// Searches bookmarks by title, description, URL, or tags.
///
/// # Errors
///
/// Returns an error if database query fails.
pub async fn search_user_bookmarks(
    pool: &SqlitePool,
    user_id: &[u8],
    search_term: &str,
    limit: i64,
    offset: i64,
) -> Result<Vec<BookmarkWithTags>> {
    let search_pattern = format!("%{search_term}%");

    let bookmarks = sqlx::query!(
        r#"
        select distinct
            b.bookmark_id,
            b.url,
            b.title,
            b.description,
            b.created_at,
            u.username as created_by
        from bookmarks b
        join users u on b.user_id = u.user_id
        left join bookmark_tags bt on b.bookmark_id = bt.bookmark_id
        left join tags t on bt.tag_id = t.tag_id
        where b.user_id = ? and b.is_archived = 0 
        and (
            b.title like ? or 
            b.description like ? or 
            b.url like ? or
            t.name like ?
        )
        order by b.created_at desc
        limit ? offset ?
        "#,
        user_id,
        search_pattern,
        search_pattern,
        search_pattern,
        search_pattern,
        limit,
        offset
    )
    .fetch_all(pool)
    .await?;

    let mut result = Vec::new();
    for bookmark in bookmarks {
        // Get tags for this bookmark
        let tags = sqlx::query!(
            r#"
            select t.name
            from bookmark_tags bt
            join tags t on bt.tag_id = t.tag_id
            where bt.bookmark_id = ?
            order by t.name
            "#,
            bookmark.bookmark_id
        )
        .fetch_all(pool)
        .await?;

        let tag_infos: Vec<TagInfo> = tags.into_iter().map(|tag| TagInfo { name: tag.name }).collect();

        let formatted_date = format_timestamp(bookmark.created_at);

        result.push(BookmarkWithTags {
            bookmark_id: bookmark.bookmark_id,
            url: bookmark.url,
            title: bookmark.title,
            description: bookmark.description,
            created_at: bookmark.created_at,
            formatted_date,
            created_by: bookmark.created_by,
            tags: tag_infos,
        });
    }

    Ok(result)
}

/// Gets count of bookmarks for pagination.
///
/// # Errors
///
/// Returns an error if database query fails.
pub async fn count_user_bookmarks(pool: &SqlitePool, user_id: &[u8]) -> Result<i64> {
    let result = sqlx::query!(
        "select count(*) as count from bookmarks where user_id = ? and is_archived = 0",
        user_id
    )
    .fetch_one(pool)
    .await?;

    Ok(result.count)
}

/// Formats a Unix timestamp into a human-readable date string.
fn format_timestamp(timestamp: i64) -> String {
    use chrono::{DateTime, Utc};

    let dt = DateTime::from_timestamp(timestamp, 0).unwrap_or_else(Utc::now);

    dt.format("%b %d, %Y").to_string()
}
