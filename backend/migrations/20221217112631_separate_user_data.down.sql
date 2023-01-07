-- Add down migration script here

alter table users
add login text, add password text,
drop profile_picture_url;

update users
set (login, password) = (credentials.login, credentials.password)
from credentials
where users.id = credentials.id;

alter table users
alter column login set not null,
alter column password set not null;

drop table credentials;
