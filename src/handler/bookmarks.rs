//! Bookmark-related handlers and templates.

use askama::Template;
use axum::{Extension, Form, Json, extract::State, http::StatusCode, response::IntoResponse};
use axum_extra::extract::Query;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sqlx::decode;
use tl::VDom;
use tracing::{debug, error, warn};

use crate::{
    ApiState,
    db::{
        bookmarks::{self, BookmarkItem},
        users::User,
    },
    handler::{AuthState, HomeTemplate, HtmlTemplate},
    search::SearchQuery,
};

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
    pub bookmarks: Vec<BookmarkItem>,
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
    // TODO: Implement proper pagination based on total count
    let pagination = if i64::try_from(db_bookmarks.len()).unwrap_or(0) == DEFAULT_LIMIT {
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
        bookmarks: db_bookmarks.into(),
        pagination,
    })
}

#[derive(Template)]
#[template(path = "pages/bookmarks_new.html")]
pub struct BookmarkNewTemplate<'a> {
    pub title: &'a str,
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

#[derive(Serialize)]
pub struct FetchTitleResponse {
    pub title: Option<String>,
    pub description: Option<String>,
    pub corrected_url: String,
}

/// Handler for displaying the bookmark creation form
pub async fn bookmark_new_handler() -> impl IntoResponse {
    HtmlTemplate(BookmarkNewTemplate {
        title: "Add Bookmark",
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
        Ok(_bookmark_id) => HtmlTemplate(HomeTemplate {
            title: "Home",
            auth_state: AuthState::Authenticated,
            is_error: false,
        })
        .into_response(),
        Err(err) => {
            error!("🚨 Failed to create bookmark: {}", err);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to create bookmark").into_response()
        }
    }
}

/// Handler for fetching page title & description from URL
pub async fn scrape_site_handler(State(state): ApiState, Form(request): Form<FetchTitleRequest>) -> impl IntoResponse {
    if request.url.len() < 3 {
        return Json(FetchTitleResponse {
            title: None,
            description: None,
            corrected_url: request.url,
        });
    }

    match scrape_title_description(&state.http_client, &request.url).await {
        Ok(result) => {
            let LinkScrapeResult {
                description,
                title,
                final_url,
            } = result;
            debug!(final_url, title, description, "Scraped input site.");
            Json(FetchTitleResponse {
                title: Some(title),
                description,
                corrected_url: final_url,
            })
        }
        Err(err) => {
            warn!(url = request.url, %err, "🌐 Failed to fetch title");
            // Return whatever we can - at minimum the corrected URL
            Json(FetchTitleResponse {
                title: None,
                description: None,
                corrected_url: request.url,
            })
        }
    }
}

#[derive(Debug)]
struct LinkScrapeResult {
    description: Option<String>,
    title: String,
    final_url: String,
}

/// Fetches the title from a webpage
///
/// # Errors
///
/// Returns an error if the HTTP request fails or HTML parsing fails.
async fn scrape_title_description(client: &Client, url: &str) -> anyhow::Result<LinkScrapeResult> {
    let default_title = url.to_string();
    let mut url = match url {
        url if url.starts_with("http://") || url.starts_with("https://") => url.to_string(),
        no_proto => format!("http://{no_proto}"),
    };

    // Fetch the page
    let mut response = client.get(&url).send().await?;

    // Check if response is successful
    if !response.status().is_success() {
        // If not, try https instead
        debug!("Fetch failed with https, trying http");
        url = url.replace("https://", "http://");
        response = client.get(&url).send().await?;
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("HTTP error: {}", response.status()));
        }
    }

    let html = response.text().await?;

    // Parse HTML and extract title
    let dom = tl::parse(&html, tl::ParserOptions::default())?;
    let parser = dom.parser();
    let title = dom
        .query_selector("title")
        .and_then(|mut iter| iter.next())
        .and_then(|node| node.get(parser))
        .map(|node| decode_html_entities(&node.inner_text(parser)).trim().to_string())
        .filter(|title| !title.is_empty())
        .unwrap_or(default_title);

    let description = get_meta_description(&dom);

    Ok(LinkScrapeResult {
        description,
        title,
        final_url: url,
    })
}

fn get_meta_description(dom: &VDom<'_>) -> Option<String> {
    let parser = dom.parser();

    // Try Open Graph description first
    if let Some(og_desc) = dom
        .query_selector("meta[property=\"og:description\"]")
        .and_then(|mut iter| iter.next())
        .and_then(|node| node.get(parser))
        .and_then(|node| node.as_tag())
        .and_then(|tag| tag.attributes().get("content")?)
        .map(|content| content.as_utf8_str().to_string())
    {
        Some(og_desc)
    } else {
        // Fall back to standard meta description
        dom.query_selector("meta[name=\"description\"]")
            .and_then(|mut iter| iter.next())
            .and_then(|node| node.get(parser))
            .and_then(|node| node.as_tag())
            .and_then(|tag| tag.attributes().get("content")?)
            .map(|content| content.as_utf8_str().to_string())
    }
    .map(|text| decode_html_entities(&text))
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
