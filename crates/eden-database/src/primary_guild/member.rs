use bon::Builder;
use error_stack::{Report, ResultExt};
use sqlx::FromRow;
use std::str::FromStr;
use thiserror::Error;
use twilight_model::guild::Member as GuildMember;
use twilight_model::id::{Id, marker::UserMarker};

use crate::{Snowflake, Timestamp};

/// Represents a member who joined the primary guild's Minecraft server.
#[derive(Debug, Clone, FromRow)]
pub struct Member {
    pub discord_user_id: Snowflake,
    pub joined_at: Timestamp,
    pub name: String,
    pub updated_at: Option<Timestamp>,
}

impl Member {
    pub async fn find_by_discord_user_id(
        conn: &mut eden_sqlite::Connection,
        id: Snowflake,
    ) -> Result<Option<Self>, Report<MemberQueryError>> {
        sqlx::query_as::<_, Member>(
            r#"
            SELECT * FROM members
            WHERE discord_user_id = ?"#,
        )
        .bind(id)
        .fetch_optional(conn)
        .await
        .change_context(MemberQueryError)
        .attach("while trying to find player by user id")
    }

    pub fn upsert<'a>() -> UpsertMemberBuilder<'a> {
        UpsertMember::builder()
    }
}

/// Error type representing a failure to query with the [`Member`] table.
#[derive(Debug, Error)]
#[error("Failed to query member table from the database")]
pub struct MemberQueryError;

#[derive(Builder)]
#[cfg_attr(test, derive(Debug))]
pub struct UpsertMember<'a> {
    pub discord_user_id: Id<UserMarker>,
    #[builder(default = Timestamp::now())]
    pub joined_at: Timestamp,
    pub name: &'a str,
}

impl<'a> UpsertMember<'a> {
    pub async fn perform(
        &self,
        conn: &mut eden_sqlite::Transaction<'_>,
    ) -> Result<Member, Report<MemberQueryError>> {
        sqlx::query_as::<_, Member>(
            r#"
            INSERT INTO members (discord_user_id, joined_at, name)
            VALUES (?, ?, ?)
            ON CONFLICT (discord_user_id)
                DO UPDATE
                SET name = excluded.name,
                    updated_at = current_timestamp
            RETURNING *"#,
        )
        .bind(Snowflake::new(self.discord_user_id.cast()))
        .bind(self.joined_at)
        .bind(self.name)
        .fetch_one(&mut **conn)
        .await
        .change_context(MemberQueryError)
        .attach("while trying to upsert member")
    }

    #[must_use]
    pub fn from_guild(member: &'a GuildMember) -> Self {
        let joined_at = member.joined_at.map(|v| {
            Timestamp::from_str(&v.iso_8601().to_string())
                .expect("twilight should emit correct ISO 8601")
        });

        Self::builder()
            .discord_user_id(member.user.id)
            .maybe_joined_at(joined_at)
            .name(&member.user.name)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::UpsertMember;
    use crate::Timestamp as DbTimestamp;

    use eden_sqlite::Pool;
    use std::str::FromStr;
    use twilight_model::guild::{Member, MemberFlags};
    use twilight_model::id::Id;
    use twilight_model::user::User;
    use twilight_model::util::Timestamp;

    #[tokio::test]
    async fn upsert_should_update_if_exists() {
        eden_common::testing::init();

        let joined_at = DbTimestamp::from_str("2024-01-01T00:00:00Z").unwrap();
        let pool = Pool::memory(None).await;
        crate::migrations::perform(&pool).await.unwrap();

        let mut conn = pool.begin().await.unwrap();
        let initial = UpsertMember::builder()
            .discord_user_id(Id::new(123455))
            .joined_at(joined_at)
            .name("john")
            .build()
            .perform(&mut conn)
            .await
            .unwrap();

        let now = UpsertMember::builder()
            .discord_user_id(Id::new(123455))
            .joined_at(joined_at)
            .name("john2")
            .build()
            .perform(&mut conn)
            .await
            .unwrap();

        assert_eq!(now.name, "john2");
        assert_eq!(now.joined_at, initial.joined_at);
        assert_eq!(now.discord_user_id, initial.discord_user_id);
    }

    #[tokio::test]
    async fn upsert_should_insert_if_not_exists() {
        eden_common::testing::init();

        let joined_at = DbTimestamp::from_str("2024-01-01T00:00:00Z").unwrap();

        let pool = Pool::memory(None).await;
        crate::migrations::perform(&pool).await.unwrap();

        let mut conn = pool.begin().await.unwrap();
        let member = UpsertMember::builder()
            .discord_user_id(Id::new(123455))
            .joined_at(joined_at)
            .name("john")
            .build()
            .perform(&mut conn)
            .await
            .unwrap();

        assert_eq!(member.discord_user_id.get(), 123455);
        assert_eq!(member.joined_at, joined_at);
        assert_eq!(member.name, "john");
    }

    #[test]
    fn test_upsert_member_builder_from_guild_member() {
        let member = Member {
            avatar: None,
            avatar_decoration_data: None,
            banner: None,
            communication_disabled_until: None,
            deaf: false,
            flags: MemberFlags::empty(),
            joined_at: Some(Timestamp::parse("2022-01-01T00:00:00+00:00").unwrap()),
            mute: false,
            nick: None,
            pending: false,
            premium_since: None,
            roles: Vec::new(),
            user: User {
                accent_color: None,
                avatar: None,
                avatar_decoration: None,
                avatar_decoration_data: None,
                banner: None,
                bot: false,
                discriminator: 0,
                email: None,
                flags: None,
                global_name: None,
                id: Id::new(12234),
                locale: None,
                mfa_enabled: None,
                name: "foo".to_string(),
                premium_type: None,
                primary_guild: None,
                public_flags: None,
                system: None,
                verified: None,
            },
        };

        let query = UpsertMember::from_guild(&member);
        insta::assert_debug_snapshot!(query);
    }
}
