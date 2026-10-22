# EPIC-110: bowery — the `<bowery-table>` control

> **One-line:** The work that builds this crate. The contract lives in bower
> [EPIC-20](https://github.com/ImperialBower/bower/blob/main/docs/EPIC-20_Bowery.md)
> (and [EPIC-21](https://github.com/ImperialBower/bower/blob/main/docs/EPIC-21_Bowery_Block.md)
> for the bower block). This page is a pointer, as pkarena0-web's EPIC-56 was
> to bower EPIC-19. Design, work items and verification are not repeated here.

> **Numbering:** pkcore's `ROADMAP.md` registers EPIC-110 through EPIC-119 for
> bowery (registered 2026-10-19). This is the first; 111–119 are free.

## Status

Phases 0 and 1 complete; Phase 2 landed, waiting for green CI. Phase numbers and item letters are EPIC-20's.

| Component | Status |
|---|---|
| Phase 0 — repo, numbering, licence, toolchain, release skeleton | Done — 0a–0d |
| Phase 1 — `Config` and `check` | Done — 1a–1c |
| Phase 2 — `Game`: replay, takeover, bots, next hand | Done — 2a–2d; waiting for green CI |
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

### Phase 1 notes

- **1a.** `Config` (`src/config.rs`): serde, `deny_unknown_fields`,
  `from_yaml` / `to_yaml`, round-trip test. EPIC-20 left `Moment`,
  `TableSetup`, `PlayerSetup` and `Controls` unspecified; their shapes are
  in the rustdoc. `Controls` holds the two toggles EPIC-20 names (👁 view,
  ▦ layout).
- **1b.** `check` (`src/check.rs`): 20 fixtures under `tests/config/`, one
  rule each. Rules beyond EPIC-20's list, because play cannot start without
  them: play needs `table.stakes` and a stack per player; a seat is not taken
  twice; a seat is inside `table.seats`.
- **1c.** `fixtures/the_hand.yaml` (pkcore `v0.17.2`, byte for byte) checks
  clean. The pre-DEFECT_040 copy fails with hand findings whose rules are
  exactly {Seats, Result, Money}.
- pkcore enters at 0.17.2 (EPIC-20 says 0.17.1; 0.17.2 is the current
  release and the same API).

### Phase 2 notes

- **2a.** Replay: the moments are the cursor's events, one for one, each with
  the cursor's table (`game__replay_frames_are_cursor_events`). Phase 2 has
  no `Frame` yet: `Game::event()` and `Game::table()` are what Phase 3's
  `Frame` is built from.
- **2b.** Interactive: `take_seat`, an action that matches the record stays
  on it, any other goes off it; `back_to_record`. Negreanu taken at the flop,
  checking and calling: the river is 8♠.
- **2c.** Play: chairs, stacks, blinds, ante, `deck` for hand 1, seeded bots
  and shuffles, `next_hand` (button to the next seated player, stacks carry
  over, busted players leave).
- **2d.** D1–D5 and the Return rule, each tested on the record and off it.
- **The end of a hand off the record.** pkcore's settlement (return, showdown,
  awards) is private to its cursor. bowery rebuilds the hand as a hand
  history from the table's own event log when it ends and reads those
  moments from pkcore's cursor, so it computes no settlement of its own.
  That needed pkcore 0.17.3: before it, a replayed table did not log the
  streets it laid (pkcore DEFECT_041, found here and fixed upstream). The
  log keeps a flop as a set, so the rebuilt flop takes its deal order from
  the table's board.
- **API changes from EPIC-20's sketch.** `new`, `seek`, `advance` return
  `Result`; `frame()` waits for Phase 3; `at()`, `event()`, `table()`,
  `reader()`, `record_state()`, `hand_number()` are added. Seats with no
  `bot` other than the reader's are played by pkcore's `gto` profile.
- **A check rule added:** play's stakes are whole chips
  (`tests/config/stakes_not_whole.yaml`).
- `rand` 0.9 (`small_rng`) and `uuid` join the dependencies; both are
  pkcore's versions. pkcore goes to 0.17.3.
