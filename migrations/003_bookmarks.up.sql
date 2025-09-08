create table if not exists bookmarks (
    bookmark_id     blob primary key not null default (randomblob(16)),
    user_id         blob not null,
    url             text not null,
    title           text not null,
    description     text,
    created_at      integer not null default (unixepoch()),
    updated_at      integer not null default (unixepoch()),
    is_archived     boolean not null default false,
    is_private      boolean not null default false,
    
    foreign key(user_id) references users(user_id) on delete cascade
);

create index idx_bookmarks_user_id on bookmarks(user_id);
create index idx_bookmarks_created_at on bookmarks(created_at desc);
create index idx_bookmarks_user_created on bookmarks(user_id, created_at desc);
create index idx_bookmarks_active on bookmarks(user_id, is_archived) where is_archived = false;