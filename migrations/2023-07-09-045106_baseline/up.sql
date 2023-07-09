-- Your SQL goes here
CREATE TABLE users
(
    pubkey       TEXT PRIMARY KEY NOT NULL,
    date_created TIMESTAMP        NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE user_nwc
(
    request_key  TEXT PRIMARY KEY NOT NULL,
    response_key TEXT UNIQUE      NOT NULL,
    relay_url    TEXT             NOT NULL,
    user_pubkey  TEXT             NOT NULL,
    date_created TIMESTAMP        NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_pubkey) REFERENCES users (pubkey)
);
create unique index user_nwc_response_key_uindex on user_nwc (response_key);
create unique index user_nwc_request_key_uindex on user_nwc (request_key);
create index user_nwc_user_pubkey_index on user_nwc (user_pubkey);

-- todo add spending conditions
CREATE TABLE service_nwc
(
    request_key  TEXT PRIMARY KEY NOT NULL,
    response_key TEXT UNIQUE      NOT NULL,
    relay_url    TEXT             NOT NULL,
    service_name TEXT             NOT NULL,
    user_pubkey  TEXT             NOT NULL,
    date_created TIMESTAMP        NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_pubkey) REFERENCES users (pubkey)
);
create unique index service_nwc_response_key_uindex on service_nwc (response_key);
create unique index service_nwc_request_key_uindex on service_nwc (request_key);
create index service_nwc_user_pubkey_index on service_nwc (user_pubkey);
