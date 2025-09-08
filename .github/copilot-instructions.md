# Copilot Instructions for pagepouch-rs

## Project Overview

This is a self-hosted Rust web application for managing bookmarks. It uses SQLite for storage, Askama for server-side HTML templating, and provides a web UI with Gruvbox color scheme.
The app supports multi-user authentication (username + password), user roles (admin, reader), robust tag management, import/export in Netscape format, and fuzzy search.

## Architecture

- **Rust Backend**: Handles HTTP requests, authentication, bookmark CRUD, tag management, import/export, and search.
- **SQLite Database**: Stores users, bookmarks, tags, and metadata. Schema planning is ongoing; see `PLANNING.md` for details.
- **Askama Templates**: Used for rendering HTML pages. Minimal JS is hand-written for UI enhancements (e.g., fuzzy tag typeahead).

## Developer Workflows

- **Build**: Use `cargo build --release` for production builds.
- **Run**: Use `cargo run` to start the HTTP server locally.
- **Test**: Use `cargo nextest run` for unit/integration tests (to be implemented).
- **Database**: The SQLite file is stored locally; backup by downloading from the settings page or copying the file.
- **Import/Export**: Netscape format import/export is supported; duplicate handling is interactive with bulk options.

## Conventions & Patterns

- **Authentication**: Use industry best practices for password hashing/salting. Multi-user, with roles and optional unauthenticated view.
- **Tag Management**: Tags are flat, editable, mergeable, and support fuzzy typeahead. Bulk tag operations are planned.
- **Pagination**: Default is 20 items per page, user-configurable.
- **Search**: Fuzzy search across URL, title, and tags. User can specify search dimension.
- **UI**: Server-side rendered with Askama. Gruvbox color scheme, light/dark toggle. Minimal JS for interactivity.
- **Deployment**: Designed to run behind a reverse proxy (e.g., nginx); TLS is not handled by the app.

## Key Files

- `PLANNING.md`: Requirements, architecture, and next steps.
- (Future) `src/`: Rust source code for backend and web server.
- (Future) `templates/`: Askama HTML templates.

## Integration Points

- **External**: No external APIs required; all metadata extraction is done server-side when adding bookmarks.
- **Frontend/Backend**: Communication is via HTTP requests; all rendering is server-side except for minimal JS enhancements.

## Example Patterns

- When adding a bookmark, fetch page title and metadata server-side.
- When importing bookmarks, prompt for duplicate handling with bulk options.
- Use fuzzy typeahead for tag entry to prevent similar/duplicate tags.
