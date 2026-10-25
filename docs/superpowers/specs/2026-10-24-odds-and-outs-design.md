# Odds and outs on the table

**Date:** 24 October 2026. **Status:** implemented on branch `odds`
(plan `docs/superpowers/plans/2026-10-24-odds-and-outs.md`), 0.2.0.

## Intent

Poker TV shows every player's chance to win beside their cards, and when
two players are left with one card to come, it lists the cards that would
rescue the one who is behind. `<bowery-table>` should do the same, as an
option the reader can switch off and the author can drop.

The author of this repo asked for it in these words: "as soon as it's
possible to show odds, show them. When someone folds, adjust it." And for
the outs: "an outs bar that isn't bound by the dimensions of the player
box, that only appears ... when there are exactly two players ... when
there is one card to go, and should show the outs of the player who is
down."

## Goals

- Every live seat shows its win percent from the first moment hole cards
  are out, through the river.
- A fold, or any other change in who is live, changes the other seats'
  numbers at that moment.
- When exactly two seats are live and the board holds four cards, an outs
  bar lists the river cards that win the pot for the seat that is behind.
- The reader can turn the whole thing off with one button. The author can
  drop the button.
- Replays are deterministic: the same config always shows the same numbers,
  so the golden SVGs stay stable.
- The math comes from pkcore. bowery does not evaluate hands or count
  runouts itself.

## Non-goals

- Pot odds, expected value, or any advice to the reader.
- Odds against unknown hands. There is no "your equity versus a random
  hand" number in the `seat` view.
- Outs for three or more live seats, or with two cards to come.
- A decimal place on the percent. pkcore's own precision table says a
  decimal is noise at its default sample count.
- Animations or transitions when the numbers change. The theme is CSS and
  may add them later; the renderer stays static SVG.

## Anti-goals (what would make this a failure even if it works)

- Giving a hidden hand away. If any live seat's cards are face down, no
  odds and no outs appear anywhere on the table.
- Slowing the table when odds are off. With the toggle off, no equity
  request runs.
- A different number on each reload of a replay.
- Overlapping text. Every new box goes through `draw::geometry::plan` and
  the disjoint-boxes proptest.

## Constraints

- pkcore `0.17.3` with the `equity` feature, which is already on. The
  engine is `pkcore::analysis::equity::EquityRequest::compute`. Exact when
  every seat is known and the runouts are at or under `exact_threshold`
  (flop: 990; turn: 44; river: 1). Preflop is Monte Carlo. Any `wasm32`
  build is serial: pkcore's table says about 600 ms for a six-way preflop
  request at the default 25,000 samples.
- The wasm size gate in the `Makefile` (`WASM_GATE`). The equity engine is
  already compiled in, so growth should be small. `make size` decides.
- `draw::svg` emits integer coordinates, attribute colours, no CSS, no
  script. Colour and emphasis belong to `js/bowery.css` and the
  `--bowery-*` properties.
- Text metrics: 11 px monospace is 6.6 px a character. Every new box must
  be sized for its widest text.
