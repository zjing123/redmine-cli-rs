use assert_cmd::Command;
use predicates::prelude::PredicateBooleanExt;
use predicates::str::contains;
use tempfile::tempdir;

#[test]
fn config_set_and_get_redacts_api_key() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("config.yaml");

    let mut set = Command::cargo_bin("redmine-cli").unwrap();
    set.env("REDMINE_CONFIG", &path)
        .args([
            "config",
            "set",
            "prod",
            "--url",
            "https://redmine.example.com/",
            "--api-key",
            "secret-key",
        ])
        .assert()
        .success()
        .stdout(contains("\"saved\":true"))
        .stdout(predicates::str::contains("secret-key").not());

    let mut get = Command::cargo_bin("redmine-cli").unwrap();
    get.env("REDMINE_CONFIG", &path)
        .args(["config", "get", "prod"])
        .assert()
        .success()
        .stdout(contains("api_key"))
        .stdout(predicates::str::contains("secret-key").not());
}

#[test]
fn config_remove_requires_confirmation() {
    let mut command = Command::cargo_bin("redmine-cli").unwrap();
    command
        .args(["config", "remove", "prod"])
        .assert()
        .failure()
        .stdout(contains("remove requires --yes"));
}
