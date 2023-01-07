-- Add down migration script here
alter table group_users drop role_id;

drop function set_group_role_for_user;
drop function add_group_roles;

drop table group_roles;

drop table roles;

drop type default_privileges;
drop type group_user;
drop type user_role;