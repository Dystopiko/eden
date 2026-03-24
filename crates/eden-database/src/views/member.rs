use eden_timestamp::Timestamp;
use error_stack::{Report, ResultExt};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Type};
use std::fmt;
use thiserror::Error;
use twilight_model::id::{Id, marker::UserMarker};

use crate::Snowflake;

#[derive(Clone, Debug, FromRow)]
pub struct MemberView {
    pub discord_user_id: Snowflake,
    pub joined_at: Timestamp,
    pub name: String,
    pub rank: MemberRank,
}

/// Error type representing a failure to query with the [`MemberView`] table.
#[derive(Debug, Error)]
#[error("Failed to query member view from the database")]
pub struct MemberViewQueryError;

impl MemberView {
    pub async fn find_by_discord_user_id(
        conn: &mut eden_sqlite::Connection,
        id: Id<UserMarker>,
    ) -> Result<Self, Report<MemberViewQueryError>> {
        sqlx::query_as::<_, MemberView>(
            r#"
            SELECT * FROM member_view
            WHERE discord_user_id = ?"#,
        )
        .bind(Snowflake::new(id.cast()))
        .fetch_one(conn)
        .await
        .change_context(MemberViewQueryError)
        .attach("while trying to find member by user id in view")
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Hash, Serialize, Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "member_rank", rename_all = "lowercase")]
pub enum MemberRank {
    Admin,
    Staff,
    Contributor,
    Member,
}

impl MemberRank {
    #[must_use]
    pub const fn is_base(&self) -> bool {
        matches!(self, Self::Member)
    }

    #[must_use]
    pub const fn is_admin(&self) -> bool {
        matches!(self, Self::Admin)
    }

    #[must_use]
    pub const fn is_staff(&self) -> bool {
        matches!(self, Self::Admin | Self::Staff)
    }

    #[must_use]
    pub const fn is_contributor(&self) -> bool {
        matches!(self, Self::Contributor)
    }
}

impl fmt::Display for MemberRank {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Admin => f.write_str("admin"),
            Self::Staff => f.write_str("staff"),
            Self::Contributor => f.write_str("contributor"),
            Self::Member => f.write_str("member"),
        }
    }
}

#[cfg(test)]
mod tests {
    use twilight_model::id::Id;

    use crate::{
        primary_guild::{Member, contributor::Contributor, staff::Staff},
        views::{MemberRank, MemberView},
    };

    #[tokio::test]
    async fn rank_should_be_member_by_default() {
        let pool = crate::testing::setup().await;
        let mut conn = pool.begin().await.unwrap();

        let member = setup_member(&mut conn).await;
        let view = MemberView::find_by_discord_user_id(&mut conn, member.discord_user_id.cast())
            .await
            .unwrap();

        assert_eq!(view.rank, MemberRank::Member);
    }

    #[tokio::test]
    async fn rank_should_be_contributor() {
        let pool = crate::testing::setup().await;
        let mut conn = pool.begin().await.unwrap();

        let member = setup_member(&mut conn).await;
        Contributor::upsert()
            .member_id(member.discord_user_id.cast())
            .build()
            .perform(&mut conn)
            .await
            .unwrap();

        let view = MemberView::find_by_discord_user_id(&mut conn, member.discord_user_id.cast())
            .await
            .unwrap();

        assert_eq!(view.rank, MemberRank::Contributor);
    }

    #[tokio::test]
    async fn rank_should_be_staff() {
        let pool = crate::testing::setup().await;
        let mut conn = pool.begin().await.unwrap();

        let member = setup_member(&mut conn).await;
        Staff::upsert()
            .member_id(member.discord_user_id)
            .build()
            .perform(&mut conn)
            .await
            .unwrap();

        let view = MemberView::find_by_discord_user_id(&mut conn, member.discord_user_id.cast())
            .await
            .unwrap();

        assert_eq!(view.rank, MemberRank::Staff);
    }

    #[tokio::test]
    async fn rank_should_be_admin() {
        let pool = crate::testing::setup().await;
        let mut conn = pool.begin().await.unwrap();

        let member = setup_member(&mut conn).await;
        Staff::upsert()
            .member_id(member.discord_user_id)
            .admin(true)
            .build()
            .perform(&mut conn)
            .await
            .unwrap();

        let view = MemberView::find_by_discord_user_id(&mut conn, member.discord_user_id.cast())
            .await
            .unwrap();

        assert_eq!(view.rank, MemberRank::Admin);
    }

    #[must_use]
    async fn setup_member(conn: &mut eden_sqlite::Transaction<'_>) -> Member {
        Member::upsert()
            .discord_user_id(Id::new(1234))
            .name("steve")
            .build()
            .perform(conn)
            .await
            .unwrap()
    }
}
