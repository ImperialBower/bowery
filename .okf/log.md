# Update Log

## 2026-10-21
* **Creation**: Repository scaffolded by dev-playbook (rust); bundle seeded with `okf_init.py` — see [getting started](getting-started.md), [toolchain](decisions/toolchain.md), [quality gate](processes/quality-gate.md).
* **Update**: `make wasm` (EPIC-20 item 0d) joins the [quality gate](processes/quality-gate.md) and CI; `bin/security-scan` was hidden by a global excludes file, so `.gitignore` re-includes `/bin/`. `docs/EPIC-110_Bowery.md` points to bower EPIC-20 (item 0b).
* **Update**: EPIC-20 Phase 1 — `Config` and `check` land, with pkcore 0.17.2 (`hand-histories`, `bot-profiles`, `equity`) and `serde_yaml_bw`; `deny.toml` allows BSD-3-Clause and Unicode-3.0 for them. See [config and check](concepts/config-and-check.md).

## 2026-10-22
* **Update**: EPIC-20 Phase 2 — `Game`. See [game](concepts/game.md). A check rule added: play's stakes are whole chips.
* **Update**: pkcore 0.17.3 (DEFECT_041, found here) lets `settle` rebuild a hand from the whole table log; the cut-and-continue workaround is gone, and the rebuilt flop keeps its deal order. See [game](concepts/game.md).
* **Update**: EPIC-20 Phase 3 — `Frame`, `draw::geometry` (disjoint-boxes proptest), `draw::svg` in three layouts with 24 golden SVGs, `bowery-still`. See [frame and draw](concepts/frame-and-draw.md).
* **Update**: EPIC-20 Phase 4 — `element` and `wasm` behind the `wasm` feature, the size profile and gate (1,295,901 bytes; gate 1,425,491), `js/bowery.js`, `js/bowery.css`, the demo. See [the element](concepts/element.md).
* **Update**: EPIC-20 Phase 5 — Playwright in `tests/e2e/` (D1–D8, multi, origin, fallback); found and fixed a first-frame width bug. See [browser proof](concepts/browser-proof.md).
* **Update**: `.tool-versions` was hidden by the same global excludes file as `bin/`; `.gitignore` re-includes it (CI `Browser proof` failed on `setup-node`).
* **Update**: EPIC-20 Phase 6a prepared — `scripts/dist.sh`, `release.yml`, `npm/package.json`, `CHANGELOG.md`, versions at 0.1.0; nothing published. See [release](processes/release.md).
* **Update**: v0.1.0 released (crates.io, npm with provenance, GitHub). npm trusted publishing needed "Allow npm publish" ticked; see [release](processes/release.md).
* **Update**: v0.1.1 released, the first with no stored tokens. `scripts/dist.sh` now also checks that the README's CDN link names the crate version; the link stays exact. See [release](processes/release.md).

## 2026-10-23
* **Update**: `/backlog` review pass after v0.1.1: 25 machine-suggested findings in `docs/TECHNICAL_DEBT.md` (🤖), five checked by hand — keyboard seek in the shadow DOM, non-atomic `next_hand`, the unapplied `table:` override, missing play rules in `check`, unpinned `actions/*` in `release.yml`.
* **Update**: three review bugs fixed test-first — keyboard seek in the shadow DOM (`composedPath`), `check` play rules (button on the table, blinds and stacks at least 1, big ≥ small), `check` refuses `table` beside a recorded hand. `next_hand` non-atomicity probed: not observable; covered by a test. See [config and check](concepts/config-and-check.md).
* **Update**: v0.1.2 prepared: `release.yml` pins `actions/*` to commit SHAs (checkout v7.0.1, upload-artifact v4.6.2, download-artifact v4.3.0, setup-node v4.4.0); versions at 0.1.2. See [release](processes/release.md).
* **Update**: the demo goes to GitHub Pages (`.github/workflows/pages.yaml`, from `main`, after `make e2e`; `site/e2e/` left out; actions pinned). Needs Settings → Pages → Source: GitHub Actions. See [release](processes/release.md).

