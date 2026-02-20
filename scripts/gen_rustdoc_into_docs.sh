#!/usr/bin/env bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2026 Richard Majewski - Varanid Works

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

mkdir -p docs/rustdoc
rm -rf docs/rustdoc/api
mkdir -p docs/rustdoc/api

cargo doc --workspace --all-features --no-deps
cp -a target/doc/* docs/rustdoc/api/

echo "rustdoc copied to docs/rustdoc/api/"

# Provide a friendly redirect for direct navigation to docs/rustdoc/api/index.html
# (useful when browsing the built site outside of MkDocs routing).
cat > docs/rustdoc/api/_podci_redirect.html <<'HTML'
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2026 Richard Majewski - Varanid Works -->
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta http-equiv="refresh" content="0; url=index.html" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>podCI Rust API docs</title>
    <link rel="icon" href="../../assets/favicon.png" />
    <style>
      body{font-family:system-ui,-apple-system,Segoe UI,Roboto,Ubuntu,Helvetica,Arial,sans-serif;margin:2rem;}
      .wrap{max-width:56rem;margin:0 auto;}
      .logo{display:flex;align-items:center;gap:1rem;margin-bottom:1rem;}
      .logo img{width:56px;height:56px;}
    </style>
  </head>
  <body>
    <div class="wrap">
      <div class="logo">
        <img src="../../assets/logo.webp" alt="podCI" />
        <h1>podCI Rust API docs</h1>
      </div>
      <p>Redirecting to <a href="index.html">index.html</a>…</p>
      <p>If the redirect does not work, open the link above.</p>
    </div>
  </body>
</html>
HTML

