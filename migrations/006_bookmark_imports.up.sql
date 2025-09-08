create table if not exists bookmark_imports (
    import_id       blob primary key not null default (randomblob(16)),
    user_id         blob not null,
    source_name     text,
    total_count     integer not null default 0,
    success_count   integer not null default 0,
    error_count     integer not null default 0,
    started_at      integer not null default (unixepoch()),
    completed_at    integer,
    
    foreign key(user_id) references users(user_id) on delete cascade,
    check (total_count >= 0),
    check (success_count >= 0),
    check (error_count >= 0),
    check (success_count + error_count <= total_count)
);

create index idx_bookmark_imports_user on bookmark_imports(user_id, started_at desc);