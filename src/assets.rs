//! Embedded static assets for self-contained binary distribution.

use axum::{
    body::Body,
    extract::Path,
    http::{Response, StatusCode},
    response::IntoResponse,
};
use rust_embed_for_web::{EmbedableFile, RustEmbed};

/// Embedded assets directory containing CSS, JS, images, and other static files.
///
/// In debug builds, files are read from the filesystem for hot reloading.
/// In release builds, files are embedded into the binary at compile time.
#[derive(RustEmbed)]
#[folder = "assets/"]
pub struct Assets;

/// Handler for serving embedded static assets.
///
/// Uses rust-embed-for-web which provides better HTTP caching support
/// than the standard rust-embed crate, including `ETag` handling.
///
/// # Arguments
///
/// * `path` - The file path relative to the assets directory
///
/// # Returns
///
/// Returns the file contents with proper HTTP caching headers, or 404 if not found.
pub async fn assets_handler(Path(path): Path<String>) -> impl IntoResponse {
    match Assets::get(&path) {
        Some(content) => {
            let mime = content.mime_type().unwrap_or("octet-stream".into());
            let mut res = Response::builder()
                .status(StatusCode::OK)
                .header("content-type", mime)
                .header("etag", content.etag())
                .header("cache-control", "public, max-age=31536000");

            if let Some(last_modified) = content.last_modified() {
                res = res.header("Last-Modified", last_modified);
            }

            let response: Result<Response<Body>, axum::http::Error> = if let Some(body) = content.data_br() {
                res.header("Content-Encoding", "br").body(body.into())
            } else if let Some(body) = content.data_gzip() {
                res.header("Content-Encoding", "gzip").body(body.into())
            } else {
                res.body(content.data().into())
            };

            response.unwrap_or_else(|_| {
                Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body("Failed to build response".into())
                    .unwrap()
            })
        }
        None => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body("Asset not found".into())
            .unwrap(),
    }
}
