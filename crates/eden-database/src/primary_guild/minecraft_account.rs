use bon::Builder;
use error_stack::{Report, ResultExt};
use sqlx::{FromRow, Type};
use thiserror::Error;
use twilight_model::id::{Id, marker::UserMarker};
use uuid::Uuid;

use crate::{Snowflake, Timestamp};

/// A Minecraft account linked to a primary guild member.
#[derive(Debug, Clone, FromRow)]
pub struct MinecraftAccount {
    pub id: i32,
    pub linked_at: Timestamp,
    pub discord_user_id: Snowflake,
    pub uuid: Uuid,
    pub username: String,
    #[sqlx(rename = "type")]
    pub kind: McAccountType,
}

impl MinecraftAccount {
    pub async fn find_by_uuid(
        conn: &mut eden_sqlite::Connection,
        uuid: Uuid,
    ) -> Result<Option<Self>, Report<MinecraftAccountQueryError>> {
        sqlx::query_as::<_, MinecraftAccount>(
            r#"
            SELECT * FROM minecraft_accounts
            WHERE uuid = ?"#,
        )
        .bind(uuid)
        .fetch_optional(conn)
        .await
        .change_context(MinecraftAccountQueryError)
        .attach("while trying to find minecraft account by uuid")
    }
}

impl MinecraftAccount {
    #[allow(clippy::new_ret_no_self)]
    pub fn new<'a>() -> NewMinecraftAccountBuilder<'a> {
        NewMinecraftAccount::builder()
    }
}

/// Error type representing a failure to query with the [`MinecraftAccount`] table.
#[derive(Debug, Error)]
#[error("Failed to query minecraft account table from the database")]
pub struct MinecraftAccountQueryError;

#[derive(Builder)]
pub struct NewMinecraftAccount<'a> {
    pub discord_user_id: Id<UserMarker>,
    pub uuid: Uuid,
    pub username: &'a str,
    pub account_type: McAccountType,
}

impl<'a> NewMinecraftAccount<'a> {
    pub async fn create(
        &self,
        conn: &mut eden_sqlite::Transaction<'_>,
    ) -> Result<MinecraftAccount, Report<MinecraftAccountQueryError>> {
        sqlx::query_as::<_, MinecraftAccount>(
            r#"
            INSERT INTO minecraft_accounts (
                discord_user_id, uuid, username, "type"
            )
            VALUES (?, ?, ?, ?)
            RETURNING *"#,
        )
        .bind(Snowflake::new(self.discord_user_id.cast()))
        .bind(self.uuid)
        .bind(self.username)
        .bind(self.account_type)
        .fetch_one(&mut **conn)
        .await
        .change_context(MinecraftAccountQueryError)
        .attach("while trying to create minecraft account")
    }
}

/// Differentiates between Minecraft editions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Type)]
#[sqlx(type_name = "mc_account_type", rename_all = "lowercase")]
pub enum McAccountType {
    Java,
    Bedrock,
}

impl MinecraftAccount {
    /// Returns `true` if this account is a Java edition account.
    #[must_use]
    pub const fn is_java(&self) -> bool {
        matches!(self.kind, McAccountType::Java)
    }

    /// Returns `true` if this account is a Bedrock edition account.
    #[must_use]
    pub const fn is_bedrock(&self) -> bool {
        matches!(self.kind, McAccountType::Bedrock)
    }
}

#[cfg(test)]
mod tests {
    use claims::assert_err;
    use eden_common::testing::expect_error_containing;
    use twilight_model::id::Id;
    use twilight_model::id::marker::UserMarker;
    use uuid::Uuid;

    use crate::primary_guild::member::Member;
    use crate::primary_guild::minecraft_account::{McAccountType, MinecraftAccount};

    #[must_use]
    async fn setup_member(
        conn: &mut eden_sqlite::Transaction<'_>,
        id: Id<UserMarker>,
        name: &str,
    ) -> Member {
        Member::upsert()
            .discord_user_id(id)
            .name(name)
            .build()
            .perform(conn)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn should_not_hold_account_by_same_user_id_and_uuid() {
        eden_common::testing::init();

        let pool = eden_sqlite::Pool::memory(None).await;
        crate::migrations::perform(&pool).await.unwrap();

        let id = Id::new(1234);
        let uuid = Uuid::new_v4();

        let mut conn = pool.begin().await.unwrap();
        _ = setup_member(&mut conn, id, "john").await;

        MinecraftAccount::new()
            .account_type(McAccountType::Java)
            .discord_user_id(id)
            .username("john")
            .uuid(uuid)
            .build()
            .create(&mut conn)
            .await
            .unwrap();

        // Case #1: duplicated UUID and user's ID with different name
        let result = MinecraftAccount::new()
            .account_type(McAccountType::Java)
            .discord_user_id(id)
            .username("john2")
            .uuid(uuid)
            .build()
            .create(&mut conn)
            .await;

        assert_err!(&result);

        let error = result.unwrap_err();
        expect_error_containing(error, "(code: 2067) UNIQUE constraint failed:");

        // Case #2: duplicated UUID and user's ID with different account type
        //          (Bedrock XUID is different when it comes to generating it compared to Java UUID)
        let result = MinecraftAccount::new()
            .account_type(McAccountType::Bedrock)
            .discord_user_id(id)
            .username("john")
            .uuid(uuid)
            .build()
            .create(&mut conn)
            .await;

        assert_err!(&result);

        let error = result.unwrap_err();
        expect_error_containing(error, "(code: 2067) UNIQUE constraint failed:");
    }

    #[tokio::test]
    async fn should_insert_account() {
        eden_common::testing::init();

        let pool = eden_sqlite::Pool::memory(None).await;
        crate::migrations::perform(&pool).await.unwrap();

        let id = Id::new(1234);
        let uuid = Uuid::new_v4();

        let mut conn = pool.begin().await.unwrap();
        _ = setup_member(&mut conn, id, "john").await;

        let account = MinecraftAccount::new()
            .account_type(McAccountType::Java)
            .discord_user_id(id)
            .username("john")
            .uuid(uuid)
            .build()
            .create(&mut conn)
            .await
            .unwrap();

        assert_eq!(account.kind, McAccountType::Java);
        assert_eq!(account.username, "john");
        assert_eq!(account.uuid, uuid);
        assert_eq!(account.discord_user_id.get(), 1234);
    }
}
