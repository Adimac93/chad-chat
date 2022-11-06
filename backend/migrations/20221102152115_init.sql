-- Add migration script here
create table users(
    id uuid not null default gen_random_uuid() primary key,
    login varchar unique not null,
    password varchar not null
);

create table groups(
    id uuid not null default gen_random_uuid() primary key,
    name varchar not null
);

create table group_users(
    user_id uuid not null,
    group_id uuid not null,
    foreign key (user_id) references users(id),
    foreign key (group_id) references groups(id)
);

create table messages(
    id uuid not null default gen_random_uuid() primary key,
    user_id uuid not null,
    group_id uuid not null,
    content text not null,
    foreign key (user_id) references users(id),
    foreign key (group_id) references groups(id)
);