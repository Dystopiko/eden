// TODO: Write test cases for configuration
pub mod editable;
pub mod error;
pub mod sections;

pub use self::editable::EditableConfig;
pub use self::root::Config;

mod root;
mod validate;
