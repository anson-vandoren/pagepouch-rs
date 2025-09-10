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

    // Handle text input changes on keyup (after character is entered)
    this.searchInput.addEventListener('keyup', (event) => {
      // Only update suggestions if we didn't handle navigation
      if (!this.isNavigationKey(event.key)) {
        this.debouncedHandleTagInput();
      }
    });

    this.searchInput.addEventListener('blur', (event) => {
      // Won't get an Esc key event if the search input is selected, so key on blur instead
      this.handleEscapeKey(event);
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
   * Handle keyboard navigation in dropdown
   */
  async handleKeyNavigation(event) {
    const hasDropdown = this.suggestionsDiv.style.display !== 'none';

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
        this.handleSpaceKey(event);
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
   * Handle Tab/Shift+Tab navigation
   */
  handleTabNavigation(event) {
    if (!event.shiftKey) {
      this.navigateDown();
    } else {
      this.navigateUp();
    }
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
    const isDropdownOpen = this.suggestionsDiv.style.display !== 'none';

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
  handleSpaceKey(event) {
    this.commitIfValidTag(event);
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
    if (this.selectedSuggestionIndex < 0 || this.selectedSuggestionIndex >= items.length) {
      items.forEach((item) => item.classList.remove('selected'));
      return;
    }

    // Try to scroll the container such that the selected item is in the middle
    const selectedItem = items[this.selectedSuggestionIndex];
    const top =
      selectedItem.offsetTop - this.suggestionsDiv.clientHeight / 2 + selectedItem.offsetHeight / 2;
    this.suggestionsDiv.scroll({
      top,
      behavior: 'smooth',
      left: 0,
    });

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
      this.moveTagToInactive(tagToRemove);
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
      const tagsToDeactivate = Array.from(this.committedTags);
      this.committedTags.clear();
      tagsToDeactivate.forEach((tag) => this.moveTagToInactive(tag));
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
      this.moveTagToActive(tagName);
      this.triggerSearchWithCommittedTags();
    } finally {
      this.isUpdatingTags = false;
    }
  }

  ensureTagsContainers() {
    let isOk = true;
    if (!this.inactiveTagsContainer) {
      this.inactiveTagsContainer = document.getElementById('inactive-tags');
      if (!this.inactiveTagsContainer) {
        console.error('Inactive tags container not found');
        isOk = false;
      }
    }
    if (!this.activeTagsContainer) {
      this.activeTagsContainer = document.getElementById('active-tags');
      if (!this.activeTagsContainer) {
        console.error('Active tags container not found');
        isOk = false;
      }
    }

    return isOk;
  }

  /**
   * Move a tag from inactive to active section, applying filter styling
   * @param {string} tagName - Name of the tag to move to active section
   */
  moveTagToActive(tagName) {
    if (!this.ensureTagsContainers()) return;

    // Find the tag element in inactive section
    const tagElem = Array.from(this.inactiveTagsContainer.querySelectorAll('.tag-list-item')).find(
      (el) => el.textContent.trim() === tagName
    );

    if (!tagElem) return;

    // Apply active styling and move to active section
    tagElem.classList.add('tag-list-active');
    tagElem.setAttribute('aria-pressed', 'true');
    tagElem.title = `Remove ${tagName} filter`;

    this.activeTagsContainer.appendChild(tagElem);
  }

  /**
   * Move a tag from active to inactive section, removing filter styling and maintaining alphabetical order
   * @param {string} tagName - Name of the tag to move to inactive section
   */
  moveTagToInactive(tagName) {
    if (!this.ensureTagsContainers()) return;

    // Find the tag element in active section
    const tagElem = Array.from(this.activeTagsContainer.querySelectorAll('.tag-list-item')).find(
      (el) => el.textContent.trim() === tagName
    );

    if (!tagElem) return;

    // Remove active styling and attributes
    tagElem.classList.remove('tag-list-active');
    tagElem.setAttribute('aria-pressed', 'false');
    tagElem.title = `Add ${tagName} filter`;

    // Insert back into inactive section in alphabetical order
    const nextHighestNode = Array.from(
      this.inactiveTagsContainer.querySelectorAll('.tag-list-item')
    ).find((el) => el.textContent.trim() > tagName);

    if (nextHighestNode) {
      this.inactiveTagsContainer.insertBefore(tagElem, nextHighestNode);
    } else {
      this.inactiveTagsContainer.appendChild(tagElem);
    }
  }
}

// Initialize tag completion when DOM is ready
document.addEventListener('DOMContentLoaded', function () {
  const tagCompletion = new TagCompletion();
  tagCompletion.init();
});

/** @typedef {{text: string, start: number, end: number}} TagInfo */
