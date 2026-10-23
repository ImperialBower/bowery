# Technical Debt

> Maintained by the `/backlog` skill. Items tagged 🤖 were proposed by automated
> review — review and edit them; they are suggestions, not facts.

## Tracked debt
<!-- Human-authored and code-comment-sourced items. -->

- [ ] **`SmallRng` differs between 32- and 64-bit targets** — EPIC-20 names
  `SmallRng`, whose algorithm is platform-dependent, so a play table seeded
  the same way can deal differently in the browser (wasm32) and natively.
  Only play mode's bots and shuffles use it. (`src/game.rs`, `Game::new`)

## 🤖 Automated review findings
<!-- Machine-proposed. Promote good ones up to "Tracked debt", delete the rest. -->

Review pass of 2026-10-23 (v0.1.1, `2f9e919`), two reviewers: kernel
(`config`, `check`, `game`) and surface (`frame`, `draw`, `element`, `wasm`,
`js/`, release). Items marked ✔ were checked against the code by hand; the rest
are as the reviewer reported them.

### Likely bugs

- [x] 🤖 ✔ **Keyboard seek eats keys in the bet box and seat picker** — `_key`
  listens on the host, and shadow-DOM events are retargeted to the host, so
  `event.target !== this` never filters them. Arrow, Home and End in the
  `<input type=number>` and `<select>` seek moments instead. Suggested: use
  `event.composedPath()[0] === this`; add a Playwright keyboard test.
  (`js/bowery.js:309-322`, listener at `js/bowery.js:203`)
  **Fixed 2026-10-23** as suggested; `tests/e2e/keyboard.spec.ts`.
