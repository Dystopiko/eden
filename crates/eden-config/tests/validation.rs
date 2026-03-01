use eden_common::testing::expect_error_containing;

use crate::common::TempConfig;

#[path = "common.rs"]
mod common;

#[test]
fn accepts_database_replica_if_readonly_not_explicitly_set() {
    eden_common::testing::init();

    let tmpdir = tempfile::tempdir().unwrap();
    let contents = r#"
        bot.application_id = "1"
        bot.token = "a"
        bot.primary_guild.id = "1"

        [database.replica]
        url = "sqlite://somewhere.db"
    "#;

    let mut config = TempConfig::builder()
        .contents(contents)
        .load_on_build(false)
        .path(&tmpdir.path().join("eden.toml"))
        .build();

    config
        .reload()
        .expect("should accept if readonly is not set explicitly");
}

#[test]
fn rejects_database_replica_set_readonly_to_false() {
    eden_common::testing::init();

    let tmpdir = tempfile::tempdir().unwrap();
    let contents = r#"
        bot.application_id = "1"
        bot.token = "a"
        bot.primary_guild.id = "1"

        [database.replica]
        url = "sqlite://somewhere.db"
        readonly = false
    "#;

    let mut config = TempConfig::builder()
        .contents(contents)
        .load_on_build(false)
        .path(&tmpdir.path().join("eden.toml"))
        .build();

    let error = config.reload().unwrap_err();
    expect_error_containing(
        error,
        "Replica databases must not be writable. Set readonly to `false`",
    );
}

#[test]
fn rejects_empty_bot_token() {
    eden_common::testing::init();

    let tmpdir = tempfile::tempdir().unwrap();
    let contents = r#"
        [bot]
        application_id = "1"
        token = ""

        [bot.primary_guild]
        id = "1"
    "#;

    let mut config = TempConfig::builder()
        .contents(contents)
        .load_on_build(false)
        .path(&tmpdir.path().join("eden.toml"))
        .build();

    let error = config.reload().unwrap_err();
    expect_error_containing(error, "Got invalid Discord token");
}

#[test]
fn rejects_bot_token_with_control_characters() {
    eden_common::testing::init();

    let tmpdir = tempfile::tempdir().unwrap();
    let contents = r#"
        [bot]
        application_id = "1"
        token = "\t\n"

        [bot.primary_guild]
        id = "1"
    "#;

    let mut config = TempConfig::builder()
        .contents(contents)
        .load_on_build(false)
        .path(&tmpdir.path().join("eden.toml"))
        .build();

    let error = config.reload().unwrap_err();
    expect_error_containing(error, "Got invalid Discord token");
}

#[test]
fn accepts_valid_bot_token() {
    eden_common::testing::init();

    let tmpdir = tempfile::tempdir().unwrap();
    let contents = r#"
        [bot]
        application_id = "1"
        token = "13.ab.a"

        [bot.primary_guild]
        id = "1"
    "#;

    let mut config = TempConfig::builder()
        .contents(contents)
        .load_on_build(false)
        .path(&tmpdir.path().join("eden.toml"))
        .build();

    config.reload().expect("should accept a valid bot token");
}
