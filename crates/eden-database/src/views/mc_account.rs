use eden_timestamp::Timestamp;
use error_stack::{Report, ResultExt};
use sqlx::FromRow;
use thiserror::Error;
use uuid::Uuid;

use crate::{primary_guild::McAccountType, snowflake::Snowflake, views::MemberRank};

#[derive(Clone, Debug, FromRow)]
pub struct McAccountView {
    pub member_id: Snowflake,
    pub member_name: String,
    pub member_rank: MemberRank,
    pub joined_at: Timestamp,
    pub uuid: Uuid,
    pub username: String,
    #[sqlx(rename = "type")]
    pub kind: McAccountType,
    pub last_login_at: Option<Timestamp>,
}

/// Error type representing a failure to query with the [`LoggedInEvent`] table.
#[derive(Debug, Error)]
#[error("Failed to query McAccountView table from the database")]
pub struct ViewQueryError;

impl McAccountView {
    pub async fn find_by_mc_uuid(
        conn: &mut eden_sqlite::Connection,
        uuid: Uuid,
    ) -> Result<Self, Report<ViewQueryError>> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM mc_account_view
            WHERE uuid = ?"#,
        )
        .bind(uuid)
        .fetch_one(conn)
        .await
        .change_context(ViewQueryError)
        .attach("while trying to find McAccountView row by Minecraft UUID")
    }
}

#[cfg(test)]
mod tests {
    use eden_timestamp::Timestamp;
    use std::net::{IpAddr, Ipv4Addr};
    use twilight_model::id::Id;
    use uuid::Uuid;

    use crate::{
        primary_guild::{
            LoggedInEvent, McAccount, McAccountType, Member, contributor::Contributor,
        },
        views::{McAccountView, MemberRank},
    };

    async fn setup(conn: &mut eden_sqlite::Transaction<'_>) -> (Member, McAccount) {
        let query = Member::upsert()
            .discord_user_id(Id::new(123456))
            .name("dummy")
            .build();

        let member = query.perform(conn).await.unwrap();

        let query = McAccount::new()
            .account_type(McAccountType::Java)
            .discord_user_id(member.discord_user_id.cast())
            .username("dummy")
            .uuid(Uuid::new_v4())
            .build();

        let account = query.create(conn).await.unwrap();
        (member, account)
    }

    #[tokio::test]
    async fn should_provide_if_member_is_contributor() {
        let pool = crate::testing::setup().await;

        let mut conn = pool.begin().await.unwrap();
        let (member, account) = setup(&mut conn).await;

        Contributor::upsert()
            .member_id(member.discord_user_id.cast())
            .build()
            .perform(&mut conn)
            .await
            .unwrap();

        let view = McAccountView::find_by_mc_uuid(&mut conn, account.uuid)
            .await
            .unwrap();

        assert_eq!(view.uuid, account.uuid);
        assert_eq!(view.username, account.username);
        assert_eq!(view.kind, account.kind);
        assert_eq!(view.member_rank, MemberRank::Contributor);
    }

    #[tokio::test]
    async fn should_provide_last_login_at() {
        let pool = crate::testing::setup().await;

        let mut conn = pool.begin().await.unwrap();
        let (member, account) = setup(&mut conn).await;

        LoggedInEvent::new_event()
            .ip_address(IpAddr::V4(Ipv4Addr::LOCALHOST))
            .kind(McAccountType::Java)
            .player_uuid(account.uuid)
            .member_id(member.discord_user_id.cast())
            .username(account.username.to_owned())
            .build()
            .create(&mut conn)
            .await
            .unwrap();

        let timestamp = Timestamp::now();
        LoggedInEvent::new_event()
            .created_at(timestamp)
            .ip_address(IpAddr::V4(Ipv4Addr::LOCALHOST))
            .kind(McAccountType::Java)
            .player_uuid(account.uuid)
            .member_id(member.discord_user_id.cast())
            .username(account.username.to_owned())
            .build()
            .create(&mut conn)
            .await
            .unwrap();

        let view = McAccountView::find_by_mc_uuid(&mut conn, account.uuid)
            .await
            .unwrap();

        assert_eq!(view.uuid, account.uuid);
        assert_eq!(view.username, account.username);
        assert_eq!(view.kind, account.kind);
        assert_eq!(view.last_login_at, Some(timestamp));
        assert_eq!(view.member_rank, MemberRank::Member);

        assert_eq!(view.member_id, member.discord_user_id);
        assert_eq!(view.member_name, member.name);
        assert_eq!(view.joined_at, member.joined_at);
    }

    #[tokio::test]
    async fn should_query_as_usual() {
        let pool = crate::testing::setup().await;

        let mut conn = pool.begin().await.unwrap();
        let (member, account) = setup(&mut conn).await;

        let view = McAccountView::find_by_mc_uuid(&mut conn, account.uuid)
            .await
            .unwrap();

        assert_eq!(view.uuid, account.uuid);
        assert_eq!(view.username, account.username);
        assert_eq!(view.kind, account.kind);
        assert_eq!(view.last_login_at, None);
        assert_eq!(view.member_rank, MemberRank::Member);

        assert_eq!(view.member_id, member.discord_user_id);
        assert_eq!(view.member_name, member.name);
        assert_eq!(view.joined_at, member.joined_at);
    }
}
