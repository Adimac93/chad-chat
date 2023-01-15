-- Add down migration script here

do $$
declare
    role_type_var user_role;
    privs jsonb;
    privilege_arr default_privileges[] := array[
        ('owner', '{"can_invite": "Yes", "can_send_messages": {"Yes": 0}}'),
        ('admin', '{"can_invite": "Yes", "can_send_messages": {"Yes": 0}}'),
        ('member', '{"can_invite": "Yes", "can_send_messages": {"Yes": 0}}')
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
