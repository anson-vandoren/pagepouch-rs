//! Search query parsing and processing.
//!
//! This module handles parsing search queries with support for:
//! - OR logic by default (space-separated terms)
//! - AND override with explicit "AND"/"and"
//! - Quoted strings for exact phrases
//! - Future: Tag syntax (#tag) and fuzzy matching

use std::fmt;

/// Represents a parsed search query with different term types and logic operations.
#[derive(Clone, Debug, PartialEq)]
pub struct SearchQuery {
    pub general_terms: Vec<SearchTerm>,
    pub tag_filters: Vec<String>,
    pub logic: SearchLogic,
}

/// Individual search terms that can be words or phrases.
#[derive(Clone, Debug, PartialEq)]
pub enum SearchTerm {
    Word(String),
    Phrase(String),
}

/// Logic operation between search terms.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SearchLogic {
    Or,
    And,
}

impl fmt::Display for SearchTerm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SearchTerm::Word(w) => write!(f, "{w}"),
            SearchTerm::Phrase(p) => write!(f, "\"{p}\""),
        }
    }
}

impl SearchQuery {
    /// Creates a new empty search query.
    pub fn new() -> Self {
        Self {
            general_terms: Vec::new(),
            tag_filters: Vec::new(),
            logic: SearchLogic::Or,
        }
    }

    /// Parses a search query string into structured components.
    ///
    /// # Examples
    ///
    /// ```
    /// use pagepouch_rs::search::{SearchQuery, SearchTerm, SearchLogic};
    ///
    /// // Basic OR search
    /// let query = SearchQuery::parse("rust programming");
    /// assert_eq!(query.logic, SearchLogic::Or);
    /// assert_eq!(query.general_terms.len(), 2);
    ///
    /// // AND search
    /// let query = SearchQuery::parse("rust AND programming");
    /// assert_eq!(query.logic, SearchLogic::And);
    ///
    /// // Quoted phrases
    /// let query = SearchQuery::parse("\"web development\" rust");
    /// assert!(matches!(query.general_terms[0], SearchTerm::Phrase(_)));
    /// ```
    pub fn parse(input: &str) -> Self {
        let mut query = Self::new();

        if input.trim().is_empty() {
            return query;
        }

        // Check if query contains logical operators (case-insensitive)
        let lower_input = input.to_lowercase();
        if lower_input.contains(" and ") {
            query.logic = SearchLogic::And;
        } else if lower_input.contains(" or ") {
            query.logic = SearchLogic::Or; // Explicitly set OR (though it's default)
        }

        // Parse terms, handling quotes and AND keywords
        let terms = Self::tokenize(input);

        for term in terms {
            match term {
                Token::Word(word) => {
                    // Skip logical operator keywords when building terms
                    let lower_word = word.to_lowercase();
                    if lower_word != "and" && lower_word != "or" {
                        query.general_terms.push(SearchTerm::Word(word));
                    }
                }
                Token::Phrase(phrase) => {
                    query.general_terms.push(SearchTerm::Phrase(phrase));
                }
                Token::Tag(tag) => {
                    query.tag_filters.push(tag);
                }
            }
        }

        query
    }

    /// Tokenizes input string, respecting quoted phrases and #tag syntax.
    /// Only treats tags as complete when followed by whitespace or at string end.
    fn tokenize(input: &str) -> Vec<Token> {
        let mut tokens = Vec::new();
        let mut current_token = String::new();
        let mut in_quotes = false;
        let mut quote_char = None;
        let mut is_tag = false;
        let chars = input.chars().peekable();

        for ch in chars {
            match ch {
                '"' | '\'' if !in_quotes => {
                    // Start of quoted phrase
                    if !current_token.is_empty() {
                        let token = if is_tag {
                            // Only add tag if it's complete (we're about to start a quote)
                            Token::Tag(current_token.trim().to_string())
                        } else {
                            Token::Word(current_token.trim().to_string())
                        };
                        tokens.push(token);
                        current_token.clear();
                        is_tag = false;
                    }
                    in_quotes = true;
                    quote_char = Some(ch);
                }
                '"' | '\'' if in_quotes && Some(ch) == quote_char => {
                    // End of quoted phrase
                    if !current_token.is_empty() {
                        tokens.push(Token::Phrase(current_token.trim().to_string()));
                        current_token.clear();
                    }
                    in_quotes = false;
                    quote_char = None;
                }
                '#' if !in_quotes && current_token.is_empty() => {
                    // Start of tag - don't include the # in the token
                    is_tag = true;
                }
                ' ' | '\t' | '\n' if !in_quotes => {
                    // Whitespace outside quotes - this commits the current token
                    if !current_token.is_empty() {
                        let token = if is_tag {
                            Token::Tag(current_token.trim().to_string())
                        } else {
                            Token::Word(current_token.trim().to_string())
                        };
                        tokens.push(token);
                        current_token.clear();
                        is_tag = false;
                    }
                }
                _ => {
                    // Regular character - add to current token
                    current_token.push(ch);
                }
            }
        }

        // Handle final token - only add tags if they appear to be complete
        // (i.e., we're not in the middle of typing)
        if !current_token.is_empty() {
            if in_quotes {
                // Unclosed quote - treat as phrase anyway
                tokens.push(Token::Phrase(current_token.trim().to_string()));
            } else if is_tag {
                // For tags at the end, we need to be more careful
                // Only add if it looks complete (not currently being typed)
                // This is a heuristic - we'll skip incomplete tags at end of string
                // unless they're clearly meant to be complete
                if Self::is_tag_complete(&current_token, input) {
                    tokens.push(Token::Tag(current_token.trim().to_string()));
                }
                // Skip incomplete tags entirely - they should not affect search results
            } else {
                tokens.push(Token::Word(current_token.trim().to_string()));
            }
        }

        tokens
    }

