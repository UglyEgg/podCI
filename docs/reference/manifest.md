<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2026 Richard Majewski - Varanid Works -->

# Manifest

## Stability guarantees

- The manifest format is **versioned**. This document describes **v1**.
- **Non-breaking additions:** new fields may be added. Consumers must ignore unknown fields.
- **Breaking changes:** removal/renaming of fields or semantic changes require a major version bump (and a new schema version).
- Field values are designed to be deterministic given the same inputs (namespace/env_id derivation is treated as a compatibility surface).

Every `podci run` writes a machine-readable manifest describing what executed and what happened.

- **Primary purpose:** stable structured output for CI automation.
- **Secondary purpose:** quick human inspection when debugging.

If you need something reliably parseable, consume the manifest. Logs are diagnostic and best-effort.

## Stability contract

- The manifest is **explicitly versioned** via `schema`.
- **Additive** fields may be introduced in `podci-manifest.v1` without breaking consumers.
- **Breaking** changes require a new schema string (e.g. `podci-manifest.v2`) and migration notes.

## Storage location

Manifests are written under XDG state:

- Typical state dir: `~/.local/state/podci/`
- Latest: `manifest.json`
- Per-run: `runs/<run_id>/manifest.json`

`XDG_STATE_HOME` overrides the base directory.

## Schema: `podci-manifest.v1`

Top-level fields:

| Field | Type | Notes |
|---|---:|---|
| `schema` | string | Always `podci-manifest.v1` |
| `podci_version` | string | `CARGO_PKG_VERSION` of the running binary |
| `timestamp_utc` | string | RFC3339 UTC timestamp (time the manifest was written) |
| `project` | string | From `podci.toml` |
| `job` | string | Job name executed |
| `profile` | string | Profile name resolved for the run |
| `namespace` | string | Derived namespace (do not parse; treat as opaque) |
| `env_id` | string | Derived environment fingerprint (opaque) |
| `base_image_digest` | string\|null | Base image digest when known |
| `base_image_digest_status` | string\|null | Best-effort status for digest capture (`present`, `unavailable`, `error`) |
| `steps` | array | Ordered `ManifestStepV1` entries |
| `result` | object | Overall `ManifestResultV1` |

### `steps[]` entries (`ManifestStepV1`)

| Field | Type | Notes |
|---|---:|---|
| `name` | string | Step name |
| `argv` | array<string> | The argv executed inside the container |
| `duration_ms` | number\|null | Duration if available |
| `exit_code` | number\|null | Exit code if the step ran |
| `stdout_path` | string\|null | Relative path (from `runs/<run_id>/`) to captured stdout |
| `stderr_path` | string\|null | Relative path (from `runs/<run_id>/`) to captured stderr |

### `result` (`ManifestResultV1`)

| Field | Type | Notes |
|---|---:|---|
| `ok` | bool | `true` if the run succeeded |
| `exit_code` | number | Overall exit code |
| `error` | string\|null | Error summary when failing |

## Example

```json
{
  "schema": "podci-manifest.v1",
  "podci_version": "0.5.0",
  "timestamp_utc": "2026-02-19T09:51:12Z",
  "project": "example",
  "job": "default",
  "profile": "dev",
  "namespace": "podci_example_default_…",
  "env_id": "…",
  "base_image_digest": "sha256:…",
  "base_image_digest_status": "present",
  "steps": [
    {
      "name": "fmt",
      "argv": ["cargo", "fmt", "--all", "--", "--check"],
      "duration_ms": 412,
      "exit_code": 0
    }
  ],
  "result": { "ok": true, "exit_code": 0, "error": null }
}
```

## Related

- **Operations → Manifests** (where files live and how to view them)
- **Reference → CLI** (`podci manifest show`)
