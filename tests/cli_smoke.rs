use assert_cmd::Command;
use predicates::prelude::PredicateBooleanExt;
use predicates::str::contains;
use std::fs;
use tempfile::tempdir;

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

#[test]
fn renamed_resource_commands_are_available() {
    for name in ["issues", "projects", "users", "attachments", "time", "wiki"] {
        Command::cargo_bin("redmine-cli")
            .unwrap()
            .args([name, "--help"])
            .assert()
            .success();
    }
}

#[test]
fn version_identifies_rust_implementation() {
    Command::cargo_bin("redmine-cli")
        .unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(contains("(Rust)"));
}

#[test]
fn issue_get_help_lists_include_shortcuts_and_download_option() {
    Command::cargo_bin("redmine-cli")
        .unwrap()
        .args(["issue", "get", "--help"])
        .assert()
        .success()
        .stdout(contains("--journals"))
        .stdout(contains("--children"))
        .stdout(contains("--relations"))
        .stdout(contains("--attachments"))
        .stdout(contains("--download-attachments"))
        .stdout(contains("Examples:"))
        .stdout(contains(
            "# Numeric issue IDs use -p/--profile, REDMINE_PROFILE, or active_profile",
        ))
        .stdout(contains(
            "# A full issue URL can automatically select a matching configured profile",
        ))
        .stdout(contains("--fields"))
        .stdout(contains("--include"))
        .stdout(contains("REDMINE_PROFILE"))
        .stdout(contains(
            "automatically select a matching configured profile",
        ));
}

#[test]
fn issue_list_help_documents_filters_and_short_projection() {
    let output = Command::cargo_bin("redmine-cli")
        .unwrap()
        .args(["issue", "list", "--help"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let help = String::from_utf8(output.stdout).unwrap();
    assert!(help.contains("--filter <KEY=VALUE>"));
    assert!(help.contains("--assignee <ASSIGNEE>"));
    assert!(help.contains("'me' for the current user"));
    assert!(!help.contains("--assigned-to-id"));
    assert!(help.contains("--short"));
    assert!(help.contains("cf_ID=VALUE"));
    assert!(help.contains(
        "# List open issues assigned to the current user\n  redmine-cli issues list --project-id 1709 --status open --assignee me"
    ));
    assert!(help.contains("--tracker-id 2 --priority-id 4 --subject \"登录\""));
    assert!(
        help.contains(
            "--include attachments,relations --sort updated_on:desc --limit 20 --offset 0"
        )
    );
    assert!(help.lines().any(|line| {
        line.contains("--project-id <PROJECT_ID>") && line.contains("Filter by project ID")
    }));
    assert!(!help.contains("--custom-fields"));
}

#[test]
fn issue_list_fields_and_short_are_mutually_exclusive() {
    Command::cargo_bin("redmine-cli")
        .unwrap()
        .args(["issue", "list", "--fields", "id", "--short"])
        .assert()
        .failure()
        .stderr(contains("cannot be used with"));
}

#[test]
fn issue_create_and_update_use_assignee_option() {
    for action in ["create", "update"] {
        let output = Command::cargo_bin("redmine-cli")
            .unwrap()
            .args(["issues", action, "--help"])
            .output()
            .unwrap();
        assert!(output.status.success());
        let help = String::from_utf8(output.stdout).unwrap();
        assert!(help.contains("--assignee <ID>"));
        assert!(!help.contains("--assigned-to-id"));
    }
}

#[test]
fn command_help_uses_single_line_options_and_comment_examples() {
    let cases = [
        (
            ["issue", "get", "--help"].as_slice(),
            "--fields <FIELDS>",
            "Return only these comma-separated fields",
            "# Numeric issue IDs use -p/--profile, REDMINE_PROFILE, or active_profile",
        ),
        (
            ["project", "list", "--help"].as_slice(),
            "--limit <LIMIT>",
            "Maximum number of results to return",
            "# List resources for a project",
        ),
        (
            ["auth", "login", "--help"].as_slice(),
            "-s, --server <SERVER>",
            "Redmine server URL",
            "# Enter missing values interactively; the name defaults from the server hostname",
        ),
        (
            ["attachment", "download", "--help"].as_slice(),
            "--output <OUTPUT>",
            "Output file path",
            "# Download an attachment to the default Downloads directory",
        ),
        (
            ["search", "--help"].as_slice(),
            "--limit <LIMIT>",
            "Maximum number of results to return",
            "# Search Redmine for a phrase",
        ),
    ];

    for (args, argument, description, example_comment) in cases {
        let output = Command::cargo_bin("redmine-cli")
            .unwrap()
            .args(args)
            .output()
            .unwrap();
        assert!(output.status.success());
        let help = String::from_utf8(output.stdout).unwrap();
        assert!(
            help.lines()
                .any(|line| { line.contains(argument) && line.contains(description) }),
            "{argument} description is not on the same line:\n{help}"
        );
        assert!(
            help.contains(&format!("{example_comment}\n  redmine-cli")),
            "{help}"
        );
    }
}

#[test]
fn global_profile_option_has_its_own_subcommand_help_section() {
    let root_output = Command::cargo_bin("redmine-cli")
        .unwrap()
        .arg("--help")
        .output()
        .unwrap();
    let root_help = String::from_utf8(root_output.stdout).unwrap();
    assert!(root_help.contains("Options:"));
    assert!(root_help.contains("-p, --profile"));
    assert!(!root_help.contains("Global Options:"));

    let child_output = Command::cargo_bin("redmine-cli")
        .unwrap()
        .args(["issue", "list", "--help"])
        .output()
        .unwrap();
    let child_help = String::from_utf8(child_output.stdout).unwrap();
    assert!(child_help.contains("Options:"));
    assert!(child_help.contains("Global Options:"));
    let global_options = child_help.split("Global Options:").nth(1).unwrap();
    assert!(
        global_options
            .lines()
            .any(|line| line.contains("-p, --profile"))
    );
}

#[test]
fn read_only_config_rejects_issue_mutations() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("config.yaml");
    fs::write(
        &path,
        "read_only: true\nactive_profile: prod\nprofiles:\n  prod:\n    url: https://redmine.example.com/\n    api_key: key\n",
    )
    .unwrap();

    Command::cargo_bin("redmine-cli")
        .unwrap()
        .env("REDMINE_CONFIG", path)
        .args(["issue", "create", "--json", "{}"])
        .assert()
        .failure()
        .stdout(contains("configuration is read-only"));
}

#[test]
fn read_only_config_rejects_generic_resource_mutations() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("config.yaml");
    fs::write(
        &path,
        "read_only: true\nactive_profile: prod\nprofiles:\n  prod:\n    url: https://redmine.example.com/\n    api_key: key\n",
    )
    .unwrap();

    Command::cargo_bin("redmine-cli")
        .unwrap()
        .env("REDMINE_CONFIG", path)
        .args(["project", "delete", "1"])
        .assert()
        .failure()
        .stdout(contains("configuration is read-only"));
}

