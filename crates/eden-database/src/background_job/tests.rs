use eden_timestamp::Timestamp;
use serde_json::json;
use std::str::FromStr;
use uuid::Uuid;

use crate::background_job::{BackgroundJob, JobStatus};

#[tokio::test]
async fn test_requeue_or_fail_requeue_should_fail_retries_reached_max_retries() {
    let pool = crate::testing::setup().await;
    let mut conn = pool.begin().await.unwrap();

    let job = enqueue_job()
        .conn(&mut conn)
        .kind("job1")
        .data(json!({}))
        .call()
        .await;

    BackgroundJob::requeue_or_fail(&mut conn, job, None)
        .await
        .unwrap();

    let metadata = BackgroundJob::find_by_id(&mut conn, job).await.unwrap();
    assert_eq!(metadata.status, JobStatus::Enqueued);

    sqlx::query("UPDATE background_jobs SET retries = 3 WHERE id = ?")
        .bind(job)
        .execute(&mut *conn)
        .await
        .unwrap();

    BackgroundJob::requeue_or_fail(&mut conn, job, Some(1))
        .await
        .unwrap();

    let metadata = BackgroundJob::find_by_id(&mut conn, job).await.unwrap();
    assert_eq!(metadata.status, JobStatus::Failed);
}

#[tokio::test]
async fn test_requeue_or_fail_requeue_if_max_retries_is_not_present() {
    let pool = crate::testing::setup().await;
    let mut conn = pool.begin().await.unwrap();

    let job = enqueue_job()
        .conn(&mut conn)
        .kind("job1")
        .data(json!({}))
        .call()
        .await;

    BackgroundJob::requeue_or_fail(&mut conn, job, None)
        .await
        .unwrap();

    let job = BackgroundJob::find_by_id(&mut conn, job).await.unwrap();
    assert_eq!(job.status, JobStatus::Enqueued);
}

#[tokio::test]
async fn test_pull_next_pending_should_include_pending_jobs_if_timeout_is_passed() {
    let pool = crate::testing::setup().await;
    let mut conn = pool.begin().await.unwrap();

    let [task_one, task_two, task_three, task_four] = prepare_jobs_for_queueing(&mut conn).await;
    sqlx::query(
        "
        UPDATE background_jobs
        SET last_retry = ?,
            retries = retries + 1
        WHERE id = ? OR id = ?",
    )
    .bind(Timestamp::from_str("2024-01-01T00:00:00Z").unwrap())
    .bind(task_three)
    .bind(task_four)
    .execute(&mut *conn)
    .await
    .unwrap();

    let ts = Timestamp::from_str("2023-12-25T00:00:00Z").unwrap();
    assert_next_job(&mut conn, Some(ts), Some(task_two)).await;
    assert_next_job(&mut conn, Some(ts), Some(task_one)).await;
    assert_next_job(&mut conn, Some(ts), None).await;

    let ts = Timestamp::from_str("2024-01-01T00:05:00Z").unwrap();
    assert_next_job(&mut conn, Some(ts), Some(task_three)).await;
    assert_next_job(&mut conn, Some(ts), Some(task_four)).await;
    assert_next_job(&mut conn, Some(ts), None).await;
}

#[tokio::test]
async fn test_pull_next_pending_should_move_to_next_job() {
    let pool = crate::testing::setup().await;
    let mut conn = pool.begin().await.unwrap();

    let [task_one, task_two, task_three, task_four] = prepare_jobs_for_queueing(&mut conn).await;
    let expected_sequence = [
        Some(task_two),
        Some(task_one),
        Some(task_three),
        Some(task_four),
        None,
    ];

    for expected in expected_sequence {
        assert_next_job(&mut conn, None, expected).await;
    }
}

