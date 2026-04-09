use crate::macros::{IntoMetricType, metrics};
use prometheus::{Gauge, HistogramVec, IntCounter, IntCounterVec, IntGaugeVec};

metrics! {
    pub struct InstanceMetrics {
        "eden_discord_bot" => {
            /// Number of gateway events processed in all shards
            pub events_processed: IntCounter,

            /// Latency of a running shard (`[0, 1]`)
            pub shard_latency: Gauge,
        },
        "eden_gateway_api" => {
            /// Number of requests processed by this instance
            pub requests_total: IntCounter,

            /// Response times of each endpoints
            pub response_times: HistogramVec["endpoint", "method"],

            /// Total sessions granted by Gateway API per player type
            pub sessions_granted: IntCounterVec["type"],
        },
        "eden_kernel" => {
            /// Number of idle database connections in the poo
            pub database_idle_conns: IntGaugeVec["pool"],

            /// Number of used database connections in the pool
            pub database_used_conns: IntGaugeVec["pool"],

            /// Total time required to acquire a database connection
            pub database_time_to_acquire_connection: HistogramVec["pool"],
        },
    }
}
