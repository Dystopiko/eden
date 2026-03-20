CREATE TABLE contributors (
    member_id   BIGINT NOT NULL,
    created_at  TIMESTAMP NOT NULL,
    updated_at  TIMESTAMP,

    PRIMARY KEY (member_id),
    FOREIGN KEY (member_id)
        REFERENCES members(discord_user_id)
        ON DELETE CASCADE
);
