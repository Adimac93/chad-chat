-- Add up migration script here
create type user_role as enum ('owner', 'admin', 'member');
create type group_user as (user_id uuid, group_id uuid);

create table roles(
    id uuid not null default gen_random_uuid() primary key,
    privileges jsonb not null
);

create table group_roles(
    primary key (group_id, role_id),
    group_id uuid not null,
    role_id uuid not null,
    role_type user_role not null,
    foreign key(group_id) references groups(id),
    foreign key(role_id) references roles(id)
);

create type default_privileges as (role user_role, privileges jsonb);
create function add_group_roles(group_id uuid) returns void as $$
    declare
        role_type user_role;
        privs jsonb;
        privilege_arr default_privileges[] := array[
            ('owner', '{"can_invite": "Yes", "can_send_messages": {"Yes": 0}}'),
            ('admin', '{"can_invite": "Yes", "can_send_messages": {"Yes": 0}}'),
            ('member', '{"can_invite": "Yes", "can_send_messages": {"Yes": 0}}')
        ];
    begin
    for role_type, privs in select role, privileges from unnest(privilege_arr)
    loop
        declare new_role_id uuid;
        begin
            insert into roles(privileges)
                values (privs)
                returning id into new_role_id;

            insert into group_roles(group_id, role_id, role_type)
            values (group_id, new_role_id, role_type);
        end;
    end loop;
    end;
$$ language plpgsql;

create function set_group_role_for_user(user_id uuid, group_id uuid, role user_role) returns void as $$
    begin
    update group_users
        set role_id = (
            select role_id
                from group_roles
                where group_roles.group_id = group_id
                and group_roles.role_type = role
        )
        where group_roles.user_id = user_id;
    end;
$$ language plpgsql;

do $$
declare
current_group_id uuid;
begin
for current_group_id in select id from groups
loop
    perform add_group_roles(current_group_id);
end loop;
end $$;

alter table group_users
    add role_id uuid;

update group_users set role_id = (
    select role_id
        from group_roles
        where group_roles.group_id = group_users.group_id
        and group_roles.role_type = 'member'
);

do $$
declare
owner_array uuid[] := array(
    select distinct on (group_id) user_id
    from group_users
);
begin
update group_users
    set role_id = (
    select role_id
        from group_roles
        where group_roles.group_id = group_users.group_id
        and group_roles.role_type = 'owner'
    )
    where group_users.user_id = any(owner_array);

end $$;

alter table group_users
    add foreign key (role_id) references roles(id);

alter table group_users
    alter column role_id set not null;