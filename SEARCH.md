# Search Feature Enhancement PRD

## Overview

Enhance the current basic search functionality with advanced query parsing, tag filtering, and improved UX.

## Current State

- Basic search across URL, title, description, and tags
- Simple `%search_term%` pattern matching
- Single search input with 300ms delay

## Enhanced Requirements

### 1. Query Parsing Logic

#### Space Handling (OR by default)

- **Input**: `rust programming`
- **Behavior**: Search for bookmarks containing "rust" OR "programming" in any field
- **Database**: `WHERE (field LIKE '%rust%' OR field LIKE '%programming%')`

#### AND Override

- **Input**: `rust AND programming`
- **Behavior**: Search for bookmarks containing both "rust" AND "programming"
- **Case**: Both `AND` and `and` should work
- **Database**: `WHERE (field LIKE '%rust%') AND (field LIKE '%programming%')`

#### Quoted Strings

- **Input**: `"web development" rust`
- **Behavior**: Search for exact phrase "web development" OR "rust"
- **Quotes**: Both single (`'web development'`) and double (`"web development"`) quotes supported

### 2. Tag-Specific Search

#### Tag Syntax

- **Simple tag**: `#rust` - searches only in tags for "rust"
- **Spaced tag**: `#"my tag"` - searches only in tags for exact "my tag"
- **Mixed query**: `#rust web development` - (tag search for "rust") AND (general search for "web" OR "development")
- **Multiple tags**: `#rust #axum web development` - (tag search for "rust") AND (tag search for "axum") AND
  (general search for "web" or "development")

#### Tag Completion Behavior

- **Space after tag**: `#rust ` (space) - commits the tag filter
- **Tab after tag**: `#rust<TAB>` - commits the tag filter
- **Space after quoted tag**: `#"my tag" ` - commits the tag filter
- **Tab after quoted tag**: `#"my tag"<TAB>` - commits the tag filter

### 3. UI Enhancements

#### Tag Filter Display

- **Location**: Below search bar/header area
- **Appearance**: Each active tag in its own styled box/pill
- **Interaction**: Click to remove from query
- **Example**: `[× rust] [× web development]`

#### Search Input Behavior

- **Real-time parsing**: As user types, parse and update UI
- **Tag highlighting**: Show committed tags below search bar
- **Input clearing**: When tag is committed, remove from input but keep in filter display

## Technical Implementation Decisions

1. **Query Parser**: Custom parser for full control over `#tag` syntax and tag pills integration
2. **Tag Autocompletion**: Fuzzy tag auto-complete using `fuzzy-matcher` crate
3. **Search History**: Future enhancement, not implemented in this phase
4. **URL Updates**: Search state will be reflected in the URL for bookmarking
5. **Database Query**: "Good enough" performance approach - single user, 100-5000 bookmarks, optimize as needed
6. **Mobile UX**: Separate epic, not a priority for this implementation
7. **Tag Normalization**: Normalize to lowercase before saving to database, no migration needed
8. **Fuzzy Matching**: Use `fuzzy-matcher` crate for tag autocompletion

## Example Use Cases

### Case 1: Basic OR Search

- **Input**: `javascript react`
- **Parsed**: `["javascript", "react"]` (OR)
- **Query**: Find bookmarks with "javascript" OR "react" in any field

### Case 2: AND Search

- **Input**: `javascript AND react`
- **Parsed**: `["javascript", "react"]` (AND)
- **Query**: Find bookmarks with both "javascript" AND "react" in any field

### Case 3: Mixed Tag + General Search

- **Input**: `#tutorial javascript`
- **Parsed**: Tag filter: `["tutorial"]`, General: `["javascript"]`
- **Query**: Find bookmarks tagged "tutorial" AND containing "javascript" in any field

### Case 4: Complex Query

- **Input**: `#"web dev" #react "best practices" AND performance`
- **Parsed**:
  - Tags: `["web dev", "react"]`
  - General: `["best practices", "performance"]` (AND)
- **Query**: (Tagged "web dev" AND "react") AND (containing both "best practices" AND "performance")

## Technical Requirements

- In every case possible, stick to sqlx compile-time queries. In cases where that is not possible or would
  negatively impact performance in a meaningful way, or is otherwise overly cumbersome, add a justification comment.

## Implementation Phases

### Phase 1: Basic Query Parser ✅

- ✅ OR/AND logic for general terms
- ✅ Quoted string support (single and double quotes)
- ✅ Foundation for more complex parsing
- ✅ OR keyword recognition (case-insensitive)

### Phase 2: Tag Syntax & UI ✅

