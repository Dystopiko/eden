DROP VIEW member_view;

CREATE VIEW member_view AS
SELECT
    member.discord_user_id,
    member.joined_at,
    member.name,
    CASE
        WHEN staff.admin = TRUE THEN 'admin'
        WHEN staff.member_id IS NOT NULL THEN 'staff'
        WHEN contributor.member_id IS NOT NULL THEN 'contributor'
        ELSE 'member'
    END AS rank
FROM members member
LEFT JOIN staffs staff
    ON staff.member_id = member.discord_user_id
LEFT JOIN contributors contributor
    ON contributor.member_id = member.discord_user_id;

ALTER TABLE members
DROP invited_by;
