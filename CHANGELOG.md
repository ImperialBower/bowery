# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
