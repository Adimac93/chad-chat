-- Add up migration script here

create type privilege_type as enum('can_invite', 'can_send_messages');

do $$
declare
    role_type_var user_role;
    privs jsonb;
    privilege_arr default_privileges[] := array[
        ('owner', '{"can_invite": "yes", "can_send_messages": {"yes": 0}}'),
        ('admin', '{"can_invite": "yes", "can_send_messages": {"yes": 0}}'),
        ('member', '{"can_invite": "yes", "can_send_messages": {"yes": 0}}')
    ];
begin
for role_type_var, privs in select role, privileges from unnest(privilege_arr)
loop
    update roles
        set privileges = privs
        from group_roles
        where group_roles.role_type = role_type_var;
end loop;
end $$;
