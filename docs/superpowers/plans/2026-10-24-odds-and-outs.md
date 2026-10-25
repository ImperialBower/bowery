# Odds and Outs Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Every live seat shows its win percent, and with two seats left and one card to come an outs bar lists the trailing seat's out cards, as a toggle the reader can switch off and the author can drop.

**Architecture:** A new kernel module `src/odds.rs` asks pkcore's equity engine and case evaluator and returns an `Odds` value the `Frame` carries as `frame.odds: Option<Odds>`. `draw::geometry` reserves a box for the percent inside each plate and a bar under the pot; `draw` renders both as classed SVG. `element::Table` owns the on/off switch, computes odds only when the view is `all`, and memoises the result per moment.

**Tech Stack:** Rust 2024 (MSRV in `.tool-versions`), pkcore 0.17.3 (`equity` feature, already on), wasm-bindgen behind the `wasm` feature, Playwright 1.59.1 for the browser proof.

**Spec:** `docs/superpowers/specs/2026-10-24-odds-and-outs-design.md`

## Global Constraints

- `make ayce` passes before any task is called done (`CLAUDE.md`). It includes `make size`: the wasm must stay at or under `WASM_GATE` in the `Makefile`.
- Never run a git command that changes state. Every commit step below hands the exact command to the user to run (`~/.claude/CLAUDE.md`).
- Library code has no `unwrap()`, `expect()` or `panic!()`. Every public item has a doc test and a unit test. Test names are `<thing>__<scenario>` in a `#[cfg(test)] #[allow(non_snake_case)] mod tests`.
- The renderer emits integer coordinates and attribute colours only: no CSS, no script, nothing hidden (`tests/draw.rs::draw__no_script_no_css_nothing_hidden`).
- Text metrics: 11 px monospace is 6.6 px a character (`geometry::SMALL_CHAR_TENTHS`); 18 px bold is 10.8 px a character. Every new box is sized for its widest text.
- Percents are whole numbers 0..=100. No decimal anywhere.
- No test, golden comparison in the browser, guide page or `.okf` note quotes a preflop percent. Native golden files may hold one.
- Odds and the bar never appear when any live seat's card is face down, when odds are off, when fewer than two seats are live, or once the hand has paid out.
- The outs bar never lists a card a folded seat holds, even though the percents count that card as live until pkcore EPIC-123 ships.

## Review Focus

1. **A live seat whose hole cards are `Card::BLANK`** (play mode between hands, or a record with no `hole_cards`): `odds::compute` must return `None`, not panic or send blanks to pkcore. Pinned in Task 2 by `odds__blank_hole_card_gives_none`.
2. **The ledger at 320 px with 44 outs**: the bar must wrap to several rows and nothing may overlap or leave the canvas. Pinned in Task 4 by the proptest's `outs` strategy spanning `0..=44` at `320..=1_400`.
3. **The seat view at a showdown**: every card is visible then, but the spec ties odds to the `all` view, not to visibility. Pinned in Task 6 by `element__odds_stay_off_in_seat_view_at_showdown`.
4. **Acting off the record after a cached moment**: the cache keyed by moment index must be cleared when the line changes, or a stale percent shows. Pinned in Task 6 by `element__odds_follow_an_action_off_the_record`.
5. **The `Return` moment with one seat left**: pkcore refuses fewer than two hands; the frame must show nothing rather than an error. Pinned in Task 2 by `odds__one_live_seat_gives_none`.

---

## File Structure

| File | Responsibility |
|---|---|
| `src/config.rs` | `Controls.odds: Option<bool>` (author drops the button) |
| `src/odds.rs` (new) | `Odds`, `OutsBar`, `compute`: the only code that talks to pkcore's equity engine and case evaluator |
| `src/frame.rs` | `Frame.odds: Option<Odds>`, `Frame::with_odds`, the Display column |
| `src/game.rs` | `Game::seed()` and `Game::frame_with_odds(view)` |
| `src/draw/geometry.rs` | `plan` takes the bar's card count; `Plan.outs`; the ring grows for the bar; constants `OUTS_LABEL_H`, `OUTS_ROW_H`, `ODDS_W` |
| `src/draw/mod.rs` | the percent on a plate and a ledger row; `draw_outs` |
| `src/element.rs` | `Table.odds`, `State.odds`, `Controls.odds`, `set_odds`, the per-moment cache |
| `src/wasm.rs` | `set_odds` wrapper |
| `src/lib.rs` | module link and a quick-start line |
| `js/bowery.js`, `js/bowery.css` | the `%` button; `.odds` and `.outs` rules |
| `examples/bowery-still.rs` | `--odds` |
| `tests/hands/three_way_outs.yaml` (new) | a hand where the trailing seat has outs and a folded seat holds one of them |
| `tests/odds.rs` (new), `tests/frame.rs`, `tests/geometry.rs`, `tests/draw.rs`, `tests/element.rs` | integration tests |
| `tests/golden/*-turn-odds-*.svg` (12 new) | golden files |
| `tests/e2e/pages/outs.html` (new), `tests/e2e/odds.spec.ts` (new), `Makefile` | browser proof |
| `demo/guide/*.html`, `.okf/concepts/frame-and-draw.md`, `.okf/concepts/element.md`, `.okf/log.md`, `docs/EPIC-110_Bowery.md`, `CHANGELOG.md` | docs |

---

### Task 1: The config flag

**Files:**
- Modify: `src/config.rs:489-496`
- Test: `src/config.rs` tests module

**Interfaces:**
- Produces: `config.controls.odds: Option<bool>`, serde default `None`, skipped when `None`.

- [ ] **Step 1: Write the failing test** in `src/config.rs`'s tests module:

```rust
#[test]
fn controls__odds_reads_and_round_trips() {
    let config = Config::from_yaml("mode: play\ncontrols: { odds: false }\n").unwrap();
    assert_eq!(Some(false), config.controls.odds);
    assert_eq!(None, Config::from_yaml("mode: play\n").unwrap().controls.odds);
    let again = Config::from_yaml(&config.to_yaml().unwrap()).unwrap();
    assert_eq!(config, again);
}
```

