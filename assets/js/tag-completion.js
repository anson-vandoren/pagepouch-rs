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
    this.currentTagInput = '';
    this.currentTagPosition = -1;
    this.tagSuggestions = [];
    this.selectedSuggestionIndex = -1;
    this.searchInput = null;
    this.suggestionsDiv = null;
    this.suggestionsList = null;
    /**
     * Whether the tag suggestions dropdown has been explicitly hidden by the user via <Esc> press
     */
    this.isDropdownCanceled = false;
    this.debounceTimeout = null;
    this.searchTimeout = null;
    this.committedTags = new Set(); // Track committed tags separately
  }

  init() {
    /** @type {HTMLInputElement} */
    this.searchInput = document.getElementById('bookmark-search');
    this.suggestionsDiv = document.getElementById('tag-suggestions');
    this.suggestionsList = document.getElementById('tag-suggestions-list');

    if (!this.searchInput) return;

    this.bindEvents();
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

    // Handle text input changes on keyup (after character is entered)
    this.searchInput.addEventListener('keyup', (event) => {
      // Only update suggestions if we didn't handle navigation
      if (!this.isNavigationKey(event.key)) {
        this.debouncedHandleTagInput();
      }
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

    // Listen for tag pill removal events
    document.addEventListener('removeCommittedTag', (event) => {
      this.removeCommittedTag(event.detail.tag);
    });

    document.addEventListener('clearAllCommittedTags', (event) => {
      this.clearAllCommittedTags();
    });

    document.addEventListener('commitTag', (event) => {
      this.addTag(event.detail.tag);
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
      const response = await fetch(`/api/tags/autocomplete?q=${encodeURIComponent(query)}`);
      if (response.ok) {
        return await response.json();
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
    this.currentTagInput = tagInfo.text;
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
      item.dataset.index = index;

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
    this.isDropdownCanceled = false; // Reset for next tag
  }

  /**
   * Check if a key is a navigation key that shouldn't trigger input updates
   */
  isNavigationKey(key) {
    return ['ArrowDown', 'ArrowUp', 'Tab', 'Enter', 'Escape'].includes(key);
  }

  /**
   * Handle keyboard navigation in dropdown
   */
  handleKeyNavigation(event) {
    if (this.suggestionsDiv.style.display === 'none') {
      return false;
    }

    switch (event.key) {
      case 'ArrowDown':
        event.preventDefault();
        this.navigateDown();
        return true;

      case 'ArrowUp':
        event.preventDefault();
        this.navigateUp();
        return true;

      case 'Tab':
        return this.handleTabNavigation(event);

      case 'Enter':
        return this.handleEnterKey(event);

      case 'Escape':
        return this.handleEscapeKey(event);

      case ' ':
        return this.handleSpaceKey(event);
    }

    return false;
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
   * Handle Tab/Shift+Tab navigation
   */
  handleTabNavigation(event) {
    if (!event.shiftKey) {
      // Tab forward
      if (this.selectedSuggestionIndex < 0) {
        // First tab selects first item
        event.preventDefault();
        this.selectedSuggestionIndex = 0;
        this.updateSuggestionSelection();
        this.updateTagTextFromSelection();
        return true;
      } else if (this.selectedSuggestionIndex < this.tagSuggestions.length - 1) {
        // Navigate to next item
        event.preventDefault();
        this.selectedSuggestionIndex++;
        this.updateSuggestionSelection();
        this.updateTagTextFromSelection();
        return true;
      }
      // At end, let normal tab behavior continue
    } else {
      // Shift+Tab backward
      if (this.selectedSuggestionIndex > 0) {
        event.preventDefault();
        this.selectedSuggestionIndex--;
        this.updateSuggestionSelection();
        this.updateTagTextFromSelection();
        return true;
      } else if (this.selectedSuggestionIndex === 0) {
        // At beginning, deselect and hide
        event.preventDefault();
        this.selectedSuggestionIndex = -1;
        this.updateSuggestionSelection();
        this.hideDropdown();
        return false; // Let normal tab behavior take over
      }
    }
    return false;
  }

  /**
   * Handle Enter key - commit selected tag
   */
  handleEnterKey(event) {
    if (this.selectedSuggestionIndex >= 0) {
      event.preventDefault();
      const selectedTag = this.tagSuggestions[this.selectedSuggestionIndex];
      this.commitTagSuggestion(selectedTag.name);
      return true;
    }
    return false;
  }

  /**
   * Handle Escape key - unselect or hide
   */
  handleEscapeKey(event) {
    event.preventDefault();

    if (this.selectedSuggestionIndex >= 0) {
      // First escape unselects
      this.selectedSuggestionIndex = -1;
      this.updateSuggestionSelection();
      this.restoreOriginalTagText();
    } else {
      // Second escape hides dropdown
      this.hideDropdown();
      this.isDropdownCanceled = true; // Prevent re-showing until new # typed
    }
    return true;
  }

  /**
   * Handle Space key - commit if exact match exists
   */
  handleSpaceKey(event) {
    const cursorPos = this.searchInput.selectionStart || 0;
    const tagInfo = this.extractCurrentTag(cursorPos);

    if (!tagInfo) return false;

    // Check if current tag text matches any suggestion exactly
    const exactMatch = this.tagSuggestions.find((s) => s.name === tagInfo.text);

    if (exactMatch) {
      event.preventDefault();
      this.commitTagSuggestion(exactMatch.name);
      return true;
    } else {
      // Invalid tag - show error styling
      this.showTagError(tagInfo);
      return false;
    }
  }

  /**
   * Update tag text in input while navigating (live replacement)
   */
  updateTagTextFromSelection() {
    if (this.selectedSuggestionIndex >= 0) {
      const selectedTag = this.tagSuggestions[this.selectedSuggestionIndex];
      this.replaceCurrentTagText(selectedTag.name);
    }
  }

  /**
   * Restore original tag text after escape
   */
  restoreOriginalTagText() {
    this.replaceCurrentTagText(this.currentTagInput);
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
      this.committedTags.add(tagName);

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

    // Use HTMX to make request with custom parameters
    htmx.ajax('GET', '/api/bookmarks', {
      values: { q: searchTerms, tags },
      target: '#bookmark-content',
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

    items.forEach((item, index) => {
      if (index === this.selectedSuggestionIndex) {
        item.classList.add('selected');
      } else {
        item.classList.remove('selected');
      }
    });
  }

  /**
   * Remove a specific committed tag (called from tag pill events)
   */
  removeCommittedTag(tagToRemove) {
    this.committedTags.delete(tagToRemove);
    this.triggerSearchWithCommittedTags();
  }

  /**
   * Clear all committed tags (called from tag pill events)
   */
  clearAllCommittedTags() {
    this.committedTags.clear();
    this.triggerSearchWithCommittedTags();
  }

  /**
   * Add a (presumably) valid tag and trigger a search. Called via handler
   * when clicking on an existing tag from a bookmark item or the tag column
   */
  addTag(tagName) {
    this.committedTags.add(tagName);
    this.triggerSearchWithCommittedTags();
  }
}

// Initialize tag completion when DOM is ready
document.addEventListener('DOMContentLoaded', function () {
  const tagCompletion = new TagCompletion();
  tagCompletion.init();
});

/** @typedef {{text: string, start: number, end: number}} TagInfo */

document.addValidTag = function (tagName) {
  document.dispatchEvent(new CustomEvent('commitTag', { detail: { tag: tagName } }));
};
