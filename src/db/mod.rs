//! Database module for managing `SQLite` connections and migrations.
//!
//! This module provides database connection pooling, automatic migrations,
//! and submodules for specific database operations.

pub mod bookmarks;
pub mod tags;
pub mod user_session;
pub mod users;
use anyhow::{Context as _, Result};
use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};

const MAX_CONNECTIONS: u32 = 10;

/// Establishes a connection pool to the `SQLite` database.
///
/// This function:
/// 1. Creates a connection pool with configured limits
/// 2. Runs any pending database migrations
/// 3. In debug mode, creates a default admin user if needed
///
/// # Errors
///
/// Returns an error if:
/// - Database connection fails
/// - Migrations fail to run
/// - Debug initialization fails
pub async fn connect(pool_uri: &str) -> Result<SqlitePool> {
    let pool = SqlitePoolOptions::new()
        .max_connections(MAX_CONNECTIONS)
        .connect(pool_uri)
        .await
        .context("Error: 🔥 unable to connect to the database!")?;

    sqlx::migrate!()
        .run(&pool)
        .await
        .with_context(|| format!("🚨 Could not run database migrations for database at '{pool_uri}'"))?;

    #[cfg(debug_assertions)]
    init_for_dev(&pool).await?;

    println!("✅ Successfully connected to database!");
    Ok(pool)
}

/// Initializes development-specific database data.
///
/// Creates a default admin user for testing purposes in debug builds.
/// Username: admin
/// Password: admin123 (pre-hashed with Argon2)
///
/// # Errors
///
/// Returns an error if database operations fail.
#[cfg(debug_assertions)]
async fn init_for_dev(pool: &SqlitePool) -> Result<()> {
    const ADMIN_USERNAME: &str = "admin";
    const ADMIN_PASSWORD_HASH: &str = "$argon2id$v=19$m=19456,t=2,p=1$LL8PlWjHaOuA6gLK2+x1fQ$LY791mB/ymrCS/HgwSHqj4Mc9eEnOcZB/OT5bu9+GFY";
    let has_admin = sqlx::query!(
        r#"
            select user_id from users
            where username = ?
        "#,
        "admin"
    )
    .fetch_optional(pool)
    .await?;

    if has_admin.is_none() {
        println!("👤 No dev-mode admin user found, adding...");
        let _res = sqlx::query!(
            r#"
                insert into users (
                    username,
                    password_hash
                )
                values (?, ?)
            "#,
            ADMIN_USERNAME,
            ADMIN_PASSWORD_HASH
        )
        .execute(pool)
        .await?;
        println!("✨ Added dev user: admin");
    }

    // Add dummy data for development
    populate_dummy_data(pool).await?;

    Ok(())
}

/// Populates the database with dummy bookmarks and tags for development.
///
/// Only runs once - checks if data already exists before adding.
///
/// # Errors
///
/// Returns an error if database operations fail.
#[cfg(debug_assertions)]
#[allow(clippy::too_many_lines)] // Dev-only function with extensive test data - splitting would hurt readability
async fn populate_dummy_data(pool: &SqlitePool) -> Result<()> {
    // Check if we already have bookmarks data
    let existing_bookmarks = sqlx::query!("select count(*) as count from bookmarks").fetch_one(pool).await?;

    if existing_bookmarks.count > 0 {
        return Ok(()); // Data already exists
    }

    println!("📚 Adding dummy bookmarks and tags for development...");

    // Get admin user ID
    let admin_user = sqlx::query!("select user_id from users where username = 'admin'")
        .fetch_one(pool)
        .await?;

    // Create tags first
    let tag_data = [
        "rust",
        "programming",
        "web-dev",
        "javascript",
        "htmx",
        "css",
        "web-framework",
        "axum",
        "systems",
        "tokio",
        "framework",
        "simple",
        "backend",
        "database",
        "sqlite",
        "authentication",
        "api",
        "async",
        "http",
        "server",
        "middleware",
        "routing",
        "templates",
        "bookmarks",
        "self-hosted",
    ];

    // Insert tags
    for name in &tag_data {
        sqlx::query!("insert or ignore into tags (name) values (?)", name)
            .execute(pool)
            .await?;
    }

    // Create bookmarks with associated tags
    let bookmark_data = [
        (
            "https://rust-lang.org",
            "The Rust Programming Language",
            "A systems programming language that is blazingly fast, memory-safe, and thread-safe.",
            &["rust", "programming", "systems"][..],
        ),
        (
            "https://htmx.org",
            "HTMX - High Power Tools for HTML",
            "htmx allows you to access modern browser features directly from HTML, rather than using JavaScript.",
            &["web-dev", "javascript", "htmx"],
        ),
        (
            "https://github.com/tokio-rs/axum",
            "Axum Web Framework for Rust",
            "Axum is a web application framework that focuses on ergonomics and modularity.",
            &[
                "rust",
                "web-framework",
                "axum",
                "tokio",
                "async",
                "http",
                "server",
                "middleware",
                "routing",
            ],
        ),
        (
            "https://simplecss.org",
            "Simple.css - A CSS Framework for Semantic HTML",
            "A CSS framework for developers who want their websites to look good without the complexity.",
            &["css", "framework", "simple"],
        ),
        (
            "https://sqlite.org",
            "SQLite Database Engine",
            "SQLite is a C library that provides a lightweight disk-based database.",
            &["database", "sqlite", "backend"],
        ),
        (
            "https://docs.rs/askama/latest/askama/",
            "Askama Template Engine",
            "Type-safe, compiled Jinja-like templates for Rust.",
            &["rust", "templates", "web-dev"],
        ),
        (
            "https://github.com/launchbadge/sqlx",
            "SQLx - Rust SQL Toolkit",
            "The Rust SQL toolkit. An async, pure Rust SQL crate featuring compile-time checked queries.",
            &["rust", "database", "async", "api"],
        ),
        (
            "https://developer.mozilla.org/en-US/docs/Web/API/Fetch_API",
            "Fetch API - Web APIs | MDN",
            "The Fetch API provides an interface for fetching resources.",
            &["javascript", "web-dev", "api", "http"],
        ),
        (
            "https://github.com/SergioBenitez/Rocket",
            "Rocket - Web Framework for Rust",
            "A web framework for Rust that makes it simple to write fast, secure web applications.",
            &["rust", "web-framework", "server", "api"],
        ),
        (
            "https://tailwindcss.com",
            "Tailwind CSS",
            "A utility-first CSS framework for rapidly building custom user interfaces.",
            &["css", "framework", "web-dev"],
        ),
    ];

    // Insert bookmarks and link to tags
    for (url, title, description, tag_names) in &bookmark_data {
        // Insert bookmark and get the generated bookmark_id
        let bookmark_result = sqlx::query!(
            r#"
            insert into bookmarks (user_id, url, title, description)
            values (?, ?, ?, ?)
            returning bookmark_id
            "#,
            admin_user.user_id,
            url,
            title,
            description
        )
        .fetch_one(pool)
        .await?;

        let bookmark_id = &bookmark_result.bookmark_id;

        // Link bookmark to tags
        for tag_name in *tag_names {
            let tag = sqlx::query!("select tag_id from tags where name = ?", tag_name)
                .fetch_one(pool)
                .await?;

            sqlx::query!(
                "insert into bookmark_tags (bookmark_id, tag_id) values (?, ?)",
                bookmark_id,
                tag.tag_id
            )
            .execute(pool)
            .await?;
        }
    }

    println!(
        "✨ Added {} tags and {} bookmarks for development!",
        tag_data.len(),
        bookmark_data.len()
    );
    Ok(())
}
