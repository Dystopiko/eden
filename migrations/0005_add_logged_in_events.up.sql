CREATE TABLE logged_in_events (
    event_id        BLOB(32) NOT NULL,
    player_uuid     BLOB(32) NOT NULL,

    created_at      TIMESTAMP NOT NULL,
    username        VARCHAR(20),
    ip_address      VARCHAR(50) NOT NULL,
    "type"          VARCHAR(10) NOT NULL,
    member_id       BIGINT,

    PRIMARY KEY (event_id, player_uuid),
    FOREIGN KEY (member_id)
        REFERENCES members(discord_user_id)
        ON DELETE SET NULL,

    CONSTRAINT mc_account_type_enum CHECK ("type" IN ('java', 'bedrock'))
);
