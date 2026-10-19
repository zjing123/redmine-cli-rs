# Time-entry queries

```bash
# List time entries for a project
redmine-cli -p work time list --project-id 1 --limit 20

# Filter by user and select fields
redmine-cli -p work time list --user-id 5 --limit 20 \
  --fields id,issue,hours,spent_on,user

# Get one time entry by ID
redmine-cli -p work time get 1 --fields id,issue,hours,spent_on,user

# Inspect available response fields
redmine-cli time fields
```

Use `--project-id` or `--user-id` to filter the list, and `--fields`, `--limit`,
and `--offset` to shape the response. `fields` and `schema` are local metadata
commands and do not contact Redmine.
