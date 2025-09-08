create table if not exists tags (
    tag_id          blob primary key not null default (randomblob(16)),
    name            text unique collate nocase not null,
    color           text,
    created_at      integer not null default (unixepoch()),
    
    check (length(trim(name)) > 0),
    check (name = lower(trim(name)))
);

create unique index idx_tags_name on tags(name collate nocase);