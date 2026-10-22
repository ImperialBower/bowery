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
