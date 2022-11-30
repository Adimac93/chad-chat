-- Add migration script here
create table group_invitations (
    id varchar unique not null primary key,
    expiration_date timestamptz,
    uses_left int,
    user_id uuid not null,
    group_id uuid not null,
    foreign key (user_id) references users(id),
    foreign key (group_id) references groups(id)
);