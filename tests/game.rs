//! One table's game: replay, takeover, play, and the end of a hand (D1–D5).
#![allow(non_snake_case)]

use std::fs;

use bowery::check::Checked;
use bowery::config::Config;
use bowery::game::{Advance, Event, Game, GameError, RecordState};
use pkcore::cards::Cards;
use pkcore::casino::action::PlayerAction;
use pkcore::casino::table::Table;
use pkcore::hand_history::{HandHistory, ReplayStreet};

const NEGREANU: u8 = 4;
const HANSEN: u8 = 3;

/// Games, configs and the reader's moves for the tests.
struct Fixture;

impl Fixture {
    fn game(config: &Config) -> Game {
        Game::new(Checked::new(config).unwrap()).unwrap()
    }

    /// The config in `head` with the hand history at `hand_path` as its `hand`.
    fn with_hand(head: &str, hand_path: &str) -> Config {
        let mut config = Config::from_yaml(head).unwrap();
        config.hand =
            Some(HandHistory::from_yaml(&fs::read_to_string(hand_path).unwrap()).unwrap());
        config
    }

    fn the_hand(head: &str) -> Game {
        Self::game(&Self::with_hand(head, "fixtures/the_hand.yaml"))
    }

    fn play(yaml: &str) -> Game {
        Self::game(&Config::from_yaml(yaml).unwrap())
    }

    /// The reader's simplest line: check when it can, call when it must.
    fn check_or_call(game: &Game) -> PlayerAction {
        if game.legal().contains(&PlayerAction::Check) {
            PlayerAction::Check
        } else {
            PlayerAction::Call
        }
    }

    /// Advances to the end of the hand, acting `reader` whenever it is the
    /// reader's turn.
    fn finish(game: &mut Game, reader: fn(&Game) -> PlayerAction) {
        for _ in 0..500 {
            match game.advance().unwrap() {
                Advance::Moved => {}
                Advance::Waiting => game.act(reader(game)).unwrap(),
                Advance::Ended => return,
            }
        }
        panic!("the hand did not end in 500 steps");
    }

    /// Every moment of the game, from the first.
    fn moments(game: &mut Game) -> Vec<(Event, Table)> {
        (0..game.moments())
            .map(|index| {
                game.seek(index).unwrap();
                (game.event().clone(), game.table().clone())
            })
            .collect()
    }
}

// ── Replay ─────────────────────────────────────────────────────────────────

#[test]
fn game__replay_frames_are_cursor_events() {
    let mut game = Fixture::the_hand("mode: replay\n");
    let hand =
        HandHistory::from_yaml(&fs::read_to_string("fixtures/the_hand.yaml").unwrap()).unwrap();
    let mut replay = hand.cursor().unwrap();

    assert_eq!(replay.len(), game.moments());
    for index in 0..replay.len() {
        replay.seek(index).unwrap();
        game.seek(index).unwrap();
        assert_eq!(
            Event::from_replay(replay.event(index).unwrap()),
            Some(game.event().clone())
        );
        assert_eq!(replay.table(), game.table(), "moment {index}");
    }
    assert_eq!(RecordState::OnRecord, game.record_state());
}

#[test]
fn game__replay_opens_at_start() {
    let game = Fixture::the_hand("mode: replay\nstart: flop\n");

    assert_eq!(Event::Deal(ReplayStreet::Flop), *game.event());
    assert_eq!(10, game.at());
}

#[test]
fn game__replay_advance_moves_then_ends() {
    let mut game = Fixture::the_hand("mode: replay\nstart: 23\n");

    assert_eq!(Advance::Moved, game.advance().unwrap());
    assert_eq!(24, game.at());
    assert_eq!(Advance::Ended, game.advance().unwrap());
    assert_eq!(24, game.at());
}

#[test]
fn game__replay_cannot_take_a_seat() {
    let mut game = Fixture::the_hand("mode: replay\nstart: flop\n");

    assert_eq!(Err(GameError::NotInteractive), game.take_seat(NEGREANU));
}

#[test]
fn game__seek_out_of_range() {
    let mut game = Fixture::the_hand("mode: replay\n");

    assert_eq!(
        Err(GameError::OutOfRange {
            index: 25,
            moments: 25
        }),
        game.seek(25)
    );
}

