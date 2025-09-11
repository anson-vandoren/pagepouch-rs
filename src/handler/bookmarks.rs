//! Bookmark-related handlers and templates.

use askama::Template;
use axum::{Extension, Form, extract::State, http::StatusCode, response::IntoResponse};
use axum_extra::extract::Query;
use serde::Deserialize;
use tracing::{debug, error, warn};

use crate::{
    ApiState,
    db::{bookmarks, users::User},
    handler::{AuthState, HomeTemplate, HtmlTemplate, Toast, Toasts},
    search::SearchQuery,
};

// Template data structures for display
#[derive(Clone)]
pub struct Bookmark {
    pub url: String,
    pub title: String,
    pub tags: Vec<super::tags::Tag>,
    pub created_by: String,
    pub created_at: String,
    pub formatted_date: String,
}

#[derive(Clone)]
pub struct Pagination {
    pub has_prev: bool,
    pub has_next: bool,
    pub page_links: Vec<PageLink>,
}

#[derive(Clone)]
pub struct PageLink {
    pub number: i64,
    pub is_current: bool,
    pub is_ellipsis: bool,
}

#[derive(Template)]
#[template(path = "components/bookmark_content.html")]
pub struct BookmarkContentTemplate {
    pub bookmarks: Vec<Bookmark>,
    pub pagination: Option<Pagination>,
}

#[derive(Debug, Deserialize)]
pub struct BookmarkQuery {
    pub q: Option<String>, // Search query
    /// Filter by tags - complete/committed tags will not be part of the `q`, only partial tags that need
    /// auto-complete, or invalid tags (non-existing tags followed by whitespace) which should be ignored
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    pub page: Option<i64>, // Page number
}

const DEFAULT_LIMIT: i64 = 20;
/// API handler for bookmark content (HTMX lazy loading)
pub async fn bookmark_content_handler(
    State(state): ApiState,
    Extension(user): Extension<User>,
    Query(params): Query<BookmarkQuery>,
) -> impl IntoResponse {
    debug!(?params, "Bookmark content handler queried");

    let page = params.page.unwrap_or(1);
    let offset = (page - 1) * DEFAULT_LIMIT;

    // Parse search query to extract tags and determine search type
    let tags: Vec<String> = params.tags.unwrap_or_default();
    let db_bookmarks = if !tags.is_empty() {
        // TODO: we should be able to search by both tags and regular query
        // Committed tags from new tag completion system
        bookmarks::search_by_tags_only(&state.pool, user.user_id, &tags, DEFAULT_LIMIT, offset)
            .await
            .unwrap_or_default()
    } else if let Some(ref search_query_str) = params.q {
        // Parse the search query and use advanced search
        let search_query = SearchQuery::parse(search_query_str);
        debug!("Parsed search query: {:?}", search_query);

        bookmarks::search_user_bookmarks_advanced(&state.pool, user.user_id, &search_query, DEFAULT_LIMIT, offset)
            .await
            .unwrap_or_default()
    } else {
        // No filters
        bookmarks::get_user_bookmarks(&state.pool, user.user_id, DEFAULT_LIMIT, offset)
            .await
            .unwrap_or_default()
    };

    // Convert database results to template format
    let template_bookmarks: Vec<Bookmark> = db_bookmarks
        .into_iter()
        .map(|db_bookmark| {
            let tags: Vec<super::tags::Tag> = db_bookmark
                .tags
                .into_iter()
                .map(|tag_info| super::tags::Tag { name: tag_info.name })
                .collect();

            Bookmark {
                url: db_bookmark.url,
                title: db_bookmark.title,
                tags,
                created_by: db_bookmark.created_by,
                created_at: db_bookmark.created_at.to_string(),
                formatted_date: db_bookmark.formatted_date,
            }
        })
        .collect();

    // TODO: Implement proper pagination based on total count
    let pagination = if i64::try_from(template_bookmarks.len()).unwrap_or(0) == DEFAULT_LIMIT {
        Some(Pagination {
            has_prev: page > 1,
            has_next: true, // Assume there might be more
            page_links: vec![PageLink {
                number: page,
                is_current: true,
                is_ellipsis: false,
            }],
        })
    } else {
        None // No pagination needed
    };

    HtmlTemplate(BookmarkContentTemplate {
        bookmarks: template_bookmarks,
        pagination,
    })
}