- [ ] 🤖 ✔ **`next_hand` is not atomic** (not observable) — it ends the hand, eliminates the
  busted, bumps `hand_number`, then can fail (`TableBroken`, or `deal`'s `?`)
  with `line`/`at` still on the old hand; a retry calls `end_hand` twice.
  Suggested: do the fallible steps first; commit `hand_number` and `line` only
  after `deal` succeeds. (`src/game.rs:1087-1125`)
  **Checked 2026-10-23:** a second call after `TableBroken` gives
  `TableBroken` again with the table, moments and hand number unchanged
  (`game__next_hand_on_a_broken_table_changes_nothing`), and `deal` cannot
  fail on a checked config. The ordering is still fragile; reorder it if
  `deal` ever gains a reachable error.
- [x] 🤖 ✔ **`table:` override is documented but never applied** — `Config`
  says `table` overrides a recorded hand's table, but `open_record` never reads
  it, while `check` still validates it: a false "checked". Suggested: apply it,
  or have `check` reject `table.button`/`table.stakes` alongside a `hand`; fix
  the doc either way. (`src/config.rs:63-65`, `src/game.rs:1201-1232`)
  **Fixed 2026-10-23:** `check` refuses `table` beside a recorded hand; docs
  say play only. Fixture `replay_with_table`.
- [x] 🤖 ✔ **`check` misses play-table rules** — no button rule (past the
  chairs or on an empty chair), zero or inverted blinds, zero stacks. A checked
  config can still hit `Setup`/`Engine`, against `Game::new`'s promise.
  Suggested: `check` rules for button in range, `big_blind >= small_blind > 0`,
  `stack > 0`. (`src/check.rs:189-233`, `src/game.rs:1348`)
  **Fixed 2026-10-23:** button off the table, a zero blind and a zero stack
  each broke the game (probed); `big_blind < small_blind` played but is
  refused too. A button on an empty chair plays fine and stays allowed.
  Fixtures `play_button_off_the_table`, `play_blind_zero`,
  `play_blinds_inverted`, `play_stack_zero`.
- [ ] 🤖 **Chair-count rule skipped in play without `table.seats`** — `players:
  [{seat: 200}, {seat: 1}]` builds a 201-chair table; seat 255 passes `check`
  and fails in `play_table`. Suggested: derive the implicit chair count in
  `seats()` and apply the 2–10 rule. (`src/check.rs:430-434`, `src/game.rs:1327`)
- [ ] 🤖 **`back_to_record` keeps the advanced RNG** — taking over again at
  the same moment lets bots play differently, breaking "same config, same
  game". Suggested: snapshot or re-seed the RNG per takeover; add a test.
  (`src/game.rs:900-907`)

### Error handling

- [ ] 🤖 **`settle` swallows rebuild errors** — two `unwrap_or_default()`
  calls turn a failure into "no streets", and a later, misleading error hides
  the cause. Suggested: return `GameError::Setup` with the real cause.
  (`src/game.rs:1481-1483`)
- [ ] 🤖 **JSON fallbacks hide failures** — `state_json` falls back to `"{}"`
  (JS then throws on `state.controls`), and `problems_json` to `"[]"` (an empty
  error box). Suggested: return `Result`, or an `{"error": …}` the JS checks.
  (`src/element.rs:327`, `src/element.rs:657`)
- [ ] 🤖 **`Result<_, String>` instead of error types** — `Table` methods
  flatten errors to text (one returns raw problems JSON, which lands in the
  aria-live status), and the `FromStr`/`TryFrom` impls in `config` use
  `String`. House rule: custom error types. Suggested: a `TableError` enum,
  converted to `JsValue` once in `wasm.rs`; a small parse error type in
  `config`. (`src/element.rs:425-616`, `src/config.rs:195,249,358,384`,
  `js/bowery.js:377`)
- [ ] 🤖 **Four JS handlers bypass `_try`** — `_toggleView`, `_toggleLayout`,
  `_seek` and `_back` let a thrown `JsValue` escape, skipping `_render` and
  `bowery:error`. Suggested: route them through `_try`.
  (`js/bowery.js:324-345,362-371`)
- [ ] 🤖 **Silent fallbacks in the game** — `default_seed` hashes `""` if YAML
  serialization fails; `filter_map(.. event(i).cloned())` drops missing events
  and shifts indices; `bot_act` silently swaps a refused bot action for
  check/call/fold. Suggested: fail with `Setup`, or at least flag the
  override. (`src/game.rs:1266`, `src/game.rs:1212-1214,1511-1513`,
  `src/game.rs:1283-1302`)

### Browser element

- [ ] 🤖 **Element lifecycle leaks and breaks on reconnect** —
  `disconnectedCallback` stops only the ResizeObserver: the MutationObserver,
  `matchMedia` and keydown listeners live on, and `free()` is never called.
  `_reload` re-adds every listener. A moved element keeps no timer or
  observer. Suggested: keep handles on `this`; tear down in
  `disconnectedCallback`/`_reload`, re-arm on connect. (`js/bowery.js:83-86`,
  `js/bowery.js:92-97`, `js/bowery.js:198-203`)
- [ ] 🤖 **Origin comment overstates the check** — only `src` is
  origin-checked (and `fetch` follows redirects); `base` may load glue and wasm
  from anywhere, which the CDN use needs. Suggested: correct the comment;
  check `response.url`'s origin. (`js/bowery.js:14-15,69,136-138`)

### Drawing

- [ ] 🤖 **Width from JS is unbounded `u32`** — large widths overflow the
  geometry arithmetic (panic in debug, garbage in release); the proptest
  covers 320–1400 only. Suggested: clamp in `plan`, or use saturating/`u64`
  math; add a `u32::MAX` case. (`src/draw/mod.rs:172`,
  `src/draw/geometry.rs:273,303,313`)
- [ ] 🤖 **Names: XML-illegal characters and wide glyphs** — `escape` handles
  all five entities, but keeps U+0000–001F, which strict XML readers (the book
  path) reject. `cut` counts `char`s, so CJK or emoji names overflow plates.
  Suggested: map control characters to U+FFFD; width-aware cut or a `check`
  rule on names. (`src/draw/mod.rs:677-701`)

### Release and supply chain

- [ ] 🤖 ✔ **`actions/*` unpinned in jobs that publish** — `checkout@v7`,
  `upload-artifact@v4`, `download-artifact@v4`, `setup-node@v4` are mutable
  tags in jobs with `id-token: write`. Third-party actions are already pinned.
  Suggested: pin to SHAs with a version comment.
  (`.github/workflows/release.yml:53,71,88,107,110,129,130`)
- [ ] 🤖 **Floating toolchain in the release build** — `stable` and an
  unpinned `wasm-pack` make the published wasm depend on the tag day.
  Suggested: pin both, or record them in the manifest.
  (`.github/workflows/release.yml:55-60`)
- [ ] 🤖 **`dist.sh` skips the tag gate on bad args** — `--tag` with no value,
  or any other second argument, skips the tag-vs-version check silently. CI
  is fine; local runs are not. Suggested: strict arg parsing, `die` on unknown.
  (`scripts/dist.sh:19,38`)

### House rules and tests

- [ ] 🤖 **Lints do not enforce the error rules** — only clippy `all` and
  `pedantic`. Suggested: `unwrap_used`, `expect_used`, `panic`,
  `indexing_slicing` as `warn`, allowed in tests. (`Cargo.toml:54-56`)
- [ ] 🤖 **One panicking index in library code** — `self.line[..]` in `step()`
  holds only by the "line is never empty" invariant. Suggested: `.get()` or
  keep the first step as its own field. (`src/game.rs:1129`)
- [ ] 🤖 **Strings for fixed value sets** — `Legal.action`, `State.record`,
  `Table::advance()` ("dealt"/"acted"/"waiting"/"over") and `ActRequest.action`.
  Suggested: enums with `serde(rename_all = "snake_case")`.
  (`src/element.rs:85,117,125,469-486,508`)
- [ ] 🤖 **`Moment` does not round-trip** — `Event(3)` displays as `event 3`,
  which `from_str` rejects; YAML `start: "7"` fails while `"7".parse()` works.
  (`src/config.rs:352,367`)
- [ ] 🤖 **`deck_fault` re-tokenizes the deck** — its own separators and
  `DECK_SIZE` can disagree with pkcore's parser and give odd messages
  ("has 1 cards", "`` is not a card"). Suggested: count from the parsed
  `Cards`. (`src/check.rs:400-424`) **Note:** the suggested fix is wrong as
  written: `Cards::from_str` drops a repeated card silently, which is why the
  rule counts tokens (`.okf/concepts/config-and-check.md`). Only the odd
  messages are real.
- [ ] 🤖 **Single-letter names** — `Rect { x, y, w, h }` and `w`/`h` locals.
  Suggested: `width`/`height` at least. (`src/draw/geometry.rs:82-91`,
  `src/draw/mod.rs:366,486,557`)
- [ ] 🤖 **Test gaps** — no tests for `GameError::{TableBroken, NoReader,
  Illegal, Engine, Setup, HandIsOver}`, `SeatNotLive` on an empty seat,
  `next_hand` with busted players, `act` in replay, or `impl Display for
  Frame` (`src/frame.rs:340`); no `wasm-bindgen-test` for `wasm.rs` error
  paths; geometry proptest never tries 0–1 seats.

### Clean

Both reviewers found no `unwrap`/`expect`/`panic!` in library code, no
overflow in the chip arithmetic, no re-implementation of pkcore, no untrusted
`innerHTML` (all user text goes through `escape` or `textContent`), no float
math, and a doc test on every public item in scope.
