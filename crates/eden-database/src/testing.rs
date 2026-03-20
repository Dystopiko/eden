use sqlx::migrate::Migrator;

pub(crate) static MIGRATIONS: Migrator = sqlx::migrate!("../../migrations");

pub async fn perform_migrations(pool: &eden_sqlite::Pool) {
    let mut conn = pool.begin().await.unwrap();

    // `run_direct` is being used here because there's a conflict
    // between lifetimes of the connection and the function here.
    //
    // The implementation `.run(...)` to acquire a connection to the
    // database is just acquiring it then call `.run_direct(...)` afterwards
    // but the parameter requires that is implemented with `Acquire<'a>`.
    MIGRATIONS.run_direct(&mut *conn).await.unwrap();
    conn.commit().await.unwrap();
}

#[cfg(test)]
pub(crate) async fn setup() -> eden_sqlite::Pool {
    eden_utils::testing::init();

    let pool = eden_sqlite::Pool::memory(None).await;
    crate::testing::perform_migrations(&pool).await;

    pool
}

#[cfg(test)]
#[tokio::test]
async fn all_migrations_should_run_successfully() {
    let pool = eden_sqlite::Pool::memory(None).await;
    perform_migrations(&pool).await;
}
