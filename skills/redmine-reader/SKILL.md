---
name: redmine-reader
description: >
  Use the Rust implementation of redmine-cli for read-only Redmine work:
  inspect, list, filter, and search issues; review issue journals, attachments,
  and relations; and query project, user, time-entry, or wiki data. Trigger when
  the user asks about Redmine issues/tickets, their assigned work, project
  context, or provides a Redmine issue URL. This skill is specific to the Rust
  CLI and must use its command names, flags, profile handling, and JSON envelope.
  Do not use it to create, update, or delete Redmine data or to change local auth
  profiles.
---

# Redmine Reader (Rust CLI)

Use the Rust `redmine-cli` for read-only Redmine queries. Check the exact
commands and examples in [`references/commands.md`](references/commands.md), and
profile behavior in [`references/configuration.md`](references/configuration.md).

## Workflow

1. If the executable or its version is uncertain, run `redmine-cli --version`
   and confirm the output identifies the Rust implementation; use the current
   Rust command help when a flag is unclear.
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
5. Parse the Rust CLI JSON envelope: successful output is
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

## Maintenance rule

When Rust CLI commands, flags, profile resolution, output envelopes, or read
behavior change, update this skill and its references in the same change. Verify
the documented examples against the Rust CLI help and add or adjust CLI tests
when the documented interface changes. Keep this skill aligned with the Rust
implementation.