#[tokio::test]
async fn test_find_by_id() {
    let pool = crate::testing::setup().await;
    let mut conn = pool.begin().await.unwrap();

    let job_id = enqueue_job()
        .conn(&mut conn)
        .kind("task1")
        .data(json!({}))
        .call()
        .await;

    let metadata = BackgroundJob::find_by_id(&mut conn, job_id).await.unwrap();
    assert_eq!(metadata.id, job_id);
    assert_eq!(metadata.data, "{}");
    assert_eq!(metadata.last_retry, None);
    assert_eq!(metadata.retries, 0);
    assert_eq!(metadata.status, JobStatus::Enqueued);
}

#[tokio::test]
async fn test_enqueue() {
    let pool = crate::testing::setup().await;
    let mut conn = pool.begin().await.unwrap();

    let id = Uuid::from_str("8d7b519e-6b0e-40de-98f7-c85f7792f7fc").unwrap();
    let builder = BackgroundJob::new()
        .id(id)
        .kind("test")
        .priority(100)
        .data(json!({ "world": "hello" }))
        .expect("data should be serializable");

    let result = builder.build().enqueue(&mut conn).await.unwrap();
    assert_eq!(result, id);
}

#[tokio::test]
async fn test_uniquely_enqueue() {
    let pool = crate::testing::setup().await;
    let mut conn = pool.begin().await.unwrap();

    let id = Uuid::from_str("8d7b519e-6b0e-40de-98f7-c85f7792f7fc").unwrap();
    let current_job_id = enqueue_job()
        .conn(&mut conn)
        .id(id)
        .kind("test")
        .priority(100)
        .data(json!({ "hello": "world" }))
        .call()
        .await;

    assert_eq!(current_job_id, id);

    let builder = BackgroundJob::new()
        .kind("test")
        .priority(12)
        .data(json!({ "world": "hello" }))
        .expect("data should be serializable");

    let query = builder.build().enqueue_unique(&mut conn).await.unwrap();
    assert!(query.is_none());
}

#[bon::builder]
async fn enqueue_job(
    conn: &mut eden_sqlite::Connection,
    id: Option<Uuid>,
    created_at: Option<Timestamp>,
    kind: &'static str,
    priority: Option<i16>,
    data: serde_json::Value,
) -> Uuid {
    let created_at = created_at.unwrap_or_else(Timestamp::now);
    let builder = BackgroundJob::new()
        .maybe_id(id)
        .created_at(created_at)
        .kind(kind)
        .maybe_priority(priority)
        .data(data)
        .expect("data should be serializable");

    builder.build().enqueue(conn).await.unwrap()
}

async fn assert_next_job(
    conn: &mut eden_sqlite::Connection,
    ts: Option<Timestamp>,
    expected: Option<Uuid>,
) {
    let actual = BackgroundJob::pull_next_pending(conn, ts)
        .await
        .unwrap()
        .map(|v| v.id);

    assert_eq!(actual, expected, "next job is not ordered as expected");
}

async fn prepare_jobs_for_queueing(conn: &mut eden_sqlite::Connection) -> [Uuid; 4] {
    #[rustfmt::skip]
    let jobs = [
        ("2024-01-01T00:00:00Z", "a6b4fa28-40e7-4a07-b03d-2e3173016865", "job1", 10),
        ("2024-01-01T00:00:00Z", "03ced58f-7792-4ac1-b9bd-b0e97c906948", "job2", 100),
        ("2024-01-01T01:00:00Z", "43ca3d78-d3fd-4a30-95d3-d0c1d50f27f0", "job3", 10),
        ("2024-01-01T01:30:00Z", "c7fa0962-73b9-4c25-bbbd-b4bea4f14e3f", "job4", 1),
    ];

    let mut ids = [Uuid::nil(); 4];
    for (i, (created_at, id, kind, priority)) in jobs.into_iter().enumerate() {
        let id = BackgroundJob::new()
            .id(Uuid::from_str(id).unwrap())
            .created_at(Timestamp::from_str(created_at).unwrap())
            .kind(kind)
            .priority(priority)
            .data(json!({}))
            .unwrap()
            .build()
            .enqueue(conn)
            .await
            .unwrap();

        ids[i] = id;
    }

    ids
}
