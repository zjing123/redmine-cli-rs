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
fn validates_api_key_profiles() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.yaml");
    fs::write(
        &path,
        r#"
profiles:
  prod:
    url: https://redmine.example.com/
    api_key: api-key
"#,
    )
    .unwrap();

    let config = Config::load(Some(&path)).unwrap();
    let (_, profile) = config.profile(Some("prod")).unwrap();
    assert_eq!(profile.api_key.as_deref(), Some("api-key"));
}

#[test]
fn resolves_the_active_profile_when_no_override_is_given() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.yaml");
    fs::write(
        &path,
        r#"
active_profile: staging
profiles:
  prod:
    url: https://prod.example.com/
    api_key: prod-key
  staging:
    url: https://staging.example.com/
    api_key: staging-key
"#,
    )
    .unwrap();

    let config = Config::load(Some(&path)).unwrap();

    assert_eq!(config.profile(None).unwrap().0, "staging");
}

#[test]
fn explicit_profile_overrides_the_active_profile() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.yaml");
    fs::write(
        &path,
        r#"
active_profile: staging
profiles:
  prod:
    url: https://prod.example.com/
    api_key: prod-key
  staging:
    url: https://staging.example.com/
    api_key: staging-key
"#,
    )
    .unwrap();

    let config = Config::load(Some(&path)).unwrap();

    assert_eq!(config.profile(Some("prod")).unwrap().0, "prod");
}

#[test]
fn profile_read_only_overrides_global_read_only() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.yaml");
    fs::write(
        &path,
        r#"
read_only: true
profiles:
  writable:
    url: https://redmine.example.com/
    api_key: key
    read_only: false
  inherited:
    url: https://redmine.example.com/
    api_key: key
"#,
    )
    .unwrap();

    let config = Config::load(Some(&path)).unwrap();

    assert!(!config.is_read_only_for("writable"));
}

#[test]
fn profile_read_only_can_enable_read_only_over_global_false() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.yaml");
    fs::write(
        &path,
        r#"
read_only: false
profiles:
  protected:
    url: https://redmine.example.com/
    api_key: key
    read_only: true
"#,
    )
    .unwrap();

    let config = Config::load(Some(&path)).unwrap();

    assert!(config.is_read_only_for("protected"));
}

#[test]
fn uses_global_read_only_for_profiles_without_an_override() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.yaml");
    fs::write(
        &path,
        r#"
read_only: true
profiles:
  inherited:
    url: https://redmine.example.com/
    api_key: key
"#,
    )
    .unwrap();

    let config = Config::load(Some(&path)).unwrap();

    assert!(config.is_read_only_for("inherited"));
}

#[test]
fn removing_the_active_profile_clears_active_selection() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.yaml");
    fs::write(
        &path,
        "active_profile: prod\nprofiles:\n  prod:\n    url: https://redmine.example.com/\n    api_key: key\n",
    )
    .unwrap();

    let mut config = Config::load(Some(&path)).unwrap();
    config.remove_profile("prod");

    assert_eq!(config.active_profile(), None);
}
