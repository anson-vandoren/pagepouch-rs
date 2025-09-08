//! Tag-related handlers and templates.

use askama::Template;
use axum::{Extension, extract::State, response::IntoResponse};

use crate::{
    ApiState,
    db::{tags, users::User},
    handler::HtmlTemplate,
};

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
pub async fn tag_list_handler(State(state): ApiState, Extension(user): Extension<User>) -> impl IntoResponse {
    // Convert user_id to bytes for database query
    let user_id_bytes = user.user_id.as_bytes().to_vec();

    // Get tags from database
    let db_tags = tags::get_user_tags(&state.pool, &user_id_bytes).await.unwrap_or_default();

    // Convert database results to template format
    let template_tags: Vec<Tag> = db_tags.into_iter().map(|db_tag| Tag { name: db_tag.name }).collect();

    HtmlTemplate(TagListTemplate { tags: template_tags })
}
