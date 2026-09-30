# Config Auth and Search Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend the forked `redmine-api` on branch `dev` with HTTP Basic Auth and Search support, then add profile management, connection testing, and Search commands to `redmine-cli-rs`.

**Architecture:** Add an authentication enum to the fork's synchronous and asynchronous clients while preserving the existing API-Key constructors. Implement Search as a typed endpoint using the library's existing endpoint/request abstractions. The CLI will use the fork for `config test` and `search`; existing generic resource requests remain unchanged in this phase.

**Tech Stack:** Rust 2024, `redmine-api` fork branch `dev`, `reqwest`, `serde`, `serde_json`, `serde_yaml`, `clap`, `rpassword`, Cargo tests.

## Global Constraints

- Use `/home/liuchuan/Documents/liuchuan/redmine-api` as the library repository and its existing `dev` branch.
- Do not expose API keys or passwords in output, errors, `Debug`, or test failure messages.
- Preserve existing API-Key constructors and existing CLI commands.
- `config validate` is local-only; `config test` performs an authenticated HTTP request.
- `config remove` requires explicit `--yes` and must not modify the file otherwise.
- Use mock HTTP tests; do not require a live Redmine server.

---

### Task 1: Add authentication abstraction to `redmine-api`

**Files:**
- Modify: `/home/liuchuan/Documents/liuchuan/redmine-api/src/api.rs`
- Modify: `/home/liuchuan/Documents/liuchuan/redmine-api/src/lib.rs`
- Test: `/home/liuchuan/Documents/liuchuan/redmine-api/src/api.rs` or a focused test module beside it

**Interfaces:**
- Produce public `Authentication` with `ApiKey(String)` and `Basic { username: String, password: String }` variants.
- Preserve `Redmine::new(Client, Url, &str)` and `RedmineAsync::new(Client, Url, &str)` as API-Key constructors.
- Add `Redmine::with_authentication(Client, Url, Authentication)` and the async equivalent.
- Add `Redmine::from_basic_auth(...)` and the async equivalent only if the existing constructor style requires a convenience API; otherwise use `with_authentication` consistently.

- [ ] **Step 1: Write request-header tests** asserting API-Key requests send `x-redmine-api-key` and Basic Auth requests send an `Authorization: Basic ...` header without asserting or printing the password.
- [ ] **Step 2: Run the focused library tests and confirm the new Basic Auth test fails before implementation.**
- [ ] **Step 3:** Replace the internal `api_key` fields with the authentication enum in both clients and centralize request authentication in the sync and async request methods.
- [ ] **Step 4:** Keep the existing `new` methods delegating to `Authentication::ApiKey` and add the new constructor.
- [ ] **Step 5:** Run `cargo test --locked` in the `redmine-api` repository.
- [ ] **Step 6:** Run `cargo fmt --check` and `cargo clippy --all-targets --all-features --locked -- -D warnings`.

### Task 2: Implement `redmine-api` Search endpoint

**Files:**
- Modify: `/home/liuchuan/Documents/liuchuan/redmine-api/src/api/search.rs`
- Modify: `/home/liuchuan/Documents/liuchuan/redmine-api/src/api.rs` only if shared pagination or response helpers are required
- Test: `/home/liuchuan/Documents/liuchuan/redmine-api/src/api/search.rs`

**Interfaces:**
- Produce `Search` endpoint builder with required `q` and optional `limit`, `offset`, and resource-type flags.
- Produce a deserializable `SearchResponse` containing the returned result collections and pagination fields while preserving unknown Redmine fields through `serde_json::Value` where the API response is not stable.

- [ ] **Step 1:** Add endpoint tests for `q`, `limit`, `offset`, and selected resource flags using the library's existing request test helpers.
- [ ] **Step 2:** Run the focused Search tests and confirm they fail because `search.rs` is currently only a placeholder.
- [ ] **Step 3:** Implement the endpoint path `search.json`, query serialization, and response types using the existing `Endpoint`, `ReturnsJsonResponse`, and pagination conventions.
- [ ] **Step 4:** Run focused Search tests, then the complete `redmine-api` test suite.
- [ ] **Step 5:** Format and lint the library.

### Task 3: Switch CLI dependency to the fork and extend profile authentication

**Files:**
- Modify: `/home/liuchuan/Documents/liuchuan/redmine-cli-rs/Cargo.toml`
- Modify: `/home/liuchuan/Documents/liuchuan/redmine-cli-rs/src/config.rs`
- Modify: `/home/liuchuan/Documents/liuchuan/redmine-cli-rs/src/lib.rs`
- Test: `/home/liuchuan/Documents/liuchuan/redmine-cli-rs/tests/config.rs`

**Interfaces:**
- `Profile` retains `url`, `api_key`, `username`, and `password` values.
- Add `Profile::authentication()` returning the fork's `redmine_api::api::Authentication` or a CLI-owned equivalent converted at the client boundary.
- Validation accepts API Key, or a complete username/password pair, with API Key taking precedence.
- Add a serializable redacted profile view containing name, URL, authentication type, and optional username but no secrets.