- [ ] **Step 2: Run it to see it fail**

Run: `cargo test --features wasm controls__odds_reads_and_round_trips`
Expected: FAIL, no field `odds` on `Controls`.

- [ ] **Step 3: Add the field** to `Controls` at `src/config.rs:489`, after `layout`, with the same serde attributes as `view`, documented: "The `%` odds toggle. Default: shown when the view is `all`."

- [ ] **Step 4: Run the test and the doc tests**

Run: `cargo test --features wasm config`
Expected: PASS.

- [ ] **Step 5: Hand the commit to the user**

```bash
git add src/config.rs && git commit -m "config: controls.odds" -m "Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 2: The percents

**Files:**
- Create: `src/odds.rs`, `tests/odds.rs`, `tests/hands/three_way_outs.yaml`
- Modify: `src/frame.rs:60-84` (field), `:250-338` (`with_odds`), `:340-365` (Display); `src/game.rs:357-368` (field), `:414-440` (store the seed), `:603-605` (sibling); `src/lib.rs` (module and overview)

**Interfaces:**
- Produces:
  - `pub struct Odds { pub percents: Vec<Option<u8>>, pub outs: Option<OutsBar> }` in `src/odds.rs`; `percents` is aligned with `frame.seats`. Derives `Clone, Debug, PartialEq, Eq`.
  - `pub struct OutsBar { pub seat: u8, pub name: String, pub percent: u8, pub cards: Vec<Card> }`; `cards` empty means drawing dead. Derives `Clone, Debug, PartialEq, Eq`. `Display`: `Gus Hansen · 8 outs · 18%`, or `Gus Hansen · drawing dead · 0%`.
  - `pub fn compute(table: &pkcore::casino::table::Table, seats: &[SeatFrame], board: &[Option<Card>; 5], paid: bool, seed: u64) -> Option<Odds>`. In this task `outs` is always `None`; Task 3 fills it.
  - `Frame.odds: Option<Odds>`; `Frame::new` sets `None`; `pub fn with_odds(game: &Game, view: View) -> Frame`.
  - `Game::seed(&self) -> u64`; `Game::frame_with_odds(&self, view: View) -> Frame`.
- Consumes: `pkcore::analysis::equity::{EquityOptions, EquityRequest, PlayerSpec}`, `pkcore::arrays::two::Two::new`, `pkcore::play::board::Board` (`TryFrom<Cards>` at pkcore `play/board.rs:213`, or `FromStr`), `PlayerEquity::equity_pct`.

- [ ] **Step 1: Write the fixture** `tests/hands/three_way_outs.yaml`, modelled on `tests/hands/heads_up_all_in.yaml`: `pkcore_version: "0.17.3"`, three seats, button 0, NL 50/100; seat 0 `Draw` 1000 `8♠ 7♠`; seat 1 `Set` 1000 `6♥ 6♦`; seat 2 `Gone` 1000 `T♠ 3♣`; board `9♣ 6♠ K♥ 2♦ 5♣`; preflop actions seat 0 `all_in`, seat 1 `all_in`, seat 2 `fold`; flop, turn, river with no actions. A header comment says what it is for: at the turn seat 0 trails with eight outs, and seat 2's folded `T♠` is one of them. Check it reads:

Run: `cargo run -q --example bowery-still -- tests/hands/three_way_outs.yaml --at turn | head -c 200`
Expected: an `<svg` with `aria-label="Turn · 2♦"`. If pkcore's `validate` refuses the action order, put the fold first and re-run until it reads; the comment records the final order.

- [ ] **Step 2: Write the failing tests** in `tests/odds.rs`. Reuse `tests/frame.rs`'s `game`, `the_hand`, `frame_at` and `seat` helpers by copying them (integration tests cannot share modules). Constants `HANSEN = 3`, `NEGREANU = 4`. A helper `turn_of(game: &Game) -> usize` finds the index with `Moment::Turn.index_in(&replay)` as `tests/draw.rs::drawn` does.

```rust
#[test]
fn odds__every_live_seat_has_a_percent_on_the_flop() {
    let mut game = the_hand("mode: replay\nstart: flop\n");
    let frame = Frame::with_odds(&game, View::All);
    let odds = frame.odds.expect("odds on");
    let hansen = odds.percents[index_of(&frame, HANSEN)].unwrap();
    let negreanu = odds.percents[index_of(&frame, NEGREANU)].unwrap();
    assert!(negreanu > hansen, "{negreanu} vs {hansen}");
    assert_eq!(100, u32::from(hansen) + u32::from(negreanu) , "two live seats share the pot");
    for (seat, percent) in frame.seats.iter().zip(&odds.percents) {
        assert_eq!(seat.status == SeatStatus::Folded, percent.is_none(), "{}", seat.name);
    }
    let _ = &mut game;
}

#[test]
fn odds__a_fold_changes_the_other_percents() {
    // Preflop, before and after Brunson's fold: Hansen's number moves.
    let mut game = the_hand("mode: replay\n");
    let before = frame_at(&mut game, 0, View::All, true);
    let folds = (1..game.moments())
        .find(|index| matches!(game.event_at(*index), Some(Event::Action { action: PlayerAction::Fold, .. })))
        .unwrap();
    let after = frame_at(&mut game, folds, View::All, true);
    let (before, after) = (before.odds.unwrap(), after.odds.unwrap());
    assert_eq!(before.percents.iter().flatten().count() - 1, after.percents.iter().flatten().count());
    let hansen = index_of(&frame_at(&mut game, 0, View::All, false), HANSEN);
    assert_ne!(before.percents[hansen], after.percents[hansen], "one hand fewer, a new share");
}

#[test]
fn odds__same_config_same_preflop_percents() {
    let first = Frame::with_odds(&the_hand("mode: replay\n"), View::All).odds;
    let second = Frame::with_odds(&the_hand("mode: replay\n"), View::All).odds;
    assert_eq!(first, second);
}

