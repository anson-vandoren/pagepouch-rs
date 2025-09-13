create view if not exists bookmark_with_tags as
select
    b.*,
    group_concat(t.name) as tags_string
from bookmarks b
left join bookmark_tags bt on b.bookmark_id = bt.bookmark_id
left join tags t on bt.tag_id = t.tag_id
group by b.bookmark_id;
