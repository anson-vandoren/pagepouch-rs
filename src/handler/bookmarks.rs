//! Bookmark-related handlers and templates.

use askama::Template;
use axum::{
    Extension,
    extract::{Query, State},
    response::IntoResponse,
};
use serde::Deserialize;

use crate::{ApiState, db::users::User, handler::HtmlTemplate};

// Mock data structures - replace with your actual models
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
    pub number: usize,
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
    pub page: Option<usize>, // Page number
}

/// API handler for bookmark content (HTMX lazy loading)
pub async fn bookmark_content_handler(
    State(_state): ApiState,
    Extension(_user): Extension<User>,
    Query(params): Query<BookmarkQuery>,
) -> impl IntoResponse {
    // Mock data - replace with actual database queries
    let bookmarks = create_mock_bookmarks();

    // Filter bookmarks based on query parameters
    let filtered_bookmarks = filter_bookmarks(bookmarks, &params);

    let pagination = Some(Pagination {
        has_prev: true,
        has_next: true,
        page_links: vec![
            PageLink {
                number: 1,
                is_current: false,
                is_ellipsis: false,
            },
            PageLink {
                number: 0,
                is_current: false,
                is_ellipsis: true,
            },
            PageLink {
                number: 4,
                is_current: false,
                is_ellipsis: false,
            },
            PageLink {
                number: 5,
                is_current: true,
                is_ellipsis: false,
            },
            PageLink {
                number: 6,
                is_current: false,
                is_ellipsis: false,
            },
            PageLink {
                number: 0,
                is_current: false,
                is_ellipsis: true,
            },
            PageLink {
                number: 32,
                is_current: false,
                is_ellipsis: false,
            },
        ],
    });

    HtmlTemplate(BookmarkContentTemplate {
        bookmarks: filtered_bookmarks,
        pagination,
    })
}

fn create_mock_bookmarks() -> Vec<Bookmark> {
    use super::tags::Tag;

    vec![
        Bookmark {
            url: "https://rust-lang.org".to_string(),
            title: "The Rust Programming Language".to_string(),
            tags: vec![
                Tag {
                    name: "programming".to_string(),
                },
                Tag { name: "rust".to_string() },
                Tag {
                    name: "systems".to_string(),
                },
            ],
            created_by: "anson".to_string(),
            created_at: "2024-12-15".to_string(),
            formatted_date: "Dec 15, 2024".to_string(),
        },
        Bookmark {
            url: "https://htmx.org".to_string(),
            title: "HTMX - High Power Tools for HTML".to_string(),
            tags: vec![
                Tag {
                    name: "web-dev".to_string(),
                },
                Tag {
                    name: "javascript".to_string(),
                },
                Tag { name: "htmx".to_string() },
            ],
            created_by: "anson".to_string(),
            created_at: "2024-12-10".to_string(),
            formatted_date: "Dec 10, 2024".to_string(),
        },
        Bookmark {
            url: "https://github.com/tokio-rs/axum".to_string(),
            title: "Axum Web Framework for Rust".to_string(),
            tags: vec![
                Tag { name: "rust".to_string() },
                Tag {
                    name: "web-framework".to_string(),
                },
                Tag { name: "axum".to_string() },
                Tag { name: "tokio".to_string() },
                Tag { name: "async".to_string() },
                Tag { name: "http".to_string() },
                Tag {
                    name: "server".to_string(),
                },
                Tag {
                    name: "middleware".to_string(),
                },
                Tag {
                    name: "routing".to_string(),
                },
            ],
            created_by: "anson".to_string(),
            created_at: "2024-12-08".to_string(),
            formatted_date: "Dec 8, 2024".to_string(),
        },
        Bookmark {
            url: "https://simplecss.org".to_string(),
            title: "Simple.css - A CSS Framework for Semantic HTML".to_string(),
            tags: vec![
                Tag { name: "css".to_string() },
                Tag {
                    name: "framework".to_string(),
                },
                Tag {
                    name: "simple".to_string(),
                },
            ],
            created_by: "anson".to_string(),
            created_at: "2024-12-05".to_string(),
            formatted_date: "Dec 5, 2024".to_string(),
        },
    ]
}

fn filter_bookmarks(mut bookmarks: Vec<Bookmark>, params: &BookmarkQuery) -> Vec<Bookmark> {
    // Filter by tag
    if let Some(tag_name) = &params.tag {
        bookmarks.retain(|bookmark| bookmark.tags.iter().any(|tag| tag.name == *tag_name));
    }

    // Filter by search query
    if let Some(query) = &params.q {
        let query = query.to_lowercase();
        bookmarks.retain(|bookmark| {
            bookmark.title.to_lowercase().contains(&query)
                || bookmark.url.to_lowercase().contains(&query)
                || bookmark.tags.iter().any(|tag| tag.name.to_lowercase().contains(&query))
        });
    }

    // TODO: Handle pagination
    bookmarks
}
