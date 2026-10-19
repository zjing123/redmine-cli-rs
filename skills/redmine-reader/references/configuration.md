# Rust CLI profiles and configuration

The Rust CLI stores its YAML configuration by default at
`~/.config/redmine-cli/config.yaml`. `REDMINE_CONFIG` overrides that path.
Use `redmine-cli config path` to print the active path without displaying its
contents.

Select a profile in this order:

1. `-p/--profile <PROFILE>` on the command line.
2. `REDMINE_PROFILE` in the environment.
3. `active_profile` in the configuration file.

Inspect configured profiles without exposing API keys:

```bash
redmine-cli auth list
redmine-cli auth status
redmine-cli -p work auth status
```

`auth list` reports profile names, server URLs, and which profile is active;
`auth status` contacts Redmine and reports the selected profile and current
user. Do not print configuration-file contents or API keys. If no profile is
available, stop and ask the user to configure authentication outside this
read-only workflow; do not invoke `auth login`, `auth logout`, or `auth switch`.

For issue requests, a full issue URL can match a configured profile by server
URL. Numeric issue IDs require the profile to be selected through one of the
three mechanisms above.
