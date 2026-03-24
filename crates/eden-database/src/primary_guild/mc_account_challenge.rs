use bon::Builder;
use eden_timestamp::Timestamp;
use error_stack::{Report, ResultExt};
use sqlx::{FromRow, prelude::Type};
use std::{net::IpAddr, time::Duration};
use thiserror::Error;
use uuid::Uuid;

/// Represents a Minecraft account challenge.
#[derive(Debug, Clone, FromRow)]
pub struct McAccountChallenge {
    pub id: Uuid,
    pub hashed_code: String,

    pub created_at: Timestamp,
    pub expires_at: Timestamp,

    pub uuid: Uuid,
    pub username: String,
    pub java: bool,

    #[sqlx(try_from = "crate::extractors::IpAddrString")]
    pub ip_address: IpAddr,
    pub status: McAccountChallengeStatus,
    pub updated_at: Option<Timestamp>,
}

impl McAccountChallenge {
    pub fn new_challenge<'a>() -> NewMcAccountChallengeBuilder<'a> {
        NewMcAccountChallenge::builder()
    }
}

impl McAccountChallenge {
    pub async fn find_in_progress(
        conn: &mut eden_sqlite::Connection,
        uuid: Uuid,
    ) -> Result<McAccountChallenge, Report<AccountChallengeQueryError>> {
        sqlx::query_as::<_, McAccountChallenge>(
            r#"SELECT * FROM mc_account_challenges
               WHERE uuid = ?
                 AND current_timestamp < expires_at"#,
        )
        .bind(uuid)
        .fetch_one(conn)
        .await
        .change_context(AccountChallengeQueryError)
        .attach("while trying to find a minecraft account challenge")
    }

    pub async fn find_by_hashed_code(
        conn: &mut eden_sqlite::Connection,
        hashed_code: &str,
    ) -> Result<McAccountChallenge, Report<AccountChallengeQueryError>> {
        sqlx::query_as::<_, McAccountChallenge>(
            r#"SELECT * FROM mc_account_challenges
               WHERE status = 'in-progress'
                 AND hashed_code = ?
                 AND current_timestamp < expires_at"#,
        )
        .bind(hashed_code)
        .fetch_one(conn)
        .await
        .change_context(AccountChallengeQueryError)
        .attach("while trying to find a minecraft account challenge by hashed code")
    }

    pub async fn mark_cancelled(
        conn: &mut eden_sqlite::Transaction<'_>,
        id: Uuid,
    ) -> Result<(), Report<AccountChallengeQueryError>> {
        sqlx::query(
            r#"
            UPDATE mc_account_challenges
            SET "status" = 'cancelled',
                hashed_code = '<deleted>',
                updated_at = ?
            WHERE id = ?"#,
        )
        .bind(Timestamp::now())
        .bind(id)
        .execute(&mut **conn)
        .await
        .change_context(AccountChallengeQueryError)
        .attach("while trying to mark a minecraft account challenge cancelled")?;

        Ok(())
    }

    pub async fn mark_done(
        conn: &mut eden_sqlite::Transaction<'_>,
        id: Uuid,
    ) -> Result<(), Report<AccountChallengeQueryError>> {
        sqlx::query(
            r#"
            UPDATE mc_account_challenges
            SET "status" = 'done',
                hashed_code = '<deleted>',
                updated_at = ?
            WHERE id = ?"#,
        )
        .bind(Timestamp::now())
        .bind(id)
        .execute(&mut **conn)
        .await
        .change_context(AccountChallengeQueryError)
        .attach("while trying to mark a minecraft account challenge done")?;

        Ok(())
    }
}

/// Error type representing a failure to query the mc_account_challenges table.
#[derive(Debug, Error)]
#[error("Failed to query mc_account_challenges table from the database")]
pub struct AccountChallengeQueryError;

#[derive(Builder)]
pub struct NewMcAccountChallenge<'a> {
    #[builder(default = Uuid::new_v4())]
    pub id: Uuid,
    pub hashed_code: &'a str,

    #[builder(default = Timestamp::now())]
    pub created_at: Timestamp,
    pub ttl: Duration,

    pub uuid: Uuid,
    pub username: &'a str,
    pub java: bool,
    pub ip_address: IpAddr,
}

