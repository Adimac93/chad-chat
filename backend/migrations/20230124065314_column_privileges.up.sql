-- Add up migration script here
alter table roles
	add can_invite bool,
	add can_send_messages int;

do $$
declare
    current_role_id uuid;
    json_value jsonb;
    can_invite_from_json bool;
    can_send_messages_from_json int;
begin
for current_role_id, json_value in select id, privileges from roles
loop
    if json_value ->> 'can_invite' = 'yes' then
        can_invite_from_json := true;
    else
        can_invite_from_json := false;
    end if;
    if json_value ->> 'can_send_messages' = 'no' then
        can_send_messages_from_json := -1;
    else
        can_send_messages_from_json := (json_value #> '{can_send_messages, yes}')::int;
    end if;
    update roles
        set can_invite = can_invite_from_json
		where id = current_role_id;
	update roles
		set can_send_messages = can_send_messages_from_json
        where id = current_role_id;
end loop;
end $$;

alter table roles
    alter column can_invite set not null,
    alter column can_send_messages set not null,
    drop privileges;