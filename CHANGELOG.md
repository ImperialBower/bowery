# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- **Breaking: `odds::compute` is now `Odds::compute`.** Same arguments, same
  result; a call `odds::compute(…)` becomes `Odds::compute(…)` with
  `use bowery::odds::Odds;`. The module's private helpers moved onto `Odds`
  and `OutsBar` as well, so `src/odds.rs` has no free functions. The next
  release is therefore `0.3.0`.

- **Odds percents now treat a folded seat's known cards as dead.** pkcore
  `0.18.0` added `EquityRequest::dead` (pkcore EPIC-123); bowery passes each
  folded seat's known hole cards through it, so they can no longer come back
  as a runout card for a live seat's win percent. Percents move slightly
  (closer to correct) wherever a seat has folded with known cards: at The
  Hand's turn Negreanu reads 3%, not 2%.
- **The outs bar asks pkcore to leave folded cards out.** pkcore `0.19.0`
  added `Game::with_dead` (pkcore DEFECT_042), so the river enumeration no
  longer deals a card a folded seat holds. bowery passes those cards in and
  no longer removes them from the result by hand. The bar's cards are the
  same as before.
  - `pkcore` bumped to `0.20.0`.

## [0.2.0] - 2026-10-24

### Added

- **Odds on the plates and an outs bar.** Every live seat shows its win
  percent (a whole number), from pkcore's equity engine, and when two seats
  are left with one card to come, a bar under the pot lists the river cards
  that win the trailing seat the pot alone (`drawing dead` when there are
  none). Nothing shows in the `seat` view, when any live seat's card is face
  down, with fewer than two live seats, or after the payout. The numbers are
  seeded, so a replay shows the same figures each time on one target; a
  preflop percent is sampled, and 32-bit wasm and 64-bit native differ, so a
  still's preflop figure may differ from the browser's.
  - New public API: the `odds` module (`Odds`, `OutsBar`, `compute`);
    `Frame.odds` and `Frame::with_odds`; `Game::seed` and
    `Game::frame_with_odds`; `set_odds` on the element and the wasm face;
    `controls.odds` in the config; a `%` button; `bowery-still --odds`.
  - Odds start on in replay and interactive mode when the view is `all`, and
    off in play mode: bots run on a timer, and a nine-way preflop step is a
    25,000-sample Monte Carlo, about a second of the browser's one thread.
    The reader turns them on with `%`. `controls: { odds: false }` drops the
    button only, not the odds.
  - Folded cards: pkcore 0.17 has no dead-cards input, so a folded seat's
    cards still count as live in the percents. The outs bar removes a folded
    seat's cards by hand. pkcore's
    [EPIC-123](https://github.com/ImperialBower/pkcore/blob/main/docs/epics/EPIC-123_Dead_Cards.md)
    (pkcore 0.18.0) adds `EquityRequest::dead`; bowery will pass the folded
    cards then.
  - 12 new golden files; the guides explain the `%` button and
    `controls.odds`.
  - The wasm grows from 1,295,901 to 1,313,728 bytes (`make size`:
    `bowery_wasm_bg.wasm: 1313728 bytes (gate 1425491)`).
- **The demo on GitHub Pages**, at <https://imperialbower.github.io/bowery/>,
  linked from the README. `.github/workflows/pages.yaml` publishes `site/`
  from `main` after the browser proof passes, without the proof's own pages.
- **A theme switch on the demo page**: system, paper or night. It sets
  mdBook's theme class on `<html>` (`light`, `coal`), which every
  `<bowery-table>` already reads and watches, so the tables redraw at once.
- **A guide page for each demo table**, at `guide/replay.html`,
  `guide/interactive.html`, `guide/play.html` and `guide/nine.html` on the
  demo site (`demo/guide/`), linked from each table. Each shows the table
  live and the setup step by step: the script line, the config, the element,
  serving it, the settings and the events. A browser test keeps the configs
  shown equal to the configs the tables run, and `scripts/dist.sh` checks
  their CDN links name the crate version, as it does the README's.

### Changed

- **A colour theme in `bowery.css`.** Light is warm paper, dark is slate
  "night": light card faces, red pips, patterned backs, the seat to act in a
  hue nothing else uses (amber on light, teal on dark), folded seats faded,
  a bet or raise as a warm pill, the pot in gold. Every colour is a
  `--bowery-*` custom property on `:host`, so a page can override any of
  them. The SVG the renderer emits is unchanged: the rules select its
  classes and win over its own fill and stroke attributes.

## [0.1.2] - 2026-10-23

### Fixed

- **Arrow keys in the controls.** Arrow, Home and End in the seat picker or
  a bet box no longer seek moments; they seek only when the table itself has
  focus.
- **`check` refuses play tables the game cannot run:** a button off the
  table, a blind of 0, a big blind smaller than the small blind, and a stack
  of 0.
- **`check` refuses `table` beside a recorded hand.** It was documented as an
  override but did nothing; replay and interactive play the hand's own table.

### Changed

- **Every action in the release workflow is pinned to a commit.** GitHub's
  own `actions/*` were on tags, which can be moved, in the jobs that hold
  the publish tokens.
- **The release checks the README.** `scripts/dist.sh` refuses a README whose
  `@imperialbower/bowery@<version>` CDN link is not the crate's version. The
  link stays exact: the element loads its glue and wasm from beside it, and a
  range such as `@0.1` could serve them from two releases while the CDN's
  cache catches up.

## [0.1.1] - 2026-10-22

### Changed

- **Releases need no stored tokens.** The release workflow publishes to
  crates.io and npm by trusted publishing (OIDC): each registry trusts this
  repository's `release.yml`, and the tokens it uses last one job.
- **README.** The CDN example names 0.1.1, and a note says how to check a
  release: `npm audit signatures` for the npm package's provenance,
  `MANIFEST.sha256` for the tarball.

## [0.1.0] - 2026-10-22

The first release: bower's EPIC-20, as bowery's EPIC-110
([docs/EPIC-110_Bowery.md](docs/EPIC-110_Bowery.md)).

### Added

- **`Config` and `check`.** A table is a YAML config: the mode (replay,
  interactive, play), view, layout, an inline recorded hand, the players,
  stakes, deck, seed and controls. `check` returns every problem at once; the
  recorded hand is checked by pkcore's `HandHistory::validate`.
- **`Game`.** Replay is pkcore's replay cursor, one moment per event.
  Interactive takes a seat and goes off the record at the first action that
  differs, with the hand's own deck still to come. Play deals hands against
  pkcore's bots, seeded. Every hand ends in its own moments: each street,
  any chips returned, the showdown, each pot share won (the eight defects of
  bower EPIC-19's table, D1–D5, fixed in the kernel).
- **`Frame` and `draw::svg`.** What is on screen, with the view applied while
  the frame is built, drawn as SVG in three layouts (ring, spotlight,
  ledger), light and dark: no script, no CSS, every box placed and checked
  for overlap before anything is drawn (D6–D7). The pot shown counts the
  bets in front of the seats (D8).
- **`<bowery-table>`.** The `wasm` feature's `BoweryTable` and
  `js/bowery.js`: lazy, one module shared by every table on a page, the
  fallback picture kept on any failure, keyboard and screen-reader controls,
  five events. The wasm is 1,295,945 bytes, under a 1,425,491-byte gate.
- **Proof.** Golden SVGs of The Hand, a disjoint-boxes proptest, and a
  Playwright suite for D1–D8, three tables on one page, a single origin and
  the fallback.
- **`bowery-still`**, an example that turns a config or a hand history into
  one SVG.
