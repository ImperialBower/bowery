# Backlog

> Maintained by the `/backlog` skill. Refreshed 2026-10-23. Debt detail lives
> in [docs/TECHNICAL_DEBT.md](docs/TECHNICAL_DEBT.md). Items tagged 🤖 are
> machine-suggested: keep, edit or delete them.

## Recently completed

- **EPIC-110 / bower EPIC-20, Phases 0–6.** Shipped: v0.1.0 (22 October
  2026) on crates.io, npm (with provenance) and GitHub. See
  [docs/EPIC-110_Bowery.md](docs/EPIC-110_Bowery.md).
- **v0.1.1** (22 October 2026): trusted publishing (OIDC) only; no stored tokens.
- **v0.1.2** (prepared 23 October 2026): the four review bugs above, and
  `release.yml` pinned. Tag to publish.
- **Release check** (`2f9e919`): `scripts/dist.sh` refuses a README whose CDN
  link is not the crate version. In `CHANGELOG.md` under Unreleased.

## Next — consumers of the release

- [ ] **bower EPIC-21 — the ```` ```bowery ```` block.** All phases Planned;
  its only blocker was bowery `v0.1.0`, now shipped. The work is in bower,
  not here, but it is the first real consumer and will find bowery's API
  gaps. ([bower `docs/EPIC-21_Bowery_Block.md`](https://github.com/ImperialBower/bower/blob/main/docs/EPIC-21_Bowery_Block.md))

## Features — named follow-ons (bower EPIC-20 § "What this EPIC does not do")

None is committed. EPIC-111–119 are free for them (pkcore `ROADMAP.md`).

- [ ] **pkarena0-web's own table on bowery.** "A possible follow-on, not a goal."
- [ ] **Omaha and stud tables.** Hold'em only today; "come later".
- [ ] **A hand log.** Out of v0.1 scope, with sound, animation, chip graphics,
  avatars and extra themes.

## Release housekeeping

- [ ] **Ship the Unreleased changelog entry.** Tooling only (the README check);
  it can ride along with the next real change rather than a release of its own.

## Tech debt

One tracked item: `SmallRng` deals differently on wasm32 and 64-bit targets
for the same seed (`src/game.rs:359`, `src/game.rs:429`).

🤖 The 2026-10-23 review pass added 25 findings. These five were checked
against the code by hand:

- [x] 🤖 Keyboard seek eats arrow keys in the bet box and seat picker (`js/bowery.js:309`).
- [ ] 🤖 `next_hand` mutates, then can fail (`src/game.rs:1087`). Checked: not observable today; test added.
- [x] 🤖 `table:` override is documented and checked but never applied (`src/game.rs:1201`).
- [x] 🤖 `check` misses play rules: button, blinds, zero stacks (`src/check.rs:189`).
- [x] 🤖 `actions/*` unpinned in the publishing jobs (`.github/workflows/release.yml:53`).

See [docs/TECHNICAL_DEBT.md](docs/TECHNICAL_DEBT.md).

## Code comments

No `TODO`, `FIXME`, `XXX`, `HACK` or `BUG` markers in the tree.

## Issues

No open GitHub issues.
