# Issue queries

All examples assume `work` is a configured profile. Numeric IDs use the
selected profile; a full issue URL can match a configured profile automatically.

```bash
# Get an issue by ID or URL
redmine-cli -p work issues get 123
redmine-cli issues get https://redmine.example.com/issues/123

# List a project's open issues, using a compact response
redmine-cli -p work issues list --project-id 1 --status open --limit 20 --short

# List issues assigned to the current API user
redmine-cli -p work issues list --assignee me --status open --limit 20 \
  --fields id,subject,status,priority,updated_on

# Filter by assignee ID or unassigned state
redmine-cli -p work issues list --assignee 42 --limit 20
redmine-cli -p work issues list --assignee none --limit 20

# Repeat a field filter; custom fields use cf_<ID>
redmine-cli -p work issues list --filter status_id=1 --filter cf_12=frontend

# Include related data
redmine-cli -p work issues get 123 --journals --children --relations --attachments
```

`issues list` supports `--project-id`, `--tracker-id`, `--priority-id`,
`--status`, `--assignee`, `--subject`, repeatable `--filter KEY=VALUE`,
`--include`, `--sort`, `--fields`, `--short`, `--limit`, and `--offset`.
`--assignee` accepts a numeric user/group ID, `me`, or `none`; it does not
resolve names or logins. `--fields` and `--short` are mutually exclusive and
filter output after the response is fetched.

Related data can be requested with `--include` or the `--journals`,
`--children`, `--relations`, and `--attachments` shortcuts. See
[`attachments.md`](attachments.md) before saving attachment files locally.