    /// Determines if a tag at the end of input should be considered complete.
    /// This helps avoid showing incomplete tags in autocomplete while typing.
    fn is_tag_complete(tag_content: &str, full_input: &str) -> bool {
        // If the tag is empty or very short, it's probably incomplete
        if tag_content.len() < 2 {
            return false;
        }

        // If the input ends with a space after the tag, it's always complete
        if full_input.trim_end() != full_input {
            return true;
        }

        // For tags at the end of input (no trailing space), be more strict about completeness
        // Only consider them complete if they're reasonably long (3+ chars) AND there's other content
        let tag_position = full_input.rfind('#');
        if let Some(pos) = tag_position {
            let before_tag = full_input[..pos].trim();
            if !before_tag.is_empty() {
                // There's content before this tag
                // Only treat as complete if tag is 3+ characters (more intentional)
                return tag_content.len() >= 3;
            }
        }

        // If it's just a single tag at the beginning, assume incomplete unless space follows
        false
    }

    /// Checks if the query is empty (no search terms).
    pub fn is_empty(&self) -> bool {
        self.general_terms.is_empty() && self.tag_filters.is_empty()
    }
}

impl Default for SearchQuery {
    fn default() -> Self {
        Self::new()
    }
}

/// Internal token representation during parsing.
#[derive(Clone, Debug)]
enum Token {
    Word(String),
    Phrase(String),
    Tag(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_query() {
        let query = SearchQuery::parse("");
        assert!(query.is_empty());
        assert_eq!(query.logic, SearchLogic::Or);
    }

    #[test]
    fn test_single_word() {
        let query = SearchQuery::parse("rust");
        assert_eq!(query.general_terms.len(), 1);
        assert!(matches!(query.general_terms[0], SearchTerm::Word(ref w) if w == "rust"));
        assert_eq!(query.logic, SearchLogic::Or);
    }

    #[test]
    fn test_multiple_words_or() {
        let query = SearchQuery::parse("rust programming");
        assert_eq!(query.general_terms.len(), 2);
        assert_eq!(query.logic, SearchLogic::Or);
    }

    #[test]
    fn test_and_logic() {
        let query = SearchQuery::parse("rust AND programming");
        assert_eq!(query.general_terms.len(), 2);
        assert_eq!(query.logic, SearchLogic::And);

        // Test case-insensitive
        let query = SearchQuery::parse("rust and programming");
        assert_eq!(query.logic, SearchLogic::And);
    }

    #[test]
    fn test_quoted_phrases() {
        let query = SearchQuery::parse("\"web development\" rust");
        assert_eq!(query.general_terms.len(), 2);
        assert!(matches!(query.general_terms[0], SearchTerm::Phrase(ref p) if p == "web development"));
        assert!(matches!(query.general_terms[1], SearchTerm::Word(ref w) if w == "rust"));
    }

    #[test]
    fn test_single_quotes() {
        let query = SearchQuery::parse("'hello world' test");
        assert_eq!(query.general_terms.len(), 2);
        assert!(matches!(query.general_terms[0], SearchTerm::Phrase(ref p) if p == "hello world"));
    }

    #[test]
    fn test_complex_query() {
        let query = SearchQuery::parse("\"best practices\" AND rust performance");
        assert_eq!(query.general_terms.len(), 3);
        assert_eq!(query.logic, SearchLogic::And);
        assert!(matches!(query.general_terms[0], SearchTerm::Phrase(ref p) if p == "best practices"));
    }

    #[test]
    fn test_tag_syntax() {
        let query = SearchQuery::parse("#rust programming");
        assert_eq!(query.tag_filters.len(), 1);
        assert_eq!(query.tag_filters[0], "rust");
        assert_eq!(query.general_terms.len(), 1);
        assert!(matches!(query.general_terms[0], SearchTerm::Word(ref w) if w == "programming"));
    }

    #[test]
    fn test_multiple_tags() {
        let query = SearchQuery::parse("#rust #web development");
        assert_eq!(query.tag_filters.len(), 2);
        assert_eq!(query.tag_filters[0], "rust");
        assert_eq!(query.tag_filters[1], "web");
        assert_eq!(query.general_terms.len(), 1);
        assert!(matches!(query.general_terms[0], SearchTerm::Word(ref w) if w == "development"));
    }

    #[test]
    fn test_tag_with_quotes_and_and() {
        let query = SearchQuery::parse("#rust AND \"web development\" #backend");
        assert_eq!(query.tag_filters.len(), 2);
        assert_eq!(query.tag_filters[0], "rust");
        assert_eq!(query.tag_filters[1], "backend");
        assert_eq!(query.general_terms.len(), 1);
        assert!(matches!(query.general_terms[0], SearchTerm::Phrase(ref p) if p == "web development"));
        assert_eq!(query.logic, SearchLogic::And);
    }

    #[test]
    fn test_partial_tag_search_bug() {
        // This tests the bug where "api and web-" doesn't find "web-framework" tags
        let query1 = SearchQuery::parse("api and web");
        assert_eq!(query1.general_terms.len(), 2);
        assert!(matches!(query1.general_terms[0], SearchTerm::Word(ref w) if w == "api"));
        assert!(matches!(query1.general_terms[1], SearchTerm::Word(ref w) if w == "web"));
        assert_eq!(query1.logic, SearchLogic::And);

        let query2 = SearchQuery::parse("api and web-");
        assert_eq!(query2.general_terms.len(), 2);
        assert!(matches!(query2.general_terms[0], SearchTerm::Word(ref w) if w == "api"));
        assert!(matches!(query2.general_terms[1], SearchTerm::Word(ref w) if w == "web-"));
        assert_eq!(query2.logic, SearchLogic::And);
    }

    #[test]
    fn test_or_keyword_recognition() {
        let query = SearchQuery::parse("rust OR programming");
        assert_eq!(query.general_terms.len(), 2);
        assert_eq!(query.logic, SearchLogic::Or);
        assert!(matches!(query.general_terms[0], SearchTerm::Word(ref w) if w == "rust"));
        assert!(matches!(query.general_terms[1], SearchTerm::Word(ref w) if w == "programming"));

        // Test case-insensitive OR
        let query2 = SearchQuery::parse("rust or programming");
        assert_eq!(query2.general_terms.len(), 2);
        assert_eq!(query2.logic, SearchLogic::Or);
    }

    #[test]
    fn test_tag_fuzzy_matching_expectation() {
        // Tags specified with #tag syntax should still match partially like general terms
        let query = SearchQuery::parse("#web ");
        assert_eq!(query.tag_filters.len(), 1);
        assert_eq!(query.tag_filters[0], "web");

        // This should match tags like "web-dev", "web-framework", "awesome-web", etc.
        // The database query should use LIKE '%web%' not exact match
    }

    #[test]
    fn test_incomplete_tag_not_filtered() {
        // Incomplete tags (being typed) should be completely ignored
        let query1 = SearchQuery::parse("#we");
        assert_eq!(query1.tag_filters.len(), 0);
        assert_eq!(query1.general_terms.len(), 0); // Incomplete tags are completely ignored

        // But complete tags (with space) should appear in filters
        let query2 = SearchQuery::parse("#web ");
        assert_eq!(query2.tag_filters.len(), 1);
        assert_eq!(query2.tag_filters[0], "web");
        assert_eq!(query2.general_terms.len(), 0);
    }

    #[test]
    fn test_tag_completion_with_space() {
        // Tags followed by space should be committed to filters
        let query = SearchQuery::parse("#rust #web ");
        assert_eq!(query.tag_filters.len(), 2);
        assert_eq!(query.tag_filters[0], "rust");
        assert_eq!(query.tag_filters[1], "web");
        assert_eq!(query.general_terms.len(), 0);
    }

    #[test]
    fn test_mixed_complete_incomplete_tags() {
        // Mix of complete and incomplete tags
        let query = SearchQuery::parse("#rust #we");
        assert_eq!(query.tag_filters.len(), 1);
        assert_eq!(query.tag_filters[0], "rust");
        assert_eq!(query.general_terms.len(), 0); // Incomplete tags are completely ignored
    }
}
