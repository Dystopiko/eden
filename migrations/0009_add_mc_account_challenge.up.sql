CREATE TABLE mc_account_challenges (
    id             BLOB(32) NOT NULL PRIMARY KEY,
    hashed_code    VARCHAR(255) NOT NULL,

    created_at     TIMESTAMP NOT NULL,
    expires_at     TIMESTAMP NOT NULL,

    uuid           BLOB(32) NOT NULL,
    username       VARCHAR(20) NOT NULL,
    java           BOOLEAN NOT NULL,

    ip_address     VARCHAR(50) NOT NULL,
    "status"       VARCHAR(10) NOT NULL DEFAULT 'in-progress',
    updated_at     TIMESTAMP,

    CONSTRAINT mac_status_type_enum
        CHECK ("status" IN ('done', 'in-progress', 'cancelled'))
);
