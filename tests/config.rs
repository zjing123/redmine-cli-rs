use std::fs;

use redmine_cli::config::{Config, parse_issue_reference};

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

#[test]
fn accepts_basic_auth_profiles_and_prioritizes_api_key() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.yaml");
    fs::write(
        &path,
        r#"
profiles:
  basic:
    url: https://redmine.example.com/
    username: alice
    password: password
  both:
    url: https://redmine.example.com/
    api_key: api-key
    username: ignored
    password: ignored
"#,
    )
    .unwrap();

    let config = Config::load(Some(&path)).unwrap();
    let (_, basic) = config.profile(Some("basic")).unwrap();
    assert!(matches!(
        basic.authentication().unwrap(),
        redmine_api::api::Authentication::Basic { .. }
    ));
    let (_, both) = config.profile(Some("both")).unwrap();
    assert!(matches!(
        both.authentication().unwrap(),
        redmine_api::api::Authentication::ApiKey(_)
    ));
}
