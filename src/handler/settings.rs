//! Settings-related handlers and templates.

use askama::Template;
use axum::response::IntoResponse;
use axum_extra::extract::CookieJar;
use cookie::time::Duration;
use serde::Deserialize;

use crate::handler::{AuthState, HtmlTemplate};

#[derive(Template)]
#[template(path = "pages/settings.html")]
pub struct SettingsTemplate<'a> {
    pub title: &'a str,
    pub auth_state: AuthState,
    pub is_error: bool,
    pub current_theme: String,
}

#[derive(Deserialize)]
pub struct ThemeUpdate {
    pub theme: String, // "light" or "dark" or "auto"
}

/// Handler for the settings page
pub async fn settings_handler(jar: CookieJar) -> impl IntoResponse {
    // Get current theme from cookie, default to "auto"
    let current_theme = jar
        .get("theme")
        .map_or_else(|| "auto".to_string(), |cookie| cookie.value().to_string());

    HtmlTemplate(SettingsTemplate {
        title: "Settings",
        auth_state: crate::handler::AuthState::Authenticated,
        is_error: false,
        current_theme,
    })
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

    // Just return success - JavaScript handles the UI update
    (jar, "OK")
}
