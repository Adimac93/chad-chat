-- Add up migration script here

create table networks (
    ip inet not null primary key,
    geolocation_data jsonb not null

);

-- unique browser agent
create table browser_agents (
    id uuid not null default gen_random_uuid() primary key,
    user_agent_data jsonb not null unique
);

create table user_networks (
    primary key (user_id, network_ip),
    is_trusted boolean not null,
    network_ip inet not null,
    user_id uuid not null,
    foreign key (network_ip) references networks(ip),
    foreign key (user_id) references users(id)
);

-- browser agents used from the same ip and user
create table user_network_browser_agents (
    primary key (user_id, user_network_ip, browser_agent_id),
--     is_trusted boolean not null,
    user_network_ip inet not null,
    browser_agent_id uuid not null,
    user_id uuid not null,
    foreign key (user_network_ip, user_id) references user_networks(network_ip, user_id) ,
    foreign key (browser_agent_id) references browser_agents(id)
);