#[test]
fn game__replay_has_no_one_to_act_for_the_reader() {
    let game = Fixture::the_hand("mode: replay\nstart: flop\n");

    assert_eq!(Some(HANSEN), game.to_act());
    assert_eq!(Vec::<PlayerAction>::new(), game.legal());
}

// ── Interactive ────────────────────────────────────────────────────────────

#[test]
fn game__take_seat_on_record_then_off() {
    let mut game = Fixture::the_hand("mode: interactive\nstart: flop\n");
    game.take_seat(NEGREANU).unwrap();

    assert_eq!(Advance::Moved, game.advance().unwrap()); // Hansen checks, on the record
    assert_eq!(Advance::Waiting, game.advance().unwrap()); // Negreanu bet 8,000 here
    assert_eq!(
        vec![
            PlayerAction::Check,
            PlayerAction::Bet(600),
            PlayerAction::AllIn
        ],
        game.legal()
    );

    game.act(PlayerAction::Check).unwrap();

    assert_eq!(RecordState::OffRecord { since: 11 }, game.record_state());
    assert_eq!(
        Event::Action {
            seat: NEGREANU,
            action: PlayerAction::Check,
            street: ReplayStreet::Flop
        },
        *game.event()
    );
}

#[test]
fn game__matching_action_stays_on_record() {
    let mut game = Fixture::the_hand("mode: interactive\nstart: flop\n");
    game.take_seat(NEGREANU).unwrap();
    game.advance().unwrap();

    game.act(PlayerAction::Bet(8_000)).unwrap();

    assert_eq!(RecordState::OnRecord, game.record_state());
    assert_eq!(12, game.at());
}

#[test]
fn game__takeover_board_is_the_real_board() {
    let mut game = Fixture::the_hand("mode: interactive\nstart: flop\n");
    game.take_seat(NEGREANU).unwrap();

    Fixture::finish(&mut game, Fixture::check_or_call);

    assert!(matches!(game.record_state(), RecordState::OffRecord { .. }));
    assert_eq!("9♣ 6♦ 5♥ 5♠ 8♠", game.table().board.to_string());
}

#[test]
fn game__back_to_record_returns_to_takeover() {
    let mut game = Fixture::the_hand("mode: interactive\nstart: flop\n");
    game.take_seat(NEGREANU).unwrap();
    game.advance().unwrap();
    game.advance().unwrap();
    game.act(PlayerAction::Check).unwrap();

    game.back_to_record();

    assert_eq!(RecordState::OnRecord, game.record_state());
    assert_eq!(11, game.at());
    assert_eq!(25, game.moments());
}

#[test]
fn game__take_seat_refuses_a_folded_seat() {
    let mut game = Fixture::the_hand("mode: interactive\nstart: flop\n");

    assert_eq!(Err(GameError::SeatNotLive { seat: 0 }), game.take_seat(0));
}

#[test]
fn game__take_seat_refuses_after_the_hand() {
    let mut game = Fixture::the_hand("mode: interactive\nstart: 24\n");

    assert_eq!(Err(GameError::HandIsOver), game.take_seat(HANSEN));
}

#[test]
fn game__act_out_of_turn_is_refused() {
    let mut game = Fixture::the_hand("mode: interactive\nstart: flop\n");
    game.take_seat(NEGREANU).unwrap();

    assert_eq!(Err(GameError::NotYourTurn), game.act(PlayerAction::Check));
}

// ── Play ───────────────────────────────────────────────────────────────────

const HEADS_UP_PLAY: &str = "\
mode: play
seat: 0
table: { seats: 2, button: 0, stakes: { small_blind: 50.0, big_blind: 100.0 } }
players:
  - { seat: 0, name: Reader, stack: 1000 }
  - { seat: 1, name: Robo, stack: 1000, bot: abc }
";

