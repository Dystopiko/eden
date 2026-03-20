CREATE TABLE settings (
    guild_id BIGINT NOT NULL,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP,

    -- Allow entry of guests in the Minecraft server
    allow_guests BOOLEAN NOT NULL,

    PRIMARY KEY (guild_id)
);