#[test]
fn profile_read_only_false_overrides_global_read_only() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("config.yaml");
    fs::write(
        &path,
        "read_only: true\nactive_profile: prod\nprofiles:\n  prod:\n    url: http://127.0.0.1:1/\n    api_key: key\n    read_only: false\n",
    )
    .unwrap();

    Command::cargo_bin("redmine-cli")
        .unwrap()
        .env("REDMINE_CONFIG", path)
        .args(["issue", "create", "--project-id", "1", "--subject", "Test"])
        .assert()
        .failure()
        .stdout(predicates::str::contains("configuration is read-only").not());
}

#[test]
fn issue_url_read_only_policy_uses_the_url_matched_profile() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("config.yaml");
    fs::write(
        &path,
        "active_profile: writable\nprofiles:\n  writable:\n    url: https://write.example.com/\n    api_key: key\n  protected:\n    url: https://readonly.example.com/\n    api_key: key\n    read_only: true\n",
    )
    .unwrap();

    Command::cargo_bin("redmine-cli")
        .unwrap()
        .env("REDMINE_CONFIG", path)
        .args(["issue", "delete", "https://readonly.example.com/issues/123"])
        .assert()
        .failure()
        .stdout(contains("configuration is read-only"));
}

#[test]
fn redmine_profile_environment_overrides_active_profile() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("config.yaml");
    fs::write(
        &path,
        "active_profile: writable\nprofiles:\n  writable:\n    url: http://127.0.0.1:1/\n    api_key: key\n    read_only: false\n  protected:\n    url: http://127.0.0.1:1/\n    api_key: key\n    read_only: true\n",
    )
    .unwrap();

    Command::cargo_bin("redmine-cli")
        .unwrap()
        .env("REDMINE_CONFIG", &path)
        .env("REDMINE_PROFILE", "protected")
        .args(["issue", "create", "--project-id", "1", "--subject", "Test"])
        .assert()
        .failure()
        .stdout(contains("configuration is read-only"));
}

#[test]
fn command_line_profile_overrides_redmine_profile_environment() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("config.yaml");
    fs::write(
        &path,
        "profiles:\n  writable:\n    url: http://127.0.0.1:1/\n    api_key: key\n    read_only: false\n  protected:\n    url: http://127.0.0.1:1/\n    api_key: key\n    read_only: true\n",
    )
    .unwrap();

    Command::cargo_bin("redmine-cli")
        .unwrap()
        .env("REDMINE_CONFIG", path)
        .env("REDMINE_PROFILE", "protected")
        .args([
            "-p",
            "writable",
            "issue",
            "create",
            "--project-id",
            "1",
            "--subject",
            "Test",
        ])
        .assert()
        .failure()
        .stdout(predicates::str::contains("configuration is read-only").not());
}
