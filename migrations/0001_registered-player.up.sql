CREATE TABLE "primary_guild.registered_player" (
    discord_user_id BIGINT PRIMARY KEY,
    created_at TIMESTAMP NOT NULL,
    name VARCHAR(50) NOT NULL,
    updated_at TIMESTAMP
);

CREATE TABLE "primary_guild.mc_player_info" (
    uuid VARCHAR(32) NOT NULL PRIMARY KEY,
    discord_user_id BIGINT NOT NULL,
    account_type VARCHAR(10) NOT NULL CHECK(account_type IN ('java', 'bedrock')),
    username VARCHAR(20) NOT NULL UNIQUE,

    FOREIGN KEY (discord_user_id)
        REFERENCES "primary_guild.registered_player" (discord_user_id)
        ON DELETE CASCADE,

    UNIQUE (uuid, discord_user_id, account_type)
);