# Database Code Review - TODO Items

Based on comprehensive analysis of the database code in `src/db/` and related files, prioritized by safety, correctness, and performance.

## **CORRECTNESS ISSUES** 🔧

### ❌ **NEEDS FIX** - Missing Foreign Key Validation

- **Issue**: `user_sessions` table lacks `ON DELETE CASCADE` for user_id foreign key
- **Risk**: Orphaned sessions when users are deleted
- **Priority**: MEDIUM
- **Fix**: Add migration to update the foreign key constraint

## **PERFORMANCE OPTIMIZATIONS** 🚀

### ⚠️ **SUBOPTIMAL** - Index Optimizations

**Good existing indexes**, but missing some key ones:

- ✅ Has: `idx_bookmarks_user_created` for user + created_at
- ❌ Missing: Composite index for search queries
- ❌ Missing: `user_sessions(expires_at)` for cleanup

**Recommended additions**:

```sql
CREATE INDEX idx_user_sessions_cleanup ON user_sessions(expires_at);
CREATE INDEX idx_bookmarks_search ON bookmarks(user_id, is_archived, title, description);
```

### ❌ **CRITICAL** - Missing SQLite Production Configuration

- **Issue**: No SQLite PRAGMAs configured for production performance/safety
- **Priority**: HIGH
- **Location**: `src/db/mod.rs:28` in `connect()` function
- **Required PRAGMAs**:

```rust
.after_connect(|conn, _meta| {
    Box::pin(async move {
        sqlx::query("PRAGMA journal_mode = WAL").execute(conn).await?;      // Write-Ahead Logging for concurrency
        sqlx::query("PRAGMA synchronous = NORMAL").execute(conn).await?;    // Balance safety/performance
        sqlx::query("PRAGMA foreign_keys = ON").execute(conn).await?;       // Enable FK constraints
        sqlx::query("PRAGMA temp_store = MEMORY").execute(conn).await?;     // Temp tables in memory
        sqlx::query("PRAGMA cache_size = -64000").execute(conn).await?;     // 64MB page cache
        Ok(())
    })
})
```

### ⚠️ **SUBOPTIMAL** - Connection Pool Configuration

- **Issue**: `MAX_CONNECTIONS = 10` in `src/db/mod.rs:13` might be too low for concurrent users
- **Priority**: MEDIUM
- **Recommendation**: Make configurable and consider higher limits (20-50) based on expected load

### 💡 **OPPORTUNITY** - Full-Text Search Implementation

- **Current Issue**: Text search uses `LIKE` patterns which don't scale well and lack ranking
- **Location**: `src/db/bookmarks.rs` search functions (lines 204-463)
- **Priority**: MEDIUM
- **Benefits**: Better search relevance, performance, phrase matching, stemming

**Implementation Options**:

1. **SQLite FTS5 (Recommended)**:

   ```sql
   -- Create virtual table for full-text search
   CREATE VIRTUAL TABLE bookmarks_fts USING fts5(
       title, description, url, tags,
       content='',  -- External content table
       tokenize='porter'  -- Enable stemming
   );

   -- Populate from existing data
   INSERT INTO bookmarks_fts(rowid, title, description, url, tags)
   SELECT
       b.rowid,
       b.title,
       b.description,
       b.url,
       GROUP_CONCAT(t.name, ' ')
   FROM bookmarks b
   LEFT JOIN bookmark_tags bt ON b.bookmark_id = bt.bookmark_id
   LEFT JOIN tags t ON bt.tag_id = t.tag_id
   GROUP BY b.bookmark_id;
   ```

2. **Usage in Rust**:

   ```rust
   // Replace LIKE queries with FTS5 MATCH
   sqlx::query!(
       "SELECT b.* FROM bookmarks b
       JOIN bookmarks_fts fts ON b.rowid = fts.rowid
       WHERE bookmarks_fts MATCH ?
       ORDER BY rank",
       search_term
   )
   ```

3. **Advanced Features**:

   - **Phrase queries**: `"exact phrase"`
   - **Boolean operators**: `rust AND (web OR framework)`
   - **Proximity**: `NEAR(rust web, 5)`
   - **Ranking**: Built-in BM25 relevance scoring

4. **Maintenance**:
   - Add triggers to keep FTS table in sync with bookmarks/tags
   - Periodic `OPTIMIZE` for performance
   - Consider `rebuild` for schema changes

### 💡 **OPPORTUNITY** - Query Result Caching

- Consider caching frequently accessed data like user tags
- Implement at application level with time-based expiration
- **Priority**: LOW

## **SUMMARY RECOMMENDATIONS BY PRIORITY**

### **HIGH PRIORITY** (Correctness & Performance)

1. ❌ **CRITICAL**: Add SQLite production PRAGMAs (`src/db/mod.rs`)
2. ❌ Add missing foreign key cascades

### **MEDIUM PRIORITY** (Performance)

1. ❌ Add recommended database indexes
2. ❌ Make connection pool size configurable
3. ❌ Implement full-text search (FTS5) for better search performance
4. ❌ Consider result caching for tags

### **LOW PRIORITY** (Maintainability)

1. ❌ Break down large query functions
2. ❌ Add query performance monitoring
3. ❌ Consider prepared statement reuse for hot paths

```

```
