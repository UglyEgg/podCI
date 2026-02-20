<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2026 Richard Majewski - Varanid Works -->

# Logging

podCI uses `tracing` for structured logging.

Two output formats are supported:

- `human`: developer-friendly text
- `jsonl`: one JSON object per line (intended for log ingestion)

If you need reliably parseable run results, consume the **manifest** (Reference → Manifest). Logs are diagnostic.

## Selecting a log format

CLI flag:

```bash
podci run --log-format human
podci run --log-format jsonl
```

Environment variable (default when `--log-format` is not set):

```bash
export PODCI_LOG_FORMAT=jsonl
```

## Log level filtering

podCI uses `RUST_LOG` via `tracing_subscriber::EnvFilter`.

Examples:

```bash
# Default (if unset): info

# More detail from podCI crates
RUST_LOG=podci=debug,podci_podman=debug podci run --job default

# Everything (noisy)
RUST_LOG=trace podci run --job default
```

## Color behavior (human mode)

Color is enabled only when stdout is a terminal.

Color is disabled when:

- `NO_COLOR` is set, or
- `TERM=dumb`, or
- stdout is not a TTY

## JSONL format

### Coarse stability guarantees

- **Stable:** one JSON object per line.
- **Mostly stable:** podCI’s structured fields (e.g. `run_id`, `job`, `step`, `namespace`, `cmd`, `exit_code`) are intended to remain stable within a major version.
- **Not stable:** exact top-level JSON structure emitted by the `tracing-subscriber` JSON formatter (it may add fields over time).

### Typical record shape

podCI currently uses `tracing-subscriber`’s JSON formatter with the current span included.

A typical line looks like:

```json
{
  "timestamp": "2026-02-19T23:12:34.567Z",
  "level": "INFO",
  "target": "podci",
  "fields": {
    "message": "step_start",
    "job": "default",
    "step": "test"
  }
}
```

Treat the record as a JSON object; do not rely on field ordering.

### Event taxonomy

podCI emits a small set of consistent “event messages” (the `message` field in JSONL, and the visible line label in human mode):

- `run_start` (includes `run_id`, `project`, `job`, `profile`, `namespace`)
- `step_start` / `step_end` (includes `job`, `step`)
- `podman_start` / `podman_exit` (includes `cmd`, plus `exit_code`/`duration_ms` on exit)
- `manifest_written` (includes `path`)

Warnings are also emitted with clear messages, for example:

- `existing_volume_missing_podci_labels`
- `base_image_digest_missing_reproducibility_weakened`

For large step output, use the captured per-step logs and the manifest paths (Operations → Manifests).
