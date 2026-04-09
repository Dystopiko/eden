use error_stack::{Report, ResultExt};
use sqlx::migrate::{Migrate, Migrator};
use std::{collections::HashMap, time::Instant};
use thiserror::Error;

pub(crate) static MIGRATIONS: Migrator = sqlx::migrate!("../../migrations");

#[derive(Debug, Error)]
#[error("Failed to check database migrations")]
pub struct CheckMigrationsError;

#[derive(Debug, Error)]
#[error("Failed to run database migrations")]
pub struct RunMigrationsError;

pub async fn needs_migration(
    pool: &eden_sqlite::Pool,
) -> Result<bool, Report<CheckMigrationsError>> {
    let mut conn = pool.begin().await.change_context(CheckMigrationsError)?;
    conn.ensure_migrations_table()
        .await
        .change_context(CheckMigrationsError)?;

    let applied_migrations = conn
        .list_applied_migrations()
        .await
        .change_context(CheckMigrationsError)?;

    let applied_migrations: HashMap<_, _> = applied_migrations
        .into_iter()
        .map(|m| (m.version, m))
        .collect();

    let needs_migration = MIGRATIONS
        .iter()
        .filter(|v| !v.migration_type.is_down_migration())
        .any(|v| !applied_migrations.contains_key(&v.version));

    Ok(needs_migration)
}

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
