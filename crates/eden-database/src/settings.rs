use bon::Builder;
use eden_config::sections::setup::InitialSettings;
use eden_timestamp::Timestamp;
use error_stack::{Report, ResultExt};
use sqlx::FromRow;
use thiserror::Error;
use twilight_model::id::{Id, marker::GuildMarker};

use crate::Snowflake;

#[derive(Clone, Debug, FromRow)]
pub struct Settings {
    pub guild_id: Snowflake,
    pub created_at: Timestamp,
    pub updated_at: Option<Timestamp>,
    pub allow_guests: bool,
}

impl Settings {
    pub async fn find_or_insert(
        conn: &mut eden_sqlite::Transaction<'_>,
        guild_id: Id<GuildMarker>,
        setup: &InitialSettings,
    ) -> Result<Settings, Report<SettingsQueryError>> {
        let existing = sqlx::query_as::<_, Settings>(
            r#"
            SELECT * FROM settings
            WHERE guild_id = ?"#,
        )
        .bind(Snowflake::new(guild_id.cast()))
        .fetch_optional(&mut **conn)
        .await
        .change_context(SettingsQueryError)
        .attach("while checking whether specific settings exists")?;

        if let Some(existing) = existing {
            return Ok(existing);
        }

        Self::upsert(guild_id, setup).perform(conn).await
    }

    pub fn upsert(guild_id: Id<GuildMarker>, setup: &InitialSettings) -> UpsertSettings {
        UpsertSettings::builder()
            .guild_id(Snowflake::new(guild_id.cast()))
            .allow_guests(setup.allow_guests)
            .build()
    }
}

/// Error type representing a failure to query with the [`Settings`] table.
#[derive(Debug, Error)]
#[error("Failed to query settings table from the database")]
pub struct SettingsQueryError;

#[derive(Builder)]
pub struct UpsertSettings {
    pub guild_id: Snowflake,
    pub allow_guests: bool,
}

impl UpsertSettings {
    pub async fn perform(
        &self,
        conn: &mut eden_sqlite::Transaction<'_>,
    ) -> Result<Settings, Report<SettingsQueryError>> {
        sqlx::query_as::<_, Settings>(
            r#"
            INSERT INTO settings
            VALUES (?, ?, NULL, ?)
            ON CONFLICT (guild_id)
                DO UPDATE
                SET updated_at = excluded.created_at,
                    allow_guests = excluded.allow_guests
            RETURNING *
            "#,
        )
        .bind(Snowflake::new(self.guild_id.cast()))
        .bind(Timestamp::now())
        .bind(self.allow_guests)
        .fetch_one(&mut **conn)
        .await
        .change_context(SettingsQueryError)
        .attach("while trying to upsert settings")
    }
}

#[cfg(test)]
mod tests {
    use eden_config::sections::setup::InitialSettings;
    use twilight_model::id::Id;

    use crate::Settings;

    #[allow(clippy::needless_update)]
    #[tokio::test]
    async fn should_upsert() {
        let pool = crate::testing::setup().await;

        let mut conn = pool.begin().await.unwrap();
        let initial = InitialSettings::default();
        let settings = Settings::upsert(Id::new(1234), &initial)
            .perform(&mut conn)
            .await
            .unwrap();

        assert_eq!(settings.guild_id.into_inner(), Id::new(1234));
        assert_eq!(settings.allow_guests, initial.allow_guests);

        // upsert once more and it should set updated_at to Some(..)
        let settings = Settings::upsert(
            Id::new(1234),
            &InitialSettings {
                allow_guests: false,
                ..Default::default()
            },
        )
        .perform(&mut conn)
        .await
        .unwrap();

        assert_eq!(settings.guild_id.into_inner(), Id::new(1234));
        assert_ne!(settings.allow_guests, initial.allow_guests);
        assert!(settings.updated_at.is_some());
    }
}
