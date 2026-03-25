use std::sync::Arc;

use eden_config::sections::Sentry as SentryConfig;
use http::header::{AUTHORIZATION, COOKIE};
use sentry::{ClientInitGuard, ClientOptions, protocol::Event};

pub mod report;
pub use self::report::capture_report;

#[must_use]
pub fn init(config: Option<&SentryConfig>) -> Option<ClientInitGuard> {
    let config = config?;
    let before_send = |mut event: Event<'static>| {
        if let Some(request) = &mut event.request {
            // Remove any sensitive parts of the request so it never gets sent to Sentry.
            request.headers.retain(is_not_sensitive);
        }
        Some(event)
    };

    Some(sentry::init(ClientOptions {
        dsn: Some(config.dsn.clone().take()),
        environment: Some(config.environment.clone().into()),
        release: sentry::release_name!(),
        traces_sample_rate: 1.0,
        before_send: Some(Arc::new(before_send)),
        ..Default::default()
    }))
}

#[allow(clippy::ptr_arg)]
#[must_use]
fn is_not_sensitive(name: &String, _value: &mut String) -> bool {
    name.as_str() != AUTHORIZATION && name.as_str() != COOKIE
}