#[derive(Template)]
#[template(path = "pages/bookmarks_new.html")]
pub struct BookmarkNewTemplate<'a> {
    pub title: &'a str,
    pub toasts: Toasts,
    pub auth_state: AuthState,
    pub is_error: bool,
}

#[derive(Deserialize)]
pub struct BookmarkForm {
    pub url: String,
    pub title: String,
    pub description: Option<String>,
    pub tags: Option<String>,
}

#[derive(Deserialize)]
pub struct FetchTitleRequest {
    pub url: String,
}

#[derive(Template)]
#[template(
    source = r#"<input type="text" id="title" name="title" required value="{{ title }}">"#,
    ext = "html"
)]
pub struct TitleInputTemplate {
    pub title: String,
}

/// Handler for displaying the bookmark creation form
pub async fn bookmark_new_handler() -> impl IntoResponse {
    HtmlTemplate(BookmarkNewTemplate {
        title: "Add Bookmark",
        toasts: Toasts::default(),
        auth_state: AuthState::Authenticated,
        is_error: false,
    })
}

/// Handler for creating a new bookmark
pub async fn bookmark_create_handler(
    State(state): ApiState,
    Extension(user): Extension<User>,
    Form(form): Form<BookmarkForm>,
) -> impl IntoResponse {
    // Parse tags from comma-separated string
    let tag_names: Vec<String> = form
        .tags
        .as_ref()
        .map(|tags| {
            tags.split(',')
                .map(|tag| tag.trim().to_lowercase())
                .filter(|tag| !tag.is_empty())
                .collect()
        })
        .unwrap_or_default();

    // Create the bookmark in the database
    match bookmarks::create_bookmark(
        &state.pool,
        user.user_id,
        &form.url,
        &form.title,
        form.description.as_deref(),
        &tag_names,
    )
    .await
    {
        Ok(_bookmark_id) => {
            // Return home page with success toast instead of redirect
            let success_toast = Toast {
                is_success: true,
                message: format!("✨ Bookmark \"{}\" saved successfully!", form.title),
            };

            HtmlTemplate(HomeTemplate {
                title: "Home",
                auth_state: AuthState::Authenticated,
                toasts: vec![success_toast],
                is_error: false,
            })
            .into_response()
        }
        Err(err) => {
            error!("🚨 Failed to create bookmark: {}", err);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to create bookmark").into_response()
        }
    }
}

/// Handler for fetching page title from URL
pub async fn fetch_title_handler(Form(request): Form<FetchTitleRequest>) -> impl IntoResponse {
    match fetch_page_title(&request.url).await {
        Ok(title) => {
            debug!(url = request.url, title, "Fetched title");
            HtmlTemplate(TitleInputTemplate { title })
        }
        Err(err) => {
            warn!("🌐 Failed to fetch title for {}: {}", request.url, err);
            // Return the URL as fallback title
            let fallback_title = extract_domain_from_url(&request.url).unwrap_or_else(|| request.url.clone());
            HtmlTemplate(TitleInputTemplate { title: fallback_title })
        }
    }
}

/// Fetches the title from a webpage.
///
/// # Errors
///
/// Returns an error if the HTTP request fails or HTML parsing fails.
async fn fetch_page_title(url: &str) -> anyhow::Result<String> {
    debug!(url, "Starting title fetch");
    // Create HTTP client with timeout
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .user_agent("PagePouch/1.0")
        .build()?;

    // Fetch the page
    let response = client.get(url).send().await?;

    // Check if response is successful
    if !response.status().is_success() {
        return Err(anyhow::anyhow!("HTTP error: {}", response.status()));
    }

    let html = response.text().await?;

    // Parse HTML and extract title
    let dom = tl::parse(&html, tl::ParserOptions::default())?;
    let title = dom
        .query_selector("title")
        .and_then(|mut iter| iter.next())
        .and_then(|node| node.get(dom.parser()))
        .map(|node| decode_html_entities(&node.inner_text(dom.parser())).trim().to_string())
        .filter(|title| !title.is_empty())
        .unwrap_or_else(|| extract_domain_from_url(url).unwrap_or_else(|| url.to_string()));

    debug!(url, title, "Finished title fetch");
    Ok(title)
}

/// Extracts domain name from URL as fallback title.
fn extract_domain_from_url(url: &str) -> Option<String> {
    url.strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .and_then(|rest| rest.split('/').next())
        .map(std::string::ToString::to_string)
}

/// Decodes common HTML entities in text.
fn decode_html_entities(text: &str) -> String {
    text.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
}
