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
