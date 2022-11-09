-- Add migration script here
alter table messages drop column id;
alter table messages add id serial primary key;
alter table messages add sent_at timestamptz not null default now();