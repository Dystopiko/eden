ALTER TABLE members
ADD invited_by BIGINT;

DROP VIEW member_view;

CREATE VIEW member_view AS
SELECT
    member.discord_user_id,
    member.joined_at,
    member.name,
    inviter.discord_user_id as invited_by,
    inviter.name as inviter_name,
    CASE
        WHEN staff.admin = TRUE THEN 'admin'
        WHEN staff.member_id IS NOT NULL THEN 'staff'
        WHEN contributor.member_id IS NOT NULL THEN 'contributor'
        ELSE 'member'
    END AS rank
FROM members member
LEFT JOIN staffs staff
    ON staff.member_id = member.discord_user_id
LEFT JOIN members inviter
    ON (inviter.discord_user_id = member.invited_by)
LEFT JOIN contributors contributor
    ON contributor.member_id = member.discord_user_id;
