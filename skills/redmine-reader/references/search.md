# Search

Search Redmine text and paginate results with `--limit` and `--offset`:

```bash
redmine-cli -p work search "login error" --limit 20
redmine-cli -p work search "customer" --limit 20 --offset 20
```
