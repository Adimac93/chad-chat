-- Add down migration script here
alter table messages drop column id;
alter table messages add id uuid not null default gen_random_uuid() primary key;
alter table messages drop sent_at;
