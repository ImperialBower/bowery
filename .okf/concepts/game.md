---
type: concept
title: Game
description: How a bowery Game holds a hand as moments, goes off the record, and gets every hand's end from pkcore.
tags: [game, pkcore, replay, epic-20]
timestamp: '2026-10-22T00:00:00Z'
---

# Game

`src/game.rs`. A `Game` is a line of moments, each an `Event` and the pkcore
`Table` after it. It is built from a `Checked` config (see
[config and check](config-and-check.md)) and owns all its state.

- **On the record** the line is the cursor's events, materialised once in
  `open_record`. `seek` and `advance` only move `at`.
- **Off the record** a `Live` holds a `PokerSession` built from the cursor's
  table at the takeover. Its deck is the hand's own, so the board stays the
  record's. Bots act on the game's `SmallRng`.
- **Play** is a `Live` with no record; `next_hand` calls `end_hand`, moves
  the button to the next occupied chair and shuffles on the same `SmallRng`.

## The end of a hand

pkcore's return/showdown/award logic is private to `Replay`. When a live hand
completes, `settle` rebuilds it as a `HandHistory` and takes those last
moments from `cursor()`. It also checks the rebuilt hand has exactly as many
moments before its end as the game played.

- The streets come from `Streets::from_event_log` over the table's own log:
  all of it off the record, the hand's part in play. This needs pkcore
  0.17.3, where a replayed table logs the streets the record laid
  (pkcore DEFECT_041, found by bowery). Before it the log had no flop entry
  and the rebuild misfiled every later action.
- The log holds a flop as a `Bard`, a set, so `from_event_log` returns it
  sorted. `settle` restores the deal order from `table.board`, which keeps
  it; otherwise the flop would reorder at the hand's last moments
  (`game__board_keeps_its_deal_order_to_the_end`).
- The rebuilt hand has no `board`: the record's may be shorter than the hand
  played off it, and the streets hold every card anyway.
