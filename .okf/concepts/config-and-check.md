---
type: concept
title: Config and check
description: How bowery reads an author's YAML and refuses a config that is not true; the pkcore facts the rules depend on.
tags: [config, check, pkcore, epic-20]
timestamp: '2026-10-21T00:00:00Z'
---

# Config and check

`src/config.rs` reads the author's YAML into a `Config` (unknown fields are
refused). `src/check.rs` turns it into a `Checked`, or returns every
`Problem` at once. The rule list is in `check.rs`'s module docs; each rule has
a failing fixture under `tests/config/`, and `check__every_fixture_has_a_test`
fails if a fixture has no `check__<name>` test.

- The hand is checked by pkcore's `HandHistory::validate` only. Its findings
  come back as `Problem::Hand`, with paths relative to `hand` (`Display`
  adds `hand.`).
- `start` is looked up in pkcore's replay cursor (`Moment::index_in`) only
  when the hand validates clean.

## pkcore facts the rules depend on (0.17.2)

- `Cards::from_str` collects into a set, so a repeated card is dropped
  silently. The deck rule counts the tokens itself, splitting on whitespace,
  `,` and `-` as pkcore's `Terminal::index_cleaner` does.
- `Cards::DECK_SIZE` is private; `check.rs` has its own `DECK_SIZE`.
- `data/hands/the_hand.yaml` is not in the published crate. `fixtures/the_hand.yaml`
  is a copy from the pkcore repo at tag `v0.17.2`; re-copy it when pkcore
  re-records The Hand.
- `tests/hands/DEFECT_040_the_hand_before.yaml` starts with a `---` document
  marker, so it cannot be inlined by indenting its lines. Tests set
  `config.hand` from `HandHistory::from_yaml` instead.

See the [quality gate](../processes/quality-gate.md).
