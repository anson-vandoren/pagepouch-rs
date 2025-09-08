//! Tag-related handlers and templates.

use askama::Template;
use axum::{Extension, extract::State, response::IntoResponse};

use crate::{ApiState, db::users::User, handler::HtmlTemplate};

#[derive(Clone)]
pub struct Tag {
    pub name: String,
}

#[derive(Template)]
#[template(path = "components/tag_list.html")]
pub struct TagListTemplate {
    pub tags: Vec<Tag>,
}

/// API handler for tag cloud (HTMX lazy loading)
pub async fn tag_list_handler(State(_state): ApiState, Extension(_user): Extension<User>) -> impl IntoResponse {
    // Mock data - replace with actual database queries
    let tags = create_mock_tags();

    HtmlTemplate(TagListTemplate { tags })
}

fn create_mock_tags() -> Vec<Tag> {
    vec![
        Tag { name: "rust".to_string() },
        Tag {
            name: "programming".to_string(),
        },
        Tag {
            name: "web-dev".to_string(),
        },
        Tag {
            name: "javascript".to_string(),
        },
        Tag { name: "htmx".to_string() },
        Tag { name: "css".to_string() },
        Tag {
            name: "web-framework".to_string(),
        },
        Tag { name: "axum".to_string() },
        Tag {
            name: "systems".to_string(),
        },
        Tag { name: "tokio".to_string() },
        Tag {
            name: "framework".to_string(),
        },
        Tag {
            name: "simple".to_string(),
        },
        Tag {
            name: "backend".to_string(),
        },
        Tag {
            name: "database".to_string(),
        },
        Tag {
            name: "sqlite".to_string(),
        },
        Tag {
            name: "authentication".to_string(),
        },
        Tag { name: "api".to_string() },
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
        Tag {
            name: "templates".to_string(),
        },
        Tag {
            name: "bookmarks".to_string(),
        },
        Tag {
            name: "self-hosted".to_string(),
        },
    ]
}
