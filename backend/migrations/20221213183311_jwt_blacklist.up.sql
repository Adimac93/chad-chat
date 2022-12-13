-- Add up migration script here
create table jwt_blacklist (
    token_id uuid not null primary key,
    expiry timestamptz not null
);