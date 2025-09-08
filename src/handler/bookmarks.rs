//! Bookmark-related handlers and templates.

use askama::Template;
use axum::{
    Extension, Form,
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use serde::Deserialize;

use crate::{
    ApiState,
    db::{bookmarks, users::User},
    handler::{AuthState, HtmlTemplate, Toasts},
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

#[derive(Deserialize)]
pub struct BookmarkQuery {
    pub q: Option<String>,   // Search query
    pub tag: Option<String>, // Filter by tag
    pub page: Option<i64>,   // Page number
}

/// API handler for bookmark content (HTMX lazy loading)
pub async fn bookmark_content_handler(
    State(state): ApiState,
    Extension(user): Extension<User>,
    Query(params): Query<BookmarkQuery>,
) -> impl IntoResponse {
    const DEFAULT_LIMIT: i64 = 20;

    let limit = DEFAULT_LIMIT;
    let page = params.page.unwrap_or(1);
    let offset = (page - 1) * DEFAULT_LIMIT;

    // Get bookmarks from database based on filters
    let db_bookmarks = if let Some(ref tag_name) = params.tag {
        bookmarks::get_user_bookmarks_by_tag(&state.pool, user.user_id, tag_name, limit, offset)
            .await
            .unwrap_or_default()
    } else if let Some(ref search_query) = params.q {
        bookmarks::search_user_bookmarks(&state.pool, user.user_id, search_query, limit, offset)
            .await
            .unwrap_or_default()
    } else {
        bookmarks::get_user_bookmarks(&state.pool, user.user_id, limit, offset)
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
    let pagination = if i64::try_from(template_bookmarks.len()).unwrap_or(0) == limit {
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
            // Use HTMX redirect header to navigate back to home
            let mut headers = HeaderMap::new();
            headers.insert("HX-Redirect", "/".parse().unwrap());
            (StatusCode::OK, headers).into_response()
        }
        Err(err) => {
            tracing::error!("🚨 Failed to create bookmark: {}", err);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to create bookmark").into_response()
        }
    }
}

/// Handler for fetching page title from URL
pub async fn fetch_title_handler(Form(request): Form<FetchTitleRequest>) -> impl IntoResponse {
    // TODO: Implement actual title fetching
    let title = format!("Title for {}", request.url);
    HtmlTemplate(TitleInputTemplate { title })
}