- ✅ `#tag` parsing and filtering
- ✅ Tag pill UI component below search bar
- ✅ Tag removal by clicking pills
- ✅ Fuzzy LIKE matching for tag searches
- ✅ Tag cloud click integration
- ✅ Bookmark tag click integration
- ✅ Tag pill styling and alignment

### Phase 3: Fuzzy Tag Autocompletion ✅

- ✅ `fuzzy-matcher` crate integration with SkimMatcherV2
- ✅ Real-time tag suggestions API endpoint
- ✅ Dropdown UI with proper positioning
- ✅ Fuzzy matching with scoring and ranking
- ✅ Keyboard navigation (arrows, tab/shift-tab, enter, escape)
- ✅ Mouse click selection
- ✅ Auto-hiding when clicking outside
- ✅ Integration with existing search system

### Phase 4: URL State Management 🔄

- Reflect search state in URL
- Bookmarkable search queries
- Back/forward navigation support

## Status Legend

- ⏳ In Progress
- ✅ Complete
- 🔄 Planned

## Tag Completion

The goal is to assist users in finding existing tags when they start typing a tag name, even if they can't remember
exactly what tags they have used before, hence fuzzy-matching.

- In the search box, tag completion is only active while a `#` followed by at least one alphanumeric character
  or underscore precede the current cursor position.
  - When tag completion is active, the UI shall show a dropdown of existing tags that fuzzily match the string between
    `#` and the current cursor position.
  - The dropdown shall show one suggested tag per row, with no header.
  - The results shall be displayed in descending order by fuzzy-match score from the matching crate.
  - Immediately when the dropdown is displayed, no result row shall be selected.
  - When no result row is selected, and the user presses the `<Tab>` key:
    - The first result in the dropdown shall appear "selected".
    - The current tag that the user is editing in the search box shall be replaced with the text of the now-selected tag, but not committed
  - While the dropdown is active and a selection is made, `<tab>` or the down arrow shall move the selection to the next/lower tag, while
    `<shift-tab>` or the up arrow shall move the selection to the previous/higher tag.
    - Selection shall wrap from top or bottom of the list.
    - When the selection changes, the currently-edited tag shall be updated to have the text of the selection
  - Pressing `<Enter>` while the dropdown has a row selected shall "commit" the tag on that row, meaning that it is added to the tag filter list and removed from the input box text.
  - Pressing `<Space>` when the user has typed a full tag that is valid (i.e., appears in the current dropdown results) shall commit that tag,
    remove it from the search input, add it to the tag filter list, and position the cursor where the `#` was.
  - While the dropdown is displayed, pressing `<Esc>` once shall "unselect" the row in the dropdown. Pressing `<Esc>` from the state
    in which there is no dropdown selection shall hide the dropdown, maintaining focus and position in the input box.
  - Pressing `<Enter>` while the dropdown is visible but there is no currently selected row (i.e., after a single `<Esc>`) shall have no effect.
  - If the dropdown has been hidden with `<Esc>` it shall not be re-displayed until a new `#` is typed in a position that
    is valid for a tag start (i.e., at the beginning of input or after a `<Space>` character).
- When the user attempts to commit a tag (via `<Enter>` or `<Space>`) but the current tag text does not appear in the dropdown results:
  - No tag shall be added to the tag filter.
  - The tag text (`#` through the tag name) shall be colored red to indicate the error.
  - The cursor position shall remain unchanged.
  - It is not legal to filter on a tag that does not exist.
- Tag validity is determined by presence in dropdown results: if a tag appears in fuzzy-match results, it exists and can be committed.
- When a tag is not "committed", it shall not be used to filter results. Only tags that are committed, removed from the search input,
  and added to the tag filter list shall be used to filter results.

## TODOs found during dev

For each of these bugs, create one or more test case(s), verify failing, fix, verify passing.

- `api and web` finds links that have tags `api` AND (`web-dev` OR `web-framework`), which is correct, however
  `api and web-` finds links that have tags `api` AND `web-dev`, but not links that have `api` AND `web-framework`,
  which is perplexing and incorrect. Even more confusingly, `rust and web-` correctly finds both cases :thinking:.
  This is still broken even with `test_partial_tag_search_bug`.
- Search terms inside of quotes (single or double) should only match exactly, not fuzzy-matching
- Removing via clear-all button a hyphenated tag leaves the second part of the hyphen term in the search box

- Shift-tabbing back up the list changes the input which triggers a re-search which narrows the list to one
- `<Esc>` the second time does not close the suggestion dropdown
