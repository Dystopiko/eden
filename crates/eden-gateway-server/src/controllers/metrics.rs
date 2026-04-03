use eden_core::InstanceMetrics;

use crate::{
    controllers::ApiResult,
    errors::{ApiError, ErrorCode},
    extract::Kernel,
};

pub async fn prometheus(Kernel(kernel): Kernel) -> ApiResult<String> {
    let Some(metrics) = kernel.metrics.as_ref() else {
        return Err(ApiError::from_static(
            ErrorCode::NotFound,
            "Metrics are disabled on this instance",
        ));
    };

    refresh_pool_stats("primary", kernel.pools.primary_db(), metrics);
    if let Some(replica) = kernel.pools.replica_db() {
        refresh_pool_stats("replica", replica, metrics);
    }

    Ok(metrics.encode()?)
}

fn refresh_pool_stats(key: &str, pool: &eden_sqlite::Pool, metrics: &InstanceMetrics) {
    let idle = i64::try_from(pool.idle_connections()).unwrap_or(0);
    metrics
        .database_idle_conns
        .get_metric_with_label_values(&[key])
        .expect("should only require one label")
        .set(idle);

    let used = pool.connections() as i64;
    metrics
        .database_used_conns
        .get_metric_with_label_values(&[key])
        .expect("should only require one label")
        .set(used.saturating_sub(idle));
}
