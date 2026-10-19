# User queries

```bash
# List active users and select response fields
redmine-cli -p work users list --status 1 --limit 20 \
  --fields id,login,firstname,lastname

# Get one user by ID
redmine-cli -p work users get 5 --fields id,login,firstname,lastname

# Inspect fields available in user responses
redmine-cli users fields
```

The Redmine users-list endpoint requires administrator privileges. The `status`
filter accepts a Redmine user status ID. `fields` and `schema` are local metadata
commands and do not contact Redmine.
