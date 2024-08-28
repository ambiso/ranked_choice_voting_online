create table election
(
    id uuid primary key ,
    title text not null
);

-- nominees for a given election
create table "candidate"
(
    election uuid references election not null,
    username text not null,
    id bigserial primary key,
    unique (election, username)
);

-- a voter's vote
create table "vote"
(
    election uuid references election not null,
    id uuid primary key
);

-- the ranked choices
create table "vote_preferences"
(
    vote uuid references vote not null,
    candidate bigint references candidate not null,
    preference int not null
);
-- todo: add constraint that candidate is actually part of this election