use std::{fs, io::Read, net::TcpListener, thread};

use assert_cmd::Command;
use predicates::prelude::PredicateBooleanExt;
use predicates::str::contains;
use redmine_cli::config::Config;
use tempfile::tempdir;

fn mock_redmine(status: &str, body: &str) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let status = status.to_owned();
    let body = body.to_owned();
    let thread = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = [0; 4096];
        let _ = stream.read(&mut request).unwrap();
        use std::io::Write;
        write!(
            stream,
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
        .unwrap();
    });
    (format!("http://{address}/"), thread)
}

#[test]
fn config_path_remains_available() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("config.yaml");

    Command::cargo_bin("redmine-cli")
        .unwrap()
        .env("REDMINE_CONFIG", &path)
        .args(["config", "path"])
        .assert()
        .success()
        .stdout(contains(path.display().to_string()));
}

#[test]
fn auth_login_help_documents_non_interactive_flags() {
    Command::cargo_bin("redmine-cli")
        .unwrap()
        .args(["auth", "login", "--help"])
        .assert()
        .success()
        .stdout(contains("--name <NAME>"))
        .stdout(contains("-k, --api-key <API_KEY>"))
        .stdout(contains("-s, --server <SERVER>"));
}

#[test]
fn auth_list_redacts_credentials() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("config.yaml");
    fs::write(
        &path,
        "active_profile: work\nprofiles:\n  work:\n    url: https://redmine.example.com/\n    api_key: secret-key\n",
    )
    .unwrap();

    Command::cargo_bin("redmine-cli")
        .unwrap()
        .env("REDMINE_CONFIG", &path)
        .args(["auth", "list"])
        .assert()
        .success()
        .stdout(contains("redmine.example.com"))
        .stdout(contains("secret-key").not())
        .stdout(contains("work"));
}

#[test]
fn auth_login_validates_then_creates_and_updates_profile() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("config.yaml");

    let (server, request) = mock_redmine("200 OK", r#"{"user":{"id":1,"login":"tester"}}"#);
    Command::cargo_bin("redmine-cli")
        .unwrap()
        .env("REDMINE_CONFIG", &path)
        .args([
            "auth",
            "login",
            "--name",
            "work",
            "-k",
            "first-key",
            "-s",
            &server,
        ])
        .assert()
        .success()
        .stdout(contains("work"))
        .stdout(contains("first-key").not());
    request.join().unwrap();

    let mut config = Config::load(Some(&path)).unwrap();
    assert_eq!(config.active_profile(), Some("work"));
    assert_eq!(
        config.profile(Some("work")).unwrap().1.api_key.as_deref(),
        Some("first-key")
    );
    config.profile_mut("work").unwrap().read_only = Some(true);
    config.save().unwrap();

    let (server, request) = mock_redmine("200 OK", r#"{"user":{"id":1,"login":"tester"}}"#);
    Command::cargo_bin("redmine-cli")
        .unwrap()
        .env("REDMINE_CONFIG", &path)
        .args([
            "auth",
            "login",
            "--name",
            "work",
            "-k",
            "updated-key",
            "-s",
            &server,
        ])
        .assert()
        .success();
    request.join().unwrap();

    config = Config::load(Some(&path)).unwrap();
    assert_eq!(
        config.profile(Some("work")).unwrap().1.api_key.as_deref(),
        Some("updated-key")
    );
    assert_eq!(
        config.profile(Some("work")).unwrap().1.read_only,
        Some(true)
    );
}

#[test]
fn auth_login_derives_profile_name_from_server_hostname() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("config.yaml");
    let (server, request) = mock_redmine("200 OK", r#"{"user":{"id":1,"login":"tester"}}"#);

    Command::cargo_bin("redmine-cli")
        .unwrap()
        .env("REDMINE_CONFIG", &path)
        .args(["auth", "login", "-k", "secret-key", "-s", &server])
        .assert()
        .success()
        .stdout(contains("127-0-0-1"));
    request.join().unwrap();

    let config = Config::load(Some(&path)).unwrap();
    assert!(config.profile(Some("127-0-0-1")).is_ok());
}

#[test]
fn auth_status_redacts_api_key_from_user_response() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("config.yaml");
    let (server, request) = mock_redmine(
        "200 OK",
        r#"{"user":{"id":1,"login":"tester","api_key":"response-secret"}}"#,
    );
    fs::write(
        &path,
        format!(
            "active_profile: work\nprofiles:\n  work:\n    url: {server}\n    api_key: config-secret\n"
        ),
    )
    .unwrap();

    Command::cargo_bin("redmine-cli")
        .unwrap()
        .env("REDMINE_CONFIG", &path)
        .args(["auth", "status"])
        .assert()
        .success()
        .stdout(contains("tester"))
        .stdout(contains("response-secret").not())
        .stdout(contains("config-secret").not());
    request.join().unwrap();
}

