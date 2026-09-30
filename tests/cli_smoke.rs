use assert_cmd::Command;
use predicates::str::contains;

#[test]
fn fields_does_not_require_config() {
    let mut command = Command::cargo_bin("redmine-cli").unwrap();
    command
        .args(["issue", "fields"])
        .assert()
        .success()
        .stdout(contains("scalar"));
}

#[test]
fn schema_does_not_require_config() {
    let mut command = Command::cargo_bin("redmine-cli").unwrap();
    command
        .args(["issue", "schema"])
        .assert()
        .success()
        .stdout(contains("operations"));
}

#[test]
fn search_command_is_available() {
    let mut command = Command::cargo_bin("redmine-cli").unwrap();
    command
        .args(["search", "--help"])
        .assert()
        .success()
        .stdout(contains("<QUERY>"));
}
