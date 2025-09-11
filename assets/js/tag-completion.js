/**
 * Tag completion functionality for PagePouch search
 *
 * Implements sophisticated tag completion behavior with:
 * - Fuzzy matching dropdown suggestions
 * - Keyboard navigation (arrows, tab/shift-tab, enter, escape)
 * - Space-commit for exact matches only
 * - Tag validity checking and error styling
 * - Proper cursor management
 * - Distinction between committed and uncommitted tags
 */

class TagCompletion {
  constructor() {
    this.currentTagPosition = -1;
    this.tagSuggestions = [];
    this.selectedSuggestionIndex = -1;

    // Cached DOM elements
    this.searchInput = null;
    this.suggestionsDiv = null;
    this.suggestionsList = null;
    this.tagColumn = null;
    this.activeTagsContainer = null;
    this.inactiveTagsContainer = null;
    this.bookmarkContent = null;

    /**
     * Whether the tag suggestions dropdown has been explicitly hidden by the user via <Esc> press
     */
    this.isDropdownCanceled = false;
    this.debounceTimeout = null;
    this.searchTimeout = null;
    this.committedTags = new Set(); // Track committed tags separately

    // Prevent race conditions with operation locks
    this.isUpdatingTags = false;
  }

  init() {
    // Cache all DOM elements upfront for better performance
    this.searchInput = document.getElementById('bookmark-search');
    this.suggestionsDiv = document.getElementById('tag-suggestions');
    this.suggestionsList = document.getElementById('tag-suggestions-list');
    this.tagColumn = document.getElementById('tag-column');
    this.activeTagsContainer = document.getElementById('active-tags');
    this.inactiveTagsContainer = document.getElementById('inactive-tags');
    this.bookmarkContent = document.getElementById('bookmark-content');

    if (!this.searchInput) return;

    this.bindEvents();
    this.setupTagColumnEventDelegation();
    this.setupBookmarkTagsEventDelegation();
  }

