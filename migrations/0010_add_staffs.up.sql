CREATE TABLE staffs (
    member_id   BIGINT NOT NULL,
    joined_at   TIMESTAMP NOT NULL,
    updated_at  TIMESTAMP DEFAULT current_timestamp,

    admin       BOOLEAN NOT NULL DEFAULT false,

    PRIMARY KEY (member_id),
    FOREIGN KEY (member_id)
        REFERENCES members(discord_user_id)
        ON DELETE CASCADE
);

-- CREATE VIEW member_view AS
-- SELECT
--     member.discord_user_id as id
--     member.name
-- FROM members member;

-- -- DROP VIEW mc_account_view;

-- -- CREATE VIEW mc_account_view AS
-- -- SELECT
-- --     mc_account.discord_user_id as member_id,
-- --     member.name as member_name,
-- --     member.joined_at,
-- --     mc_account.uuid,
-- --     mc_account.username,
-- --     CASE
-- --         WHEN staff.admin = TRUE THEN 'admin'
-- --         WHEN staff.member_id IS NOT NULL THEN 'staff'
-- --         WHEN EXISTS (
-- --             SELECT 1 FROM contributors
-- --             WHERE member_id = mc_account.discord_user_id
-- --         ) THEN 'contributor'
-- --         ELSE 'member'
-- --     END AS member_type,
-- --     mc_account."type",
-- --     last_session.created_at as "last_login_at"
-- -- FROM minecraft_accounts mc_account
-- -- LEFT JOIN (
-- --     SELECT member_id, MAX(created_at) as created_at
-- --     FROM logged_in_events
-- --     GROUP BY member_id
-- -- ) last_session
-- --     ON last_session.member_id = mc_account.discord_user_id
-- -- LEFT JOIN staffs staff
-- --     ON staff.member_id = mc_account.discord_user_id
-- -- INNER JOIN members member
-- --     ON member.discord_user_id = mc_account.discord_user_id;
