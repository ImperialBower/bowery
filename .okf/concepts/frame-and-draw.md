---
type: concept
title: Frame and draw
description: How a moment becomes a Frame with the view applied, how geometry places every box, and how the one SVG renderer and its golden files work.
tags: [frame, draw, svg, geometry, odds, epic-20]
timestamp: '2026-10-24T00:00:00Z'
---

# Frame and draw

`src/frame.rs`, `src/draw/geometry.rs`, `src/draw/mod.rs`. Builds on the
[game](game.md).

- **Frame.** `Game::frame(view)`. The view is applied while the frame is
  built, so a `seat` view never holds another seat's card before a showdown.
  The viewer is the config's `seat`, else the reader; with neither, every
  card is down.
- **Geometry first.** `plan(layout, live, folded, width)` returns every box
  as a `Rect`. Each box is sized for its widest text, using monospace metrics:
  11 px text is 6.6 px a character, a name is cut to 16 characters, and
  `format_chips` never gives more than 11. So a box holds its text, and
  boxes that do not intersect cannot cover each other's text.
  `geometry__boxes_disjoint` is the proptest; `strip_lines` wraps spotlight's
  folded names, and plan and draw both use it, so they agree on its height.
- **Ring** needs 520 px, and 574 px for its side columns; where its seats do
  not fit, it falls back to ledger.
- **Ledger's one-line row** starts at `LEDGER_WIDE` (464 px), not `WIDE`
  (420 px, which spotlight uses for two seats beside the board): a one-line row
  has to hold the cards, a `100%` and the widest stack (`999,999,999`) on the
  same baseline, and below 464 px they overlap. `geometry::ledger` and
  `draw_row` both read `LEDGER_WIDE`, so the plan and the drawing agree; below
  it a row is two lines and the percent ends the name line.
- **One renderer.** `svg(frame, layout, width, theme)`: integer coordinates,
  colours as attributes, no CSS or script. Hole cards that are up carry
  `data-card="5♦"`, which is what `draw__seat_view_draws_no_hidden_card`
  looks for.
- **The theme is CSS, not the renderer.** The attribute colours are the
  fallback: stills, and the first frame before `bowery.css` arrives. In the
  element, `js/bowery.css` selects the SVG's classes (`.seat.to-act .plate`,
  `.card.red text`, `.bet.raise .slot`, ...) and a stylesheet rule beats a
  presentation attribute, so recolouring never touches `draw` or the golden
  files. Every colour is a `--bowery-*` property on `:host`; a page can
  override any of them. A new seat or bet state needs a class here and a
  rule there.
- **Golden files.** `tests/golden/*.svg`, compared byte for byte.
  `BOWERY_BLESS=1 cargo test --test draw` rewrites them. Render them with
  `rsvg-convert` to look before accepting.
- `usize` is 32 bits on wasm32: count chips in `u64` where they can pass four
  billion (`format_chips`). `make wasm` catches an overflowing literal.
- **Odds.** `src/odds.rs`. `Game::frame_with_odds(view)` (and `Frame::with_odds`)
  fills `Frame.odds`; plain `frame` never asks pkcore. `Odds::compute` hands the live
  seats' hole cards and the board to pkcore's `EquityRequest`, seeded with the
  game's seed (`Game::seed`), so a replay shows the same numbers each time on
  one target. It returns `None` once the hand has paid out, with fewer than two
  live seats, and when a live seat's card is face down, so a hidden hand is
  never given away. Percents are whole numbers, 0..=100; a row may sum to 99
  or 101.
- **Folded cards are dead cards.** `folded_cards` reads a folded seat's hole
  cards off the pkcore table, and both odds paths hand them to pkcore as dead:
  the percents through `EquityRequest::dead` (pkcore EPIC-123, 0.18.0) and the
  outs bar's river enumeration through `Game::with_dead` (pkcore DEFECT_042,
  0.19.0). bowery filters nothing by hand. Before 0.18.0 the percents counted
  those cards as live; moving to 0.18.0 changed The Hand's turn (Negreanu 2% →
  3%) and `three_way_outs` (Draw 18% → 17%), and the goldens were re-blessed.
- **The outs bar** (`Odds.outs`, `OutsBar`) needs odds on, every live card up,
  exactly two live seats and a four-card board. It names the trailing seat and
  the river cards that win it the pot alone (a card that only splits does not
  count); a tie has no bar; zero outs reads `drawing dead`. The Hand at the
  turn is not that case: Negreanu holds one out. The bar sits under the pot
  line (under the board in ledger), wraps after `outs_per_row` cards, and the
  ring grows taller to hold it (`Plan.outs`, `OUTS_*`); it goes through `plan`,
  so the disjoint-boxes proptest covers it.
- **Odds classes.** `.odds` is the percent on a plate or ledger row;
  `.outs .label` is the bar's caption and `.outs .dead` the drawing-dead text.
  `js/bowery.css` colours all three (`--bowery-fg`, `--bowery-to-act-fg`,
  `--bowery-muted`); the renderer's attribute colours are the fallback.
- **Preflop is sampled.** Exact once pkcore can enumerate the runouts (flop,
  turn, river); preflop is seeded Monte Carlo on a `SmallRng`, and 32-bit wasm
  and 64-bit native give different numbers. Never quote a preflop percent in a
  test (outside native goldens), guide or note. `bowery-still --odds` draws a
  frame with odds.
