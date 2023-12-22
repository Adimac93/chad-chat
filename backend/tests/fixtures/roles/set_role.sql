insert into users (id, username, tag, activity_status)
values
    ('ba34ff10-4b89-44cb-9b36-31eb57c41556','Adimac93', 0000, 'idle'),
    ('263541a8-fa1e-4f13-9e5d-5b250a5a71e6','HubertK05', 0000, 'idle'),
    ('e287ccab-fb33-4314-8d81-bfa9d6e52928', '_SomeUser_', 0000, 'idle');

INSERT INTO credentials (id, email, password)
VALUES
    ('ba34ff10-4b89-44cb-9b36-31eb57c41556','adam@gmail.com','$argon2i$v=19$m=4096,t=3,p=1$M0g3ODVzWmQ$fHLpcolZURzJzej/xbDQqTb+OINmUOl8uEFVLah0z8Y'),
    ('263541a8-fa1e-4f13-9e5d-5b250a5a71e6','hubert@gmail.com','$argon2i$v=19$m=4096,t=3,p=1$M0g3ODVzWmQ$fHLpcolZURzJzej/xbDQqTb+OINmUOl8uEFVLah0z8Y'),
    ('e287ccab-fb33-4314-8d81-bfa9d6e52928','some_user@gmail.com', '$argon2i$v=19$m=4096,t=3,p=1$M0g3ODVzWmQ$fHLpcolZURzJzej/xbDQqTb+OINmUOl8uEFVLah0z8Y');

INSERT INTO groups (id, name)
VALUES ('b8c9a317-a456-458f-af88-01d99633f8e2','Chadders');

INSERT INTO group_roles (group_id, role_type, privileges)
VALUES
    ('b8c9a317-a456-458f-af88-01d99633f8e2', 'owner', 3),
    ('b8c9a317-a456-458f-af88-01d99633f8e2', 'admin', 3),
    ('b8c9a317-a456-458f-af88-01d99633f8e2', 'member', 1);

INSERT INTO group_users (user_id, group_id, role_type, nickname)
VALUES
    ('ba34ff10-4b89-44cb-9b36-31eb57c41556', 'b8c9a317-a456-458f-af88-01d99633f8e2', 'owner', 'Adimac93'),
    ('263541a8-fa1e-4f13-9e5d-5b250a5a71e6', 'b8c9a317-a456-458f-af88-01d99633f8e2', 'admin', 'HubertK05'),
    ('e287ccab-fb33-4314-8d81-bfa9d6e52928', 'b8c9a317-a456-458f-af88-01d99633f8e2', 'member', '_SomeUser_');