- [ ] **Step 1:** Add tests for API-Key profiles, complete Basic Auth profiles, incomplete Basic Auth profiles, and API-Key precedence.
- [ ] **Step 2:** Change `Cargo.toml` to use `redmine-api` from `ssh://git@github.com/zjing123/redmine-api.git`, branch `dev`, then run `cargo check`.
- [ ] **Step 3:** Implement authentication validation and redacted profile conversion.
- [ ] **Step 4:** Run `cargo test --locked --test config` and the existing CLI tests.

### Task 4: Add config read-only and mutating commands

**Files:**
- Create: `/home/liuchuan/Documents/liuchuan/redmine-cli-rs/src/config_cli.rs`
- Modify: `/home/liuchuan/Documents/liuchuan/redmine-cli-rs/src/cli.rs`
- Modify: `/home/liuchuan/Documents/liuchuan/redmine-cli-rs/src/config.rs`
- Modify: `/home/liuchuan/Documents/liuchuan/redmine-cli-rs/src/lib.rs`
- Test: `/home/liuchuan/Documents/liuchuan/redmine-cli-rs/tests/config_cli.rs`
- Modify: `/home/liuchuan/Documents/liuchuan/redmine-cli-rs/Cargo.toml` to add `rpassword` for hidden credential input

**Interfaces:**
- Add `ConfigCommand` variants: `List`, `Get { profile }`, `Path`, `Validate`, `Set`, `Update`, `Test`, and `Remove { profile, yes }`.
- `set` requires URL and creates a new profile; `update` changes only explicitly supplied fields.
- API Key is accepted by `--api-key`; username by `--username`; missing secret input is requested through `rpassword` only on an interactive terminal.
- `remove` rejects absent `--yes` before loading or writing the configuration.

- [ ] **Step 1:** Add CLI parsing tests for every subcommand and the `remove` confirmation flag.
- [ ] **Step 2:** Add filesystem tests using `REDMINE_CONFIG` and temporary files for list/get/path/validate/set/update/remove.
- [ ] **Step 3:** Implement configuration path resolution and YAML read/write helpers while preserving unrelated profiles and fields.
- [ ] **Step 4:** Implement redacted output and non-interactive missing-secret errors.
- [ ] **Step 5:** Wire `Config` into the top-level dispatcher and run all CLI tests.

### Task 5: Add `config test` and top-level `search`

**Files:**
- Create: `/home/liuchuan/Documents/liuchuan/redmine-cli-rs/src/redmine_client.rs`
- Modify: `/home/liuchuan/Documents/liuchuan/redmine-cli-rs/src/config_cli.rs`
- Modify: `/home/liuchuan/Documents/liuchuan/redmine-cli-rs/src/cli.rs`
- Modify: `/home/liuchuan/Documents/liuchuan/redmine-cli-rs/src/lib.rs`
- Test: `/home/liuchuan/Documents/liuchuan/redmine-cli-rs/tests/config_cli.rs`
- Test: `/home/liuchuan/Documents/liuchuan/redmine-cli-rs/tests/search.rs`

**Interfaces:**
- `RedmineClient::from_profile(&Profile)` constructs the fork client using API Key or Basic Auth.
- `RedmineClient::current_user()` calls the existing current-user endpoint and returns a redacted JSON value.
- `RedmineClient::search(query, limit, offset, resource_types)` calls the fork Search endpoint and returns JSON.

- [ ] **Step 1:** Add mock HTTP tests proving `config test` sends API-Key or Basic Auth and returns current-user data without the credential.
- [ ] **Step 2:** Add Search CLI tests for query, profile selection, pagination, and JSON output.
- [ ] **Step 3:** Implement the shared CLI client wrapper around the fork's blocking client.
- [ ] **Step 4:** Implement `config test` and `search` dispatch with existing success/error output conventions.
- [ ] **Step 5:** Run `cargo test --locked`, `cargo fmt --check`, and `cargo clippy --all-targets --all-features --locked -- -D warnings` in the CLI repository.

### Task 6: Documentation and final verification

**Files:**
- Modify: `/home/liuchuan/Documents/liuchuan/redmine-cli-rs/README.md`
- Modify: `/home/liuchuan/Documents/liuchuan/redmine-cli-rs/docs/PROGRESS.md`

- [ ] **Step 1:** Document API-Key and Basic Auth profiles, hidden credential input, all config subcommands, and Search usage.
- [ ] **Step 2:** Run the complete test and lint commands in both repositories.
- [ ] **Step 3:** Review `git diff` in both repositories and verify no secrets, generated files, or unrelated changes are included.
- [ ] **Step 4:** Commit library changes on `dev` and CLI changes on the current CLI branch with separate descriptive commits.
