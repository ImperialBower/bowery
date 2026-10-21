---
type: decision
title: Toolchain
description: Which tools bowery's gate uses, at which versions, and why.
tags: [toolchain, rust, quality]
timestamp: '2026-10-21T22:36:56Z'
---

# Toolchain

| Tool | Version | Why |
|---|---|---|
| Rust | 1.99.0 (MSRV; `.tool-versions`) | bowery links pkcore, whose `rust-version` is 1.99.0; the MSRV tracks it. Edition 2024. |
| rustfmt | ships with Rust | One format, no debate; `max_width = 100`. |
| clippy (pedantic) | ships with Rust | Pedantic catches API-design and idiom issues `clippy::all` misses; a few justified `#[allow]`s cost less than the bugs it prevents. |
| cargo-deny | installed via `make install-tools` | One tool for licences, banned or duplicate crates, advisories and source allow-listing. |
| cargo-audit | installed via `make install-tools` | The dedicated RustSec client; runs beside `cargo deny check advisories` because their data paths and cadences differ. |
| cargo-watch | installed via `make install-tools` | Powers `make watch`, the rebuild-on-save loop. Not part of the gate. |
| rustdoc, warnings as errors | ships with Rust | `RUSTDOCFLAGS="-D warnings"` keeps broken intra-doc links and doc debt from piling up. |
| miri | not used | Off: bowery has no `unsafe`, and miri is slow. |