const DECK: &str = "A♠ K♠ A♥ K♥ Q♠ J♠ T♠ 9♠ 8♠ 7♠ 6♠ 5♠ 4♠ 3♠ 2♠ Q♥ J♥ T♥ 9♥ 8♥ 7♥ 6♥ 5♥ 4♥ 3♥ 2♥ A♦ K♦ Q♦ J♦ T♦ 9♦ 8♦ 7♦ 6♦ 5♦ 4♦ 3♦ 2♦ A♣ K♣ Q♣ J♣ T♣ 9♣ 8♣ 7♣ 6♣ 5♣ 4♣ 3♣ 2♣";

#[test]
fn game__play_first_hand_from_deck() {
    let game = Fixture::play(&format!("{HEADS_UP_PLAY}deck: {DECK}\n"));

    assert_eq!(Event::Forced, *game.event());
    assert_eq!(RecordState::NotAReplay, game.record_state());
    let seat = |index: u8| {
        game.table()
            .seats
            .get_seat(index)
            .unwrap()
            .cards
            .to_string()
    };
    let dealt = format!("{} {}", seat(0), seat(1));
    assert_eq!(4, dealt.split_whitespace().count());
    for card in dealt.split_whitespace() {
        assert!(
            "A♠ K♠ A♥ K♥".contains(card),
            "{card} is not from the top of the deck"
        );
    }
}

#[test]
fn game__play_waits_for_the_reader() {
    let mut game = Fixture::play(HEADS_UP_PLAY);

    assert_eq!(Advance::Waiting, game.advance().unwrap());
    assert_eq!(Some(0), game.to_act());
    assert!(game.legal().contains(&PlayerAction::Fold));
}

#[test]
fn game__play_is_the_same_twice() {
    let mut first = Fixture::play(HEADS_UP_PLAY);
    let mut second = Fixture::play(HEADS_UP_PLAY);

    Fixture::finish(&mut first, Fixture::check_or_call);
    Fixture::finish(&mut second, Fixture::check_or_call);

    let events = |game: &mut Game| {
        Fixture::moments(game)
            .into_iter()
            .map(|(event, _)| event)
            .collect::<Vec<_>>()
    };
    assert_eq!(events(&mut first), events(&mut second));
    assert_eq!(first.table(), second.table());
}

#[test]
fn game__next_hand_before_the_end_is_refused() {
    let mut game = Fixture::play(HEADS_UP_PLAY);

    assert_eq!(Err(GameError::HandNotOver), game.next_hand());
}

#[test]
fn game__next_hand_moves_the_button_and_carries_stacks() {
    let mut game = Fixture::play(HEADS_UP_PLAY);
    Fixture::finish(&mut game, |_| PlayerAction::Fold);
    let after: Vec<usize> = (0..2)
        .map(|seat| game.table().seats.get_seat(seat).unwrap().player.chips)
        .collect();
    assert_eq!(vec![950, 1050], after);

    game.next_hand().unwrap();

    assert_eq!(2, game.hand_number());
    assert_eq!(1, game.table().button);
    assert_eq!(0, game.at());
    assert_eq!(Event::Forced, *game.event());
    let committed = |seat: u8| {
        let player = &game.table().seats.get_seat(seat).unwrap().player;
        player.chips + player.bet
    };
    assert_eq!(vec![950, 1050], vec![committed(0), committed(1)]);
}

impl Fixture {
    /// Heads-up for 2 chips each at 1/2: the big blind is all in from the post,
    /// the reader calls, and the aces take every chip, so the table is broken.
    fn broken_table() -> Game {
        let front = [
            "A♠", "7♦", "A♥", "2♣", "K♥", "Q♦", "9♣", "5♠", "3♥", "8♦", "4♣", "J♥",
        ];
        let rest: Vec<String> = Cards::deck()
            .to_string()
            .split_whitespace()
            .filter(|card| !front.contains(card))
            .map(str::to_string)
            .collect();
        let deck = format!("{} {}", front.join(" "), rest.join(" "));
        let mut game = Self::play(&format!(
            "mode: play\nseat: 0\n\
         table: {{ seats: 2, button: 0, stakes: {{ small_blind: 1.0, big_blind: 2.0 }} }}\n\
         players:\n  - {{ seat: 0, stack: 2 }}\n  - {{ seat: 1, stack: 2, bot: abc }}\n\
         deck: {deck}\n"
        ));
        Self::finish(&mut game, Self::check_or_call);
        let stacks: Vec<usize> = (0..2)
            .map(|seat| game.table().seats.get_seat(seat).unwrap().player.chips)
            .collect();
        assert!(stacks.contains(&0), "nobody busted: {stacks:?}");
        game
    }
}