  /**
   * Set up event delegation for tag column to handle clicks and keyboard interactions
   * This approach is more performant, prevents memory leaks, and supports accessibility
   */
  setupTagColumnEventDelegation() {
    if (!this.tagColumn) return;

    // Handle click events
    this.tagColumn.addEventListener('click', (e) => {
      if (e.target.classList.contains('tag-list-item')) {
        e.preventDefault();
        this.toggleTagState(e.target);
      }
    });

    // Handle keyboard events for accessibility
    this.tagColumn.addEventListener('keydown', (e) => {
      if (e.target.classList.contains('tag-list-item')) {
        if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault();
          this.toggleTagState(e.target);
        }
      }
    });
  }

  /**
   * Set up event delegation for bookmark item tags to handle clicks and keyboard interactions
   * This handles tags within bookmark items using the same consistent approach
   */
  setupBookmarkTagsEventDelegation() {
    if (!this.bookmarkContent) return;

    // Handle click events on bookmark tags
    this.bookmarkContent.addEventListener('click', (e) => {
      if (e.target.classList.contains('tag')) {
        e.preventDefault();
        const tagName = e.target.textContent.trim();
        this.addCommittedTag(tagName);
      }
    });

    // Handle keyboard events for bookmark tags accessibility
    this.bookmarkContent.addEventListener('keydown', (e) => {
      if (e.target.classList.contains('tag')) {
        if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault();
          const tagName = e.target.textContent.trim();
          this.addCommittedTag(tagName);
        }
      }
    });
  }

  /**
   * Toggle a tag between active and inactive states
   * @param {HTMLElement} tagElement - The tag element to toggle
   */
  toggleTagState(tagElement) {
    const tagName = tagElement.textContent.trim();

    if (tagElement.classList.contains('tag-list-active')) {
      // Active tag clicked - remove it
      this.removeCommittedTag(tagName);
    } else {
      // Inactive tag clicked - add it
      this.addCommittedTag(tagName);
    }
  }

  bindEvents() {
    // Handle input events for tag suggestions and search updates
    this.searchInput.addEventListener('input', (event) => {
      this.debouncedHandleTagInput();
      // Also trigger search with committed tags after input changes
      this.debouncedTriggerSearch();
    });

    // Handle keyboard navigation on keydown for immediate response
    this.searchInput.addEventListener('keydown', (event) => {
      this.handleKeyNavigation(event);
    });

    this.searchInput.addEventListener('blur', (event) => {
      // Won't get an Esc key event if the search input is selected, so key on blur instead
      // Small delay to let click events on suggestions fire first
      setTimeout(() => this.handleEscapeKey(event), 100);
    });

    this.searchInput.addEventListener('click', (event) => {
      this.debouncedHandleTagInput();
    });

    // Hide suggestions when clicking outside
    document.addEventListener('click', (event) => {
      if (!event.target.closest('#bookmark-search') && !event.target.closest('#tag-suggestions')) {
        this.hideDropdown();
      }
    });

    // Listen for header clicks to clear all committed tags
    document.addEventListener('clearAllCommittedTags', (event) => {
      this.clearAllCommittedTags();
    });

    // Listen for HTMX events to update highlighting when bookmark content changes
    document.addEventListener('htmx:afterSwap', (event) => {
      // Check if the bookmark content was updated
      if (event.target && event.target.id === 'bookmark-content') {
        // Re-cache the bookmark content element if it was replaced
        this.bookmarkContent = document.getElementById('bookmark-content');
        // Update highlighting for all currently active tags
        this.updateBookmarkTagHighlighting();
      }
    });
  }

  debouncedHandleTagInput() {
    clearTimeout(this.debounceTimeout);
    this.debounceTimeout = setTimeout(() => {
      this.handleTagInput();
    }, 300);
  }

  debouncedTriggerSearch() {
    clearTimeout(this.searchTimeout);
    this.searchTimeout = setTimeout(() => {
      this.triggerSearchWithCommittedTags();
    }, 300);
  }

  async handleTagInput() {
    const cursorPos = this.searchInput.selectionStart || 0;
    const tagInfo = this.extractCurrentTag(cursorPos);

    // If we went outside a tag context, let the dropdown show again when we're back in
    if (!tagInfo) {
      this.isDropdownCanceled = false;
    }

    if (tagInfo && !this.isDropdownCanceled) {
      await this.showTagSuggestions(tagInfo);
    } else {
      this.hideDropdown();
    }
  }

  /**
   * Extract current tag being typed from cursor position
   * Returns null if not in tag context
   *
   * @returns {TagInfo|null}
   */
  extractCurrentTag(cursorPos) {
    const text = this.searchInput.value;

    // Find the last # before cursor position
    let tagStart = -1;
    // Check backwards from current cursor position. We are in a tag context
    // iff we find a `#` that is at the start or after a space, and there are
    // no spaces between `cursorPos` and that `#`
    for (let i = cursorPos - 1; i >= 0; i--) {
      if (text[i] === '#') {
        // Check if this # is at start or after space (valid tag start)
        if (i === 0 || text[i - 1] === ' ') {
          tagStart = i;
          break;
        }
      }
      if (text[i] === ' ') {
        break; // Space breaks the tag context
      }
    }

    if (tagStart === -1) return null;

    // Find the end of the tag (space or end of string)
    let tagEnd = cursorPos;
    for (let i = cursorPos; i < text.length; i++) {
      if (text[i] === ' ') {
        tagEnd = i;
        break;
      }
    }

    const tagText = text.substring(tagStart + 1, tagEnd);

    // Only show suggestions if tag has at least 1 character after #
    if (tagText.length < 1) return null;

    return {
      text: tagText,
      start: tagStart,
      end: tagEnd,
    };
  }

  /**
   * Fetch tag suggestions from API
   */
  async fetchTagSuggestions(query) {
    try {
      const tags = Array.from(this.committedTags);
      const params = new URLSearchParams({ q: query });

      // Add committed tags to filter the suggestions
      tags.forEach((tag) => params.append('tags', tag));

      const response = await fetch(`/api/tags/autocomplete?${params.toString()}`);
      if (response.ok) {
        const suggestions = await response.json();
        // Filter out tags that are already active in the filter
        let res = suggestions.filter((suggestion) => !this.committedTags.has(suggestion.name));
        return res;
      }
    } catch (error) {
      console.error('Failed to fetch tag suggestions:', error);
    }
    return [];
  }

  /**
   * Show tag suggestions dropdown
   * @param {TagInfo} tagInfo
   */
  async showTagSuggestions(tagInfo) {
    this.currentTagPosition = tagInfo.start;

    this.tagSuggestions = await this.fetchTagSuggestions(tagInfo.text);

    if (this.tagSuggestions.length === 0) {
      this.hideDropdown();
      return;
    }

    this.renderSuggestions();
    this.showDropdown();

    // No initial selection per spec
    this.selectedSuggestionIndex = -1;
    this.updateSuggestionSelection();
  }

  /**
   * Render suggestion items in the dropdown
   */
  renderSuggestions() {
    this.suggestionsList.innerHTML = '';

    this.tagSuggestions.forEach((suggestion, index) => {
      const item = document.createElement('div');
      item.className = 'tag-suggestion-item';
      item.textContent = suggestion.name;

      item.addEventListener('click', (event) => {
        event.preventDefault();
        event.stopPropagation();
        this.commitTagSuggestion(suggestion.name);
      });

      this.suggestionsList.appendChild(item);
    });
  }

  /**
   * Show the dropdown
   */
  showDropdown() {
    this.suggestionsDiv.style.top = 'calc(100% + 2px)';
    this.suggestionsDiv.style.left = '0';
    this.suggestionsDiv.style.width = '100%';
    this.suggestionsDiv.style.display = 'block';
    this.isDropdownCanceled = false;
  }

  /**
   * Hide the dropdown
   */
  hideDropdown() {
    this.suggestionsDiv.style.display = 'none';
    this.selectedSuggestionIndex = -1;
    // Reset suggestions
    this.tagSuggestions.length = 0;
  }

  /**
   * Check if a key is a navigation key that shouldn't trigger input updates
   */
  isNavigationKey(key) {
    return ['ArrowDown', 'ArrowUp', 'Tab', 'Enter', 'Escape'].includes(key);
  }

  /**
   * Check if dropdown is currently visible
   */
  get isDropdownVisible() {
    return this.suggestionsDiv.style.display !== 'none';
  }

  /**
   * Handle keyboard navigation in dropdown
   */
  async handleKeyNavigation(event) {
    const hasDropdown = this.isDropdownVisible;

    switch (event.key) {
      case 'ArrowDown':
        if (hasDropdown) {
          event.preventDefault();
          this.navigateDown();
        }
        break;
      case 'ArrowUp':
        if (hasDropdown) {
          event.preventDefault();
          this.navigateUp();
        }
        break;
      case 'Tab':
        if (!hasDropdown) {
          this.isDropdownCanceled = false;
          this.showDropdown();
        }
        event.preventDefault();
        if (!event.shiftKey) {
          this.navigateDown();
        } else {
          this.navigateUp();
        }
        break;
      case 'Enter':
        await this.handleEnterKey(event);
        break;
      case 'Escape':
        console.warn('Unexpectedly got Esc keypress inside search input listener.');
        break;
      case ' ':
        await this.handleSpaceKey(event);
        break;
    }
  }

  /**
   * Navigate down in suggestions
   */
  navigateDown() {
    if (this.selectedSuggestionIndex < 0) {
      this.selectedSuggestionIndex = 0;
    } else {
      this.selectedSuggestionIndex =
        (this.selectedSuggestionIndex + 1) % this.tagSuggestions.length;
    }
    this.updateSuggestionSelection();
    this.updateTagTextFromSelection();
  }

  /**
   * Navigate up in suggestions
   */
  navigateUp() {
    if (this.selectedSuggestionIndex < 0) {
      this.selectedSuggestionIndex = this.tagSuggestions.length - 1;
    } else {
      this.selectedSuggestionIndex = this.selectedSuggestionIndex - 1;
      if (this.selectedSuggestionIndex < 0) {
        this.selectedSuggestionIndex = this.tagSuggestions.length - 1;
      }
    }
    this.updateSuggestionSelection();
    this.updateTagTextFromSelection();
  }


  /**
   * Handle Enter key - commit selected tag
   */
  async handleEnterKey(event) {
    if (this.selectedSuggestionIndex >= 0) {
      event.preventDefault();
      const selectedTag = this.tagSuggestions[this.selectedSuggestionIndex];
      this.commitTagSuggestion(selectedTag.name);
    } else {
      // What we have may still be a valid tag which we should commit
      await this.commitIfValidTag(event);
    }
  }

  /**
   * Handle Escape key - unselect or hide
   */
  handleEscapeKey(_event) {
    const hasSelection = this.selectedSuggestionIndex >= 0;
    const isDropdownOpen = this.isDropdownVisible;

    if (hasSelection) {
      // First escape unselects
      this.selectedSuggestionIndex = -1;
      this.updateSuggestionSelection();
    } else if (isDropdownOpen) {
      // Second escape hides dropdown
      this.hideDropdown();
      this.isDropdownCanceled = true; // Prevent re-showing until new # typed
    } else {
      // Nothing to do if we don't have a selection and the dropdown isn't open, so avoid preventing blur/refocusing
      return;
    }
    this.searchInput.focus();
  }

  /**
   * Handle Space key - commit if exact match exists
   */
  async handleSpaceKey(event) {
    await this.commitIfValidTag(event);
  }

  async commitIfValidTag(event) {
    const cursorPos = this.searchInput.selectionStart || 0;
    const tagInfo = this.extractCurrentTag(cursorPos);

    if (!tagInfo) return;

    // Check if current tag text matches any suggestion exactly
    if (this.tagSuggestions.length === 0) {
      this.tagSuggestions = await this.fetchTagSuggestions(tagInfo.text);
    }
    const exactMatch = this.tagSuggestions.find((s) => s.name === tagInfo.text);

    // If this came from a space keypress, and the cursor has advanced, bring it back
    if ((this.searchInput.selectionStart || 0) > cursorPos) {
      this.searchInput.setSelectionRange(cursorPos, cursorPos);
    }
    if (exactMatch) {
      event.preventDefault();
      this.commitTagSuggestion(exactMatch.name);
    } else {
      // Invalid tag - show error styling
      this.showTagError(tagInfo);
    }
  }

  /**
   * Update tag text in input while navigating (live replacement)
   */
  updateTagTextFromSelection() {
    if (this.selectedSuggestionIndex >= 0) {
      const selectedTag = this.tagSuggestions[this.selectedSuggestionIndex];
      if (selectedTag) {
        this.replaceCurrentTagText(selectedTag.name);
      }
    }
  }

  /**
   * Replace the current tag text in the input
   */
  replaceCurrentTagText(newTagText) {
    const cursorPos = this.searchInput.selectionStart || 0;
    const tagInfo = this.extractCurrentTag(cursorPos);

    if (tagInfo) {
      const newValue =
        this.searchInput.value.substring(0, tagInfo.start + 1) +
        newTagText +
        this.searchInput.value.substring(tagInfo.end);

      this.searchInput.value = newValue;

      // Keep cursor at end of replaced tag
      const newCursorPos = tagInfo.start + 1 + newTagText.length;
      this.searchInput.setSelectionRange(newCursorPos, newCursorPos);
    }
  }

  /**
   * Commit a tag suggestion (remove from input, add to committed tags)
   */
  commitTagSuggestion(tagName) {
    const cursorPos = this.searchInput.selectionStart || 0;
    const tagInfo = this.extractCurrentTag(cursorPos);

    if (tagInfo) {
      // Remove the incomplete tag from input
      const beforeTag = this.searchInput.value.substring(0, tagInfo.start);
      const afterTag = this.searchInput.value.substring(tagInfo.end);
      const newValue = (beforeTag + afterTag).replace(/\s+/g, ' ').trim();

      this.searchInput.value = newValue;

      // Position cursor where the tag was removed
      this.searchInput.setSelectionRange(tagInfo.start, tagInfo.start);

      // Add to committed tags
      this.addCommittedTag(tagName);

      this.hideDropdown();
      this.clearTagError();

      // Update the search with committed tags included
      this.triggerSearchWithCommittedTags();
    }
  }

  /**
   * Trigger search with committed tags included in the query
   */
  triggerSearchWithCommittedTags() {
    if (typeof htmx === 'undefined') return;

    // Strip out any incomplete tag syntax from search input
    // Only committed tags should affect search, not tags being typed
    const searchTerms = this.stripIncompleteTagSyntax(this.searchInput.value.trim());
    let tags = Array.from(this.committedTags);

    // Only skip search if we have incomplete tag syntax in the input
    // If input is completely empty (no search terms, no committed tags), we should show all bookmarks
    if (searchTerms.length === 0 && tags.length === 0 && this.hasIncompleteTagSyntax(this.searchInput.value.trim())) return;

    // Use HTMX to make request with custom parameters
    htmx.ajax('GET', '/api/bookmarks', {
      values: { q: searchTerms, tags },
      target: '#bookmark-content',
      swap: 'innerHTML',
    });

    // Also refresh the tag column to show only relevant tags
    this.refreshTagColumn();
  }

  /**
   * Refresh the tag column to show only tags relevant to current active filters
   */
  refreshTagColumn() {
    if (typeof htmx === 'undefined') return;

    const tags = Array.from(this.committedTags);

    // Use HTMX to refresh the tag column with current active tags
    htmx.ajax('GET', '/api/tags', {
      values: { tags },
      target: '#tag-column',
      swap: 'innerHTML',
    });
  }

  /**
   * Strips out incomplete tag syntax from input string.
   * Since only committed tags should affect search, any #tag still in the input
   * should be removed before sending to backend.
   */
  stripIncompleteTagSyntax(input) {
    // Remove any #word patterns and clean up extra spaces
    return input.replace(/#\S*/g, '').replace(/\s+/g, ' ').trim();
  }

  /**
   * Check if input contains incomplete tag syntax (#word patterns)
   */
  hasIncompleteTagSyntax(input) {
    return /#\S+/.test(input);
  }

  /**
   * Show error styling for invalid tag
   */
  showTagError(tagInfo) {
    // Add error styling to the tag text
    this.searchInput.classList.add('tag-error');

    // Remove error after a delay
    setTimeout(() => {
      this.clearTagError();
    }, 2000);
  }

  /**
   * Clear tag error styling
   */
  clearTagError() {
    this.searchInput.classList.remove('tag-error');
  }

  /**
   * Update visual selection in suggestions
   */
  updateSuggestionSelection() {
    const items = document.querySelectorAll('.tag-suggestion-item');
    if (this.selectedSuggestionIndex < 0 || this.selectedSuggestionIndex >= items.length) {
      items.forEach((item) => item.classList.remove('selected'));
      return;
    }

    // Try to scroll the container such that the selected item is in the middle
    const selectedItem = items[this.selectedSuggestionIndex];
    if (selectedItem) {
      const top =
        selectedItem.offsetTop -
        this.suggestionsDiv.clientHeight / 2 +
        selectedItem.offsetHeight / 2;
      this.suggestionsDiv.scroll({
        top,
        behavior: 'smooth',
        left: 0,
      });
    }

    items.forEach((item, index) => {
      if (index === this.selectedSuggestionIndex) {
        item.classList.add('selected');
      } else {
        item.classList.remove('selected');
      }
    });
  }

  /**
   * Remove a specific committed tag (called when clicking on an active filter tag)
   * @param {string} tagToRemove - Name of the tag to remove from filters
   */
  removeCommittedTag(tagToRemove) {
    if (this.isUpdatingTags || !this.committedTags.has(tagToRemove)) return;

    this.isUpdatingTags = true;
    try {
      this.committedTags.delete(tagToRemove);
      this.unhighlightBookmarkTags(tagToRemove);
      this.triggerSearchWithCommittedTags();
    } finally {
      this.isUpdatingTags = false;
    }
  }

  /**
   * Clear all committed tags (called when clicking on Tags or Bookmarks header)
   */
  clearAllCommittedTags() {
    if (this.isUpdatingTags || this.committedTags.size === 0) return;

    this.isUpdatingTags = true;
    try {
      this.committedTags.clear();
      this.unhighlightBookmarkTags();
      this.triggerSearchWithCommittedTags();
    } finally {
      this.isUpdatingTags = false;
    }
  }

  /**
   * Add a tag to the committed filters and move it to the active section
   * @param {string} tagName - Name of the tag to add to filters
   */
  addCommittedTag(tagName) {
    if (this.isUpdatingTags || this.committedTags.has(tagName)) return;

    this.isUpdatingTags = true;
    try {
      this.committedTags.add(tagName);
      this.highlightMatchingBookmarkTags(tagName);
      this.triggerSearchWithCommittedTags();
    } finally {
      this.isUpdatingTags = false;
    }
  }


  /**
   * Highlight or unhighlight bookmark tags
   * @param {string|null} tagName - Tag to highlight/unhighlight (null for all)
   * @param {boolean} highlight - True to highlight, false to unhighlight
   */
  updateBookmarkTagHighlight(tagName = null, highlight = true) {
    if (!this.bookmarkContent) {
      console.warn('Bookmark content container not found');
      return;
    }

    if (highlight) {
      const tagsToHighlight = tagName ? [tagName] : Array.from(this.committedTags);
      tagsToHighlight.forEach((tag) => {
        this.bookmarkContent.querySelectorAll('.tag').forEach((tagElement) => {
          if (tagElement.textContent.trim() === tag) {
            tagElement.classList.add('tag-highlighted');
          }
        });
      });
    } else {
      const selector = tagName ? '.tag' : '.tag.tag-highlighted';
      this.bookmarkContent.querySelectorAll(selector).forEach((tagElement) => {
        if (!tagName || tagElement.textContent.trim() === tagName) {
          tagElement.classList.remove('tag-highlighted');
        }
      });
    }
  }

  /**
   * Highlight bookmark tags (convenience method)
   * @param {string} tagName - Name of the tag to highlight (optional - highlights all if not provided)
   */
  highlightMatchingBookmarkTags(tagName = null) {
    this.updateBookmarkTagHighlight(tagName, true);
  }

  /**
   * Remove highlighting from bookmark tags (convenience method)
   * @param {string} tagName - Name of the tag to unhighlight (optional - removes all if not provided)
   */
  unhighlightBookmarkTags(tagName = null) {
    this.updateBookmarkTagHighlight(tagName, false);
  }

  /**
   * Update bookmark tag highlighting to match current committed tags
   * Called when bookmark content is refreshed via HTMX
   */
  updateBookmarkTagHighlighting() {
    // Clear all existing highlights first
    this.unhighlightBookmarkTags();

    // Apply highlights for all committed tags
    if (this.committedTags.size > 0) {
      this.highlightMatchingBookmarkTags();
    }
  }
}

// Initialize tag completion when DOM is ready
document.addEventListener('DOMContentLoaded', function () {
  const tagCompletion = new TagCompletion();
  tagCompletion.init();
});

/** @typedef {{text: string, start: number, end: number}} TagInfo */
