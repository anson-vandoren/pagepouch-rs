# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.3](https://github.com/anson-vandoren/pagepouch-rs/compare/v0.1.2...v0.1.3) - 2025-09-12

### Changed

- Rejiggered the visuals of the login card.
- Spruced up the settings page.
- Simplified settings page backend to avoid extra network calls.
- Pinned footer to the bottom of the viewport.

### Removed

- Removed all notification toasts and all code remnants thereof.

## [0.1.2](https://github.com/anson-vandoren/pagepouch-rs/compare/v0.1.1...v0.1.2) - 2025-09-12

### Removed

- Removed the success toast message when creating a new link.
- Adding a new link will now also try to scrape a description.

### Changed

- Made title/description lookup much faster when creating a new link.

### Fixed

- Fixed incorrect bookmark sorting (should be by bookmark creation date/time).

## [0.1.1](https://github.com/anson-vandoren/pagepouch-rs/compare/v0.1.0...v0.1.1) - 2025-09-12

### Added

- Health check endpoint for monitoring.
- Pressing Ctrl+Enter (or Cmd+Enter on macOS) on the `Add Link` page will now submit the form.

### Changed

- When creating a new link, if a URL without https/http is entered it will be automatically guessed and corrected.
- URL input field is automatically selected when creating a new link.

## [0.1.0](https://github.com/anson-vandoren/pagepouch-rs.git) - 2025-09-11

### Added

- The MVP(ish) PagePouch app is now available [on the web](https://pagepouch.com).
- Basic login works (for a default user).
- Bookmarks can be added:
  - Titles are auto-scraped from the URL.
  - Description is optional and freeform.
  - Tags can be added.
- Bookmarks can be filtered:
  - By tags, which must exist and must be exact match.
  - Tags are AND'ed with each other and with any general query.
  - General queries are OR'ed by default within the search terms.
  - General query terms can be `and` or `AND` to override the default OR behavior.
  - Quoted terms `"like this"` require exact match, non-quoted terms are more like-ish match.
- Available tags show up in a sidebar column.
  - Clicking a tag adds it as a filter.
  - Tags that don't match any currently-filtered bookmarks are hidden in this column.
  - Clicking on an active tag (highlighted above the rest of the tags) will remove the tag from the active filter.
  - Clicking on the Tags or Bookmarks header will clear all existing filters/queries.
