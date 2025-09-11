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

- Pagination
- urls without a protocol should guess http
- tab'ing from an added tag should show it as a box
- Option per-user to share public links at $URL/my_links/${username}\_${nanoid}
- View-only users
- Move all inline `<script>` JS to separate files?
- Convert JS to hyperscript?
- Numbers on tags showing how many links.
- Tag cloud also filters by active tags, or else shows highlighted which tags are active. Probably the first
- Typeahead for tags, don't add the tag until completed (by space or tab or enter(?))
- Should typeahead and real-time searching be via websockets instead? Discuss
- Tags sort by alphabetical or link count (toggle)
- Row breaks between letters of tags
- Tag column toggle to also filter by current tag filter. Filter tags are highlighted in tag column.
- Filtered tags show up in the tag column, not the bookmarks column (see current version)
- Scrape meta description
- Can/should we have current tags in an arc/rwlock in state, maybe, to avoid repeated DB lookup for auto-complete?
- Search box auto-selected on page load
- Limit the number of suggestions returned to ~10 or so, ranked.
- Login page is too skinny
- Minify CSS and JS for prod
- Are HTMX/Askama still even the right choice here? This is much more of an app than a static page...