#[test]
fn odds__river_is_decided() {
    let mut game = the_hand("mode: replay\nstart: river\n");
    let frame = Frame::with_odds(&game, View::All);
    let odds = frame.odds.unwrap();
    assert_eq!(Some(100), odds.percents[index_of(&frame, HANSEN)]);
    assert_eq!(Some(0), odds.percents[index_of(&frame, NEGREANU)]);
    let _ = &mut game;
}

#[test]
fn odds__none_after_the_award() {
    let mut game = the_hand("mode: replay\n");
    let last = game.moments() - 1;
    assert_eq!(None, frame_at(&mut game, last, View::All, true).odds);
}

#[test]
fn odds__none_when_a_card_is_hidden() {
    let game = the_hand("mode: replay\nseat: 4\nstart: flop\n");
    assert_eq!(None, Frame::with_odds(&game, View::Seat).odds);
}

#[test]
fn odds__one_live_seat_gives_none() {
    // heads_up_fold: one seat folds, the other takes the pot back.
    let mut game = game("mode: replay\n", "tests/hands/heads_up_fold.yaml");
    let returned = (0..game.moments())
        .find(|index| matches!(game.event_at(*index), Some(Event::Return { .. })))
        .unwrap();
    assert_eq!(None, frame_at(&mut game, returned, View::All, true).odds);
}

#[test]
fn odds__blank_hole_card_gives_none() {
    // Play mode after the hand is over and before the next deal has no cards to count.
    let config = Config::from_yaml("
mode: play
table: { stakes: { small_blind: 1.0, big_blind: 2.0 } }
players: [ { seat: 0, stack: 200 }, { seat: 1, stack: 200, bot: abc } ]
").unwrap();
    let game = Game::new(check(&config).unwrap()).unwrap();
    let frame = Frame::with_odds(&game, View::All);
    // Both seats are dealt, so this is Some; the None case is pinned below.
    assert!(frame.odds.is_some());
    let mut blank = frame.clone();
    blank.seats[0].cards = [CardFace::None, CardFace::None];
    assert_eq!(None, odds::compute(game.table(), &blank.seats, &blank.board, false, game.seed()));
}

#[test]
fn frame__new_has_no_odds() {
    assert_eq!(None, the_hand("mode: replay\n").frame(View::All).odds);
}

#[test]
fn outs_bar__display() {
    let bar = OutsBar { seat: 3, name: "Gus Hansen".into(), percent: 18, cards: vec![card("T♠"), card("4♣")] };
    assert_eq!("Gus Hansen · 2 outs · 18%", bar.to_string());
    let dead = OutsBar { cards: Vec::new(), percent: 0, ..bar };
    assert_eq!("Gus Hansen · drawing dead · 0%", dead.to_string());
}
```

`frame_at` here takes a fourth argument `odds: bool` and calls `Frame::with_odds` when true. `index_of(frame, seat)` returns the position of `seat` in `frame.seats`.

- [ ] **Step 3: Run them to see them fail**

Run: `cargo test --features wasm --test odds`
Expected: FAIL to compile, no module `odds`.

- [ ] **Step 4: Add `Game::seed`** — store `seed: u64` on `Game` at `src/game.rs:357` (set from the `seed` local at `:416`) and add `pub fn seed(&self) -> u64` with a doc test asserting a config with `seed: 7` gives 7.

- [ ] **Step 5: Write `src/odds.rs`** with the types in Interfaces and `compute`:
  - Return `None` when `paid`, when fewer than two seats are `Live | AllIn`, or when any such seat has a card that is not `CardFace::Up`.
  - Players: one `PlayerSpec::Exact(Two::new(a, b).ok()?)` per live seat, in `seats` order.
  - Board: the `Some` cards of `board` in order, as a `Cards`, into `Board` (`TryFrom<Cards>`; `Board::default()` for none). `.ok()?` on failure.
  - `EquityOptions { seed: Some(seed), ..EquityOptions::default() }`. `request.compute().ok()?`.
  - Percent: `equity_pct().round()` clamped to `0.0..=100.0`, then to `u8`, with a `#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "clamped to 0..=100 first")]`.
  - `percents[i]` is `Some` for live seats and `None` for the rest. `outs: None` for now.
  - Module doc, type docs, a doc test on `compute` using the play config from `src/frame.rs:14-19` (both seats dealt, so `Some`), and a doc test on `OutsBar`'s Display.

- [ ] **Step 6: Wire the frame and the game** — `Frame.odds` field (doc: "Win percents and the outs bar, when odds are on and every live card is up."), `Frame::new` sets `None`, `Frame::with_odds(game, view)` builds `new` then sets `odds = odds::compute(game.table(), &frame.seats, &frame.board, frame.result.is_some(), game.seed())`. `Game::frame_with_odds(view)` calls it. Frame's Display appends ` {percent:>3}%` after the status when `odds` is `Some` and the seat's entry is `Some`. `src/lib.rs`: add `pub mod odds;` and one line in the module list.

- [ ] **Step 7: Run the tests**

Run: `cargo test --features wasm --test odds && cargo test --features wasm --doc`
Expected: PASS. `odds__every_live_seat_has_a_percent_on_the_flop` is exact (990 runouts) so the two percents sum to 100 after rounding; if rounding gives 99 or 101, change the assertion to `(99..=101).contains(..)` and note it in the test.

- [ ] **Step 8: Clippy and docs**

Run: `cargo clippy --all-targets --features wasm -- -D warnings && RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --features wasm`
Expected: clean.

- [ ] **Step 9: Hand the commit to the user**

```bash
git add src/odds.rs src/frame.rs src/game.rs src/lib.rs tests/odds.rs tests/hands/three_way_outs.yaml && git commit -m "odds: win percents per live seat from pkcore's equity engine" -m "Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 3: The outs bar

**Files:**
- Modify: `src/odds.rs` (fill `outs`), `tests/odds.rs`

**Interfaces:**
- Produces: `Odds.outs` is `Some(OutsBar)` exactly when two seats are live, the board holds four cards, and the two percents differ. `OutsBar.seat`/`name` are the trailing seat's; `percent` is its percent; `cards` are the river cards that make it the sole winner, in deck order (rank high to low, suits ♠ ♥ ♦ ♣), less any card a folded seat holds.
- Consumes: `pkcore::play::game::Game::new(HoleCards, Board)` and `turn_case_evals()` (pkcore `play/game.rs:281`, `:383`); `CaseEval::{card, win_count, winning_seats}` (pkcore `analysis/case_eval.rs:173`, `:229`, `:235`; `winning_seats` are 0-based indices into the `HoleCards` order); `HoleCards: From<Vec<Two>>` (pkcore `play/hole_cards.rs:286`); folded seats' cards from `table.seats.get_seat(seat).cards`.

- [ ] **Step 1: Write the failing tests** in `tests/odds.rs`:

```rust
#[test]
fn outs__two_left_one_card_to_come_lists_the_trailing_seats_outs() {
    let mut game = game("mode: replay\n", "tests/hands/three_way_outs.yaml");
    let frame = frame_at(&mut game, turn_of(&game), View::All, true);
    let bar = frame.odds.unwrap().outs.expect("a bar at the turn");
    assert_eq!(0, bar.seat);
    assert_eq!("Draw", bar.name);
    let cards: Vec<String> = bar.cards.iter().map(ToString::to_string).collect();
    assert_eq!(vec!["T♥", "T♦", "T♣", "5♠", "5♥", "5♦", "5♣"], cards, "T♠ is in the folded hand");
    assert_eq!(18, bar.percent, "pkcore counts the folded T♠ as live until EPIC-123");
}

#[test]
fn outs__drawing_dead_is_an_empty_bar() {
    let mut game = the_hand("mode: replay\n");
    let frame = frame_at(&mut game, turn_of(&game), View::All, true);
    let bar = frame.odds.unwrap().outs.expect("a bar at the turn");
    assert_eq!(NEGREANU, bar.seat);
    assert!(bar.cards.is_empty());
    assert_eq!(0, bar.percent);
}

#[test]
fn outs__no_bar_on_the_flop_or_the_river() {
    let mut game = the_hand("mode: replay\n");
    for moment in ["flop", "river"] {
        let index = moment_of(&game, moment);
        assert_eq!(None, frame_at(&mut game, index, View::All, true).odds.unwrap().outs, "{moment}");
    }
}

#[test]
fn outs__no_bar_with_three_live_seats() {
    // The Hand's flop before the first fold has more than two seats live
    // only preflop; use the first preflop moment: eight live, no bar.
    let mut game = the_hand("mode: replay\n");
    assert_eq!(None, frame_at(&mut game, 0, View::All, true).odds.unwrap().outs);
}
```

`moment_of(game, name)` is `turn_of` generalised over a `Moment` name. A tie case (`percents` equal) is pinned by a unit test inside `src/odds.rs` on the helper that picks the trailing seat: `trailing__none_when_tied`.

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --features wasm --test odds outs__`
Expected: FAIL, `outs` is `None`.

- [ ] **Step 3: Implement** in `src/odds.rs`: a private `fn trailing(percents: &[Option<u8>]) -> Option<usize>` (the index of the lower of exactly two `Some`s; `None` when equal) and a private `fn outs_bar(...) -> Option<OutsBar>` that builds pkcore's `Game`, filters `turn_case_evals()` to `win_count() == 1 && winning_seats()[0] == trailing position`, collects `card()`, removes cards held by any seat whose status is `Folded` (read from `table`, skipping `Card::BLANK`), and sorts by rank high to low, then suit ♠ ♥ ♦ ♣ (pkcore's `Cards::sort()` is suit-major, which is not the order the tests expect). `compute` calls it when `board` has exactly four `Some`s and exactly two live seats.

