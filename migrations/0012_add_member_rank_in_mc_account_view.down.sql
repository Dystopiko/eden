DROP VIEW mc_account_view;

CREATE VIEW mc_account_view AS
SELECT
    mc_account.discord_user_id as member_id,
    member.name as member_name,
    member.joined_at,
    mc_account.uuid,
    mc_account.username,
    mc_account."type",
    EXISTS (
        SELECT 1 FROM contributors
        WHERE member_id = mc_account.discord_user_id
    ) AS "is_contributor",
    last_session.created_at as "last_login_at"
FROM minecraft_accounts mc_account
LEFT JOIN (
    SELECT member_id, MAX(created_at) as created_at
    FROM logged_in_events
    GROUP BY member_id
) last_session
    ON last_session.member_id = mc_account.discord_user_id
INNER JOIN members member
    ON member.discord_user_id = mc_account.discord_user_id;
