use assert_cmd::Command;
use predicates::str::contains;

#[test]
fn fields_does_not_require_config() {
    let mut command = Command::cargo_bin("redmine-cli-rs").unwrap();
    command
        .args(["issue", "fields"])
        .assert()
        .success()
        .stdout(contains("scalar"));
}

#[test]
fn schema_does_not_require_config() {
    let mut command = Command::cargo_bin("redmine-cli-rs").unwrap();
    command
        .args(["issue", "schema"])
        .assert()
        .success()
        .stdout(contains("operations"));
}

#[test]
fn search_is_explicitly_unimplemented() {
    let mut command = Command::cargo_bin("redmine-cli-rs").unwrap();
    command
        .args(["search", "keyword"])
        .assert()
        .failure()
        .stdout(contains("unsupported command"));
}
