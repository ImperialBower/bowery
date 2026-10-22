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

None. The 2026-10-21 creation skipped the deep review: there is no kernel code
to review yet. Run `/backlog` again after EPIC-20 Phase 1 and ask for the
review pass.
