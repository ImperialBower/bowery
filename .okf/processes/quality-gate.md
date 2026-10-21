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
4. **wasm** — `cargo build --target wasm32-unknown-unknown --features wasm`,
   then fails if `cargo tree -e normal` holds wasm-bindgen: the rlib that
   bower links must stay free of browser crates. Needs
   `rustup target add wasm32-unknown-unknown` (`make install-tools` does it).
5. **test** — `cargo test`: unit, integration and doc tests.
6. **lint** — `cargo clippy --all-targets -- -D warnings` (pedantic, set in
   `Cargo.toml`), then `cargo deny check`.
7. **security-scan** — `./bin/security-scan`: `cargo audit` and
   `cargo deny check advisories`.
8. **docs** — `cargo doc --no-deps` with `RUSTDOCFLAGS="-D warnings"`.

Green means every phase exits 0. No work is done until it is.

`make security-scan` runs on its own too. `.github/workflows/security.yaml`
runs the same script daily, because new advisories appear every day;
`.github/workflows/ci.yaml` runs tests on stable, beta and the MSRV, plus
wasm, clippy, fmt, doc and deny, on every push and monthly.

`bin/security-scan` must be committed. A global git excludes file that
ignores `bin` directories hides it, and then the Security workflow fails
with exit 127; the repo `.gitignore` re-includes `/bin/` for that reason.
