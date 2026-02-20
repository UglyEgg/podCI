<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2026 Richard Majewski - Varanid Works -->

# Troubleshooting

This page is a runbook for common podCI failures on developer machines and CI runners.

## First step: run `podci doctor`

`podci doctor` performs a fast environment check (XDG dirs, podman presence/version, best-effort rootless status, and labeled volume create/inspect/remove).

```bash
podci doctor
```

## Podman rootless is not working

Symptoms:
- permission errors creating containers
- network failures in rootless mode

Actions:
- Ensure rootless Podman is installed and functional (`podman info` succeeds).
- Confirm `XDG_RUNTIME_DIR` is set and writable.
- On systems with SELinux, ensure volume mounts use appropriate labels when required.

## SELinux volume mount failures

Symptoms:
- container cannot read/write mounted paths
- AVC denials in audit logs

Actions:
- Prefer SELinux-aware mount options (e.g. `:Z`) where applicable.
- Avoid mounting system paths with restrictive labels into containers.

## “Cache not reused” surprises

Symptoms:
- builds re-download dependencies
- incremental build feels reset

Actions:
- Verify you are using the same profile (affects `env_id`).
- Verify the template image identity did not change (digest/tag).
- Confirm cache paths are declared and mounted for the step.

## Container storage corruption

Symptoms:
- overlay errors
- failures creating read-write layers

Actions:
- Confirm Podman storage is healthy.
- If storage is corrupt, follow your distro’s Podman cleanup guidance.
- Re-run podCI after storage recovery; caches are namespaced and can be pruned safely.

## Diagnostics

- podCI prints short operator hints for common Podman failures in human log mode.
- Error messages may truncate very large Podman stderr/stdout output. Full per-step logs are written under:
  - `$XDG_STATE_HOME/podci/runs/<run_id>/logs/` (default: `~/.local/state/podci/runs/<run_id>/logs/`)
  - If available, paths to the captured logs are included in the error message and in the run manifest.
- Re-run with JSONL logs (`--log-format jsonl`) and capture output for analysis.
- Inspect the manifest for per-step exit codes, durations, and log paths.
