-- Add down migration script here
alter table roles
    add privileges jsonb;

do $$
declare
    current_role_id uuid;
    can_invite_from_table bool;
    can_send_messages_from_table int;
begin
for current_role_id, can_invite_from_table, can_send_messages_from_table in select id, can_invite, can_send_messages from roles
loop
    if can_invite_from_table then
        update roles
            set privileges['can_invite'] = '"yes"'
            where id = current_role_id;
    else
        update roles
            set privileges['can_invite'] = '"no"'
            where id = current_role_id;
    end if;
    if can_send_messages_from_table = -1 then
        update roles
            set privileges['can_send_messages'] = '"no"'
            where id = current_role_id;
    else
        update roles
            set privileges['can_send_messages'] = '{}'
            where id = current_role_id;
        update roles
            set privileges['can_send_messages']['yes'] = to_jsonb(can_send_messages_from_table)
            where id = current_role_id;
    end if;
end loop;
end $$;

alter table roles
    drop can_invite,
    drop can_send_messages,
    alter column privileges set not null;