# Rust `redmine-cli` read commands

All successful commands emit a JSON object with `ok` and `data`. The examples
below assume `work` is a configured profile. Global `-p/--profile` can be used
before a command; `REDMINE_PROFILE` or `active_profile` can select it instead.

## Issues

```bash
# Read one issue by numeric ID or full URL
redmine-cli -p work issues get 123
redmine-cli issues get https://redmine.example.com/issues/123

# Project issue list with a compact response
redmine-cli -p work issues list --project-id 1 --status open --limit 20 --short

# Issues assigned to the current API user
redmine-cli -p work issues list --assignee me --status open --limit 20 \
  --fields id,subject,status,priority,updated_on

# Issues assigned to a user/group ID, or no one
redmine-cli -p work issues list --assignee 42 --limit 20
redmine-cli -p work issues list --assignee none --limit 20

# Repeat a Redmine field filter; custom fields use cf_<ID>
redmine-cli -p work issues list --filter status_id=1 --filter cf_12=frontend

# Include related data when needed
redmine-cli -p work issues get 123 --journals --relations --attachments

# Search Redmine text
redmine-cli -p work search "login error" --limit 20
```

`issues list` supports `--project-id`, `--tracker-id`, `--priority-id`,
`--status`, `--assignee`, `--subject`, repeatable `--filter KEY=VALUE`,
`--include`, `--sort`, `--fields`, `--short`, `--limit`, and `--offset`.
`--assignee` accepts a numeric user/group ID, `me`, or `none`; it does not
resolve names or logins. `--fields` and `--short` are mutually exclusive.

`issues get` accepts a numeric ID or issue URL. Related data can be requested
with `--include` or the `--journals`, `--children`, `--relations`, and
`--attachments` shortcuts. Attachment downloads write files locally and should
only be performed when explicitly requested.

## Projects and other resources

Use the plural Rust command names. These resources use the shared `get`,
`list`, `fields`, and `schema` commands; `projects`, `users`, `time`, and `wiki`
also expose create/update/delete commands, which this read-only skill must not
invoke.

```bash
redmine-cli -p work projects list --limit 20 --fields id,name,identifier
redmine-cli -p work projects get 1 --fields id,name,identifier,description
redmine-cli -p work users list --limit 20 --fields id,login,firstname,lastname
redmine-cli -p work time list --project-id 1 --limit 20
redmine-cli -p work wiki list --project-id 1 --limit 20
```

Check a resource's current options with `redmine-cli <resource> list --help` or
`redmine-cli <resource> get --help` before constructing less common queries.
