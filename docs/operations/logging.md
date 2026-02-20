<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2026 Richard Majewski - Varanid Works -->

# Logging operations

This page is operational guidance for collecting and using podCI logs.

For the format contract, see Reference → Logging.

## Choosing a format

For interactive use:

```bash
podci run --job default --log-format human
```

For CI ingestion:

```bash
podci run --job default --log-format jsonl
```

If you want JSONL by default for a shell session:

```bash
export PODCI_LOG_FORMAT=jsonl
```

## Capturing logs

### Human mode

Human mode is meant for a terminal. If you redirect it, you may lose operator hints that are easiest to read with color.

### JSONL mode

Redirect to a file and ingest it:

```bash
podci run --job default --log-format jsonl > podci.jsonl
```

## Per-step stdout/stderr (authoritative)

podCI captures each step’s stdout and stderr into the run directory under XDG state:

- default state dir: `~/.local/state/podci/`
- per-run directory: `runs/<run_id>/`
- per-step logs:
  - `runs/<run_id>/logs/<step>.stdout`
  - `runs/<run_id>/logs/<step>.stderr`

These files are the right place to look when Podman output is large or when the CLI truncates error summaries.

The run manifest includes the relative paths (Operations → Manifests).

## Log level escalation

Use `RUST_LOG` to raise verbosity:

```bash
RUST_LOG=podci=debug,podci_podman=debug podci run --job default
```

If you are collecting logs for triage, include:

- the full JSONL output (if used)
- the run manifest
- the failing step’s `*.stdout` / `*.stderr`
