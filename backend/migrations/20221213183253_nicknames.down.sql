-- Add down migration script here
alter table users drop nickname;
alter table group_users drop nickname;