use axum::http::{HeaderMap, HeaderName, HeaderValue};
use chrono::{TimeDelta, Utc};
use dashmap::DashMap;
use eden_database::Timestamp;
use error_stack::Report;
use std::{
    collections::HashMap,
    fmt,
    net::IpAddr,
    sync::Arc,
    time::{Duration, Instant},
};
use thiserror::Error;
use twilight_model::id::{Id, marker::UserMarker};

// TODO: Use alternatives to handle high volume of requests without using dash maps.
#[must_use]
pub struct RateLimiter {
    config: HashMap<LimitedAction, BucketConfig>,

    /// Global buckets per every possible limited action.
    global: DashMap<LimitedAction, Bucket>,

    /// Buckets assigned per action and actor identifier.
    actors: DashMap<(Actor, LimitedAction), Bucket>,
}

impl RateLimiter {
    pub fn new(config: HashMap<LimitedAction, BucketConfig>) -> Arc<Self> {
        Arc::new(Self {
            config,
            global: DashMap::new(),
            actors: DashMap::new(),
        })
    }

    pub fn clean_unused_actors(&self) {
        const GC_BUCKET_THRESHOLD: Duration = Duration::from_mins(15);

        self.global
            .retain(|_, bucket| bucket.start.elapsed() < GC_BUCKET_THRESHOLD);

        self.actors
            .retain(|_, bucket| bucket.start.elapsed() < GC_BUCKET_THRESHOLD);
    }

    #[tracing::instrument(skip_all, level = "debug", fields(?actor, ?action))]
    pub fn permit(
        &self,
        actor: Actor,
        action: LimitedAction,
    ) -> Result<BucketStats, Report<TooManyRequests>> {
        let config = match self.config.get(&action) {
            Some(config) => *config,
            None => action.default_bucket_config(),
        };

        // Lock order: Global -> Specific Actor
        let mut global = self.global.entry(action).or_default();
        let mut bucket = self.actors.entry((actor, action)).or_default();

        tracing::trace!("checking for global permit");
        if !global.check_permit(&config, true) {
            return Err(Report::new(TooManyRequests {
                action,
                stats: bucket.stats(&config, false),
            }));
        }

        tracing::trace!("checking for specific permit");
        if !bucket.check_permit(&config, false) {
            return Err(Report::new(TooManyRequests {
                action,
                stats: bucket.stats(&config, false),
            }));
        }

        global.count = global.count.saturating_add(1);
        bucket.count = bucket.count.saturating_add(1);

        tracing::trace!(?global.count, ?bucket.count, "action permitted");
        Ok(bucket.stats(&config, false))
    }
}

impl fmt::Debug for RateLimiter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RateLimiter")
            .field("config", &self.config)
            .field("actors", &self.actors.len())
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BucketStats {
    pub limit: u64,
    pub remaining: u64,
    pub reset_after: Timestamp,
}

const X_RATELIMIT_LIMIT: HeaderName = HeaderName::from_static("x-ratelimit-limit");
const X_RATELIMIT_REMAINING: HeaderName = HeaderName::from_static("x-ratelimit-remaining");
const X_RATELIMIT_RESET_AFTER: HeaderName = HeaderName::from_static("x-ratelimit-reset-after");

impl BucketStats {
    #[must_use]
    pub fn into_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            X_RATELIMIT_LIMIT,
            HeaderValue::from_str(&self.limit.to_string())
                .expect("should emit valid header value compatible payload"),
        );

        headers.insert(
            X_RATELIMIT_REMAINING,
            HeaderValue::from_str(&self.remaining.to_string())
                .expect("should emit valid header value compatible payload"),
        );

        headers.insert(
            X_RATELIMIT_RESET_AFTER,
            HeaderValue::from_str(&self.reset_after.to_string())
                .expect("should emit valid header value compatible payload"),
        );

        headers
    }
}

#[derive(Clone, Copy, Debug)]
pub struct BucketConfig {
    /// Maximum amount of requests accepted for a particular
    /// route regardless of the actor.
    pub max_global_requests: u64,

    /// Maximum amount of requests for an actor.
    pub max_requests: u64,

