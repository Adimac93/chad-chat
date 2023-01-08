-- Add up migration script here

alter table users
rename nickname to username;

alter table users
add tag int,
add constraint tagged_username unique (username, tag);

-- ! breaking change, accounts without email should be deleted manually
alter table credentials
alter column email set not null;

update users
set (username, tag) = (credentials.login, 0)
from credentials
where users.id = credentials.id;

alter table credentials
drop login;