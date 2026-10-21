---
type: process
title: Quality gate
description: What make ayce runs, what green means, and where security scanning runs.
tags: [quality, ci, make]
timestamp: '2026-10-21T22:36:56Z'
---

# Quality gate

`make ayce` (the default target) runs, in order:

1. **clean** — `cargo clean`.
2. **fmt** — `cargo fmt`.
3. **build** — `cargo build`.
4. **test** — `cargo test`: unit, integration and doc tests.
5. **lint** — `cargo clippy --all-targets -- -D warnings` (pedantic, set in
   `Cargo.toml`), then `cargo deny check`.
6. **security-scan** — `./bin/security-scan`: `cargo audit` and
   `cargo deny check advisories`.
7. **docs** — `cargo doc --no-deps` with `RUSTDOCFLAGS="-D warnings"`.

Green means every phase exits 0. No work is done until it is.

`make security-scan` runs on its own too. `.github/workflows/security.yaml`
runs the same script daily, because new advisories appear every day;
`.github/workflows/ci.yaml` runs tests on stable, beta and the MSRV, plus
clippy, fmt, doc and deny, on every push and monthly.
