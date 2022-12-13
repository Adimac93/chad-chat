-- Add up migration script here
alter table group_users add nickname varchar not null;
alter table users add nickname varchar not null;