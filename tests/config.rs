use std::fs;

use redmine_cli_rs::config::{Config, parse_issue_reference};

#[test]
fn matches_url_to_the_longest_profile_path() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.yaml");
    fs::write(
        &path,
        r#"
profiles:
  host:
    url: https://redmine.example.com/
    api_key: host-key
  nested:
    url: https://redmine.example.com/redmine/
    api_key: nested-key
"#,
    )
    .unwrap();

    let config = Config::load(Some(&path)).unwrap();
    let reference = parse_issue_reference(
        "https://redmine.example.com/redmine/issues/123",
        None,
        &config,
    )
    .unwrap();
    assert_eq!(reference.profile, "nested");
    assert_eq!(reference.id, 123);
}
