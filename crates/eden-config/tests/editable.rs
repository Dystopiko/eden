use eden_config::{Config, EditableConfig};

use crate::common::TempConfig;

#[path = "common.rs"]
mod common;

#[test]
fn test_defaults() {
    insta::assert_debug_snapshot!(Config::no_clone_default());
}

#[test]
fn should_provide_defaults_on_create() {
    eden_common::testing::init();

    let tmpdir = tempfile::tempdir().unwrap();
    let path = tmpdir.path().join("eden.toml");
    assert!(!path.exists());

    let config = EditableConfig::new(tmpdir.path().join("eden.toml"));
    config.save().unwrap();

    assert!(path.exists());

    let outcome = eden_common::path::read(&path).unwrap();
    assert_eq!(outcome, include_str!("../eden.default.toml"));
}

#[test]
fn should_edit_and_save() {
    eden_common::testing::init();

    let tmpdir = tempfile::tempdir().unwrap();
    let contents = r#"
    [bot]
    application_id = "1"
    token = "abc"

    [bot.primary_guild]
    id = "1"
    "#;

    let mut config = TempConfig::builder()
        .contents(contents)
        .load_on_build(true)
        .path(&tmpdir.path().join("eden.toml"))
        .build();

    config
        .edit(|_, toml| {
            toml.insert("food", toml_edit::value("sandwich"));
        })
        .unwrap();

    let output = eden_common::path::read(config.path()).unwrap();
    insta::assert_snapshot!(output);
}

#[test]
fn should_reload() {
    eden_common::testing::init();

    let tmpdir = tempfile::tempdir().unwrap();
    let contents = r#"
    [bot]
    application_id = "1"
    token = "abc"

    [bot.primary_guild]
    id = "1"
    "#;

    let mut config = TempConfig::builder()
        .contents(contents)
        .load_on_build(true)
        .path(&tmpdir.path().join("eden.toml"))
        .build();

    assert_eq!(config.bot.token.as_str(), "abc");

    let new_contents = r#"
        [bot]
        application_id = "1"
        token = "def"

        [bot.primary_guild]
        id = "1"
    "#;
    eden_common::path::write(config.path(), new_contents).unwrap();

    config.reload().unwrap();
    assert_eq!(config.bot.token.as_str(), "def");
}
