use eden_toml::TomlDiagnostic;
use error_stack::Report;
use serde::Deserialize;

pub mod pool;
pub use self::pool::DatabasePool;

pub mod url;
pub use self::url::SqliteUrl;

use crate::validate::{Validate, ValidationContext};

/// Configuration for the application's SQLite database connections.
///
/// Supports a mandatory primary connection and an optional read replica,
/// allowing read-heavy workloads to be offloaded from the primary.
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize)]
pub struct Database {
    #[serde(default)]
    pub primary: DatabasePool,
    pub replica: Option<DatabasePool>,
}

impl Validate for Database {
    fn validate(&self, ctx: &ValidationContext<'_>) -> Result<(), Report<TomlDiagnostic>> {
        if let Some(replica) = self.replica.as_ref() {
            let replica_table = ctx.document.get("database").and_then(|v| v.get("replica"));
            let readonly_pair = replica_table
                .and_then(|v| v.as_table_like())
                .and_then(|v| v.get_key_value("readonly"));

            if !replica.readonly && readonly_pair.is_some() {
                let pair_span = readonly_pair
                    .and_then(|(key, value)| key.span().zip(value.span()))
                    .map(|(a, b)| a.start..b.end)
                    .or_else(|| replica_table.and_then(|v| v.span()));

                let diagnostic = eden_toml::diagnostic(
                    "Replica databases must not be writable. Set readonly to `false`",
                    pair_span,
                    ctx.source,
                    ctx.path,
                );

                return Err(diagnostic);
            }
        }

        Ok(())
    }
}
