-- Add up migration script here
ALTER TABLE group_roles
DROP CONSTRAINT group_roles_role_id_fkey;

ALTER TABLE group_users
DROP CONSTRAINT group_users_role_id_fkey;

DROP TABLE roles;

ALTER TABLE group_roles
DROP CONSTRAINT group_roles_pkey;

ALTER TABLE group_roles
ADD CONSTRAINT group_roles_pkey PRIMARY KEY (group_id, role_type);

ALTER TABLE group_roles
DROP COLUMN role_id;

ALTER TABLE group_roles
ADD COLUMN privileges INT;

UPDATE group_roles
SET privileges = 3;

ALTER TABLE group_roles
ALTER COLUMN privileges
SET NOT NULL;

ALTER TABLE group_users
DROP COLUMN role_id;

ALTER TABLE group_users
ADD COLUMN role_type user_role;

UPDATE group_users
SET role_type = 'owner'
WHERE (user_id, group_id) IN (
    SELECT DISTINCT ON (group_id)
        user_id, group_id
          );

UPDATE group_users
SET role_type = 'member'
WHERE role_type IS NULL;

ALTER TABLE group_users
ALTER COLUMN role_type
SET NOT NULL;

ALTER TABLE group_users
ADD CONSTRAINT group_users_group_id_role_id_fkey FOREIGN KEY (group_id, role_type) REFERENCES group_roles (group_id, role_type)
