use bon::Builder;
use error_stack::{Report, ResultExt};
use sqlx::FromRow;
use thiserror::Error;
use twilight_model::user::User;

use crate::{Snowflake, Timestamp};

/// Represents a Discord user registered to the primary guild's Minecraft server.
#[derive(Debug, Clone, FromRow)]
pub struct RegisteredPlayer {
    /// The player's Discord snowflake ID.
    pub discord_user_id: Snowflake,

    /// When this registration was created.
    pub created_at: Timestamp,

    /// The player's Discord username.
    pub name: String,

    /// When this row was updated.
    pub updated_at: Option<Timestamp>,
}

/// Error type representing a failure to interact with the [`RegisteredPlayer`] table.
#[derive(Debug, Error)]
#[error("Failed to query registered player table from the database")]
pub struct PlayerQueryError;

impl RegisteredPlayer {
    #[tracing::instrument(skip_all, name = "db.registered_players.find_by_id")]
    pub async fn find_by_user_id(
        conn: &mut eden_sqlite::Connection,
        id: Snowflake,
    ) -> Result<Option<Self>, Report<PlayerQueryError>> {
        sqlx::query_as::<_, RegisteredPlayer>(
            r#"SELECT * FROM "primary_guild.registered_player"
            WHERE discord_user_id = ?"#,
        )
        .bind(id)
        .fetch_optional(conn)
        .await
        .change_context(PlayerQueryError)
        .attach("could not find player in the database")
    }
}

#[derive(Debug, Builder)]
pub struct UpsertPlayer<'a> {
    pub discord_user_id: Snowflake,
    #[builder(default = Timestamp::now())]
    pub created_at: Timestamp,
    pub name: &'a str,
}

type FromUserState = upsert_player_builder::SetName<upsert_player_builder::SetDiscordUserId>;

impl<'a> UpsertPlayer<'a> {
    pub fn builder_from_user(user: &'a User) -> UpsertPlayerBuilder<'a, FromUserState> {
        Self::builder()
            .discord_user_id(user.id.into())
            .name(&user.name)
    }

    #[tracing::instrument(skip_all, name = "db.registered_players.upsert")]
    pub async fn execute(
        &self,
        conn: &mut eden_sqlite::Connection,
    ) -> Result<RegisteredPlayer, Report<PlayerQueryError>> {
        sqlx::query_as::<_, RegisteredPlayer>(
            r#"
            INSERT INTO "primary_guild.registered_player" (discord_user_id, created_at, name)
            VALUES (?, ?, ?)
            ON CONFLICT (discord_user_id) DO UPDATE
                SET name = excluded.name,
                    updated_at = datetime(current_timestamp, 'utc')
            RETURNING *
            "#,
        )
        .bind(self.discord_user_id)
        .bind(self.created_at)
        .bind(self.name)
        .fetch_one(conn)
        .await
        .change_context(PlayerQueryError)
        .attach("could not upsert registered player")
    }
}

#[cfg(test)]
mod tests {
    use claims::{assert_ok, assert_some};
    use eden_sqlite::Pool;
    use std::str::FromStr;
    use twilight_model::id::Id;

    use crate::primary_guild::registered_player::{RegisteredPlayer, UpsertPlayer};
    use crate::{Snowflake, Timestamp};

    #[tokio::test]
    async fn test_find_by_user_id() {
        eden_common::testing::init();

        let pool = Pool::memory(None).await;
        crate::migrations::perform(&pool).await.unwrap();

        let mut conn = pool.acquire().await.unwrap();
        let player = UpsertPlayer::builder()
            .discord_user_id(Snowflake::new(Id::new(1234)))
            .name("eden")
            .created_at(Timestamp::from_str("2024-01-01T00:00:00Z").unwrap())
            .build()
            .execute(&mut conn)
            .await
            .unwrap();

        let query = RegisteredPlayer::find_by_user_id(&mut conn, player.discord_user_id)
            .await
            .unwrap();

        assert_some!(query);
    }

    #[tokio::test]
    async fn should_upsert_player_if_duplicated() {
        eden_common::testing::init();

        let pool = Pool::memory(None).await;
        crate::migrations::perform(&pool).await.unwrap();

        let mut conn = pool.acquire().await.unwrap();
        let player = UpsertPlayer::builder()
            .discord_user_id(Snowflake::new(Id::new(1234)))
            .name("eden")
            .created_at(Timestamp::from_str("2024-01-01T00:00:00Z").unwrap())
            .build()
            .execute(&mut conn)
            .await
            .unwrap();

        let new_player = UpsertPlayer::builder()
            .discord_user_id(player.discord_user_id)
            .name("eden-v2")
            .build()
            .execute(&mut conn)
            .await
            .unwrap();

        assert_eq!(new_player.discord_user_id, player.discord_user_id);
        assert_eq!(new_player.created_at, player.created_at);
        assert_eq!(new_player.name, "eden-v2");
        assert!(new_player.updated_at.is_some());
    }

    #[tokio::test]
    async fn should_upsert_player_if_player_not_exists() {
        eden_common::testing::init();

        let pool = Pool::memory(None).await;
        crate::migrations::perform(&pool).await.unwrap();

        let mut conn = pool.acquire().await.unwrap();
        let result = UpsertPlayer::builder()
            .discord_user_id(Snowflake::new(Id::new(1234)))
            .name("eden")
            .created_at(Timestamp::from_str("2024-01-01T00:00:00Z").unwrap())
            .build()
            .execute(&mut conn)
            .await;

        assert_ok!(&result);

        let player = result.unwrap();
        assert_eq!(player.discord_user_id, Snowflake::new(Id::new(1234)));
        assert_eq!(player.name, "eden");
        assert_eq!(
            player.created_at,
            Timestamp::from_str("2024-01-01T00:00:00Z").unwrap()
        );
    }
}
