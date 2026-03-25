use eden_toml::TomlDiagnostic;
use eden_utils::sensitive::Sensitive;
use error_stack::Report;
use sentry_core::types::Dsn;
use serde::Deserialize;

use crate::validate::{Validate, ValidationContext};

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Sentry {
    pub dsn: Sensitive<Dsn>,

    #[serde(alias = "env")]
    pub environment: String,

    #[serde(default = "default_targets")]
    pub targets: String,

    #[serde(default = "default_traces_sample_rate")]
    pub traces_sample_rate: f32,
}

impl Validate for Sentry {
    fn validate(&self, ctx: &ValidationContext<'_>) -> Result<(), Report<TomlDiagnostic>> {
        if self.environment.is_empty() {
            let env_span = ctx
                .document
                .get("sentry")
                .and_then(|v| v.get("environment"))
                .and_then(|v| v.span());

            let diagnostic = eden_toml::diagnostic(
                "Sentry environment must not be empty",
                env_span,
                ctx.source,
                ctx.path,
            );

            return Err(diagnostic);
        }

        if self.traces_sample_rate < 0.0 || self.traces_sample_rate > 1.0 {
            let rate_span = ctx
                .document
                .get("sentry")
                .and_then(|v| v.get("traces_sample_rate"))
                .and_then(|v| v.span());

            let diagnostic = eden_toml::diagnostic(
                "traces_sample_rate must be within range of 0 to 1",
                rate_span,
                ctx.source,
                ctx.path,
            );

            return Err(diagnostic);
        }

        Ok(())
    }
}

fn default_targets() -> String {
    String::from("info")
}

const fn default_traces_sample_rate() -> f32 {
    1.
}
