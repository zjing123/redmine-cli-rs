# Project queries

```bash
# List projects with selected fields
redmine-cli -p work projects list --limit 20 --fields id,name,identifier

# Get a project by ID
redmine-cli -p work projects get 1 --fields id,name,identifier,description

# Inspect fields available in project responses
redmine-cli projects fields
```

Use `--limit`, `--offset`, and `--fields` to keep list results focused. The
`fields` and `schema` commands read local metadata and do not contact Redmine.
