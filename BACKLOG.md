# Backlog

> Maintained by the `/backlog` skill. Refreshed 2026-10-21. The contract is
> bower's [EPIC-20](https://github.com/ImperialBower/bower/blob/main/docs/EPIC-20_Bowery.md)
> (work items below are its numbering). Debt detail lives in
> [docs/TECHNICAL_DEBT.md](docs/TECHNICAL_DEBT.md).

## Recently completed

- **0a.** Repo scaffolded from `/dev-playbook rust` (`8c9b9e2`).
- **0b.** pkcore `ROADMAP.md` registers EPIC-110–119; `docs/EPIC-110_Bowery.md`
  points to bower's EPIC-20.
- **0c.** Names reserved: `bowery` 0.0.0 on crates.io, `@imperialbower/bowery`
  0.0.0 on npm; README badges for both.
- **0d.** `make wasm` in `make ayce` and CI: wasm32 build with `--features wasm`,
  no wasm-bindgen in the default tree.
- **1a–1c.** `Config` (`src/config.rs`) and `check` (`src/check.rs`): one
  failing fixture per rule under `tests/config/`; The Hand checks clean; the
  pre-DEFECT_040 copy fails with exactly {Seats, Result, Money}.
- **2a–2d.** `Game` (`src/game.rs`): replay over the cursor, takeover on and
  off the record, play with bots and `next_hand`, D1–D5 kernel tests on and
  off the record (`tests/game.rs`).
- **3a–3d.** `Frame` (`src/frame.rs`), `draw::geometry` with the
  disjoint-boxes proptest, `draw::svg` in ring, spotlight and ledger with 24
  golden SVGs (`tests/golden/`), and `examples/bowery-still.rs`.
- `bin/security-scan` committed (a global excludes file hid it; the Security
  workflow failed on every push).

## EPIC-20 — open work

### Phase 4 — wasm and the element

- [ ] **4a.** `BoweryTable` (`src/wasm.rs`); size gate.
- [ ] **4b.** `js/bowery.js`, `js/bowery.css`.
- [ ] **4c.** `demo/index.html`.

### Phase 5 — The proof

- [ ] **5a–5d.** Playwright: D1–D8, three tables on one page, same-origin
  requests only, script-off fallback.

### Phase 6 — Release

- [ ] **6a.** `release.yml`: tag → crates.io, npm, tarball with `MANIFEST.sha256`.
- [ ] **6b.** `v0.1.0`.
- [ ] **6c.** pkarena0-web: retire EPIC-56.

## Tech debt

One item: `SmallRng`'s platform dependence. (The pkcore replay-log gap was
fixed upstream as pkcore DEFECT_041, 0.17.3.) See [docs/TECHNICAL_DEBT.md](docs/TECHNICAL_DEBT.md).

## Issues

No open GitHub issues.
