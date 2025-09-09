# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

PagePouch is a self-hosted Rust web application for managing bookmarks with multi-user authentication, tag management, and import/export functionality. It uses SQLite for storage, Axum for HTTP handling, and Askama for server-side HTML templating with a Gruvbox color scheme.

## Development Commands

### Building and Running

- `mise run build` - Build the application
- `mise run run` - Build and run the server (depends on build)
- `mise run watch` - Run with hot reload using bacon (depends on migrate)
- `cargo run` - Direct cargo run (server runs on port 8888)

### Testing and Quality

- `mise run test` - Run tests with nextest (`RUST_LOG=debug cargo nextest run`)
- `mise run testall` - Run all tests without failing early
- `mise run clippy` - Run clippy for all targets including tests
- `.mise-tasks/pre-commit.sh` - Pre-commit hook that runs formatting, linting, and checks

### Database Operations

- `mise run migrate` - Run database migrations and prepare SQLx queries
- `cargo sqlx database create` - Create the SQLite database
- `cargo sqlx migrate run` - Apply pending migrations
- `cargo sqlx prepare` - Generate compile-time checked query metadata

### Code Quality Tools

- `cargo fmt --config-path .rustfmt.stable.toml` - Format code with project settings
- `cargo sort` - Sort Cargo.toml dependencies alphabetically
- `cargo sort-derives` - Sort derive macros alphabetically
- `cargo +nightly udeps` - Check for unused dependencies
- `cargo upgrade --dry-run` - Check for dependency updates

## Architecture

### Core Structure

- **AppState**: Shared state containing encryption provider and database pool
- **Authentication**: Username/password with Argon2 hashing, session-based cookies
- **Database**: SQLite with migrations in `/migrations/`
- **Templates**: Askama HTML templates in `/templates/` with server-side rendering
- **Middleware**: Rate limiting (tower-governor), authentication, tracing

### Key Modules

- `src/main.rs` - Application entry point and state setup
- `src/route.rs` - HTTP routing, middleware setup, and server configuration
- `src/config.rs` - Environment variable loading and encryption key management
- `src/handler/` - Request handlers and response templates
- `src/db/` - Database operations and connection management
- `src/encryption.rs` - Cryptographic functions for passwords and tokens

### Database Schema

Current tables:

- `users` - User accounts with UUID primary keys, usernames, hashed passwords
- `user_sessions` - Session management for authentication
- `bookmarks` - URLs with title, description, creation metadata, and user ownership
- `tags` - Normalized tag names with optional colors
- `bookmark_tags` - Junction table for many-to-many bookmark-tag relationships
- `bookmark_imports` - Import history tracking for bulk operations

Database operations use SQLx query! macros for compile-time checked queries with SQLite blob UUIDs.

### Configuration

- Environment variables loaded from `.env` file
- `DATABASE_URL` required for SQLite connection
- `PAGEPOUCH_KEY_BASE_64` auto-generated encryption key (written to .env if missing)
- Server binds to `0.0.0.0:8888` by default

### Development Features

- Hot reload via bacon in debug mode
- Comprehensive tracing with separate formatters for app vs external crates
- Rate limiting: 1 req/sec for login (burst 3), 2 req/sec general (burst 500)
- Dummy data population in debug builds with realistic bookmarks and tags
- Theme switching with CSS custom properties (light/dark/auto modes)

## Code Style

- Max line width: 140 characters
- Imports grouped as StdExternalCrate with crate-level granularity
- Derive macros and dependencies sorted alphabetically
- Clippy pedantic warnings enabled
- Use `anyhow::Result` for error handling
- Comprehensive documentation with examples and error conditions

## UI Guidelines

- Pages are rendered by Askama using templates, including partials. Ensure that you have fully understood how
  Askama will construct and deliver a page before suggesting HTML changes. Templates are in the `templates/**/*` directories,
  and are mostly constructed by handlers/structs in `src/handler`, based on `src/route.rs` routes.
- CSS started with styles from `assets/css/simple.css` [SimpleCSS](https://simplecss.org), but have been modified
  both in-place and also in `assets/css/main.css`. Prefer adding/overriding styles in `main.css` when possible, but it
  is OK to do it in `simple.css` if it makes things simpler. Do not modify `simple.min.css`.
- HTMX is used extensively. Before suggesting HTML/JS changes, ensure you understand how HTMX is used in this context.

## Logging

- When writing logging/tracing lines, try to use emojis in the logged content when it makes sense.

## Backend Development Requirements

**IMPORTANT**: After completing any backend task involving code changes, you MUST:

1. **Ensure `cargo check` completes successfully** - All code must compile without errors before considering a task complete
2. **Run `mise run clippy` and fix all addressable lints** - Code quality standards must be maintained. It is almost never the right answer to address a lint by #[allow(clippy::...)]
3. **Run `mise run migrate`** if database schema or query changes were made - Ensure SQLx metadata is up to date

These steps are mandatory for task completion and ensure code quality and compilation integrity.
