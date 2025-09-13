//! Bookmark database operations.

use std::ops::Deref;

use anyhow::Result;
use sqlx::{SqlitePool, prelude::FromRow};
use uuid::Uuid;

use crate::{
    db,
    search::{SearchLogic, SearchQuery, SearchTerm},
};

/// Represents a bookmark with its associated tags for display.
#[derive(Clone, Debug)]
pub struct BookmarkItem {
    pub url: String,
    pub title: String,
    pub created_ago: String,
    pub tags: Vec<TagInfo>,
}

#[derive(Default)]
pub struct BookmarkList(Vec<BookmarkItem>);

impl IntoIterator for BookmarkList {
    type IntoIter = std::vec::IntoIter<Self::Item>;
    type Item = BookmarkItem;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl From<BookmarkList> for Vec<BookmarkItem> {
    fn from(value: BookmarkList) -> Self {
        value.0
    }
}

impl From<Vec<BookmarkItem>> for BookmarkList {
    fn from(value: Vec<BookmarkItem>) -> Self {
        Self(value)
    }
}

impl Deref for BookmarkList {
    type Target = Vec<BookmarkItem>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Tag information for bookmarks.
#[derive(Clone, Debug)]
pub struct TagInfo {
    pub name: String,
}

impl From<&str> for TagInfo {
    fn from(value: &str) -> Self {
        Self { name: value.to_string() }
    }
}

#[derive(FromRow)]
struct BookmarkRecord {
    url: String,
    title: String,
    created_at: i64,
    tags_string: Option<String>,
}

impl From<Vec<BookmarkRecord>> for BookmarkList {
    fn from(bookmarks: Vec<BookmarkRecord>) -> Self {
        let mut result = Vec::new();
        for bookmark in bookmarks {
            let tags = bookmark.tags_string.unwrap_or_default();
            let mut tags = tags.split(',').filter(|t| !t.is_empty()).collect::<Vec<_>>();
            tags.sort_unstable();
            let tags = tags.into_iter().map(TagInfo::from).collect();

            let created_ago = get_created_ago(bookmark.created_at);

            result.push(BookmarkItem {
                url: bookmark.url,
                title: bookmark.title,
                created_ago,
                tags,
            });
        }
        Self(result)
    }
}

/// Retrieves bookmarks for a user with basic filtering.
/// Optimized to avoid N+1 queries using `GROUP_CONCAT`.
///
/// # Errors
///
/// Returns an error if database query fails.
pub async fn get_user_bookmarks(pool: &SqlitePool, user_id: Uuid, limit: i64, offset: i64) -> Result<BookmarkList> {
    let bookmarks = sqlx::query_as!(
        BookmarkRecord,
        r#"
        select
            url,
            title,
            created_at,
            tags_string
        from bookmark_with_tags
        where
            user_id = $1
            and is_archived = 0
        order by created_at desc
        limit $2 offset $3
        "#,
        user_id,
        limit,
        offset
    )
    .fetch_all(pool)
    .await?;

    Ok(bookmarks.into())
}

/// Retrieves bookmarks filtered by tag.
///
/// # Errors
///
/// Returns an error if database query fails.
pub async fn get_user_bookmarks_by_tag(pool: &SqlitePool, user_id: Uuid, tag_name: &str, limit: i64, offset: i64) -> Result<BookmarkList> {
    let bookmarks = sqlx::query_as!(
        BookmarkRecord,
        r#"
        select
            url,
            title,
            created_at,
            tags_string
        from bookmark_with_tags
        where
            user_id = $1
            and is_archived = 0
            and bookmark_id in (
                select bt.bookmark_id
                from bookmark_tags bt
                join tags t on bt.tag_id = t.tag_id
                where t.name = $2
            )
        order by created_at desc
        limit $3 offset $4
        "#,
        user_id,
        tag_name,
        limit,
        offset
    )
    .fetch_all(pool)
    .await?;

    Ok(bookmarks.into())
}

/// Searches bookmarks using advanced query parsing with OR/AND logic and phrases.
///
/// # Errors
///
/// Returns an error if database query fails.
pub async fn search_user_bookmarks_advanced(
    pool: &SqlitePool,
    user_id: Uuid,
    query: &SearchQuery,
    limit: i64,
    offset: i64,
) -> Result<BookmarkList> {
    if query.is_empty() {
        return get_user_bookmarks(pool, user_id, limit, offset).await;
    }

    // Handle tag-only queries
    if query.general_terms.is_empty() && !query.tag_filters.is_empty() {
        return search_by_tags_only(pool, user_id, &query.tag_filters, limit, offset).await;
    }

    // Handle general terms with optional tag filtering
    let base_results = match (query.logic, query.general_terms.len()) {
        (SearchLogic::Or, 1) => search_single_term(pool, user_id, &query.general_terms[0], limit, offset).await?,
        (SearchLogic::Or, 2) => search_two_terms_or(pool, user_id, &query.general_terms, limit, offset).await?,
        (SearchLogic::And, _) if query.general_terms.len() >= 2 => {
            search_multiple_terms_and(pool, user_id, &query.general_terms, limit, offset).await?
        }
        // For more complex queries, fall back to the original search
        _ => {
            tracing::warn!("Complex search query not fully optimized, falling back to simple search");
            let search_term = query
                .general_terms
                .iter()
                .map(std::string::ToString::to_string)
                .collect::<Vec<_>>()
                .join(" ");
            // TODO: this is bullshit
            search_user_bookmarks(pool, user_id, &search_term, limit, offset).await?
        }
    };

    // Filter by tags if any are specified
    if query.tag_filters.is_empty() {
        Ok(base_results)
    } else {
        Ok(filter_bookmarks_by_tags(base_results, &query.tag_filters))
    }
}

/// Searches bookmarks by title, description, URL, or tags (legacy function).
///
/// # Errors
///
/// Returns an error if database query fails.
pub async fn search_user_bookmarks(pool: &SqlitePool, user_id: Uuid, search_term: &str, limit: i64, offset: i64) -> Result<BookmarkList> {
    let search_pattern = format!("%{search_term}%");

    let bookmarks = sqlx::query_as!(
        BookmarkRecord,
        r#"
        select
            url,
            title,
            created_at,
            tags_string
        from bookmark_with_tags bwt
        where user_id = $1
        and (
            title like $2 or
            description like $3 or
            url like $4 or
            exists (
                select 1 from bookmark_tags bt
                join tags t on bt.tag_id = t.tag_id
                where bt.bookmark_id = bwt.bookmark_id
                and t.name like $5
            )
        )
        order by created_at desc
        limit $6 offset $7
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

    Ok(bookmarks.into())
}

/// Searches for bookmarks with a single search term.
async fn search_single_term(pool: &SqlitePool, user_id: Uuid, term: &SearchTerm, limit: i64, offset: i64) -> Result<BookmarkList> {
    match term {
        SearchTerm::Word(word) => search_single_word(pool, user_id, word, limit, offset).await,
        SearchTerm::Phrase(phrase) => search_single_phrase(pool, user_id, phrase, limit, offset).await,
    }
}

async fn search_single_word(pool: &SqlitePool, user_id: Uuid, word: &str, limit: i64, offset: i64) -> Result<BookmarkList> {
    let search_pattern = format!("%{word}%");
    let res = sqlx::query_as!(
        BookmarkRecord,
        r#"
        select
            url,
            title,
            created_at,
            tags_string
        from bookmark_with_tags bwt
        where user_id = $1
        and (
            title like $2
            or description like $3
            or url like $4
            or exists (
                select 1 from bookmark_tags bt
                join tags t on bt.tag_id = t.tag_id
                where
                    bt.bookmark_id = bwt.bookmark_id
                    and t.name like $5
            )
        )
        order by created_at desc
        limit $6 offset $7
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

    Ok(res.into())
}

async fn search_single_phrase(pool: &SqlitePool, user_id: Uuid, phrase: &str, limit: i64, offset: i64) -> Result<BookmarkList> {
    let res = sqlx::query_as!(
        BookmarkRecord,
        r#"
        select
            url,
            title,
            created_at,
            tags_string
        from bookmark_with_tags bwt
        where
            user_id = ?
            and is_archived = 0
            and (
                instr(title, ?) > 0
                or instr(description, ?) > 0
                or instr(url, ?) > 0
                or exists (
                    select 1 from bookmark_tags bt
                    join tags t on bt.tag_id = t.tag_id
                    where
                        bt.bookmark_id = bwt.bookmark_id
                        and instr(t.name, ?) > 0
                )
            )
        order by created_at desc
        limit ? offset ?
        "#,
        user_id,
        phrase,
        phrase,
        phrase,
        phrase,
        limit,
        offset
    )
    .fetch_all(pool)
    .await?;

    Ok(res.into())
}

/// Searches for bookmarks with two terms using OR logic.
async fn search_two_terms_or(pool: &SqlitePool, user_id: Uuid, terms: &[SearchTerm], limit: i64, offset: i64) -> Result<BookmarkList> {
    // Build the WHERE conditions based on term types
    let condition1 = match &terms[0] {
        SearchTerm::Word(_) => "(b.title like ? or b.description like ? or b.url like ? or t_search.name like ?)",
        SearchTerm::Phrase(_) => {
            "(instr(b.title, ?) > 0 or instr(b.description, ?) > 0 or instr(b.url, ?) > 0 or instr(t_search.name, ?) > 0)"
        }
    };
    let condition2 = match &terms[1] {
        SearchTerm::Word(_) => "(b.title like ? or b.description like ? or b.url like ? or t_search.name like ?)",
        SearchTerm::Phrase(_) => {
            "(instr(b.title, ?) > 0 or instr(b.description, ?) > 0 or instr(b.url, ?) > 0 or instr(t_search.name, ?) > 0)"
        }
    };

    let pattern1 = match &terms[0] {
        SearchTerm::Word(word) => format!("%{word}%"),
        SearchTerm::Phrase(phrase) => phrase.clone(),
    };
    let pattern2 = match &terms[1] {
        SearchTerm::Word(word) => format!("%{word}%"),
        SearchTerm::Phrase(phrase) => phrase.clone(),
    };

    let sql = format!(
        r"
        select
            b.url,
            b.title,
            b.created_at,
            GROUP_CONCAT(distinct t2.name) as tags_string
        from bookmarks b
        left join bookmark_tags bt_search on b.bookmark_id = bt_search.bookmark_id
        left join tags t_search on bt_search.tag_id = t_search.tag_id
        left join bookmark_tags bt on b.bookmark_id = bt.bookmark_id
        left join tags t2 on bt.tag_id = t2.tag_id
        where b.user_id = ? and b.is_archived = 0
        and ({condition1} or {condition2})
        group by b.bookmark_id, b.url, b.title, b.created_at
        order by b.created_at desc
        limit ? offset ?
        "
    );

    // TODO: querybuilder instead?
    // TODO: query_as for all these
    let bookmarks: Vec<BookmarkRecord> = sqlx::query_as(&sql)
        .bind(user_id)
        .bind(&pattern1)
        .bind(&pattern1)
        .bind(&pattern1)
        .bind(&pattern1)
        .bind(&pattern2)
        .bind(&pattern2)
        .bind(&pattern2)
        .bind(&pattern2)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;

    Ok(bookmarks.into())
}

/// Searches for bookmarks with multiple terms using AND logic.
async fn search_multiple_terms_and(
    pool: &SqlitePool,
    user_id: Uuid,
    terms: &[SearchTerm],
    limit: i64,
    offset: i64,
) -> Result<BookmarkList> {
    if terms.is_empty() {
        return get_user_bookmarks(pool, user_id, limit, offset).await;
    }

    // Build conditions for each term
    let mut bookmark_conditions = Vec::new();
    let mut tag_conditions = Vec::new();
    let mut patterns = Vec::new();

    for term in terms {
        let (bookmark_cond, tag_cond, pattern) = match term {
            SearchTerm::Word(word) => (
                "(b.title like ? or b.description like ? or b.url like ?)",
                "t.name like ?",
                format!("%{word}%"),
            ),
            SearchTerm::Phrase(phrase) => (
                "(instr(b.title, ?) > 0 or instr(b.description, ?) > 0 or instr(b.url, ?) > 0)",
                "instr(t.name, ?) > 0",
                phrase.clone(),
            ),
        };
        bookmark_conditions.push(bookmark_cond);
        tag_conditions.push(tag_cond);
        patterns.push(pattern);
    }

    // Build the AND clauses using EXISTS subqueries
    let mut and_clauses = Vec::new();
    for i in 0..terms.len() {
        let clause = format!(
            "({} or exists (select 1 from bookmark_tags bt{} join tags t on bt{}.tag_id = t.tag_id where bt{}.bookmark_id = b.bookmark_id and {}))",
            bookmark_conditions[i], i, i, i, tag_conditions[i]
        );
        and_clauses.push(clause);
    }

    let sql = format!(
        r"
        select
            b.url,
            b.title,
            b.created_at,
            GROUP_CONCAT(distinct t_result.name) as tags_string
        from bookmarks b
        left join bookmark_tags bt on b.bookmark_id = bt.bookmark_id
        left join tags t_result on bt.tag_id = t_result.tag_id
        where b.user_id = ? and b.is_archived = 0
        and {}
        group by b.bookmark_id, b.url, b.title, b.created_at
        order by b.created_at desc
        limit ? offset ?
        ",
        and_clauses.join(" and ")
    );

    let mut query_builder = sqlx::query_as(&sql).bind(user_id);

    // Bind parameters for each term (4 binds per term: 3 for bookmark fields + 1 for tag)
    for pattern in &patterns {
        query_builder = query_builder
            .bind(pattern) // bookmark title
            .bind(pattern) // bookmark description
            .bind(pattern) // bookmark url
            .bind(pattern); // tag name
    }

    query_builder = query_builder.bind(limit).bind(offset);

    let bookmarks: Vec<BookmarkRecord> = query_builder.fetch_all(pool).await?;

    Ok(bookmarks.into())
}

/// Searches bookmarks by tags only (no general search terms).
pub async fn search_by_tags_only(pool: &SqlitePool, user_id: Uuid, tag_names: &[String], limit: i64, offset: i64) -> Result<BookmarkList> {
    if tag_names.is_empty() {
        return get_user_bookmarks(pool, user_id, limit, offset).await;
    }

    // For single tag, use the existing optimized function
    if tag_names.len() == 1 {
        return get_user_bookmarks_by_tag(pool, user_id, &tag_names[0], limit, offset).await;
    }

    // For multiple tags, find bookmarks that have ALL specified tags (using LIKE for fuzzy matching)
    let like_conditions = tag_names.iter().map(|_| "t.name like ?").collect::<Vec<_>>().join(" OR ");
    let sql = format!(
        r"
        select
            b.url,
            b.title,
            b.created_at,
            GROUP_CONCAT(distinct t_result.name) as tags_string
        from bookmarks b
        left join bookmark_tags bt_result on b.bookmark_id = bt_result.bookmark_id
        left join tags t_result on bt_result.tag_id = t_result.tag_id
        where b.user_id = ? and b.is_archived = 0
        and b.bookmark_id in (
            select bt.bookmark_id
            from bookmark_tags bt
            join tags t on bt.tag_id = t.tag_id
            where ({like_conditions})
            group by bt.bookmark_id
            having count(distinct t.tag_id) >= ?
        )
        group by b.bookmark_id, b.url, b.title, b.created_at
        order by b.created_at desc
        limit ? offset ?
        "
    );

    let mut query = sqlx::query_as(&sql).bind(user_id);
    for tag_name in tag_names {
        let tag_pattern = format!("%{tag_name}%");
        query = query.bind(tag_pattern);
    }
    query = query.bind(i64::try_from(tag_names.len()).unwrap_or(0)).bind(limit).bind(offset);

    let bookmarks: Vec<BookmarkRecord> = query.fetch_all(pool).await?;

    Ok(bookmarks.into())
}

/// Filters bookmark results to only include those with all specified tags (fuzzy matching).
fn filter_bookmarks_by_tags(bookmarks: BookmarkList, required_tags: &[String]) -> BookmarkList {
    if required_tags.is_empty() {
        return bookmarks;
    }

    bookmarks
        .into_iter()
        .filter(|bookmark| {
            // Check if bookmark has all required tags using fuzzy LIKE matching
            required_tags.iter().all(|required_tag| {
                bookmark
                    .tags
                    .iter()
                    .any(|tag| tag.name.to_lowercase().contains(&required_tag.to_lowercase()))
            })
        })
        .collect::<Vec<_>>()
        .into()
}

/// Formats a Unix timestamp into a human-readable "time ago" string.
fn get_created_ago(timestamp: i64) -> String {
    use chrono::{DateTime, Utc};

    let dt = DateTime::from_timestamp(timestamp, 0).unwrap_or_else(Utc::now);
    let now = Utc::now();
    let duration = now.signed_duration_since(dt);

    let total_seconds = duration.num_seconds();
    if total_seconds < 0 {
        return "now".to_string();
    }

    let days = total_seconds / 86400; // 60 * 60 * 24
    if days >= 1 {
        return if days == 1 {
            // unpluralized
            "1 day ago".to_string()
        } else {
            format!("{days} days ago")
        };
    }

    let hours = total_seconds / 3600; // 60 * 60
    if hours >= 1 {
        return if hours == 1 {
            // unpluralized
            "1 hour ago".to_string()
        } else {
            format!("{hours} hours ago")
        };
    }

    let minutes = total_seconds / 60;
    if minutes >= 1 {
        return if minutes == 1 {
            // unpluralized
            "1 minute ago".to_string()
        } else {
            format!("{minutes} minutes ago")
        };
    }

    "now".to_string()
}

/// Creates a new bookmark for a user.
///
/// # Errors
///
/// Returns an error if database operations fail.
pub async fn create_bookmark(
    pool: &SqlitePool,
    user_id: Uuid,
    url: &str,
    title: &str,
    description: Option<&str>,
    tag_names: &[String],
) -> Result<Vec<u8>> {
    // Begin transaction to ensure atomicity
    let mut tx = pool.begin().await?;

    // Insert bookmark and get the generated bookmark_id
    let bookmark_result = sqlx::query!(
        r#"
        insert into bookmarks (user_id, url, title, description)
        values (?, ?, ?, ?)
        returning bookmark_id
        "#,
        user_id,
        url,
        title,
        description
    )
    .fetch_one(&mut *tx)
    .await?;

    let bookmark_id = &bookmark_result.bookmark_id;

    // Process and link tags
    for tag_name in tag_names {
        if tag_name.trim().is_empty() {
            continue;
        }

        // Get or create the tag within the transaction
        let tag_id = db::tags::get_or_create_tag(&mut tx, tag_name).await?;

        // Link bookmark to tag
        sqlx::query!("insert into bookmark_tags (bookmark_id, tag_id) values (?, ?)", bookmark_id, tag_id)
            .execute(&mut *tx)
            .await?;
    }

    // Commit the transaction
    tx.commit().await?;

    Ok(bookmark_id.clone())
}
