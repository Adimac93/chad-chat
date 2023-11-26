CREATE TYPE status AS ENUM ('online', 'offline', 'idle');

CREATE TYPE user_role AS ENUM ('owner', 'admin', 'member');

CREATE TABLE IF NOT EXISTS users
(
    id UUID DEFAULT gen_random_UUID() NOT NULL,
    username TEXT NOT NULL,
    activity_status status NOT NULL,
    profile_picture_url TEXT,
    tag integer NOT NULL,
    PRIMARY KEY (id),
    CONSTRAINT tagged_username UNIQUE (username, tag)
);

CREATE TABLE IF NOT EXISTS groups
(
    id UUID DEFAULT gen_random_UUID() NOT NULL,
    name TEXT NOT NULL,
    PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS messages
(
    user_id UUID NOT NULL,
    group_id UUID NOT NULL,
    content TEXT NOT NULL,
    id serial,
    sent_at TIMESTAMPTZ DEFAULT now() NOT NULL,
    PRIMARY KEY (id),
    FOREIGN KEY (user_id) REFERENCES users,
    FOREIGN KEY (group_id) REFERENCES groups
);

CREATE TABLE IF NOT EXISTS group_invitations
(
    id TEXT NOT NULL,
    expiration_date TIMESTAMPTZ,
    uses_left integer,
    user_id UUID NOT NULL,
    group_id UUID NOT NULL,
    PRIMARY KEY (id),
    FOREIGN KEY (user_id) REFERENCES users,
    FOREIGN KEY (group_id) REFERENCES groups
);

CREATE TABLE IF NOT EXISTS jwt_blacklist
(
    token_id UUID NOT NULL,
    expiry TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (token_id)
);

CREATE TABLE IF NOT EXISTS user_friends
(
    note TEXT NOT NULL,
    user_id UUID NOT NULL,
    friend_id UUID NOT NULL,
    PRIMARY KEY (user_id, friend_id),
    FOREIGN KEY (user_id) REFERENCES users,
    FOREIGN KEY (friend_id) REFERENCES users
);

CREATE TABLE IF NOT EXISTS friend_requests
(
    sender_id UUID NOT NULL,
    receiver_id UUID NOT NULL,
    PRIMARY KEY (sender_id, receiver_id),
    FOREIGN KEY (sender_id) REFERENCES users,
    FOREIGN KEY (receiver_id) REFERENCES users
);

CREATE TABLE IF NOT EXISTS credentials
(
    id UUID NOT NULL,
    email TEXT NOT NULL,
    password TEXT NOT NULL,
    PRIMARY KEY (id),
    FOREIGN KEY (id) REFERENCES users
);

CREATE TABLE IF NOT EXISTS roles
(
    id UUID DEFAULT gen_random_UUID() NOT NULL,
    can_invite BOOLEAN NOT NULL,
    can_send_messages integer NOT NULL,
    PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS group_users
(
    user_id UUID NOT NULL,
    group_id UUID NOT NULL,
    nickname TEXT NOT NULL,
    role_id UUID NOT NULL,
    PRIMARY KEY (user_id, group_id),
    FOREIGN KEY (user_id) REFERENCES users,
    FOREIGN KEY (group_id) REFERENCES groups,
    FOREIGN KEY (role_id) REFERENCES roles
);

CREATE TABLE IF NOT EXISTS group_roles
(
    group_id UUID NOT NULL,
    role_id UUID NOT NULL,
    role_type user_role NOT NULL,
    PRIMARY KEY (group_id, role_id),
    FOREIGN KEY (group_id) REFERENCES groups,
    FOREIGN KEY (role_id) REFERENCES roles
);

CREATE TABLE IF NOT EXISTS networks
(
    ip INET NOT NULL,
    geolocation_data jsonb NOT NULL,
    PRIMARY KEY (ip)
);

CREATE TABLE IF NOT EXISTS browser_agents
(
    id UUID DEFAULT gen_random_UUID() NOT NULL,
    user_agent_data jsonb NOT NULL,
    PRIMARY KEY (id),
    UNIQUE (user_agent_data)
);

CREATE TABLE IF NOT EXISTS user_networks
(
    is_trusted BOOLEAN NOT NULL,
    network_ip INET NOT NULL,
    user_id UUID NOT NULL,
    PRIMARY KEY (user_id, network_ip),
    FOREIGN KEY (network_ip) REFERENCES networks,
    FOREIGN KEY (user_id) REFERENCES users
);

CREATE TABLE IF NOT EXISTS user_network_browser_agents
(
    user_network_ip INET NOT NULL,
    browser_agent_id UUID NOT NULL,
    user_id UUID NOT NULL,
    PRIMARY KEY (user_id, user_network_ip, browser_agent_id),
    FOREIGN KEY (user_network_ip, user_id) REFERENCES user_networks (network_ip, user_id),
    FOREIGN KEY (browser_agent_id) REFERENCES browser_agents
);
