CREATE TABLE chaos_metrics (
    id SERIAL PRIMARY KEY,
    created_at TIMESTAMP NOT NULL DEFAULT current_timestamp,
    crying_emoticon_times INT NOT NULL DEFAULT 0,
    updated_at TIMESTAMP NOT NULL DEFAULT current_timestamp
);
