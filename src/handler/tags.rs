//! Tag-related handlers and templates.

use askama::Template;
use axum::{
    Extension, Json,
    extract::{Query, State},
    response::IntoResponse,
};
use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};
use serde::{Deserialize, Serialize};

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
    // Get tags from database
    let db_tags = tags::get_user_tags(&state.pool, user.user_id).await.unwrap_or_default();

    // Convert database results to template format
    let template_tags: Vec<Tag> = db_tags.into_iter().map(|db_tag| Tag { name: db_tag.name }).collect();

    HtmlTemplate(TagListTemplate { tags: template_tags })
}

#[derive(Deserialize)]
pub struct TagAutocompleteQuery {
    pub q: String,
}

#[derive(Serialize)]
pub struct TagSuggestion {
    pub name: String,
    pub score: i64,
}

/// API handler for tag autocompletion with fuzzy matching
pub async fn tag_autocomplete_handler(
    State(state): ApiState,
    Extension(user): Extension<User>,
    Query(query): Query<TagAutocompleteQuery>,
) -> impl IntoResponse {
    // Get all user tags from database
    let db_tags = tags::get_user_tags(&state.pool, user.user_id).await.unwrap_or_default();

    // Skip if query is too short to avoid too many matches
    if query.q.len() < 1 {
        return Json(Vec::<TagSuggestion>::new());
    }

    // Create fuzzy matcher
    let matcher = SkimMatcherV2::default();

    // Find matching tags with scores
    let mut suggestions: Vec<TagSuggestion> = db_tags
        .into_iter()
        .filter_map(|db_tag| {
            matcher
                .fuzzy_match(&db_tag.name, &query.q)
                .map(|score| TagSuggestion { name: db_tag.name, score })
        })
        .collect();

    // Sort by score (highest first) and limit results
    suggestions.sort_by(|a, b| b.score.cmp(&a.score));
    suggestions.truncate(10); // Limit to top 10 suggestions

    Json(suggestions)
}
