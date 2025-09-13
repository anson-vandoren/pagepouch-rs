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

## TODO

- Pagination
- tab'ing from an added tag should show it as a box (on create page). Enter should work, too, and probably space.
  - Ensure that quoted tags remove the quotes for display
- Option per-user to share public links at $URL/my_links/${username}\_${nanoid}
- View-only users
- Numbers on tags showing how many links. Or tooltips?
- Tags sort by alphabetical or link count (toggle)
- Should typeahead and real-time searching be via websockets instead? Discuss
- Row breaks between letters of tags
- Tag column toggle to also filter by current tag filter. Filter tags are highlighted in tag column.
- Limit the number of suggestions returned to ~10 or so, ranked.
- **Login card not centered L/R?**
- On larger screen, keep the bookmarks column to a reasonable width and don't let tags column get too wide
- User settings table (theme for now).
  - Theme should be settable persistently but should be overrideable per device/browser. (localstorage?)
- **Theme setting probably should use JSON instead of form?**
- Are HTMX/Askama still even the right choice here? This is much more of an app than a static page...
- 3+ query OR terms fail. Probably needs a fix like search_multiple_terms_and
- Vertical align top of tags on the right with top of top bookmark
- Check settings on old app to move over
- Add & edit links as modals not different page nor inline
- Sort by link count toggle + then show the count on the tag
- Tags on new link show up like others
- Test that autologout works still
- Consolidate CSS and remove the simple.css stuff we don't need
- **Longer cookie/session timeout**
- Delete button w/ confirm
- ~~HTML-de-escaping for scraped description~~
- Show error tooltip when an error appears.
- Investigate why an error for a non-tag.
- Keep error underline permanently.

- Configurable port
- Delete admin user from prod
- Plan for key recovery: is it just a user reset? We should make this easy to do.

- Consider letting NGINX handle static files instead
- Convert release tool to... Rust? smth, anyway. BuildGitHubReleaseRS, aka bghrrs
- bookmarks.rs has a _lot_ of duplicated code
- Check for and remove unused CSS
- Minify CSS and JS for prod
- Bundle(?) & minify JS
- TESTS!!!
- Overall project structure
- Lighthouse score && fixup
- Remove tailscale from box, probably
- Move all inline `<script>` JS to separate files?
- Find all `<script>` tags and consolidate them
- Logging to /var/log
- Monitoring somewhere
- Subdomains for users? (wildcard cert)
- Product update emails
- Robots.txt
- Consider `specta` (v2) for sharing BE/FE types