- [ ] **Step 4: Run the tests**

Run: `cargo test --features wasm --test odds && cargo test --features wasm odds`
Expected: PASS. If `turn_case_evals` orders seats differently from the request, `winning_seats` positions follow `HoleCards` order, which is the live-seat order used for `players`; keep the two in step.

- [ ] **Step 5: Clippy and docs**

Run: `cargo clippy --all-targets --features wasm -- -D warnings && RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --features wasm`
Expected: clean.

- [ ] **Step 6: Hand the commit to the user**

```bash
git add src/odds.rs tests/odds.rs && git commit -m "odds: the outs bar, two left with one card to come" -m "Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 4: Geometry for the percent and the bar

**Files:**
- Modify: `src/draw/geometry.rs` (`plan` signature `:242`, `Plan` `:174-195`, `boxes` `:211`, `ring` `:347-440`, `ledger` `:444-517`, `spotlight` `:522-597`, constants `:30-68`, unit tests), `tests/geometry.rs`, `src/draw/mod.rs:141` (the one call site)

**Interfaces:**
- Produces:
  - `pub const ODDS_W: u32 = 48;` the percent's box inside a plate (right of the cards). `pub const OUTS_LABEL_H: u32 = 14;` `pub const OUTS_ROW_H: u32 = SMALL_CARD_H + GAP;`
  - `pub fn plan(layout: Layout, live: &[bool], folded: &[String], outs: Option<usize>, width: u32) -> Plan` — `outs` is `Some(card count)` when the bar is present (`Some(0)` is a drawing-dead bar).
  - `Plan.outs: Option<Rect>`, counted by `boxes()`.
  - `pub fn outs_per_row(bar_width: u32) -> u32` — how many small cards fit in one row: `(bar_width + 3) / (SMALL_CARD_W + 3)`, at least 1.
  - Bar height = `OUTS_LABEL_H + rows * OUTS_ROW_H` where `rows = max(1, ceil(cards / per_row))`.
- Placement, all layouts: the bar is as wide as its layout allows. Ring: `x = oval.x + GAP`, `w = oval.w - 2 * GAP`, `y = pot.bottom() + GAP`; `bottom_bets = max(middle + PLATE_H, bar.bottom()) + GAP`; the oval's `h` is still `bottom_y`, so it grows with the bar. Ledger: `x = MARGIN`, `w = width - 2 * MARGIN`, `y = max(pot.bottom(), board.bottom()) + GAP`; `header = bar.bottom() + GAP`. Spotlight: `x = MARGIN`, `w = width - 2 * MARGIN`; in the two-live wide case `y = max(pot.bottom(), both bet slots' bottom) + GAP` (the plates and their bet slots sit beside the board, so the bar goes under all of it) and `bottom = bar.bottom()`; in the grid case `y = pot.bottom() + GAP` and `grid_y = bar.bottom() + 2 * GAP`.

- [ ] **Step 1: Write the failing tests** — in `src/draw/geometry.rs`'s tests:

```rust
#[test]
fn plate__holds_the_percent_beside_the_cards() {
    // `100%` at 18 px bold, 10.8 px a character, right of two small cards.
    const { assert!(4 + 2 * SMALL_CARD_W + 3 + 4 + ODDS_W + 4 <= PLATE_W) };
    assert!(fits(4, 108, 4, ODDS_W));
}

#[test]
fn outs_per_row__fourteen_fit_in_the_narrowest_ring() {
    let plan = plan(Layout::Ring, &[true, true], &[], Some(14), RING_MIN_WIDTH);
    assert!(outs_per_row(plan.outs.unwrap().w) >= 14);
}

#[test]
fn ring__grows_for_the_bar_and_keeps_the_board_still() {
    let without = plan(Layout::Ring, &[true; 8], &[], None, 680);
    let with = plan(Layout::Ring, &[true; 8], &[], Some(8), 680);
    assert_eq!(without.board, with.board);
    assert_eq!(without.pot, with.pot);
    assert!(with.height > without.height);
    assert!(with.oval.unwrap().h > without.oval.unwrap().h);
    assert!(with.outs.unwrap().y >= with.pot.bottom());
}

#[test]
fn bar__wraps_to_rows() {
    let one = plan(Layout::Ledger, &[true, true], &[], Some(1), 320);
    let many = plan(Layout::Ledger, &[true, true], &[], Some(44), 320);
    assert!(many.outs.unwrap().h > one.outs.unwrap().h);
    assert_eq!(OUTS_LABEL_H + OUTS_ROW_H, plan(Layout::Ledger, &[true, true], &[], Some(0), 680).outs.unwrap().h);
}
```

In `tests/geometry.rs` extend the proptest with `outs in prop::option::of(0usize..=44)` passed to `plan`, and add:

```rust
#[test]
fn geometry__no_bar_without_outs() {
    for layout in [Layout::Ring, Layout::Spotlight, Layout::Ledger] {
        assert_eq!(None, plan(layout, &[true, true], &[], None, 680).outs, "{layout}");
    }
}
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --features wasm geometry`
Expected: FAIL to compile (arity, missing items).

- [ ] **Step 3: Implement** the constants, the field, the signature, `outs_per_row`, a private `fn bar(x: u32, w: u32, y: u32, cards: usize) -> Rect`, and the three layouts' placement as in Interfaces. Update every `plan(` call: `src/draw/mod.rs:141` passes `None` for now (Task 5 passes the real count), the doc tests in `geometry.rs`, and `tests/geometry.rs`.

- [ ] **Step 4: Run the tests**

Run: `cargo test --features wasm geometry && cargo test --features wasm --test draw`
Expected: PASS, golden files unchanged (no bar is passed yet).

- [ ] **Step 5: Clippy and docs**

Run: `cargo clippy --all-targets --features wasm -- -D warnings && RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --features wasm`
Expected: clean.

- [ ] **Step 6: Hand the commit to the user**

```bash
git add src/draw/geometry.rs src/draw/mod.rs tests/geometry.rs && git commit -m "geometry: a box for the percent and a bar under the pot" -m "Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 5: Drawing the percent and the bar

**Files:**
- Modify: `src/draw/mod.rs` (`svg` `:134-188`, `draw_plate` `:468-521`, `draw_row` `:524-607`, new `draw_outs`), `js/bowery.css`, `tests/draw.rs`
- Create: 12 golden files under `tests/golden/`

**Interfaces:**
- Produces, in the SVG:
  - Plate: `<text class="odds" … font-size="18" font-weight="700" text-anchor="end">96%</text>` at `x = plate.right() - 4`, `y = plate.y + 52`, in the plate's ink. Only when `frame.odds` is `Some` and the seat's entry is `Some`.
  - Ledger row: `<text class="odds" … font-size="11" font-weight="700">96%</text>`; wide rows: `anchor="start"`, `x = cards_x + 2 * SMALL_CARD_W + 3 + 12`, `y = name_y`; narrow rows: `anchor="end"`, `x = plate.right() - 4`, `y = name_y`.
  - Bar: `<g class="outs" data-seat="N">`, then `<text class="label" font-size="10" font-weight="700" text-anchor="middle" fill=grey>` at the bar's centre, `y = rect.y + 10`, content `GUS HANSEN · 8 OUTS · 18%` (name cut to `NAME_CHARS`, whole label uppercased); then the cards, each via `draw_card(.., small = true, CardFace::Up(card))`, rows of `outs_per_row(rect.w)`, each row centred, row `r` at `y = rect.y + OUTS_LABEL_H + r * OUTS_ROW_H`; or, for an empty `cards`, `<text class="dead" font-size="11" text-anchor="middle">drawing dead</text>` centred in the first row. Drawn after the pot and before the seats.
  - `svg` passes `frame.odds.as_ref().and_then(|o| o.outs.as_ref()).map(|bar| bar.cards.len())` to `plan`.
- CSS: `.seat .odds { fill: var(--bowery-fg) }`, `.seat.to-act .odds { fill: var(--bowery-to-act-fg) }`, `.outs .label { fill: var(--bowery-muted) }`, `.outs .dead { fill: var(--bowery-muted) }`; the bar's cards take the existing `.card` rules.

- [ ] **Step 1: Write the failing tests** in `tests/draw.rs`:

```rust
/// A hand at its turn, odds on, all cards up.
fn drawn_with_odds(hand: &str, layout: Layout, theme: Theme) -> String {
    let mut game = game_of("mode: replay\n", hand);
    game.seek(turn_index(&game)).unwrap();
    svg(&game.frame_with_odds(View::All), layout, WIDTH, theme)
}

#[test]
fn draw__golden_turn_with_odds() {
    for (file, hand) in [("the_hand", "fixtures/the_hand.yaml"), ("three_way_outs", "tests/hands/three_way_outs.yaml")] {
        for layout in [Layout::Ring, Layout::Spotlight, Layout::Ledger] {
            for (theme, theme_name) in [(Theme::Light, "light"), (Theme::Dark, "dark")] {
                golden(&format!("{file}-{layout}-turn-odds-{theme_name}"), &drawn_with_odds(hand, layout, theme));
            }
        }
    }
}

#[test]
fn draw__odds_on_every_live_plate_and_the_bar_lists_the_outs() {
    let svg = drawn_with_odds("tests/hands/three_way_outs.yaml", Layout::Ring, Theme::Light);
    assert_eq!(2, svg.matches(r#"class="odds""#).count());
    assert!(svg.contains(r#"<g class="outs" data-seat="0">"#));
    assert!(svg.contains("DRAW · 7 OUTS · 18%"));
    assert!(svg.contains(r#"data-card="T♥""#) && !svg.contains(r#"data-card="T♠""#));
}

#[test]
fn draw__drawing_dead_is_text_not_cards() {
    // No fixture hand is drawing dead at the turn (in The Hand Negreanu
    // has one out, the 6♣), so empty the bar by hand before drawing.
    let mut game = the_hand("mode: replay\n");
    game.seek(turn_index(&game)).unwrap();
    let mut frame = game.frame_with_odds(View::All);
    let bar = frame.odds.as_mut().unwrap().outs.as_mut().unwrap();
    bar.cards.clear();
    bar.percent = 0;
    let svg = svg(&frame, Layout::Ledger, WIDTH, Theme::Light);
    assert!(svg.contains(r#"class="dead""#));
    assert!(svg.contains("drawing dead"));
    assert!(svg.contains("DANIEL NEGREANU · DRAWING DEAD · 0%"));
    let bar_svg = svg.split(r#"<g class="outs""#).nth(1).unwrap();
    let bar_svg = bar_svg.split("</g>").next().unwrap();
    assert!(!bar_svg.contains("data-card"), "no cards inside the bar");
}

#[test]
fn draw__one_out_is_one_card_and_a_singular_label() {
    let svg = drawn_with_odds("fixtures/the_hand.yaml", Layout::Ledger, Theme::Light);
    assert!(svg.contains(r#"<g class="outs" data-seat="4">"#));
    assert!(svg.contains("DANIEL NEGREANU · 1 OUT · 2%"));
    assert!(svg.contains(r#"data-card="6♣""#));
}

#[test]
fn draw__seat_view_draws_no_odds_and_no_bar() {
    let mut game = the_hand("mode: replay\nseat: 4\n");
    game.seek(turn_index(&game)).unwrap();
    let svg = svg(&game.frame_with_odds(View::Seat), Layout::Ring, WIDTH, Theme::Light);
    assert!(!svg.contains(r#"class="odds""#));
    assert!(!svg.contains(r#"class="outs""#));
}
```

Rename `draw__golden_files_are_the_twenty_four` to `draw__golden_files_are_the_thirty_six` asserting 36. `game_of(head, path)` and `turn_index(game)` are helpers like `tests/odds.rs`'s `game` and `turn_of` (copy them; `turn_index` finds the first `Event::Deal(ReplayStreet::Turn)`). The bar's `</g>` must close before the seats start, so the `split` above isolates the bar.

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --features wasm --test draw`
Expected: FAIL: missing golden files and no `class="odds"`.

- [ ] **Step 3: Implement** `draw_plate`'s and `draw_row`'s percent, `draw_outs`, and the `plan` call in `svg`. `draw_plate` and `draw_row` need the seat's `Option<u8>`: add a parameter `odds: Option<u8>` to both and pass `frame.odds.as_ref().and_then(|o| o.percents.get(i).copied().flatten())` from the loop in `svg` (zip with an index).

- [ ] **Step 4: Bless and review the golden files**

Run: `BOWERY_BLESS=1 cargo test --features wasm --test draw draw__golden_turn_with_odds && for f in tests/golden/*turn-odds-light.svg; do rsvg-convert "$f" -o "/private/tmp/claude-501/-Users-christoph-src-github-com-ImperialBower-bowery/41549982-6862-4b4a-a3ba-15b530f73cf0/scratchpad/$(basename "$f" .svg).png"; done`
Then Read each PNG. Expected: percents beside the cards on both live plates, nothing on folded plates, the bar under the pot inside the oval (ring), seven out cards for `three_way_outs`, one out card (6♣) with the label `DANIEL NEGREANU · 1 OUT · 2%` for The Hand, nothing overlapping. Fix and re-bless until so.

- [ ] **Step 5: Run the whole draw suite**

Run: `cargo test --features wasm --test draw`
Expected: PASS, 36 golden files, the 24 old ones byte-identical (`git diff --stat tests/golden` shows only 12 new files).

- [ ] **Step 6: Add the CSS rules** to `js/bowery.css` next to the existing `.seat .name` rules.

- [ ] **Step 7: Clippy and docs**

Run: `cargo clippy --all-targets --features wasm -- -D warnings && RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --features wasm`
Expected: clean.

- [ ] **Step 8: Hand the commit to the user**

```bash
git add src/draw/mod.rs js/bowery.css tests/draw.rs tests/golden && git commit -m "draw: the percent on each live plate and the outs bar" -m "Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 6: The element's switch and cache

**Files:**
- Modify: `src/element.rs` (`Table` `:44-48`, `State` `:67-102`, `Controls` `:163-178`, `svg` `:243-246`, `state` `:263-309`, every mutating method, new `set_odds`), `src/wasm.rs` (after `set_view` `:293`), `tests/element.rs`

**Interfaces:**
- Produces:
  - `Table { game, layout, view, odds: bool, cache: RefCell<HashMap<usize, Option<Odds>>> }`; `odds` starts `true`.
  - `State.odds: bool` — odds are on screen now: `self.odds && self.view == View::All`.
  - `Controls.odds: bool` — the `%` button exists: `config.controls.odds.unwrap_or(true) && self.view == View::All`.
  - `pub fn set_odds(&mut self, on: bool)` on `element::Table`; `pub fn set_odds(&mut self, on: bool)` on `wasm::BoweryTable`.
  - A private `fn frame(&self) -> Frame`: `game.frame(view)`, then, when `State.odds` would be true, `frame.odds = cache[at]` or compute via `odds::compute(...)` and store. Both `svg` and `state` use it.
  - The cache is cleared in `take_seat`, `back_to_record`, `act`, `advance`, `next_hand` and `shuffle`. `seek`, `set_view`, `set_layout` and `set_odds` leave it.

- [ ] **Step 1: Write the failing tests** in `tests/element.rs`:

```rust
#[test]
fn element__odds_on_by_default_in_all_view() {
    let table = Table::new(&the_hand("mode: replay\nview: all\nstart: flop\n")).unwrap();
    let state = state(&table);
    assert_eq!(true, state["odds"]);
    assert_eq!(true, state["controls"]["odds"]);
    assert!(table.svg(680, false).contains(r#"class="odds""#));
}

#[test]
fn element__odds_toggle_off_and_on() {
    let mut table = Table::new(&the_hand("mode: replay\nview: all\nstart: flop\n")).unwrap();
    table.set_odds(false);
    assert_eq!(false, state(&table)["odds"]);
    assert!(!table.svg(680, false).contains(r#"class="odds""#));
    table.set_odds(true);
    assert_eq!(true, state(&table)["odds"]);
}

#[test]
fn element__odds_hidden_in_seat_view_and_back_with_the_eye() {
    let mut table = Table::new(&the_hand("mode: replay\nseat: 4\nstart: flop\n")).unwrap();
    assert_eq!(false, state(&table)["odds"]);
    assert_eq!(false, state(&table)["controls"]["odds"]);
    table.set_view("all").unwrap();
    assert_eq!(true, state(&table)["odds"]);
    assert_eq!(true, state(&table)["controls"]["odds"]);
}

#[test]
fn element__odds_stay_off_in_seat_view_at_showdown() {
    let mut table = Table::new(&the_hand("mode: replay\nseat: 4\n")).unwrap();
    table.seek(usize::MAX);
    assert_eq!(false, state(&table)["odds"]);
    assert!(!table.svg(680, false).contains(r#"class="odds""#));
}

#[test]
fn element__config_drops_the_odds_button_but_not_the_odds() {
    let table = Table::new(&the_hand("mode: replay\nview: all\nstart: flop\ncontrols: { odds: false }\n")).unwrap();
    assert_eq!(false, state(&table)["controls"]["odds"]);
    assert_eq!(true, state(&table)["odds"]);
}

#[test]
fn element__odds_follow_an_action_off_the_record() {
    // all-in hand, seat 1's view switched to all: a fold instead of the call
    // leaves one seat live, so the percents go away.
    let mut table = Table::new(&format!("mode: interactive\nview: all\nseat: 1\nstart: 1\nhand:\n{}", indented("tests/hands/heads_up_all_in.yaml"))).unwrap();
    let before = table.svg(680, false);
    assert!(before.contains(r#"class="odds""#));
    table.take_seat(1).unwrap();
    table.act(r#"{"action":"fold"}"#).unwrap();
    let after = table.svg(680, false);
    assert!(!after.contains(r#"class="odds""#), "one live seat shows no percent");
}
```

`indented(path)` indents a hand file under `hand:` like `the_hand` does. (If `controls.odds: false` should hide the odds themselves, the spec left that to the builder: pick, make `element__config_drops_the_odds_button_but_not_the_odds` match, and document the rule on `Controls.odds` in `src/config.rs`.)

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --features wasm --test element`
Expected: FAIL: no `odds` in state, no `set_odds`.

- [ ] **Step 3: Implement** as in Interfaces. Doc tests on `set_odds` (both layers) and on the new fields; `State`'s doc test asserts `state.odds`.

- [ ] **Step 4: Run the element and wasm tests**

Run: `cargo test --features wasm --test element && cargo test --features wasm element && cargo test --features wasm wasm`
Expected: PASS.

- [ ] **Step 5: Wire the e2e page's time budget** — no code; note for Task 7: an 8-way preflop Monte Carlo runs once per moment in the browser and is then cached.

- [ ] **Step 6: Clippy, docs, wasm**

Run: `cargo clippy --all-targets --features wasm -- -D warnings && RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --features wasm && make wasm`
Expected: clean.

- [ ] **Step 7: Hand the commit to the user**

```bash
git add src/element.rs src/wasm.rs tests/element.rs && git commit -m "element: the odds switch, off in the seat view, cached per moment" -m "Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 7: The `%` button and the browser proof

**Files:**
- Modify: `js/bowery.js:270-275` (buttons), `:365-374` (toggles), `Makefile:72-77` (`e2e-pages`)
- Create: `tests/e2e/pages/outs.html`, `tests/e2e/odds.spec.ts`

**Interfaces:**
- Produces: a button after ▦ when `state.controls.odds`: label `% on` or `% off` from `state.odds`, `aria-label="odds"`, `data-control="odds"`, click → `this._table.set_odds(!state.odds); this._render();`.
- `tests/e2e/pages/outs.html`: one `<bowery-table id="outs" base="../bowery/" src="outs-replay.yaml">`; `make e2e-pages` writes `site/e2e/outs-replay.yaml` from `tests/hands/three_way_outs.yaml` with head `mode: replay\nview: all\n`.
- `helpers.ts`'s `snapshot` gains `odds: string[]` (every `text.odds` content, in document order) and `outs: string[]` (every `g.outs [data-card]`).

- [ ] **Step 1: Write the failing spec** `tests/e2e/odds.spec.ts`:

```ts
import { expect, test } from '@playwright/test';
import { ready, snapshot } from './helpers';

test('odds: percents show, the bar lists the outs at the turn, % hides them', async ({ page }) => {
  await page.goto('/e2e/outs.html');
  await ready(page, 'outs');
  const bar = page.locator('#outs .bar');
  // Preflop: a percent per live seat, no value asserted (sampled, per target).
  let shown = await snapshot(page, 'outs');
  expect(shown.odds).toHaveLength(3);
  expect(shown.outs).toEqual([]);
  // To the turn: exact numbers from here on.
  for (let step = 0; step < 100 && !/^Turn · /.test(shown.caption); step += 1) {
    await bar.getByRole('button', { name: 'next' }).click();
    shown = await snapshot(page, 'outs');
  }
  expect(shown.odds).toEqual(['18%', '82%']);
  expect(shown.outs).toEqual(['T♥', 'T♦', 'T♣', '5♠', '5♥', '5♦', '5♣']);
  // % off hides both; the eye to seat view hides the button.
  await bar.getByRole('button', { name: 'odds' }).click();
  shown = await snapshot(page, 'outs');
  expect(shown.odds).toEqual([]);
  expect(shown.outs).toEqual([]);
  await bar.getByRole('button', { name: 'view' }).click();
  shown = await snapshot(page, 'outs');
  expect(shown.buttons).not.toContain('odds');
});
```

If the ring draws the two live plates in a different order, the expected `odds` array follows document order; set it from the first run's output and say so in a comment.

- [ ] **Step 2: Build the site and run the spec to see it fail**

Run: `make site e2e-pages && cd tests/e2e && npx playwright test odds.spec.ts`
Expected: FAIL, no `odds` in the snapshot / no `odds` button.

- [ ] **Step 3: Implement** the button, the toggle, the helper fields, the page and the Makefile line.

- [ ] **Step 4: Run the spec, then the whole proof**

Run: `make e2e`
Expected: PASS for `odds.spec.ts` and every existing spec (the `guide` and `origin` specs see no new origin).

- [ ] **Step 5: Hand the commit to the user**

```bash
git add js/bowery.js tests/e2e/pages/outs.html tests/e2e/odds.spec.ts tests/e2e/helpers.ts Makefile && git commit -m "element: the % button; browser proof of the odds and the bar" -m "Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 8: The still, the guides, the knowledge bundle, the gate

**Files:**
- Modify: `examples/bowery-still.rs:35-82`, `demo/guide/replay.html`, `demo/guide/nine.html`, `demo/guide/play.html`, `demo/guide/interactive.html`, `.okf/concepts/frame-and-draw.md`, `.okf/concepts/element.md`, `.okf/log.md`, `docs/EPIC-110_Bowery.md`, `CHANGELOG.md`, `Cargo.toml` (version), `README.md` if it lists the controls

**Interfaces:**
- Produces: `bowery-still … --odds` draws with `game.frame_with_odds(view)`; the usage string lists it.

- [ ] **Step 1: Add `--odds`** to the still: a flag like `--dark`; usage string updated; module doc example line added.

Run: `cargo run -q --example bowery-still -- tests/hands/three_way_outs.yaml --at turn --odds | grep -c 'class="outs"'`
Expected: `1`.

- [ ] **Step 2: Guides** — in each guide's controls paragraph add one sentence: "**%** shows every live seat's win percent, and with two seats left and one card to come, the trailing seat's outs; it is only there when every card is up." In each config table add a row `controls.odds` / `true`, `false` / "Show the % button. Default: shown when the view is all." Say the preflop figure is sampled and may differ from a still. `guide.spec.ts` compares configs, not prose, so it stays green; run it to be sure:

Run: `make site && cd tests/e2e && npx playwright test guide.spec.ts`
Expected: PASS.

- [ ] **Step 3: Knowledge bundle** — `.okf/concepts/frame-and-draw.md`: a bullet on `odds.rs` (what asks pkcore, what returns `None`, folded cards and EPIC-123, the bar's conditions, the ring growing), and the new classes `.odds`, `.outs .label`, `.outs .dead`. `.okf/concepts/element.md`: a bullet on `odds`/`set_odds`, the per-moment cache and why (`_render` builds the frame three times), and the per-target preflop numbers. `.okf/log.md`: one dated entry linking both. Run the OKF validator skill (`okf:validate`) and fix what it reports.

- [ ] **Step 4: EPIC and changelog** — `docs/EPIC-110_Bowery.md`: a Status row "Phase 7 — odds on the plates and the outs bar" marked Done once Task 8 is green, pointing at the spec. `CHANGELOG.md`: an entry for the new minor version (new public API: `odds` module, `Frame.odds`, `Game::seed`, `set_odds`, `controls.odds`) with the folded-cards caveat and the EPIC-123 link. `Cargo.toml` version bumped to match; `cargo build` for the lock file.

- [ ] **Step 5: The gate**

Run: `make ayce`
Expected: every target green, ending with `make size` under the gate and the e2e suite passing. Paste the size line into the CHANGELOG entry if it moved.

- [ ] **Step 6: Hand the commit to the user**

```bash
git add examples/bowery-still.rs demo/guide .okf docs/EPIC-110_Bowery.md CHANGELOG.md Cargo.toml Cargo.lock README.md && git commit -m "odds: --odds still, guides, knowledge bundle, changelog" -m "Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```
