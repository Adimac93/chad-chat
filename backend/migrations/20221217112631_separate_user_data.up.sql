-- Add up migration script here

create table credentials (
    id uuid primary key references users(id),
    login text not null,
    email text,
    password text not null
);

insert into credentials (id, login, email, password)
select id, login, 'null', password from users;

alter table users 
drop login, drop password,
add profile_picture_url text not null default 'https://st3.depositphotos.com/6672868/13701/v/600/depositphotos_137014128-stock-illustration-user-profile-icon.jpg';

 