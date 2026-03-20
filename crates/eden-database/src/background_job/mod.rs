use bon::Builder;
use error_stack::{Report, ResultExt};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use thiserror::Error;
use uuid::Uuid;

use crate::timestamp::Timestamp;

#[derive(Clone, Debug, Deserialize, Eq, FromRow, PartialEq, Serialize)]
pub struct BackgroundJob {
    pub id: Uuid,
    #[sqlx(rename = "type")]
    pub kind: String,
    pub created_at: Timestamp,
    pub data: String,
    pub last_retry: Option<Timestamp>,
    pub priority: i16,
    pub retries: i16,
    pub status: JobStatus,
}

impl BackgroundJob {
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> NewBackgroundJobBuilder {
        NewBackgroundJob::builder()
    }

    pub async fn clear(conn: &mut eden_sqlite::Connection) -> Result<u64, Report<JobQueryError>> {
        sqlx::query("TRUNCATE TABLE background_jobs")
            .execute(conn)
            .await
            .change_context(JobQueryError)
            .attach("while trying to clear all background jobs")
            .map(|v| v.rows_affected())
    }

    pub async fn delete(
        conn: &mut eden_sqlite::Connection,
        id: Uuid,
    ) -> Result<(), Report<JobQueryError>> {
        sqlx::query(
            "DELETE FROM background_jobs
            WHERE id = ?
            RETURNING *",
        )
        .bind(id)
        .fetch_one(conn)
        .await
        .change_context(JobQueryError)
        .attach("while trying to delete a background job")?;

        Ok(())
    }

    pub async fn find_by_id(
        conn: &mut eden_sqlite::Connection,
        id: Uuid,
    ) -> Result<Self, Report<JobQueryError>> {
        sqlx::query_as::<_, BackgroundJob>("SELECT * FROM background_jobs WHERE id = ?")
            .bind(id)
            .fetch_one(conn)
            .await
            .change_context(JobQueryError)
            .attach("while trying to find background job metadata by id")
    }

    pub async fn pull_next_pending(
        conn: &mut eden_sqlite::Connection,
        now: Option<Timestamp>,
    ) -> Result<Option<Self>, Report<JobQueryError>> {
        // SQLite's default bundle library does not come with power function,
        // manual implementation is needed to make it sort of work?
        //
        // This operation is a bit heavy!
        sqlx::query_as::<_, BackgroundJob>(
            r#"
            UPDATE background_jobs
            SET last_retry = CURRENT_TIMESTAMP,
                retries = retries + 1,
                status = 'running'
            WHERE id IN (
                SELECT id FROM background_jobs
                WHERE status = 'enqueued'
                   AND (last_retry IS NULL
                   OR datetime(?) >= datetime(last_retry, '+' ||
                      CASE WHEN retries <= 0 THEN 0
                      ELSE 2 << (retries - 1)
                      END || ' minutes'))
                ORDER BY priority DESC, created_at ASC
                LIMIT 1
            )
            RETURNING *"#,
        )
        .bind(now.unwrap_or_else(Timestamp::now))
        .fetch_optional(conn)
        .await
        .change_context(JobQueryError)
        .attach("while trying to find the next pending background job")
    }

    pub async fn requeue_or_fail(
        conn: &mut eden_sqlite::Connection,
        id: Uuid,
        max_retries: Option<u16>,
    ) -> Result<JobStatus, Report<JobQueryError>> {
        sqlx::query_scalar::<_, JobStatus>(
            r#"
            UPDATE background_jobs
            SET status = CASE
                WHEN ? IS NOT NULL AND retries + 1 > ? THEN 'failed'
                ELSE 'enqueued'
            END
            WHERE id = ?
            RETURNING status"#,
        )
        .bind(max_retries)
        .bind(max_retries)
        .bind(id)
        .fetch_one(conn)
        .await
        .change_context(JobQueryError)
        .attach("while trying to requeue a background job")
    }
}

// ================================================================================ //
#[derive(Builder)]
#[must_use = "this does not do anything unless it is called to execute"]
pub struct NewBackgroundJob {
    #[builder(default = Uuid::new_v4())]
    pub id: Uuid,
    pub kind: &'static str,
    pub created_at: Option<Timestamp>,
    #[builder(setters(name = "data_internal", vis = ""))]
    pub data: String,
    #[builder(default = 0)]
    pub priority: i16,
}

type DataSetBuilder<S> = NewBackgroundJobBuilder<new_background_job_builder::SetData<S>>;

impl<S> NewBackgroundJobBuilder<S>
where
    S: new_background_job_builder::State,
{
    pub fn data<D>(self, data: D) -> Result<DataSetBuilder<S>, serde_json::Error>
    where
        D: serde::Serialize,
        S::Data: new_background_job_builder::IsUnset,
    {
        let data = serde_json::to_string(&data)?;
        Ok(self.data_internal(data))
    }
}

impl NewBackgroundJob {
    pub async fn enqueue(
        self,
        conn: &mut eden_sqlite::Connection,
    ) -> Result<Uuid, Report<JobQueryError>> {
        sqlx::query_scalar::<_, Uuid>(
            r#"
            INSERT INTO background_jobs(id, created_at, type, data, priority)
            VALUES (?, ?, ?, ?, ?)
            RETURNING id"#,
        )
        .bind(self.id)
        .bind(self.created_at.unwrap_or_else(Timestamp::now))
        .bind(self.kind)
        .bind(self.data)
        .bind(self.priority)
        .fetch_one(conn)
        .await
        .change_context(JobQueryError)
        .attach("while trying to enqueue a background job to the database")
    }

    pub async fn enqueue_unique(
        self,
        conn: &mut eden_sqlite::Connection,
    ) -> Result<Option<Uuid>, Report<JobQueryError>> {
        // Delete the existing job of the same type if it failed previously.
        sqlx::query(
            r#"
            DELETE FROM background_jobs
            WHERE type = ? AND status = 'failed'"#,
        )
        .bind(self.kind)
        .execute(&mut *conn)
        .await
        .change_context(JobQueryError)
        .attach("while trying to enqueue a background job to the database")?;

        let query = sqlx::query_scalar::<_, Uuid>(
            r#"
            INSERT INTO background_jobs (id, created_at, type, data, priority)
                SELECT ?, ?, ?, ?, ?
                WHERE NOT EXISTS (SELECT * FROM background_jobs WHERE type = ?)
            RETURNING *"#,
        );

        let query = query
            .bind(self.id)
            .bind(self.created_at.unwrap_or_else(Timestamp::now))
            .bind(self.kind)
            .bind(self.data)
            .bind(self.priority)
            .bind(self.kind);

        query
            .fetch_optional(conn)
            .await
            .change_context(JobQueryError)
            .attach("while trying to enqueue a background job to the database")
    }
}

// ================================================================================ //
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum JobStatus {
    Enqueued,
    Running,
    Failed,
}

/// Error type representing a failure to query with the [`BackgroundJob`] table.
#[derive(Debug, Error)]
#[error("Failed to query background job table from the database")]
pub struct JobQueryError;

#[cfg(test)]
mod tests;
