insert into roles(id, privileges)
values

-- b8c9a317-a456-458f-af88-01d99633f8e2 - Chadders
('aad31270-fa9b-4b82-9392-d231d91f1efa', '{"can_invite": "Yes", "can_send_messages": {"Yes": 0}}'),
('f3c322e4-c1b0-41d4-a47e-afbb217d931a', '{"can_invite": "Yes", "can_send_messages": {"Yes": 2}}'),
('eb8b3214-f823-49a9-a172-2f312c8f3303', '{"can_invite": "Yes", "can_send_messages": {"Yes": 10}}'),

-- 347ac024-f8c9-4450-850f-9d85fb17c957 - Giga-chadders
('5185211c-833f-4331-b43e-8c02a646ea82', '{"can_invite": "Yes", "can_send_messages": {"Yes": 0}}'),
('36592063-606a-4a9f-b731-def05dff875a', '{"can_invite": "Yes", "can_send_messages": {"Yes": 3}}'),
('df4edf4e-5b02-4ffc-b447-963e4121eaaf', '{"can_invite": "Yes", "can_send_messages": {"Yes": 15}}'),

-- a1fd5c51-326f-476e-a4f7-2e61a692bb56 - Hard working rust programmers
('66390385-b7b3-47ac-9124-935b8c9ed0b2', '{"can_invite": "Yes", "can_send_messages": {"Yes": 0}}'),
('8c8432d2-f0cb-4f2a-a52e-3018df81ffa8', '{"can_invite": "Yes", "can_send_messages": {"Yes": 2}}'),
('7a9cfbe2-4d64-4a6a-8cf9-370f96877800', '{"can_invite": "No", "can_send_messages": {"Yes": 10}}'),

-- b9ad636d-1163-4d32-8e88-8fb2318468c4 - Indefinable JavaScript undefiners
('2d99c321-6c26-4db5-b6ab-903507c99e3e', '{"can_invite": "Yes", "can_send_messages": {"Yes": 0}}'),
('5bda9245-d498-45f8-9366-c15c0795eff1', '{"can_invite": "Yes", "can_send_messages": {"Yes": 0}}'),
('4d0b7a5e-c369-4312-a4f3-052be2bf24ad', '{"can_invite": "Yes", "can_send_messages": {"Yes": 0}}');

-- roles are sorted in order owner-admin-member