## 2026-10-24
* **Update**: `js/bowery.css` carries a colour theme (paper light, slate "night" dark) that recolours the renderer's SVG by class; the SVG and golden files are untouched. See [frame and draw](concepts/frame-and-draw.md).
* **Update**: the demo page has a theme switch (system / paper / night). It only sets `light` or `coal` on `<html>`: `prefersDark()` in `js/bowery.js` reads that class and a `MutationObserver` redraws started tables; tables not yet started (lazy, `IntersectionObserver`) pick the class up when they start. See [frame and draw](concepts/frame-and-draw.md).
* **Update**: a guide page per demo table (`demo/guide/`, served at `guide/*.html`), linked from the demo and the README; `guide.spec.ts` keeps each shown config equal to the one its table runs, and `dist.sh` checks the guides' CDN pins. See [the element](concepts/element.md).
* **Update**: odds and outs — each live seat shows its win percent from pkcore's equity engine, and with two seats left on the turn an outs bar lists the trailing seat's river cards. `Game::frame_with_odds`, `src/odds.rs`, `controls.odds`, a `%` button, `bowery-still --odds`; on by default in replay and interactive, off in play. Folded cards still count as live in the percents until pkcore EPIC-123 (0.18.0). See [frame and draw](concepts/frame-and-draw.md) and [the element](concepts/element.md).
* **Update**: odds and outs, final-review fixes — ledger rows go one line from `LEDGER_WIDE` (464 px) instead of `WIDE`, because the percent overprinted the stack from 420 to 463 px; ▶ and taking a seat no longer empty the odds cache, and an action keeps the moments before it. See [frame and draw](concepts/frame-and-draw.md) and [the element](concepts/element.md).

## 2026-10-25
* **Update**: pkcore 0.18.0 — the percents pass folded seats' cards as `EquityRequest::dead` (EPIC-123). The Hand's turn reads Negreanu 3% (was 2%), `three_way_outs` Draw 17% (was 18%); goldens re-blessed. EPIC-110 Phase 7 item 7b. See [frame and draw](concepts/frame-and-draw.md).
* **Update**: pkcore 0.20.0 — the outs bar passes the same cards to `Game::with_dead` (pkcore DEFECT_042, found here, fixed in 0.19.0) and the by-hand filter in `outs_bar` is gone. Same cards on the bar, so no golden moved. EPIC-110 Phase 7 item 7c. See [frame and draw](concepts/frame-and-draw.md).
* **Update**: `src/odds.rs` has no free functions: `odds::compute` is `Odds::compute` (breaking; next release 0.3.0), `outs_bar` and `trailing` are `OutsBar::at_turn` and `OutsBar::trailing`, the other helpers sit on `Odds`, and the test helpers on a test-only `Fixture`. See [frame and draw](concepts/frame-and-draw.md).
* **Update**: no free functions in `frame.rs`, `element.rs`, `tests/odds.rs` or `tests/draw.rs` either: `frame::format_chips` is `Frame::format_chips`, `element::problems_json` and `element::version` are `element::Table::problems_json` and `Table::version` (all breaking), the private helpers sit on `Frame`, and each test file's helpers on its own `Fixture`. The JS surface is unchanged. See [frame and draw](concepts/frame-and-draw.md) and [the element](concepts/element.md).
* **Update**: the crate has no free functions left (only the example's `main`): `check::check` is `Checked::new`, `draw::svg` is `Renderer::svg` (a new unit struct), `geometry::plan` / `outs_per_row` / `strip_lines` are `Plan::new` / `Plan::outs_per_row` / `Plan::strip_lines`, and `bowery::version` is `Bowery::version` (a new unit struct). All breaking, for 0.3.0; bower does not depend on bowery yet. See [config and check](concepts/config-and-check.md) and [frame and draw](concepts/frame-and-draw.md).
