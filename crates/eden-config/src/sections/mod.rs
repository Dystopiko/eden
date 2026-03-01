// Each module corresponds a field from the root configuration (crate::root::Config)
//
// To add a new section/field:
//
// 1. Create `src/sections/<name>.rs` implementing [`validate::Validate`].
// 2. Add a `pub <name>: <Type>` field here.
// 3. Register the module in `src/sections/mod.rs`.
// 4. Add a `self.<name>.validate(ctx)?;` call in [`Config::validate`] below.

pub mod bot;
pub use self::bot::{Bot, Token};

pub mod database;
pub use self::database::{Database, DatabasePool, SqliteUrl};
