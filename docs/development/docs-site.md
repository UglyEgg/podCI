<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2026 Richard Majewski - Varanid Works -->

# Docs site toolchain

podCI’s documentation site is generated using **Zensical**.

Context:

- MkDocs 1.x is unmaintained.
- Material for MkDocs has announced that MkDocs 2.0 is incompatible and recommends migrating to Zensical.

podCI keeps `mkdocs.yml` as the single configuration file, using Zensical’s MkDocs/Material compatibility layer.

## Requirements

- Python 3.10+

## Local workflow

From the repository root:

```bash
python -m venv .venv
. .venv/bin/activate
pip install -r docs/requirements.txt

# Generate Rust API docs into docs/ (creates docs/rustdoc/api/)
./scripts/gen_rustdoc_into_docs.sh

# Build the site into ./site/
./scripts/build_docs.sh
```

At that point you can serve `./site/` using any static file server.

## Rust API docs

Rust API docs are generated under:

- `docs/rustdoc/api/`

During a build, `scripts/build_docs.sh` copies that directory into:

- `site/rustdoc/api/`

The navigation entry **Rust API docs** points at a Markdown landing page (`docs/rustdoc/index.md`) which links into the generated rustdoc HTML.

### Common 404: local base path mismatch

If your local server is at `http://127.0.0.1:8000/` but links try to navigate to `/podCI/...`, that’s almost always caused by `site_url` being configured for GitHub Pages (`https://.../podCI/`).

Fix options:

- For local testing, temporarily override `site_url` to a root URL.
  - simplest: copy `mkdocs.yml` to `mkdocs.local.yml` and set:

    ```yaml
    site_url: http://127.0.0.1:8000/
    ```

  - then build with the local config (how to specify the config file depends on your Zensical version)

- Alternatively, serve the site under the same prefix (i.e., mount it at `/podCI/`).

## CI integration

podCI does not ship a CI workflow for the docs site. The supported build entrypoint is:

```bash
./scripts/build_docs.sh
```

Environment knobs:

- `PODCI_SKIP_RUSTDOC=1` skips regenerating rustdoc (useful for Markdown-only changes):

  ```bash
  PODCI_SKIP_RUSTDOC=1 ./scripts/build_docs.sh
  ```

### Why `build_docs.sh` has a heuristic

Zensical’s compatibility layer does not currently provide a strict “fail on broken links/config” mode comparable to MkDocs’ `--strict`. To avoid quietly producing a broken site, `scripts/build_docs.sh` fails the build if it detects:

- emitted `Error:` lines
- Python tracebacks

When Zensical provides reliable strict validation and exit codes, the heuristic should be removed.

## Version pinning

The docs toolchain is pinned in `docs/requirements.txt`.

Future work: once Python support lands for podCI workloads (see Roadmap), adopt `uv` for a fully locked dependency set for the docs toolchain.
