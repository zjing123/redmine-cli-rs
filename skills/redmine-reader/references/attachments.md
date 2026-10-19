# Attachments

To inspect an issue's attachment metadata, request attachments with the issue:

```bash
redmine-cli -p work issues get 123 --attachments
```

To download a specific attachment, use its numeric attachment ID:

```bash
redmine-cli -p work attachments download 391795 --output ./attachment.png
```

Without `--output`, the file is saved under `$HOME/Downloads` using the
attachment's filename. Existing files are not overwritten. Downloading writes
to the local filesystem, so do it only when the user explicitly asks to save
the attachment.
