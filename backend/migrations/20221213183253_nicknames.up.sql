-- Add up migration script here
alter table group_users add nickname text not null;
alter table users add nickname text not null;