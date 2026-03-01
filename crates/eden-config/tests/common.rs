// Snapshots of error messages are avoided intentionally: error-stack includes
// the caller source location in its output, and path separators differ between
// Windows and Unix, making snapshots fragile across platforms and machines.
use bon::Builder;
use eden_config::EditableConfig;
use std::path::Path;
use unindent::unindent;

#[derive(Builder)]
#[builder(finish_fn(vis = "", name = build_internal))]
pub struct TempConfig<'a> {
    pub contents: &'static str,
    pub load_on_build: bool,
    pub path: &'a Path,
}

impl<'a, S: temp_config_builder::IsComplete> TempConfigBuilder<'a, S> {
    #[track_caller]
    pub fn build(self) -> EditableConfig {
        let params = self.build_internal();
        eden_common::path::write(params.path, unindent(params.contents)).unwrap();

        let mut config = EditableConfig::new(params.path);
        if params.load_on_build {
            config.reload().unwrap();
        }

        config
    }
}
