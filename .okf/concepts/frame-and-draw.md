---
type: concept
title: Frame and draw
description: How a moment becomes a Frame with the view applied, how geometry places every box, and how the one SVG renderer and its golden files work.
tags: [frame, draw, svg, geometry, epic-20]
timestamp: '2026-10-22T00:00:00Z'
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
- **One renderer.** `svg(frame, layout, width, theme)`: integer coordinates,
  colours as attributes, no CSS or script. Hole cards that are up carry
  `data-card="5♦"`, which is what `draw__seat_view_draws_no_hidden_card`
  looks for.
- **Golden files.** `tests/golden/*.svg`, compared byte for byte.
  `BOWERY_BLESS=1 cargo test --test draw` rewrites them. Render them with
  `rsvg-convert` to look before accepting.
- `usize` is 32 bits on wasm32: count chips in `u64` where they can pass four
  billion (`format_chips`). `make wasm` catches an overflowing literal.
