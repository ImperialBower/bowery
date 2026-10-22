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
- `bin/security-scan` committed (a global excludes file hid it; the Security
  workflow failed on every push).

## EPIC-20 — open work

### Phase 2 — Game

- [ ] **2a.** Replay mode over the cursor; `frame()` per event.
- [ ] **2b.** Interactive: `take_seat`, `back_to_record`, stacked deck.
- [ ] **2c.** Play: bots, `next_hand`, button moves, stacks carry over.
- [ ] **2d.** D1–D5 as kernel tests.

### Phase 3 — Frame and draw

- [ ] **3a.** `Frame` with `View` applied at build time.
- [ ] **3b.** `geometry.rs`; disjoint-boxes proptest (D6, D7).
- [ ] **3c.** `draw::svg` for ring, spotlight, ledger; 24 golden SVGs.
- [ ] **3d.** `bowery-still` example binary.

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

None tracked. See [docs/TECHNICAL_DEBT.md](docs/TECHNICAL_DEBT.md).

## Issues

No open GitHub issues.
