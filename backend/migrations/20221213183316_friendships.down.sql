-- Add down migration script here
drop table friend_requests;
drop table user_friends;

alter table users drop activity_status;
drop type status;