- `usize` is 32 bits on wasm32. Counts of runouts and samples stay in
  `u64` (pkcore's types already are).
- The MSRV and version rules in `CLAUDE.md` are untouched.

## The design, as decided

### What the reader sees

**1. A percent on each live plate.** In ring and spotlight, the plate's
right half (to the right of the two hole cards) is empty today. The
percent goes there: large, bold, right-aligned, with the same colour as the
plate's other text. The plate does not grow. In ledger, a new column
holds the same number. Folded seats and empty chairs show nothing. A seat
that is all in is live and shows a number. On the river the numbers are
100 and 0, or the split shares. Once the hand pays out (the award moment
and after), nothing shows.

**2. The outs bar.** It appears only when all of these hold: odds are on,
every live seat's cards are face up, exactly two seats are live, and the
board holds exactly four cards. It sits inside the oval, under the pot
line, in ring and spotlight; under the board row in ledger. Its content:

- A label line above the cards: the trailing player's name, the count and
  their percent, in the small grey style of a caption, for example
  `HANSEN · 8 OUTS · 18%`.
- One row of mini cards, the same size as hole cards (24 × 34), one per
  out, in deck order (ranks high to low, suits spades, hearts, diamonds,
  clubs within a rank, or whatever order `pkcore::card::Cards` yields when
  sorted). Up to 14 fit in the oval's width at a 680 px table; beyond
  that the row wraps to a second line. Every out carries `data-card`, like
  a hole card does.
- If the trailing player has no outs, the cards row is replaced by the
  text `drawing dead`.
- If the two seats are currently tied, there is no one behind and no bar.
- The leader never appears in the bar.

**Outs** are the river cards that give the trailing seat the pot alone. A
card that only splits the pot does not count and is not shown.

### When it shows

Odds are a table-level state like the view and the layout:

- `on` or `off`. The reader flips it with a `%` button beside 👁 and ▦.
- Default: on when the table's view is `all`, in replay and interactive
  mode. Off in the `seat` view, and the button is hidden there. Switching
  the view to `all` brings the button back and the odds with it, unless the
  reader had turned them off.
- Play mode starts with odds off, button shown. (Builder's call, 24 Oct:
  bots run on a timer, and a nine-way preflop step costs a 25,000-sample
  Monte Carlo, about a second of the browser's one thread; three such
  tables on one page starved the browser proof. Lowering the sample count
  would trade pkcore's whole-percent honesty for speed, so the reader opts
  in with `%` instead.)
- The author drops the button with `controls: { odds: false }` in the
  config. With the button dropped, odds are still on by default in the
  `all` view; the author who wants no odds at all sets `odds: false` and
  `view: seat`. (This mirrors how 👁 and ▦ work: `controls` says which
  buttons exist, not what the table shows.)

  **Open to the builder:** if that mirror reads wrong in practice, the
  simpler rule is "`controls.odds: false` means never show odds". Pick
  one, document it on the field, and test it.

### Where the math runs

In the kernel, when the frame is built. `Frame` gains:

```rust
pub struct Frame {
    // ... existing fields
    /// Win percent per live seat, or `None` when odds are off, a card is
    /// hidden, or the hand has paid out. Indexed like `seats`.
    pub odds: Option<Vec<Option<u8>>>,
    /// The outs bar, when its conditions hold.
    pub outs: Option<OutsBar>,
}

pub struct OutsBar {
    pub seat: u8,
    pub name: String,
    pub percent: u8,
    /// Empty means drawing dead.
    pub cards: Vec<Card>,
}
```

(Shapes are a sketch. The builder may fold `odds` into `SeatFrame` as
`odds: Option<u8>` instead; the frame-level `Option` exists so the renderer
can tell "off" from "this seat has none" without scanning.)

`Game::frame(view)` grows a sibling, or a parameter, that says whether to
compute odds. When off, no request runs. When on:

- **Percents.** One `EquityRequest` per frame: `PlayerSpec::Exact` for each
  live seat's two cards, the dealt board, and `EquityOptions` with
  `seed: Some(game seed)`. The game already owns a seed (the config's, or a
  hash of the config) for its `SmallRng`; the equity request uses the same
  one, so a replay is deterministic. `max_samples` stays at pkcore's
  default; the builder may lower it for preflop only if the demo feels
  slow, and must say so in the field docs. Round `equity_pct()` to the
  nearest whole number. Rounding may make the row sum to 99 or 101; that is
  accepted and not corrected.
- **Outs.** Only under the bar's conditions. Build a pkcore
  `play::Game` from the two seats' `HoleCards` and the four-card `Board`,
  take `turn_case_evals()`, and keep each river card whose `CaseEval`
  names exactly one winner, that winner being the trailing seat.
  `pkcore::analysis::outs::Outs::from(&case_evals)` counts tied cases as
  wins for every tied seat, so it is not used directly; filtering the case
  evals is using pkcore's evaluation, not re-implementing it. The trailing
  seat is the one with the lower percent from the same frame's equity
  request.
- **Folded cards (decided 24 October 2026).** pkcore `0.17.x` has no
  dead-card input: its equity engine knows only the live seats and the
  board, so a folded player's cards go back into the deck for the percents,
  and `turn_case_evals` can name one of them as a river card. The decision
  is to build now and accept the gap for the percents, with two rules:
  - The outs bar drops any river card that belongs to a folded seat. That
    is a filter on pkcore's result, not new math. So the bar never lists a
    card the reader saw face up in a folded hand, even though the percent
    beside it still counts that card as live.
  - The field docs on the odds say the percents treat folded cards as
    unknown until pkcore ships dead cards.
  pkcore [EPIC-123 Dead Cards](https://github.com/ImperialBower/pkcore/blob/main/docs/epics/EPIC-123_Dead_Cards.md)
  adds `EquityRequest::dead` in `0.18.0`. When bowery pins that release,
  it passes every folded seat's two cards as `dead`, removes the by-hand
  filter, and re-blesses the golden files whose percents move.
- **One live seat left** (everyone else folded, before the award): no
  percent shows. pkcore needs two hands, and 100% tells the reader nothing.
- **Caching is required, not optional.** `js/bowery.js::_render` builds the
  frame three times per render (`state()`, `svg()`, and the pacing check),
  so without a memo an eight-way preflop step would run three Monte Carlo
  passes, two to three seconds in the browser. The memo below lands before
  odds are on by default.
- **Caching.** `Table` (in `element.rs`) renders the same moment many
  times (every control press re-renders). The builder should memoise the
  odds per `(moment index, live set)` or per frame, so a seek back and
  forth does not recompute a preflop Monte Carlo run. A simple
  `HashMap<usize, (Vec<Option<u8>>, Option<OutsBar>)>` keyed by the moment
  index, cleared when the game goes off the record or a new hand starts,
  is enough.

### Drawing

- `draw::geometry::plan` gains a box for the percent inside each live
  plate, and a box for the bar under the pot. Both are measured for their
  widest text (`100%` is four characters at the chosen size; the bar's
  label at 10 px; the cards row at 27 px per card). The proptest
  `geometry__boxes_disjoint` is extended with odds on and the bar present.
- **The ring grows for the bar (decided 24 October 2026).** The ring's
  vertical stack is fixed: top bet slots, board, pot, bottom bet slots,
  bottom plates, with the oval 196 px tall. The bet slots are reserved
  every moment and counted by `Plan::boxes()`, so there is no free room
  under the pot when the top or bottom row holds a middle plate. The
  accepted mockup had no middle plates, which is why it looked like it fit.
  So: when the bar is present, the oval grows by the bar's height (label
  plus one or two card rows) and the bottom bet slots, bottom plates and
  caption shift down by the same amount. The canvas height changes between
  moments, as spotlight's already does with its folded strip. The board and
  pot do not move. Shifting them up, as the mockup did, would land the
  board on the top-centre bet slot.
- `draw::svg` draws the percent as `<text class="odds">` inside the seat
  group, and the bar as `<g class="outs">` with a `<text class="label">`
  and a `<g class="card">` per out, each with `data-card`. Colours are
  attributes as today; `js/bowery.css` gets rules for `.odds`, `.outs
  .label` and `.outs .card`.
- Ledger: an `odds` column after the cards column; the bar as a row under
  the board row.
- Spotlight: same as ring.

### Element and JS

- `element::Controls` gains `odds: bool` (the button exists); `State` gains
  `odds: bool` (on or off now). `Table` gains `set_odds(on: bool)` and the
  wasm wrapper mirrors it.
- `js/bowery.js` adds the `%` button next to 👁 and ▦ with
  `aria-label="odds"`, labelled `% on` / `% off` like the other toggles.
- `config::Controls` gains `odds: Option<bool>`; `check` has nothing new to
  refuse.
- `examples/bowery-still.rs` gains `--odds` (on) so a still of The Hand at
  the turn shows the bar.

### Tests

- Golden files: add The Hand at the **turn** with odds on in all three
  layouts and both themes (six files). At the turn of The Hand, Hansen
  holds quad fives and Negreanu a full house, so the bar reads `drawing
  dead`. Add one more fixture hand, or a config with a stacked deck, where
  the trailing seat has outs, so the cards row is covered in at least one
  golden. Update `draw__golden_files_are_the_twenty_four` to the new count.
- `draw__seat_view_draws_no_hidden_card` gains a twin: with odds on and
  the view `seat`, the SVG holds no `class="odds"` and no `class="outs"`.
- Frame tests: a fold changes the other seats' percents; the bar is absent
  with three live seats, with three board cards, with five, and at a tie;
  the bar is present with two live seats and four board cards; drawing
  dead renders as text; percents are `None` after the award; percents are
  `None` when odds are off.
- Determinism: the same config twice gives the same preflop percents, on
  the same target. **Across targets the preflop numbers differ (decided 24
  October 2026):** pkcore seeds `SmallRng` with `seed_from_u64`, which gives
  a different stream on 32-bit (wasm32) and 64-bit, as `Game::new`'s docs
  already note for the bots. Flop, turn and river are exact and agree
  everywhere. So: no test, golden file comparison in the browser, guide page
  or `.okf` note quotes a preflop percent. The golden SVGs are native and
  may hold preflop numbers; the Playwright suite asserts only that a
  percent is present preflop, and asserts values only from the flop on.
  The guide pages say the preflop figure is sampled and may differ from a
  still.
- Playwright: the `%` button toggles `.odds` text on and off; the button is
  absent in the `seat` view and returns on 👁.
- `make ayce` passes, including `make size`.

### Docs

- Field docs on every new field and method, with doc tests, per
  `CLAUDE.md`.
- `demo/guide/` pages mention the `%` button; the e2e guide spec keeps the
  shown configs honest.
- `.okf/concepts/frame-and-draw.md` and `element.md` gain a paragraph each;
  `.okf/log.md` gets an entry.
- `docs/EPIC-110_Bowery.md` gains a phase row, or a new EPIC-111 is opened
  if the bower EPIC-20 contract is amended to cover it. The builder asks
  the author which.

## Left to the builder

- Exact font sizes and offsets for the percent inside the plate (the mockup
  used 18 px bold at the plate's right edge, baseline near the cards'
  middle).
- Whether `odds` lives on `Frame` or `SeatFrame`.
- The cache's key and lifetime (the cache itself is required; see above).
- Whether to lower `max_samples` for preflop in the browser.
- Which EPIC document records the work.

## Builder check, 24 October 2026

A reviewer read the first draft and raised four questions. All four are now
answered in the text above: the ring grows for the bar; folded cards are
accepted as live for the percents and filtered from the bar until pkcore
EPIC-123 ships; preflop numbers are per-target and never quoted; one live
seat shows nothing. The reviewer also noted that bower does not yet depend
on bowery, so `Game::frame`'s signature is bowery-internal to change.

## Accepted mockups

The brainstorm's mockups are in `.superpowers/brainstorm/24148-1792865126/content/`:
`odds-placement.html` (option A accepted: percent inside the plate),
`outs-bar.html` (option A accepted: bar inside the oval under the pot).
`outs-cards.html` was rejected: outs do not belong to the plate.
