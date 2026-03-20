CREATE TABLE minecraft_accounts (
    id                  INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    linked_at           TIMESTAMP NOT NULL DEFAULT current_timestamp,
    discord_user_id     BIGINT NOT NULL,
    uuid                BLOB(32) NOT NULL UNIQUE,
    username            VARCHAR(20) NOT NULL,
    "type"              VARCHAR(10) NOT NULL,

    UNIQUE (discord_user_id, uuid),

    FOREIGN KEY (discord_user_id)
        REFERENCES members (discord_user_id)
        ON DELETE CASCADE,

    CONSTRAINT mc_account_type_enum CHECK ("type" IN ('java', 'bedrock'))
);
