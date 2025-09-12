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
- tab'ing from an added tag should show it as a box
- Option per-user to share public links at $URL/my_links/${username}\_${nanoid}
- View-only users
- Move all inline `<script>` JS to separate files?
- Numbers on tags showing how many links.
- Should typeahead and real-time searching be via websockets instead? Discuss
- Tags sort by alphabetical or link count (toggle)
- Row breaks between letters of tags
- Tag column toggle to also filter by current tag filter. Filter tags are highlighted in tag column.
- Limit the number of suggestions returned to ~10 or so, ranked.
- ~~Login page is too skinny~~
- ~~Remove logout toast~~
- ~~Simplify theme toggle - shouldn't be its own API call...~~
- **Theme toggle should just be a small selector, not a huge thing**
- ~~Remove "more settings coming soon" thing~~
- ~~Remove "customize your pagepouch experience"~~
- **Make sure there's no box shadow on settings page**
- **Make "Theme" seem like a subheading on "Appearance"**
- User settings table (theme for now).
  - Theme should be settable persistently but should be overrideable per device/browser. (localstorage?)
- Theme setting probably should use JSON instead of form?
- Minify CSS and JS for prod
- Are HTMX/Askama still even the right choice here? This is much more of an app than a static page...
- 3+ query OR terms fail. Probably needs a fix like search_multiple_terms_and
- Padding on tags under links needs to go smaller
- Highlighting active tags in light mode needs a different color
- Vertical align top of tags on the right with top of top bookmark
- No tags still renders a little turd
- Save & cancel on add link are different heights
- Check settings on old app to move over
- Use local time not UTC
- Add & edit links as modals not different page nor inline
- Sort by link count toggle + then show the count on the tag
- Plan for key recovery: is it just a user reset? We should make this easy to do.
- Logging to /var/log
- Monitoring somewhere
- Subdomains for users? (wildcard cert)
- Product update emails
- Robots.txt
- Configurable port
- Consider letting NGINX handle static files instead
- Tags on new link show up like others
- Delete admin user from prod
- Check for and remove unused CSS
- Bundle(?) & minify JS
- TESTS!!!
- Remove tailscale from box, probably
- Find all `<script>` tags and consolidate them
- Overall project structure
- Test that autologout works still
- title_input.html seems like it's only there because of HTMX and we should just use JS w/ a JSON repsonse?
- Lighthouse score && fixup