#[test]
fn auth_login_does_not_save_when_connection_validation_fails() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("config.yaml");
    let (server, request) = mock_redmine("401 Unauthorized", r#"{"errors":["Invalid API key"]}"#);

    Command::cargo_bin("redmine-cli")
        .unwrap()
        .env("REDMINE_CONFIG", &path)
        .args([
            "auth", "login", "--name", "work", "-k", "bad-key", "-s", &server,
        ])
        .assert()
        .failure();
    request.join().unwrap();

    assert!(!path.exists());
}

#[test]
fn failed_auth_login_does_not_replace_existing_profile() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("config.yaml");
    fs::write(
        &path,
        "active_profile: work\nprofiles:\n  work:\n    url: https://old.example.com/\n    api_key: old-key\n    read_only: true\n",
    )
    .unwrap();
    let (server, request) = mock_redmine("401 Unauthorized", r#"{"errors":["Invalid API key"]}"#);

    Command::cargo_bin("redmine-cli")
        .unwrap()
        .env("REDMINE_CONFIG", &path)
        .args([
            "auth", "login", "--name", "work", "-k", "bad-key", "-s", &server,
        ])
        .assert()
        .failure();
    request.join().unwrap();

    let config = Config::load(Some(&path)).unwrap();
    assert_eq!(
        config.profile(Some("work")).unwrap().1.api_key.as_deref(),
        Some("old-key")
    );
    assert_eq!(
        config.profile(Some("work")).unwrap().1.read_only,
        Some(true)
    );
}

#[test]
fn auth_switch_persists_active_profile() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("config.yaml");
    fs::write(
        &path,
        "profiles:\n  work:\n    url: https://work.example.com/\n    api_key: secret\n  personal:\n    url: https://personal.example.com/\n    api_key: secret\n",
    )
    .unwrap();

    Command::cargo_bin("redmine-cli")
        .unwrap()
        .env("REDMINE_CONFIG", &path)
        .args(["auth", "switch", "personal"])
        .assert()
        .success();

    let config = Config::load(Some(&path)).unwrap();
    assert_eq!(config.active_profile(), Some("personal"));
}

#[test]
fn auth_logout_requires_interactive_confirmation() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("config.yaml");
    fs::write(
        &path,
        "active_profile: work\nprofiles:\n  work:\n    url: https://work.example.com/\n    api_key: secret\n",
    )
    .unwrap();

    Command::cargo_bin("redmine-cli")
        .unwrap()
        .env("REDMINE_CONFIG", &path)
        .args(["auth", "logout"])
        .assert()
        .failure()
        .stdout(contains("requires confirmation"));

    assert!(
        Config::load(Some(&path))
            .unwrap()
            .profile(Some("work"))
            .is_ok()
    );
}

#[test]
fn legacy_profile_management_config_commands_are_removed() {
    Command::cargo_bin("redmine-cli")
        .unwrap()
        .args(["config", "set", "work"])
        .assert()
        .failure()
        .stderr(contains("unrecognized subcommand"));
}
