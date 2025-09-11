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

### ⚠️ **SUBOPTIMAL** - Connection Pool Configuration

- **Issue**: `MAX_CONNECTIONS = 10` in `src/db/mod.rs:13` might be too low for concurrent users
- **Priority**: MEDIUM
- **Recommendation**: Make configurable and consider higher limits (20-50) based on expected load

### ❌ **INEFFICIENT** - Tag Filtering

- **Issue**: `src/db/bookmarks.rs:694-711` filters tags in Rust memory instead of SQL
- **Performance Impact**: Fetches all bookmarks then filters in application
- **Priority**: MEDIUM
- **Fix**: Move filtering logic to SQL queries

### 💡 **OPPORTUNITY** - Query Result Caching

- Consider caching frequently accessed data like user tags
- Implement at application level with time-based expiration
- **Priority**: LOW

## **SUMMARY RECOMMENDATIONS BY PRIORITY**

### **HIGH PRIORITY** (Correctness)

1. ❌ Add missing foreign key cascades
2. ❌ Move tag filtering to SQL

### **MEDIUM PRIORITY** (Performance)

1. ❌ Add recommended database indexes
2. ❌ Make connection pool size configurable
3. ❌ Consider result caching for tags

### **LOW PRIORITY** (Maintainability)

1. ❌ Break down large query functions
2. ❌ Add query performance monitoring
3. ❌ Consider prepared statement reuse for hot paths