impl<'a> NewMcAccountChallenge<'a> {
    pub async fn insert(
        &self,
        conn: &mut eden_sqlite::Transaction<'_>,
    ) -> Result<(Uuid, Timestamp), Report<AccountChallengeQueryError>> {
        #[derive(FromRow)]
        struct Row {
            id: Uuid,
            expires_at: Timestamp,
        }

        let ttl = self.ttl.as_secs().try_into().ok();
        let expires_at = ttl
            .and_then(|ttl: i64| self.created_at.timestamp().checked_add(ttl))
            .and_then(|v| Timestamp::from_secs(v).ok())
            .unwrap_or(self.created_at);

        // Cancel any existing in-progress attempts for same uuid or username.
        sqlx::query(
            r#"
            UPDATE mc_account_challenges
            SET "status" = 'cancelled',
                hashed_code = '<deleted>',
                updated_at = ?
            WHERE "status" = 'in-progress'
              AND (uuid = ? OR username = ?)"#,
        )
        .bind(Timestamp::now())
        .bind(self.uuid)
        .bind(self.username)
        .execute(&mut **conn)
        .await
        .change_context(AccountChallengeQueryError)
        .attach("while trying to cancel existing mc account challenge")?;

        sqlx::query_as::<_, Row>(
            r#"
            INSERT INTO mc_account_challenges (
                id, hashed_code, created_at, expires_at,
                java, uuid, username, ip_address
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            RETURNING id, expires_at"#,
        )
        .bind(self.id)
        .bind(self.hashed_code)
        .bind(self.created_at)
        .bind(expires_at)
        .bind(self.java)
        .bind(self.uuid)
        .bind(self.username)
        .bind(self.ip_address.to_string())
        .fetch_one(&mut **conn)
        .await
        .change_context(AccountChallengeQueryError)
        .attach("while trying to insert mc account challenge")
        .map(|v| (v.id, v.expires_at))
    }
}

/// Status values allowed by the table constraint.
/// Represents 'done', 'in-progress', 'cancelled'.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Type)]
#[sqlx(rename_all = "kebab-case")]
pub enum McAccountChallengeStatus {
    Done,
    InProgress,
    Cancelled,
}

#[cfg(test)]
mod tests {
    use std::{
        net::{IpAddr, Ipv4Addr},
        time::Duration,
    };
    use uuid::Uuid;

    use crate::primary_guild::{McAccountChallenge, McAccountChallengeStatus};

    #[tokio::test]
    async fn should_insert_attempt() {
        let pool = crate::testing::setup().await;

        let uuid = Uuid::new_v4();
        let username = "steve";
        let ip = IpAddr::V4(Ipv4Addr::LOCALHOST);

        let mut conn = pool.begin().await.unwrap();
        McAccountChallenge::new_challenge()
            .uuid(uuid)
            .hashed_code("hello")
            .username(username)
            .ip_address(ip)
            .ttl(Duration::ZERO)
            .java(false)
            .build()
            .insert(&mut conn)
            .await
            .unwrap();

        let fetched = McAccountChallenge::find_in_progress(&mut conn, uuid)
            .await
            .unwrap();

        assert_eq!(fetched.uuid, uuid);
        assert_eq!(fetched.hashed_code, "hello");
        assert_eq!(fetched.username, username);
        assert_eq!(fetched.ip_address, ip);
        assert_eq!(fetched.status, McAccountChallengeStatus::InProgress);
    }

    #[tokio::test]
    async fn should_cancel_existing_in_progress_when_inserting() {
        let pool = crate::testing::setup().await;

        let username = "alex";
        let ip = IpAddr::V4(Ipv4Addr::LOCALHOST);

        let mut conn = pool.begin().await.unwrap();

        // Insert an existing in-progress attempt via raw SQL
        let existing_id = Uuid::new_v4();
        let (old_challenge_id, _) = McAccountChallenge::new_challenge()
            .uuid(existing_id)
            .hashed_code("hello")
            .username(username)
            .ip_address(ip)
            .ttl(Duration::ZERO)
            .java(false)
            .build()
            .insert(&mut conn)
            .await
            .unwrap();

        // Insert a new attempt which should cancel the existing one
        McAccountChallenge::new_challenge()
            .uuid(existing_id)
            .hashed_code("hello")
            .username(username)
            .ip_address(ip)
            .ttl(Duration::ZERO)
            .java(false)
            .build()
            .insert(&mut conn)
            .await
            .unwrap();

        // Fetch the existing attempt and ensure its status is now cancelled
        let existing: McAccountChallenge =
            sqlx::query_as("SELECT * FROM mc_account_challenges WHERE uuid = ? AND id = ?")
                .bind(existing_id)
                .bind(old_challenge_id)
                .fetch_one(&mut *conn)
                .await
                .unwrap();

        assert_eq!(existing.hashed_code, "<deleted>");
        assert_eq!(existing.status, McAccountChallengeStatus::Cancelled);
    }
}
