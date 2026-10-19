# Wiki-page queries

The CLI exposes these read commands:

```bash
redmine-cli -p work wiki list --project-id 1 --limit 20
redmine-cli -p work wiki get 1
redmine-cli wiki fields
```

The current list/get implementation uses a generic `/wiki_pages/...` route.
Standard Redmine's Wiki Pages API instead addresses pages by project and page
title (`/projects/{project}/wiki/{title}`), so list/get may fail against a
standard Redmine server. `fields` and `schema` are local metadata commands.
Do not claim wiki data was retrieved unless the command succeeds.
