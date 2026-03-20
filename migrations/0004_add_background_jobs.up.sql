-- JSON type for `data` is not needed. It will be used later when
-- deserialization needed in `eden-background-worker` implementation.
CREATE TABLE background_jobs (
    id          BLOB(32) NOT NULL,
    type        VARCHAR(255) NOT NULL,
    created_at  TIMESTAMP NOT NULL DEFAULT current_timestamp,
    data        TEXT NOT NULL,
    last_retry  TIMESTAMP,
    priority    INT2 NOT NULL,
    retries     INT2 NOT NULL DEFAULT 0,
    status      VARCHAR(7) NOT NULL DEFAULT 'enqueued',

    PRIMARY KEY (id),
    CONSTRAINT bjb_status_enum CHECK (status IN ('enqueued', 'running', 'failed'))
);
