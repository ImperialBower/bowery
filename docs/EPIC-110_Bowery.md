# EPIC-110: bowery — the `<bowery-table>` control

> **One-line:** The work that builds this crate. The contract lives in bower
> [EPIC-20](https://github.com/ImperialBower/bower/blob/main/docs/EPIC-20_Bowery.md)
> (and [EPIC-21](https://github.com/ImperialBower/bower/blob/main/docs/EPIC-21_Bowery_Block.md)
> for the bower block). This page is a pointer, as pkarena0-web's EPIC-56 was
> to bower EPIC-19. Design, work items and verification are not repeated here.

> **Numbering:** pkcore's `ROADMAP.md` registers EPIC-110 through EPIC-119 for
> bowery (registered 2026-10-19). This is the first; 111–119 are free.

## Status

**Complete.** v0.1.0 released 22 October 2026 on crates.io, npm and GitHub. Phase numbers and item letters are EPIC-20's.

| Component | Status |
|---|---|
| Phase 0 — repo, numbering, licence, toolchain, release skeleton | Done — 0a–0d |
| Phase 1 — `Config` and `check` | Done — 1a–1c |
| Phase 2 — `Game`: replay, takeover, bots, next hand | Done — 2a–2d |
| Phase 3 — `Frame` and `draw::svg`, three layouts, no-overlap proptest | Done — 3a–3d |
| Phase 4 — wasm surface and `<bowery-table>` | Done — 4a–4c |
| Phase 5 — Playwright: D1–D8, multi-instance, off-origin zero | Done — 5a–5d |
| Phase 6 — release `v0.1.0`: crates.io, npm, tarball | Done — 6a–6c: v0.1.0 on crates.io, npm (with provenance) and GitHub; pkarena0-web's EPIC-56 superseded |

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

### Phase 3 notes

- **3a.** `Frame` (`src/frame.rs`), built by `Game::frame(view)`. The view is
  applied while the frame is built: in a `seat` view other seats' cards are
  `CardFace::Down` until a showdown (`frame__seat_view_hides_other_cards`).
  Beyond EPIC-20's sketch the frame carries `view` and `street`, and each seat
  its `last` action on the street, which styles its bet slot. `Game::view()`
  gives the mode's default view.
- **3b.** `draw::geometry::plan` places every box before anything is drawn.
  Boxes are sized for their widest text: a 16-character name, an
  11-character chip count (`format_chips` shortens from a billion: `4.29B`).
  The proptest (`geometry__boxes_disjoint`, 2,000 cases) covers all three
  layouts, 2–10 seats, folded strips and widths 320–1,400 px; breaking the
  ring's spacing on purpose made it fail.
- **Ring falls back to ledger** below 520 px, as EPIC-20 says, and also where
  its seats do not fit: from 574 px it has a column of seats on each side, so
  ten seats fit in the 680 px book column, but 9–10 seats do not fit between
  520 and 573 px.
- **3c.** `draw::svg` (`src/draw/mod.rs`): no script, no CSS, nothing hidden,
  colours as attributes, classes for the page to restyle. One renderer serves
  all three layouts instead of `ring.rs`, `spotlight.rs` and `ledger.rs`;
  the layouts differ only in geometry and in how a seat is drawn (plate or
  row). 24 golden SVGs of The Hand (preflop, flop, river, award × 3 layouts ×
  light and dark) in `tests/golden/`, compared byte for byte;
  `BOWERY_BLESS=1 cargo test --test draw` rewrites them for review.
- **3d.** `examples/bowery-still.rs` takes a config or a bare hand history:
  `cargo run --example bowery-still -- fixtures/the_hand.yaml --layout spotlight --at river`
  prints the same SVG as the golden file.
- The mockup (`docs/EPIC-20_Bowery_mockup.html`) is now copied into bowery,
  as Phase 0a asked.
- `Moment`, `Layout` and `View` parse from text (`FromStr`), for
  `bowery-still` and for Phase 4's `set_layout` and `set_view`.
- The `make wasm` gate caught a 32-bit overflow in `format_chips` (a
  trillion does not fit in wasm32's `usize`); it counts in `u64`.

### Phase 4 notes

- **4a.** The surface is split in two, both behind the `wasm` feature:
  `src/element.rs` is the element's logic in plain Rust (state as JSON,
  actions from JSON), tested natively in `tests/element.rs`; `src/wasm.rs`
  wraps it with wasm-bindgen and is the only wasm-bindgen code. `serde_json`
  and `wasm-bindgen` are optional, so `make wasm`'s tree check still finds
  no wasm-bindgen in the default build. `crate-type = ["rlib", "cdylib"]`.
- **Size.** The size profile (`opt-level = "z"`, `lto`, `codegen-units = 1`,
  `wasm-opt -Oz`) gives **1,295,901 bytes** raw (506,549 gzip, 373,214
  brotli), 40% under pkarena0-web's 2,156,115. The gate is that plus 10%:
  **1,425,491** (`WASM_GATE` in the `Makefile`; `make size`, run by
  `make ayce` and by CI's new `Browser build` job).
- **API beyond EPIC-20's sketch.** `shuffle(seed)` (the element passes a
  random seed; the kernel has no entropy), `advance()` returns a `Result`,
  and `state()` carries `controls` (which buttons to show), `seats` (for
  **Take this seat**), `legal` with sizes, `hand_over`, `delay_ms`. The
  👁 view toggle defaults to off in play, on otherwise.
- **File names.** The glue is `bowery_wasm.js` and `bowery_wasm_bg.wasm`, not
  `bowery_bg.wasm`, because the element itself is `bowery.js`.
- **4b.** `js/bowery.js` and `js/bowery.css`: lazy (`IntersectionObserver`),
  one module per page shared by every element, the fallback kept with the
  problems printed under it, `file://` refused, controls by mode, ← → Home
  End, the five events, dark from `prefers-color-scheme` or mdBook's
  classes, redrawn on resize, bots paced by `delay_ms`. `src` changes reload
  the table.
- **4c.** `demo/index.html`: The Hand in replay, interactive and play, and a
  nine-seat bot table configured inline: four tables on one page.
  `make site` assembles it in `site/` with fallback stills from
  `bowery-still`; `make serve` serves it.
- **Smoke-tested in Chromium** before Phase 5 (a throwaway Playwright script):
  four tables ready, one origin, no console errors, interactive goes off the
  record at moment 11, play reaches the reader's turn. It found one bug: the
  ⏮ ◀ ▶ ⏭ buttons had no accessible name (now `aria-label`).

### Phase 5 notes

- **Where.** `tests/e2e/`, with its own `package.json` pinning
  `@playwright/test` 1.59.1 (pkarena0-web's, so its browsers are reused); the
  repo root's `package.json` is left for the npm package (Phase 6).
  `make e2e` builds `site/`, adds `site/e2e/` (the pages in
  `tests/e2e/pages/` and configs generated from `tests/hands/` and
  `fixtures/`), and runs Chromium. `make ayce` and CI's `Browser proof` job
  run it.
- **5a.** `d1`–`d5.spec.ts` step through the all-in hand frame by frame on
  the record (a replay in seat 1's view) and off it (seat 1 calls where the
  record shoves, and the game runs out and pays by itself). `d6`/`d7` check
  every frame of three hands at a nine-seat table in ring and spotlight at
  680 px and at 360 px; the check runs in the page on each `bowery:moment`,
  measuring with the browser's fonts. `d8` reads the pot at The Hand's
  re-raise: `POT 8,800`, with 2,100 and 5,000 at their seats.
- **5b–5d.** `multi.spec.ts` (three replays and three play tables; acting in
  one leaves the others' SVG byte-identical), `origin.spec.ts` (one origin),
  `fallback.spec.ts` (script off; `file://`, where the browser refuses the
  module, so the fallback stays and nothing throws).
- **Failing first.** D1–D5 failed first in the kernel (Phase 2,
  `tests/game.rs`); in the browser they passed on their first run. D6–D8 were
  shown to fail against deliberately broken builds (bet slots moved into the
  plates, card backs drawn on folded seats, the pot without live bets). The
  first D6/D7 draft sampled frames on a timer and missed the broken build;
  the check moved into the page, on every frame.
- **Found a real bug.** The first frame was measured before `bowery.css`
  arrived, while the element was still inline at width 0, so a phone-width
  table drew its first frame as a 680 px ring. An inline
  `:host { display: block }` fixes it; `d6: a phone-width ring draws as
  ledger from the first frame` fails on the old element.
- The board is now one `<g class="board">` (the 24 golden SVGs changed by
  that wrapper only), so tests can find it.

### Phase 6 notes

- **6a, prepared.** `scripts/dist.sh` writes `dist/bowery/` (`make dist`),
  which is both the npm package and the tarball's contents, with
  `MANIFEST.sha256`. Before writing anything it refuses: a `Cargo.toml` and
  `npm/package.json` version that differ, a tag that is not that version,
  a wasm over `WASM_GATE`, and any `@import` or `url(` in `bowery.css`.
  `.github/workflows/release.yml` runs on a `v*` tag: build and check, then
  `cargo publish`, then `npm publish --provenance`, then the GitHub release
  with `bowery-vX.Y.Z.tar.gz` and the CHANGELOG section as notes, each only
  after the one before. There is no manual re-run: a failed publish is
  fixed with a new patch version.
- **Versions at 0.1.0** in `Cargo.toml` and `npm/package.json`;
  `CHANGELOG.md` has the `[0.1.0]` section. The crate excludes the element,
  the demo and the browser proof (`exclude` in `Cargo.toml`); they ship on
  npm and in the tarball. `cargo publish --dry-run` and `npm pack --dry-run`
  pass: 83 files (88.5 KiB compressed) and 9 files (534.6 kB).
- **6b and 6c** wait for the tag, which needs the `CARGO_REGISTRY_TOKEN`
  repository secret. npm uses trusted publishing (OIDC): npmjs.com trusts
  `release.yml` in this repository, so there is no npm token to store or
  leak, and provenance is attached automatically.
- **6b, released.** Tag `v0.1.0`. crates.io published on the first run.
  npm failed twice before it published: a 404 while no trusted publisher
  was set on npmjs.com, then "403 OIDC permission denied for this action"
  because the trusted publisher's "Allow `npm publish`" was not ticked (only
  `npm stage publish` was allowed). Both were fixed on npmjs.com, and
  `gh run rerun --failed` published the same build without running
  crates.io again. Checked afterwards: crates.io 0.1.0; npm 0.1.0 with
  provenance; the GitHub release's `bowery-v0.1.0.tar.gz`, all 8 files
  matching `MANIFEST.sha256`.
- **6c.** pkarena0-web: EPIC-56 marked superseded, its `release.yml` removed
  so later releases attach no `pk-table-*.tar.gz`, README and CHANGELOG
  updated; v0.2.1 and v0.2.2 keep their tarballs.
