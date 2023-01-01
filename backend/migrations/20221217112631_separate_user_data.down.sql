-- Add down migration script here

-- begin;
-- alter table users
-- add login text, add password text,
-- drop profile_picture_url;

-- insert into users (login, password)
-- select login, password from credentials;

-- alter table users
-- alter column login set not null,
-- alter column password set not null;

-- drop table credentials;
-- commit;