CREATE TABLE minecraft_accounts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    linked_at TIMESTAMP NOT NULL DEFAULT current_timestamp,
    discord_user_id BIGINT NOT NULL,
    uuid VARCHAR(32) NOT NULL UNIQUE,
    username VARCHAR(20) NOT NULL,
    "type" VARCHAR(10) NOT NULL CHECK ("type" IN ('java', 'bedrock')),

    FOREIGN KEY (discord_user_id)
        REFERENCES members (discord_user_id)
        ON DELETE CASCADE,

    UNIQUE (discord_user_id, uuid)
);
