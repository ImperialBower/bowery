# EPIC-110: bowery — the `<bowery-table>` control

> **One-line:** The work that builds this crate. The contract lives in bower
> [EPIC-20](https://github.com/ImperialBower/bower/blob/main/docs/EPIC-20_Bowery.md)
> (and [EPIC-21](https://github.com/ImperialBower/bower/blob/main/docs/EPIC-21_Bowery_Block.md)
> for the bower block). This page is a pointer, as pkarena0-web's EPIC-56 was
> to bower EPIC-19. Design, work items and verification are not repeated here.

> **Numbering:** pkcore's `ROADMAP.md` registers EPIC-110 through EPIC-119 for
> bowery (registered 2026-10-19). This is the first; 111–119 are free.

## Status

Phase 0 in progress. Phase numbers and item letters are EPIC-20's.

| Component | Status |
|---|---|
| Phase 0 — repo, numbering, licence, toolchain, release skeleton | In progress — 0a, 0b, 0c done; 0d (wasm CI) landed, waiting for green CI |
| Phase 1 — `Config` and `check` | Planned |
| Phase 2 — `Game`: replay, takeover, bots, next hand | Planned |
| Phase 3 — `Frame` and `draw::svg`, three layouts, no-overlap proptest | Planned |
| Phase 4 — wasm surface and `<bowery-table>` | Planned |
| Phase 5 — Playwright: D1–D8, multi-instance, off-origin zero | Planned |
| Phase 6 — release `v0.1.0`: crates.io, npm, tarball | Planned |

### Phase 0 notes

- **0a.** Scaffolded from `/dev-playbook rust` (`8c9b9e2`): MIT OR
  Apache-2.0, edition 2024, `rust-version` 1.99.0 (pkcore's).
- **0b.** Range registered in pkcore `ROADMAP.md`; this file is the pointer.
- **0c.** `bowery` 0.0.0 on crates.io and `@imperialbower/bowery` 0.0.0 on
  npm, with README badges for both.
- **0d.** `make wasm` builds `--target wasm32-unknown-unknown --features wasm`
  and fails if `cargo tree -e normal` holds wasm-bindgen. It runs in
  `make ayce` and as the `Wasm` job in `.github/workflows/ci.yaml`. The
  `wasm` feature is empty until Phase 4. `cargo test` and clippy
  `-D warnings` were already in CI.
