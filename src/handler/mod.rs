//! HTTP request handlers and response templates.

pub mod auth_handler;
pub mod bookmarks;
pub mod middlewares;
pub mod settings;
pub mod tags;
use askama::Template;
use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse},
};
use axum_extra::extract::CookieJar;
use tracing::error;

use crate::{ApiState, handler::middlewares::check_session_cookie};

/// Authentication state for template rendering.
///
/// Used to control which UI elements are shown based on authentication status.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum AuthState {
    Authenticated, // User is logged in
    #[default]
    Anonymous, // User is not logged in
    LoginPage,     // Special case for login page (hide login button)
}

/// Wrapper for Askama templates to convert them into Axum responses.
pub struct HtmlTemplate<T>(pub T);

impl<T> IntoResponse for HtmlTemplate<T>
where
    T: Template,
{
    fn into_response(self) -> axum::response::Response {
        // Try to render the template with `askama`
        match self.0.render() {
            Ok(html) => Html(html).into_response(),
            Err(err) => {
                error!(?err, "Failed to render template.");
                (StatusCode::INTERNAL_SERVER_ERROR, "Failed to render template.").into_response()
            }
        }
    }
}

#[derive(Default, Template)]
#[template(path = "pages/home.html")]
pub struct HomeTemplate<'a> {
    pub title: &'a str,
    pub auth_state: AuthState,
    pub is_error: bool,
}

#[derive(Default, Template)]
#[template(path = "error/error_404.html")]
struct Error404Template<'a> {
    title: &'a str,
    reason: &'a str,
    link: &'a str,
    auth_state: AuthState,
    is_error: bool,
}

#[derive(Default, Template)]
#[template(path = "auth/login.html")]
struct LoginTemplate<'a> {
    title: &'a str,
    auth_state: AuthState,
    is_error: bool,
}

/// Handler for the home page.
///
/// Authentication is guaranteed by middleware.
pub async fn home_handler() -> impl IntoResponse {
    HtmlTemplate(HomeTemplate {
        title: "Home",
        auth_state: AuthState::Authenticated,
        is_error: false,
    })
    .into_response()
}

/// Handler for 404 Not Found errors.
///
/// Shows a custom 404 page with appropriate navigation links
/// based on the user's authentication status.
pub async fn handle_404(State(state): ApiState, jar: CookieJar) -> impl IntoResponse {
    let auth_state = if check_session_cookie(&state, &jar).await.is_ok() {
        AuthState::Authenticated
    } else {
        AuthState::Anonymous
    };
    let link = match auth_state {
        AuthState::Authenticated => "/",
        _ => "/login",
    };

    (
        StatusCode::NOT_FOUND,
        HtmlTemplate(Error404Template {
            title: "Error 404",
            reason: "Move along, please...",
            link,
            auth_state,
            is_error: true,
        }),
    )
}
