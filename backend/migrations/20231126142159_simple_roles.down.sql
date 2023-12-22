-- Add down migration script here
ALTER TABLE group_users
DROP CONSTRAINT group_users_group_id_role_id_fkey;

ALTER TABLE group_users
DROP COLUMN role_type;

ALTER TABLE group_users
ADD COLUMN role_id UUID;

ALTER TABLE group_roles
DROP COLUMN privileges;

ALTER TABLE group_roles
ADD COLUMN role_id UUID;

ALTER TABLE group_roles
DROP CONSTRAINT group_roles_pkey;

ALTER TABLE group_roles
ADD CONSTRAINT group_roles_pkey PRIMARY KEY (group_id, role_id);

CREATE TABLE roles(
    id UUID NOT NULL DEFAULT gen_random_uuid() PRIMARY KEY,
    can_invite BOOL NOT NULL,
    can_send_messages INT NOT NULL
);

ALTER TABLE group_users
ADD CONSTRAINT group_users_role_id_fkey FOREIGN KEY (role_id) REFERENCES roles(id);

ALTER TABLE group_roles
ADD CONSTRAINT group_roles_role_id_fkey FOREIGN KEY (role_id) REFERENCES roles(id);
