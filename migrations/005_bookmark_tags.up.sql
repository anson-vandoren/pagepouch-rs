create table if not exists bookmark_tags (
    bookmark_id     blob not null,
    tag_id          blob not null,
    created_at      integer not null default (unixepoch()),
    
    primary key (bookmark_id, tag_id),
    foreign key(bookmark_id) references bookmarks(bookmark_id) on delete cascade,
    foreign key(tag_id) references tags(tag_id) on delete cascade
);

create index idx_bookmark_tags_bookmark on bookmark_tags(bookmark_id);
create index idx_bookmark_tags_tag on bookmark_tags(tag_id);