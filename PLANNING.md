# Bookmark Manager App Planning

## Overview

A self-hosted, fast, multi-user Rust app for managing bookmarks with a web UI. Features include login, user roles, pagination, import/export (Netscape format), metadata extraction, tagging, and robust tag management. Uses SQLite for storage. Gruvbox color scheme with light/dark toggle. Fuzzy typeahead for tags.

## Requirements

### Core Features

- Store bookmarks (URL, title, metadata, tags)
- Web-based UI (HTTP server)
- SQLite database
- Multi-user login/authentication (username + password)
- User roles: admin, reader
- Option for unauthenticated view access (off by default)
- Pagination for bookmark lists (default 20, user-configurable)
- Import/export bookmarks (Netscape format)
- Duplicate handling on import: prompt to overwrite, merge tags, or ignore (with 'apply to all')
- Extract page title and metadata on add. For import, offer to kick off a background job to update this if not present.
- Metadata: date added, date updated, date imported, date last visited (when clicked from app)
- Tagging system (flat tags)
- Tag sorting/filtering/management (edit, merge, bulk edit, find similar)
- Gruvbox color scheme (light/dark toggle)
- Fuzzy typeahead for tags
- Search (fuzzy, across url/title/tags, or by dimension)
- Download database file from settings for backup
- Should be able to sort tags by how many links are associated with them

## Technical Choices

- Authentication: username + password, industry best practices (hashing, salting, etc.), no 2FA for now
- UI: Askama templates, basic HTML/CSS/JS, hand-written JS as needed
- Deployment: behind nginx or other reverse proxy, no TLS handling in-app

## Open Questions / Future Considerations

- Folders/groups for bookmarks (not needed now, but keep design open)
- TLS support (future)

---

## Next Steps

- Authentication and user management design
- Tag management tooling design
- Import/export logic and UI
- Bookmark metadata extraction (title, etc.)
- Search implementation plan

## TODO:

- When search input is selected, its outline becomes bigger than the search button
- Pagination