#[test]
fn game__next_hand_on_a_broken_table_changes_nothing() {
    let mut game = Fixture::broken_table();
    let before = (
        game.table().clone(),
        game.moments(),
        game.at(),
        game.hand_number(),
    );

    assert_eq!(Err(GameError::TableBroken), game.next_hand());
    assert_eq!(
        before,
        (
            game.table().clone(),
            game.moments(),
            game.at(),
            game.hand_number()
        )
    );
    assert_eq!(Err(GameError::TableBroken), game.next_hand());
}

#[test]
fn game__next_hand_only_in_play() {
    let mut game = Fixture::the_hand("mode: replay\nstart: 24\n");

    assert_eq!(Err(GameError::NotPlay), game.next_hand());
}

// ── The end of a hand: D1–D5, on and off the record ───────────────────────

impl Fixture {
    /// Heads-up, both all in preflop, the short stack wins: every moment, on the
    /// record (a replay) and off it (the short stack calls where the record
    /// says all in, so the run-out is the game's own).
    fn all_in_lines() -> Vec<(&'static str, Vec<(Event, Table)>)> {
        let mut on = Self::game(&Self::with_hand(
            "mode: replay\n",
            "tests/hands/heads_up_all_in.yaml",
        ));
        let mut off = Self::game(&Self::with_hand(
            "mode: interactive\nstart: 1\n",
            "tests/hands/heads_up_all_in.yaml",
        ));
        off.take_seat(1).unwrap();
        assert_eq!(Advance::Waiting, off.advance().unwrap());
        off.act(PlayerAction::Call).unwrap();
        assert!(matches!(off.record_state(), RecordState::OffRecord { .. }));
        Self::finish(&mut off, Self::check_or_call);
        vec![
            ("on the record", Self::moments(&mut on)),
            ("off the record", Self::moments(&mut off)),
        ]
    }

    fn bets(table: &Table) -> usize {
        (0..2)
            .map(|seat| table.seats.get_seat(seat).unwrap().player.bet)
            .sum()
    }
}

#[test]
fn game__d1_every_street_is_its_own_frame() {
    for (line, moments) in Fixture::all_in_lines() {
        let deals: Vec<(ReplayStreet, usize)> = moments
            .iter()
            .filter_map(|(event, table)| match event {
                Event::Deal(street) => Some((*street, table.board.len())),
                _ => None,
            })
            .collect();
        assert_eq!(
            vec![
                (ReplayStreet::Flop, 3),
                (ReplayStreet::Turn, 4),
                (ReplayStreet::River, 5)
            ],
            deals,
            "{line}"
        );
    }
}

#[test]
fn game__d2_cards_are_up_at_showdown() {
    for (line, moments) in Fixture::all_in_lines() {
        let (_, table) = moments
            .iter()
            .find(|(event, _)| *event == Event::Showdown)
            .unwrap_or_else(|| panic!("{line}: no showdown"));
        assert_eq!(
            "7♦ 2♣",
            table.seats.get_seat(0).unwrap().cards.to_string(),
            "{line}"
        );
        assert_eq!(
            "A♠ A♥",
            table.seats.get_seat(1).unwrap().cards.to_string(),
            "{line}"
        );
    }
}

#[test]
fn game__d3_the_award_is_the_pot() {
    for (line, moments) in Fixture::all_in_lines() {
        let (_, showdown) = moments
            .iter()
            .find(|(event, _)| *event == Event::Showdown)
            .unwrap();
        let awarded: usize = moments
            .iter()
            .filter_map(|(event, _)| match event {
                Event::Award { amount, .. } => Some(*amount),
                _ => None,
            })
            .sum();
        assert_eq!(showdown.pot + Fixture::bets(showdown), awarded, "{line}");
        assert_eq!(400, awarded, "{line}");
    }
}