    /// Reset interval
    pub reset_interval: Duration,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum Actor {
    /// This request was performed from an IP address,
    /// most likely from a guest.
    Ip(IpAddr),

    /// This request was performed by a member registered
    /// in the primary guild.
    Member(Id<UserMarker>),

    /// This request was performed by a Minecraft server.
    McServer,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum LimitedAction {
    RequestSession { guest: bool },
    ValidateSessions,
    LinkMinecraftAccount,
    // This needs to fetch member data from Discord API
    RegisterMember,
}

impl LimitedAction {
    #[must_use]
    pub const fn message(&self) -> &'static str {
        match self {
            Self::RequestSession { .. } => "You have joined the server too quickly!",
            Self::ValidateSessions => "You requested validation of players' sessions too quickly!",
            Self::LinkMinecraftAccount => {
                "You have requested to link your Minecraft account too quickly! Please try again later."
            }
            Self::RegisterMember => "You have registered members too quickly!",
        }
    }

    #[must_use]
    const fn default_bucket_config(&self) -> BucketConfig {
        match self {
            // Guests however have limited capacity to prevent spam
            Self::RequestSession { guest: true } => BucketConfig {
                max_global_requests: 20,
                max_requests: 5,
                reset_interval: Duration::from_secs(60),
            },

            // Members will allow to request session up to 80 times a minute
            Self::RequestSession { guest: false } => BucketConfig {
                max_global_requests: 80,
                max_requests: 10,
                reset_interval: Duration::from_secs(60),
            },

            // Validate session route is read heavy, we need to regulate
            // the use of it to prevent DDoS.
            Self::ValidateSessions => BucketConfig {
                max_global_requests: 3,
                max_requests: 3,
                reset_interval: Duration::from_mins(1),
            },

            Self::LinkMinecraftAccount => BucketConfig {
                max_global_requests: 15,
                max_requests: 3,
                reset_interval: Duration::from_mins(1),
            },

            Self::RegisterMember => BucketConfig {
                max_global_requests: 10,
                max_requests: 5,
                reset_interval: Duration::from_mins(1),
            },
        }
    }
}

#[derive(Debug, Error)]
#[error("Rate limit quota got exhausted")]
pub struct TooManyRequests {
    action: LimitedAction,
    stats: BucketStats,
}

impl TooManyRequests {
    #[must_use]
    pub const fn action(&self) -> &LimitedAction {
        &self.action
    }

    #[must_use]
    pub const fn stats(&self) -> &BucketStats {
        &self.stats
    }
}

#[must_use = "buckets do not do anything unless their functions are called"]
struct Bucket {
    count: u64,
    start: Instant,
}

impl Bucket {
    fn new() -> Self {
        Self {
            count: 0,
            start: Instant::now(),
        }
    }

    fn stats(&self, config: &BucketConfig, is_global: bool) -> BucketStats {
        // Find the remaining duration until reset
        let remaining = config.reset_interval.saturating_sub(self.start.elapsed());
        let delta = TimeDelta::from_std(remaining).unwrap_or_else(|_| TimeDelta::minutes(1));

        let now = Utc::now();
        let reset_after = now.checked_add_signed(delta).unwrap_or(now);

        BucketStats {
            limit: if is_global {
                config.max_global_requests
            } else {
                config.max_requests
            },
            remaining: config.max_requests.saturating_sub(self.count),
            reset_after: reset_after.into(),
        }
    }

    fn check_permit(&mut self, config: &BucketConfig, is_global: bool) -> bool {
        if self.start.elapsed() >= config.reset_interval {
            self.count = 0;
            self.start = Instant::now();
        }

        let max = if is_global {
            config.max_global_requests
        } else {
            config.max_requests
        };

        self.count < max
    }
}

impl Default for Bucket {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use claims::assert_err;
    use pretty_assertions::assert_eq;
    use std::{
        collections::HashMap,
        net::{IpAddr, Ipv4Addr},
        time::Duration,
    };
    use twilight_model::id::Id;

    use crate::ratelimiter::{Actor, BucketConfig, LimitedAction, RateLimiter};

    #[test]
    fn test_rate_limiting_in_different_actors() {
        let mut config = HashMap::new();
        config.insert(
            LimitedAction::RequestSession { guest: false },
            BucketConfig {
                max_global_requests: u64::MAX,
                max_requests: 2,
                reset_interval: Duration::from_millis(100),
            },
        );

        let limiter = RateLimiter::new(config);
        let stats = limiter
            .permit(
                Actor::McServer,
                LimitedAction::RequestSession { guest: false },
            )
            .unwrap();

        assert_eq!(stats.limit, 2);
        assert_eq!(stats.remaining, 1);

        let stats = limiter
            .permit(
                Actor::McServer,
                LimitedAction::RequestSession { guest: false },
            )
            .unwrap();

        assert_eq!(stats.remaining, 0);

        _ = assert_err!(limiter.permit(
            Actor::McServer,
            LimitedAction::RequestSession { guest: false },
        ));

        let stats = limiter
            .permit(
                Actor::Member(Id::new(12345)),
                LimitedAction::RequestSession { guest: false },
            )
            .unwrap();

        assert_eq!(stats.remaining, 1);

        let stats = limiter
            .permit(
                Actor::Member(Id::new(12345)),
                LimitedAction::RequestSession { guest: false },
            )
            .unwrap();

        assert_eq!(stats.remaining, 0);
    }

    #[test]
    fn test_rate_limiting_in_global() {
        let mut config = HashMap::new();
        config.insert(
            LimitedAction::RequestSession { guest: false },
            BucketConfig {
                max_global_requests: 1,
                max_requests: 100,
                reset_interval: Duration::from_millis(100),
            },
        );

        let limiter = RateLimiter::new(config);
        limiter
            .permit(
                Actor::McServer,
                LimitedAction::RequestSession { guest: false },
            )
            .unwrap();

        // It should throw an error regardless of the actors
        _ = assert_err!(limiter.permit(
            Actor::McServer,
            LimitedAction::RequestSession { guest: false },
        ));
        _ = assert_err!(limiter.permit(
            Actor::Ip(IpAddr::V4(Ipv4Addr::LOCALHOST)),
            LimitedAction::RequestSession { guest: false },
        ));
        _ = assert_err!(limiter.permit(
            Actor::Member(Id::new(12345)),
            LimitedAction::RequestSession { guest: false },
        ));
    }
}
