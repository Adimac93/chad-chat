-- Add up migration script here
create table user_friends (
    primary key (user_id, friend_id),
    note text not null,
    user_id uuid not null,
    friend_id uuid not null,
    foreign key (user_id) references users(id),
    foreign key (friend_id) references users(id)
);

create table friend_requests (
    primary key (sender_id, receiver_id),
    sender_id uuid not null,
    receiver_id uuid not null,
    foreign key (sender_id) references users(id),
    foreign key (receiver_id) references users(id)
);

create type status as enum ('online', 'offline','idle');
alter table users add activity_status status not null;

-- unfinished