-- Add up migration script here

create type token_type as enum ('registration', 'network');

create table user_tokens (
  id uuid not null default gen_random_uuid() primary key,
  token token_type not null,
  user_id uuid not null,
  foreign key (user_id) references users(id)
);
