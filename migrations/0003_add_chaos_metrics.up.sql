CREATE TABLE chaos_metrics (
    id                      SERIAL NOT NULL,
    created_at              TIMESTAMP NOT NULL DEFAULT current_timestamp,
    crying_emoticon_times   INT NOT NULL DEFAULT 0,
    updated_at              TIMESTAMP NOT NULL DEFAULT current_timestamp,

    PRIMARY KEY (id)
);
