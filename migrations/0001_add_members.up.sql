CREATE TABLE members (
    discord_user_id     BIGINT NOT NULL,
    joined_at           TIMESTAMP NOT NULL,
    name                VARCHAR(50) NOT NULL,
    updated_at          TIMESTAMP DEFAULT current_timestamp,

    PRIMARY KEY (discord_user_id)
);
