-- Add down migration script here

alter table credentials
add login text;

alter table users
drop constraint tagged_username;

alter table credentials
alter column email drop not null;

alter table users
add nickname text;

update users
set nickname = concat(username,tag);

update credentials
set login = (users.nickname)
from users
where credentials.id = users.id;

alter table users
drop username,
drop tag;

alter table credentials
alter column login set not null;