//! Bookmark database operations.

use anyhow::Result;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::search::{SearchLogic, SearchQuery, SearchTerm};

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

/// Raw bookmark result from database queries (without tags).
#[derive(Debug, sqlx::FromRow)]
struct BookmarkQueryResult {
    bookmark_id: Vec<u8>,
    url: String,
    title: String,
    description: Option<String>,
    created_at: i64,
    created_by: String,
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
pub async fn get_user_bookmarks(pool: &SqlitePool, user_id: Uuid, limit: i64, offset: i64) -> Result<Vec<BookmarkWithTags>> {
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
    user_id: Uuid,
    tag_name: &str,
    limit: i64,
    offset: i64,
) -> Result<Vec<BookmarkWithTags>> {
    let tag_pattern = format!("%{tag_name}%");
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
        where b.user_id = $1 and b.is_archived = 0 and t.name like $2
        order by b.created_at desc
        limit $3 offset $4
        "#,
        user_id,
        tag_pattern,
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
) -> Result<Vec<BookmarkWithTags>> {
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
                .map(|term| term.to_string())
                .collect::<Vec<_>>()
                .join(" ");
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
pub async fn search_user_bookmarks(
    pool: &SqlitePool,
    user_id: Uuid,
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
pub async fn count_user_bookmarks(pool: &SqlitePool, user_id: Uuid) -> Result<i64> {
    let result = sqlx::query!(
        "select count(*) as count from bookmarks where user_id = ? and is_archived = 0",
        user_id
    )
    .fetch_one(pool)
    .await?;

    Ok(result.count)
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
    .fetch_one(pool)
    .await?;

    let bookmark_id = &bookmark_result.bookmark_id;

    // Process and link tags
    for tag_name in tag_names {
        if tag_name.trim().is_empty() {
            continue;
        }

        // Get or create the tag
        let tag_id = crate::db::tags::get_or_create_tag(pool, tag_name, None).await?;

        // Link bookmark to tag
        sqlx::query!("insert into bookmark_tags (bookmark_id, tag_id) values (?, ?)", bookmark_id, tag_id)
            .execute(pool)
            .await?;
    }

    Ok(bookmark_id.clone())
}

/// Searches for bookmarks with a single search term.
async fn search_single_term(pool: &SqlitePool, user_id: Uuid, term: &SearchTerm, limit: i64, offset: i64) -> Result<Vec<BookmarkWithTags>> {
    let bookmarks = match term {
        SearchTerm::Word(word) => {
            let search_pattern = format!("%{word}%");
            sqlx::query_as!(
                BookmarkQueryResult,
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
            .await?
        }
        SearchTerm::Phrase(phrase) => {
            sqlx::query_as!(
                BookmarkQueryResult,
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
                    instr(b.title, ?) > 0 or
                    instr(b.description, ?) > 0 or
                    instr(b.url, ?) > 0 or
                    instr(t.name, ?) > 0
                )
                order by b.created_at desc
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
            .await?
        }
    };

    build_bookmark_results(pool, bookmarks).await
}

/// Searches for bookmarks with two terms using OR logic.
async fn search_two_terms_or(
    pool: &SqlitePool,
    user_id: Uuid,
    terms: &[SearchTerm],
    limit: i64,
    offset: i64,
) -> Result<Vec<BookmarkWithTags>> {
    // Build the WHERE conditions based on term types
    let condition1 = match &terms[0] {
        SearchTerm::Word(_) => "(b.title like ? or b.description like ? or b.url like ? or t.name like ?)",
        SearchTerm::Phrase(_) => "(instr(b.title, ?) > 0 or instr(b.description, ?) > 0 or instr(b.url, ?) > 0 or instr(t.name, ?) > 0)",
    };
    let condition2 = match &terms[1] {
        SearchTerm::Word(_) => "(b.title like ? or b.description like ? or b.url like ? or t.name like ?)",
        SearchTerm::Phrase(_) => "(instr(b.title, ?) > 0 or instr(b.description, ?) > 0 or instr(b.url, ?) > 0 or instr(t.name, ?) > 0)",
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
        and ({} or {})
        order by b.created_at desc
        limit ? offset ?
        "#,
        condition1, condition2
    );

    let bookmarks = sqlx::query(&sql)
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

    let bookmarks: Vec<BookmarkQueryResult> = bookmarks
        .into_iter()
        .map(|row| BookmarkQueryResult {
            bookmark_id: row.get("bookmark_id"),
            url: row.get("url"),
            title: row.get("title"),
            description: row.get("description"),
            created_at: row.get("created_at"),
            created_by: row.get("created_by"),
        })
        .collect();

    build_bookmark_results(pool, bookmarks).await
}

/// Searches for bookmarks with two terms using AND logic.
async fn search_two_terms_and(
    pool: &SqlitePool,
    user_id: Uuid,
    terms: &[SearchTerm],
    limit: i64,
    offset: i64,
) -> Result<Vec<BookmarkWithTags>> {
    // Build the bookmark field conditions
    let (bookmark_condition1, bookmark_condition2) = match (&terms[0], &terms[1]) {
        (SearchTerm::Word(_), SearchTerm::Word(_)) => (
            "(b.title like ? or b.description like ? or b.url like ?)",
            "(b.title like ? or b.description like ? or b.url like ?)",
        ),
        (SearchTerm::Word(_), SearchTerm::Phrase(_)) => (
            "(b.title like ? or b.description like ? or b.url like ?)",
            "(instr(b.title, ?) > 0 or instr(b.description, ?) > 0 or instr(b.url, ?) > 0)",
        ),
        (SearchTerm::Phrase(_), SearchTerm::Word(_)) => (
            "(instr(b.title, ?) > 0 or instr(b.description, ?) > 0 or instr(b.url, ?) > 0)",
            "(b.title like ? or b.description like ? or b.url like ?)",
        ),
        (SearchTerm::Phrase(_), SearchTerm::Phrase(_)) => (
            "(instr(b.title, ?) > 0 or instr(b.description, ?) > 0 or instr(b.url, ?) > 0)",
            "(instr(b.title, ?) > 0 or instr(b.description, ?) > 0 or instr(b.url, ?) > 0)",
        ),
    };

    // Build the tag conditions
    let (tag_condition1, tag_condition2) = match (&terms[0], &terms[1]) {
        (SearchTerm::Word(_), SearchTerm::Word(_)) => ("t.name like ?", "t.name like ?"),
        (SearchTerm::Word(_), SearchTerm::Phrase(_)) => ("t.name like ?", "instr(t.name, ?) > 0"),
        (SearchTerm::Phrase(_), SearchTerm::Word(_)) => ("instr(t.name, ?) > 0", "t.name like ?"),
        (SearchTerm::Phrase(_), SearchTerm::Phrase(_)) => ("instr(t.name, ?) > 0", "instr(t.name, ?) > 0"),
    };

    let pattern1 = match &terms[0] {
        SearchTerm::Word(word) => format!("%{word}%"),
        SearchTerm::Phrase(phrase) => phrase.clone(),
    };
    let pattern2 = match &terms[1] {
        SearchTerm::Word(word) => format!("%{word}%"),
        SearchTerm::Phrase(phrase) => phrase.clone(),
    };

    // Use EXISTS subqueries to properly handle AND logic across multiple tags
    let sql = format!(
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
        where b.user_id = ? and b.is_archived = 0
        and (
            {} or exists (
                select 1 from bookmark_tags bt1 
                join tags t on bt1.tag_id = t.tag_id 
                where bt1.bookmark_id = b.bookmark_id and {}
            )
        )
        and (
            {} or exists (
                select 1 from bookmark_tags bt2 
                join tags t on bt2.tag_id = t.tag_id 
                where bt2.bookmark_id = b.bookmark_id and {}
            )
        )
        order by b.created_at desc
        limit ? offset ?
        "#,
        bookmark_condition1, tag_condition1, bookmark_condition2, tag_condition2
    );

    let bookmarks = sqlx::query(&sql)
        .bind(user_id)
        // First condition - bookmark fields
        .bind(&pattern1)
        .bind(&pattern1)
        .bind(&pattern1)
        // First condition - tag exists subquery
        .bind(&pattern1)
        // Second condition - bookmark fields
        .bind(&pattern2)
        .bind(&pattern2)
        .bind(&pattern2)
        // Second condition - tag exists subquery
        .bind(&pattern2)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;

    let bookmarks: Vec<BookmarkQueryResult> = bookmarks
        .into_iter()
        .map(|row| BookmarkQueryResult {
            bookmark_id: row.get("bookmark_id"),
            url: row.get("url"),
            title: row.get("title"),
            description: row.get("description"),
            created_at: row.get("created_at"),
            created_by: row.get("created_by"),
        })
        .collect();

    build_bookmark_results(pool, bookmarks).await
}

/// Searches for bookmarks with multiple terms using AND logic.
async fn search_multiple_terms_and(
    pool: &SqlitePool,
    user_id: Uuid,
    terms: &[SearchTerm],
    limit: i64,
    offset: i64,
) -> Result<Vec<BookmarkWithTags>> {
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
        where b.user_id = ? and b.is_archived = 0
        and {}
        order by b.created_at desc
        limit ? offset ?
        "#,
        and_clauses.join(" and ")
    );

    let mut query_builder = sqlx::query(&sql).bind(user_id);

    // Bind parameters for each term (4 binds per term: 3 for bookmark fields + 1 for tag)
    for pattern in &patterns {
        query_builder = query_builder
            .bind(pattern) // bookmark title
            .bind(pattern) // bookmark description
            .bind(pattern) // bookmark url
            .bind(pattern); // tag name
    }

    query_builder = query_builder.bind(limit).bind(offset);

    let bookmarks = query_builder.fetch_all(pool).await?;

    let bookmarks: Vec<BookmarkQueryResult> = bookmarks
        .into_iter()
        .map(|row| BookmarkQueryResult {
            bookmark_id: row.get("bookmark_id"),
            url: row.get("url"),
            title: row.get("title"),
            description: row.get("description"),
            created_at: row.get("created_at"),
            created_by: row.get("created_by"),
        })
        .collect();

    build_bookmark_results(pool, bookmarks).await
}

/// Helper function to build BookmarkWithTags results from query results.
async fn build_bookmark_results(pool: &SqlitePool, bookmarks: Vec<BookmarkQueryResult>) -> Result<Vec<BookmarkWithTags>> {
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

/// Searches bookmarks by tags only (no general search terms).
pub async fn search_by_tags_only(
    pool: &SqlitePool,
    user_id: Uuid,
    tag_names: &[String],
    limit: i64,
    offset: i64,
) -> Result<Vec<BookmarkWithTags>> {
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
        where b.user_id = ? and b.is_archived = 0
        and b.bookmark_id in (
            select bt.bookmark_id
            from bookmark_tags bt
            join tags t on bt.tag_id = t.tag_id
            where ({})
            group by bt.bookmark_id
            having count(distinct t.tag_id) >= ?
        )
        order by b.created_at desc
        limit ? offset ?
        "#,
        like_conditions
    );

    let mut query = sqlx::query(&sql).bind(user_id);
    for tag_name in tag_names {
        let tag_pattern = format!("%{tag_name}%");
        query = query.bind(tag_pattern);
    }
    query = query.bind(tag_names.len() as i64).bind(limit).bind(offset);

    let rows = query.fetch_all(pool).await?;

    let bookmarks: Vec<BookmarkQueryResult> = rows
        .into_iter()
        .map(|row| BookmarkQueryResult {
            bookmark_id: row.get("bookmark_id"),
            url: row.get("url"),
            title: row.get("title"),
            description: row.get("description"),
            created_at: row.get("created_at"),
            created_by: row.get("created_by"),
        })
        .collect();

    build_bookmark_results(pool, bookmarks).await
}

/// Filters bookmark results to only include those with all specified tags (fuzzy matching).
fn filter_bookmarks_by_tags(bookmarks: Vec<BookmarkWithTags>, required_tags: &[String]) -> Vec<BookmarkWithTags> {
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
        .collect()
}

/// Formats a Unix timestamp into a human-readable date string.
fn format_timestamp(timestamp: i64) -> String {
    use chrono::{DateTime, Utc};

    let dt = DateTime::from_timestamp(timestamp, 0).unwrap_or_else(Utc::now);

    dt.format("%b %d, %Y").to_string()
}
