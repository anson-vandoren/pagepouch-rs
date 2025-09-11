//! Tag-related handlers and templates.

use askama::Template;
use axum::{Extension, Json, extract::State, response::IntoResponse};
use axum_extra::extract::Query;
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
    pub active_tags: Vec<String>,
}

/// Query parameters for tag list filtering
#[derive(Deserialize)]
pub struct TagListQuery {
    /// Filter by active tags - committed tags that are currently filtering results
    #[serde(default)]
    pub tags: Option<Vec<String>>,
}

/// API handler for tag list (HTMX lazy loading)
pub async fn tag_list_handler(
    State(state): ApiState,
    Extension(user): Extension<User>,
    Query(params): Query<TagListQuery>,
) -> impl IntoResponse {
    // Extract active tag filters
    let active_tags = params.tags.unwrap_or_default();

    // Get tags filtered by active tag filters
    let db_tags = tags::get_tags_for_active_filters(&state.pool, user.user_id, &active_tags)
        .await
        .unwrap_or_default();

    // Convert database results to template format, filtering out active tags from inactive list
    let template_tags: Vec<Tag> = db_tags
        .into_iter()
        .filter(|db_tag| !active_tags.contains(&db_tag.name))
        .map(|db_tag| Tag { name: db_tag.name })
        .collect();

    HtmlTemplate(TagListTemplate {
        tags: template_tags,
        active_tags: active_tags.clone(),
    })
}

#[derive(Deserialize)]
pub struct TagAutocompleteQuery {
    pub q: String,
    /// Filter by active tags - committed tags that are currently filtering results
    #[serde(default)]
    pub tags: Option<Vec<String>>,
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
    // Extract active tag filters
    let active_tags = query.tags.unwrap_or_default();

    // Get tags filtered by active tag filters
    let db_tags = tags::get_tags_for_active_filters(&state.pool, user.user_id, &active_tags)
        .await
        .unwrap_or_default();

    // Skip if query is too short to avoid too many matches
    if query.q.is_empty() {
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