#[test]
fn game__d4_no_bets_or_pot_after_the_award() {
    for (line, moments) in Fixture::all_in_lines() {
        let (event, last) = moments.last().unwrap();
        assert!(matches!(event, Event::Award { .. }), "{line}: {event:?}");
        assert_eq!(0, last.pot, "{line}");
        assert_eq!(0, Fixture::bets(last), "{line}");
        let stacks: Vec<usize> = (0..2)
            .map(|seat| last.seats.get_seat(seat).unwrap().player.chips)
            .collect();
        assert_eq!(vec![800, 400], stacks, "{line}");
    }
}

#[test]
fn game__d5_nobody_to_act_after_the_hand() {
    let mut game = Fixture::game(&Fixture::with_hand(
        "mode: interactive\n",
        "tests/hands/heads_up_all_in.yaml",
    ));
    game.seek(game.moments() - 1).unwrap();

    assert_eq!(None, game.to_act());
    assert_eq!(Vec::<PlayerAction>::new(), game.legal());
    assert_eq!(Advance::Ended, game.advance().unwrap());
}

#[test]
fn game__return_is_not_a_win() {
    for (line, moments) in Fixture::all_in_lines() {
        let events: Vec<&Event> = moments.iter().map(|(event, _)| event).collect();
        assert!(
            events.contains(&&Event::Return {
                seat: 0,
                amount: 800
            }),
            "{line}: {events:?}"
        );
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, Event::Award { seat: 0, .. })),
            "{line}: {events:?}"
        );
    }
}

#[test]
fn game__second_hand_plays_to_the_end() {
    let mut game = Fixture::play(HEADS_UP_PLAY);
    Fixture::finish(&mut game, Fixture::check_or_call);
    game.next_hand().unwrap();

    Fixture::finish(&mut game, Fixture::check_or_call);

    assert!(
        game.event().to_string().contains("wins")
            || game.event().to_string().contains("takes back")
    );
    assert_eq!(0, game.table().pot);
}

#[test]
fn game__play_with_empty_chairs_and_an_ante() {
    let mut game = Fixture::play(
        "\
mode: play
seat: 2
table: { seats: 6, button: 5, stakes: { small_blind: 50.0, big_blind: 100.0, ante: 10.0 } }
players:
  - { seat: 0, stack: 2000, bot: maniac }
  - { seat: 2, stack: 2000 }
  - { seat: 5, stack: 2000, bot: tight_passive }
",
    );

    Fixture::finish(&mut game, Fixture::check_or_call);

    let chips: usize = [0, 2, 5]
        .iter()
        .map(|seat| game.table().seats.get_seat(*seat).unwrap().player.chips)
        .sum();
    assert_eq!(6_000, chips);
    assert_eq!(0, game.table().pot);
    game.next_hand().unwrap();
    assert_eq!(0, game.table().button);
}

#[test]
fn game__board_keeps_its_deal_order_to_the_end() {
    // Hole cards, a burn, then a flop dealt low, high, middle.
    let front = ["A♠", "K♠", "A♥", "K♥", "Q♠", "2♣", "9♦", "5♥"];
    let rest = DECK.split_whitespace().filter(|card| !front.contains(card));
    let deck: Vec<&str> = front.into_iter().chain(rest).collect();
    let mut game = Fixture::play(&format!("{HEADS_UP_PLAY}deck: {}\n", deck.join(" ")));
    Fixture::finish(&mut game, Fixture::check_or_call);

    let boards: Vec<String> = Fixture::moments(&mut game)
        .into_iter()
        .filter(|(_, table)| table.board.len() == 5)
        .map(|(_, table)| table.board.to_string())
        .collect();

    assert_ne!(Vec::<String>::new(), boards);
    assert!(boards.iter().all(|board| *board == boards[0]), "{boards:?}");
}

#[test]
fn game__hand_over_only_at_the_last_moment() {
    let mut replay = Fixture::the_hand("mode: replay\n");
    assert!(!replay.is_hand_over());
    replay.seek(replay.moments() - 1).unwrap();
    assert!(replay.is_hand_over());

    let mut game = Fixture::play(HEADS_UP_PLAY);
    assert!(!game.is_hand_over());
    Fixture::finish(&mut game, Fixture::check_or_call);
    assert!(game.is_hand_over());
    game.seek(0).unwrap();
    assert!(!game.is_hand_over());
}
