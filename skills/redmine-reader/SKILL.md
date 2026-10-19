---
name: redmine-reader
description: >
  Use redmine-cli for read-only Redmine work:
  inspect, list, filter, and search issues; review issue journals, attachments,
  and relations; and query project, user, time-entry, or wiki data. Trigger when
  the user asks about Redmine issues/tickets, their assigned work, project
  context, or provides a Redmine issue URL. Follow the CLI's command names,
  flags, profile handling, and JSON envelope.
  Do not use it to create, update, or delete Redmine data or to change local auth
  profiles.
---

# Redmine Reader

Use `redmine-cli` for read-only Redmine queries. Check the exact
commands and examples in the resource reference linked below. Read
[`references/configuration.md`](references/configuration.md) when profile
selection or authentication status is relevant.

## Workflow

1. If a flag or command is unclear, check the current help with
   `redmine-cli --help` or `redmine-cli <command> --help`.
2. Select a configured profile with `-p <profile>`, `REDMINE_PROFILE`, or the
   configured `active_profile`. Full issue URLs can automatically match a
   configured profile; numeric issue IDs need a selected profile.
3. For issue lists, keep responses concise: use `--limit 20` unless the user
   requests more, and prefer `--short` or a targeted `--fields` projection.
   `--fields` and `--short` filter output in the CLI; they do not reduce the
   fields fetched from Redmine.
4. Add related data only when requested. Use `--journals`, `--children`,
   `--relations`, or `--attachments` with `issues get`; use
   `--download-attachments <directory>` only when the user explicitly asks to
   save attachment files locally.
5. Parse the CLI JSON envelope: successful output is
   `{"ok":true,"data":...}`; errors are `{"ok":false,"error":"..."}`.
   For list results, the `data` object contains pagination information and an
   `issues` array. Summarize relevant results instead of dumping large payloads.

## Read-only and credential safety

- Never run Redmine mutations such as `issues create`, `issues update`,
  `issues delete`, or equivalent mutation commands for other resources.
- Do not run `auth login`, `auth logout`, or `auth switch`; they write local
  authentication configuration. If no profile is configured, explain that the
  user needs to configure one separately.
- Never print, copy into reports, or ask the user to paste an API key. Use
  `auth list` or `auth status` to inspect profile availability without reading
  or displaying config-file secrets.
- Treat downloaded attachments as local file writes; download only when asked.
- Report errors briefly and avoid echoing secrets or unnecessary config paths.

## References

Before constructing a command, read the reference for the requested resource.
Read only the relevant file(s); use more than one when a request spans
resources.

### Issues

For issue lookup, lists, filters, journals, relations, and related issue data,
read [`references/issues.md`](references/issues.md).

### Attachments

For issue attachment metadata or downloading files, read
[`references/attachments.md`](references/attachments.md). Download only when
the user asks to save files.

### Projects

For project queries, read [`references/projects.md`](references/projects.md).

### Users

For user queries and access requirements, read
[`references/users.md`](references/users.md).

### Time entries

For time-entry queries and filters, read
[`references/time.md`](references/time.md).

### Wiki pages

For wiki-page query support and its current API limitation, read
[`references/wiki.md`](references/wiki.md).

### Search

For Redmine text search and pagination, read
[`references/search.md`](references/search.md).

## Maintenance rule

When commands, flags, profile resolution, output envelopes, or read behavior
change, update this skill and its references in the same change. Verify the
documented examples against CLI help and add or adjust CLI tests when the
documented interface changes. Keep this skill aligned with the CLI.
