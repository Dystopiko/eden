use error_stack::{Report, ResultExt};
use sqlx::migrate::Migrator;
use std::time::Instant;
use thiserror::Error;

pub(crate) static MIGRATIONS: Migrator = sqlx::migrate!("../../migrations");

#[derive(Debug, Error)]
#[error("Failed to run database migrations")]
pub struct RunMigrationsError;

#[tracing::instrument(skip_all, name = "db.perform_migrations")]
pub async fn perform(pool: &eden_sqlite::Pool) -> Result<(), Report<RunMigrationsError>> {
    tracing::info!("Performing database migrations (this will may take a while)...");
    let now = Instant::now();

    // We're using `.begin()` since this function may be cancelled by
    // our watchdog (`service` function relies on `perform` function anyways).
    let mut conn = pool.begin().await.change_context(RunMigrationsError)?;

    // `run_direct` is being used here because there's a conflict
    // between lifetimes of the connection and the function here.
    //
    // The implementation `.run(...)` to acquire a connection to the
    // database is just acquiring it then call `.run_direct(...)` afterwards
    // but the parameter requires that is implemented with `Acquire<'a>`.
    MIGRATIONS
        .run_direct(&mut *conn)
        .await
        .change_context(RunMigrationsError)?;

    conn.commit().await.change_context(RunMigrationsError)?;

    let elapsed = now.elapsed();
    tracing::info!(?elapsed, "Successfully performed database migrations");

    Ok(())
}
