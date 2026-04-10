// Each module corresponds a field from the root configuration (crate::root::Config)
//
// To add a new section/field:
//
// 1. Create `src/sections/<name>.rs` implementing [`validate::Validate`].
// 2. Add a `pub <name>: <Type>` field here.
// 3. Register the module in `src/sections/mod.rs`.
// 4. Add a `self.<name>.validate(ctx)?;` call in [`Config::validate`] below.

pub mod background_jobs;
pub mod bot;
pub mod database;
pub mod gateway;
pub mod minecraft;
pub mod prometheus;
pub mod sentry;
pub mod setup;

pub use self::background_jobs::BackgroundJobs;
pub use self::bot::Bot;
pub use self::database::{Database, DatabasePool, SqliteUrl};
pub use self::gateway::Gateway;
pub use self::minecraft::Minecraft;
pub use self::prometheus::Prometheus;
pub use self::sentry::Sentry;
pub use self::setup::{InitialSettings, Setup};
