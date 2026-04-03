use error_stack::{Report, ResultExt};
use prometheus::{
    HistogramOpts, HistogramVec, IntCounter, IntCounterVec, IntGaugeVec, Opts, Registry,
    TextEncoder,
};

use crate::EncodeMetricsError;

#[derive(Debug, Clone)]
pub struct InstanceMetrics {
    registry: Registry,

    pub database_idle_conns: IntGaugeVec,
    pub database_used_conns: IntGaugeVec,
    pub database_time_to_acquire_connection: HistogramVec,
    pub events_processed: IntCounter,
    pub requests_total: IntCounter,
    pub response_times: HistogramVec,
    pub shard_latencies: HistogramVec,
    pub sessions_granted: IntCounterVec,
}

impl InstanceMetrics {
    pub fn new() -> Result<Self, prometheus::Error> {
        let registry = Registry::new();

        let database_idle_conns = IntGaugeVec::new(
            Opts::new(
                "database_idle_conns",
                "Number of idle database connections in the pool",
            )
            .namespace("eden_kernel"),
            &["pool"],
        )?;

        let database_used_conns = IntGaugeVec::new(
            Opts::new(
                "database_used_conns",
                "Number of used database connections in the pool",
            )
            .namespace("eden_kernel"),
            &["pool"],
        )?;

        let database_time_to_acquire_connection = HistogramVec::new(
            HistogramOpts::new(
                "database_time_to_acquire_connection",
                "Total time required to acquire a database connection",
            )
            .namespace("eden_kernel"),
            &["pool"],
        )?;

        let events_processed = IntCounter::with_opts(
            Opts::new(
                "events_processed",
                "Number of gateway events processed in all shards",
            )
            .namespace("eden_discord_bot"),
        )?;

        let requests_total = IntCounter::with_opts(
            Opts::new(
                "requests_total",
                "Number of requests processed by this instance",
            )
            .namespace("eden_gateway_api"),
        )?;

        let response_times = HistogramVec::new(
            HistogramOpts::new("response_times", "Response times of each endpoints")
                .namespace("eden_gateway_api")
                .buckets(HISTOGRAM_BUCKETS.to_vec()),
            &["endpoint"],
        )?;

        let shard_latencies = HistogramVec::new(
            HistogramOpts::new("shard_latencies", "Latencies per shard")
                .namespace("eden_discord_bot")
                .buckets(HISTOGRAM_BUCKETS.to_vec()),
            &["id"],
        )?;

        let sessions_granted = IntCounterVec::new(
            Opts::new(
                "sessions_granted",
                "Total sessions granted by Gateway API per player type",
            )
            .namespace("eden_gateway_api"),
            &["type"],
        )?;

        registry.register(Box::new(database_idle_conns.clone()))?;
        registry.register(Box::new(database_used_conns.clone()))?;
        registry.register(Box::new(database_time_to_acquire_connection.clone()))?;
        registry.register(Box::new(events_processed.clone()))?;
        registry.register(Box::new(requests_total.clone()))?;
        registry.register(Box::new(response_times.clone()))?;
        registry.register(Box::new(shard_latencies.clone()))?;
        registry.register(Box::new(sessions_granted.clone()))?;

        Ok(Self {
            registry,

            database_idle_conns,
            database_used_conns,
            database_time_to_acquire_connection,
            events_processed,
            requests_total,
            response_times,
            shard_latencies,
            sessions_granted,
        })
    }

    pub fn encode(&self) -> Result<String, Report<EncodeMetricsError>> {
        let encoder = TextEncoder::new();
        let families = self.registry.gather();
        encoder
            .encode_to_string(&families)
            .change_context(EncodeMetricsError)
    }
}

// Copied from: https://github.com/rust-lang/crates.io/blob/d379aecd2a3a24295a614a1ac8684f2f5963dda2/src/metrics/macros.rs#L14-L16
const HISTOGRAM_BUCKETS: &[f64] = &[
    0.0005, 0.001, 0.0025, 0.005, 0.01, 0.025, 0.05, 0.1, 0.5, 1.0, 5.0,
];
