//! Settings-related handlers and templates.

use askama::Template;
use axum::{Extension, extract::State, response::IntoResponse};
use axum_extra::extract::CookieJar;
use cookie::time::Duration;
use serde::Deserialize;

use crate::{
    ApiState,
    db::users::User,
    handler::{AuthState, HtmlTemplate, Toasts, middlewares::check_session_cookie},
};

#[derive(Template)]
#[template(path = "pages/settings.html")]
pub struct SettingsTemplate<'a> {
    pub title: &'a str,
    pub toasts: Toasts,
    pub auth_state: AuthState,
    pub is_error: bool,
}

#[derive(Template)]
#[template(path = "components/theme_toggle.html")]
pub struct ThemeToggleTemplate {
    pub current_theme: String,
}

#[derive(Deserialize)]
pub struct ThemeUpdate {
    pub theme: String, // "light" or "dark" or "auto"
}

/// Handler for the settings page
pub async fn settings_handler(State(state): ApiState, jar: CookieJar, Extension(_user): Extension<User>) -> impl IntoResponse {
    let session = check_session_cookie(&state, &jar).await;

    let toasts = if let Ok(mut session_lookup) = session {
        session_lookup.session.take_messages(&state.pool).await
    } else {
        Vec::new()
    };

    HtmlTemplate(SettingsTemplate {
        title: "Settings",
        auth_state: crate::handler::AuthState::Authenticated,
        toasts,
        is_error: false,
    })
}

/// API handler for theme toggle component (HTMX lazy loading)
pub async fn theme_toggle_handler(State(_state): ApiState, Extension(_user): Extension<User>, jar: CookieJar) -> impl IntoResponse {
    // Get current theme from cookie, default to "auto"
    let current_theme = jar
        .get("theme")
        .map_or_else(|| "auto".to_string(), |cookie| cookie.value().to_string());

    HtmlTemplate(ThemeToggleTemplate { current_theme })
}

/// API handler for updating theme preference
pub async fn update_theme_handler(
    jar: CookieJar,
    axum::extract::Form(theme_update): axum::extract::Form<ThemeUpdate>,
) -> impl IntoResponse {
    use axum_extra::extract::cookie::{Cookie, SameSite};

    // Validate theme value
    let theme = match theme_update.theme.as_str() {
        "light" | "dark" | "auto" => theme_update.theme.clone(),
        _ => "auto".to_string(),
    };

    // Set theme cookie with 1 year expiration
    let cookie = Cookie::build(("theme", theme))
        .path("/")
        .max_age(Duration::days(365))
        .same_site(SameSite::Lax)
        .http_only(false) // Allow JavaScript access for theme switching
        .build();

    let jar = jar.add(cookie);

    // Return updated theme toggle component
    let current_theme = theme_update.theme;
    (jar, HtmlTemplate(ThemeToggleTemplate { current_theme }))
}
