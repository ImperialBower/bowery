//! Win percents per live seat: when they are there, and when they are not.
#![allow(non_snake_case)]

use std::fs;
use std::str::FromStr;

use bowery::check::Checked;
use bowery::config::{Config, View};
use bowery::frame::{CardFace, Frame, SeatStatus};
use bowery::game::{Event, Game};
use bowery::odds::{Odds, OutsBar};
use pkcore::card::Card;
use pkcore::casino::action::PlayerAction;
use pkcore::hand_history::{HandHistory, ReplayStreet};

const BRUNSON: u8 = 0;
const HANSEN: u8 = 3;
const NEGREANU: u8 = 4;

/// Games, frames and cards for the tests.
struct Fixture;

impl Fixture {
    fn game(head: &str, hand_path: &str) -> Game {
        let mut config = Config::from_yaml(head).unwrap();
        config.hand =
            Some(HandHistory::from_yaml(&fs::read_to_string(hand_path).unwrap()).unwrap());
        Game::new(Checked::new(&config).unwrap()).unwrap()
    }

    fn the_hand(head: &str) -> Game {
        Self::game(head, "fixtures/the_hand.yaml")
    }

    fn frame_at(game: &mut Game, index: usize, view: View, odds: bool) -> Frame {
        game.seek(index).unwrap();
        if odds {
            Frame::with_odds(game, view)
        } else {
            game.frame(view)
        }
    }

    /// The index of the moment a street is dealt: `flop`, `turn` or `river`.
    fn moment_of(game: &Game, name: &str) -> usize {
        let street = match name {
            "flop" => ReplayStreet::Flop,
            "turn" => ReplayStreet::Turn,
            "river" => ReplayStreet::River,
            other => panic!("no street named {other}"),
        };
        (0..game.moments())
            .find(|index| game.event_at(*index) == Some(&Event::Deal(street)))
            .unwrap()
    }

    fn turn_of(game: &Game) -> usize {
        Self::moment_of(game, "turn")
    }

    fn index_of(frame: &Frame, seat: u8) -> usize {
        frame
            .seats
            .iter()
            .position(|candidate| candidate.seat == seat)
            .unwrap()
    }

    fn card(text: &str) -> Card {
        Card::from_str(text).unwrap()
    }
}

#[test]
fn odds__every_live_seat_has_a_percent_on_the_flop() {
    let game = Fixture::the_hand("mode: replay\nstart: flop\n");
    let frame = Frame::with_odds(&game, View::All);
    let odds = frame.odds.as_ref().expect("odds on");
    let hansen = odds.percents[Fixture::index_of(&frame, HANSEN)].unwrap();
    let negreanu = odds.percents[Fixture::index_of(&frame, NEGREANU)].unwrap();
    assert!(negreanu > hansen, "{negreanu} vs {hansen}");
    assert_eq!(
        100,
        u32::from(hansen) + u32::from(negreanu),
        "two live seats share the pot"
    );
    for (seat, percent) in frame.seats.iter().zip(&odds.percents) {
        assert_eq!(
            seat.status == SeatStatus::Folded,
            percent.is_none(),
            "{}",
            seat.name
        );
    }
}

#[test]
fn odds__a_fold_changes_the_other_percents() {
    // Preflop, before and after Brunson's fold: Negreanu's number moves. (The
    // first fold on record is Zeidman's, which moves Hansen's number by less
    // than a rounding step, so the test takes Brunson's.)
    let mut game = Fixture::the_hand("mode: replay\n");
    let folds = (1..game.moments())
        .find(|index| {
            matches!(
                game.event_at(*index),
                Some(Event::Action {
                    seat: BRUNSON,
                    action: PlayerAction::Fold,
                    ..
                })
            )
        })
        .unwrap();
    let before = Fixture::frame_at(&mut game, folds - 1, View::All, true);
    let after = Fixture::frame_at(&mut game, folds, View::All, true);
    let negreanu = Fixture::index_of(&after, NEGREANU);
    let (before, after) = (before.odds.unwrap(), after.odds.unwrap());
    assert_eq!(
        before.percents.iter().flatten().count() - 1,
        after.percents.iter().flatten().count()
    );
    assert_ne!(
        before.percents[negreanu], after.percents[negreanu],
        "one hand fewer, a new share"
    );
}

#[test]
fn odds__same_config_same_preflop_percents() {
    let first = Frame::with_odds(&Fixture::the_hand("mode: replay\n"), View::All).odds;
    let second = Frame::with_odds(&Fixture::the_hand("mode: replay\n"), View::All).odds;
    assert_eq!(first, second);
}

#[test]
fn odds__river_is_decided() {
    let game = Fixture::the_hand("mode: replay\nstart: river\n");
    let frame = Frame::with_odds(&game, View::All);
    let odds = frame.odds.as_ref().unwrap();
    assert_eq!(Some(100), odds.percents[Fixture::index_of(&frame, HANSEN)]);
    assert_eq!(Some(0), odds.percents[Fixture::index_of(&frame, NEGREANU)]);
}

#[test]
fn odds__none_after_the_award() {
    let mut game = Fixture::the_hand("mode: replay\n");
    let last = game.moments() - 1;
    assert_eq!(
        None,
        Fixture::frame_at(&mut game, last, View::All, true).odds
    );
}

#[test]
fn odds__none_when_a_card_is_hidden() {
    let game = Fixture::the_hand("mode: replay\nseat: 4\nstart: flop\n");
    assert_eq!(None, Frame::with_odds(&game, View::Seat).odds);
}

#[test]
fn odds__one_live_seat_gives_none() {
    // heads_up_fold: one seat folds, the other takes the pot back.
    let mut game = Fixture::game("mode: replay\n", "tests/hands/heads_up_fold.yaml");
    let returned = (0..game.moments())
        .find(|index| matches!(game.event_at(*index), Some(Event::Return { .. })))
        .unwrap();
    assert_eq!(
        None,
        Fixture::frame_at(&mut game, returned, View::All, true).odds
    );
}

#[test]
fn odds__blank_hole_card_gives_none() {
    let config = Config::from_yaml(
        "
mode: play
table: { stakes: { small_blind: 1.0, big_blind: 2.0 } }
players: [ { seat: 0, stack: 200 }, { seat: 1, stack: 200, bot: abc } ]
",
    )
    .unwrap();
    let game = Game::new(Checked::new(&config).unwrap()).unwrap();
    let frame = Frame::with_odds(&game, View::All);
    // Both seats are dealt, so this is Some; the None case is pinned below.
    assert!(frame.odds.is_some());
    let mut blank = frame.clone();
    blank.seats[0].cards = [CardFace::None, CardFace::None];
    assert_eq!(
        None,
        Odds::compute(game.table(), &blank.seats, &blank.board, false, game.seed())
    );
}

#[test]
fn frame__new_has_no_odds() {
    assert_eq!(
        None,
        Fixture::the_hand("mode: replay\n").frame(View::All).odds
    );
}

#[test]
fn outs_bar__display() {
    let bar = OutsBar {
        seat: 3,
        name: "Gus Hansen".into(),
        percent: 18,
        cards: vec![Fixture::card("T♠"), Fixture::card("4♣")],
    };
    assert_eq!("Gus Hansen · 2 outs · 18%", bar.to_string());
    let dead = OutsBar {
        cards: Vec::new(),
        percent: 0,
        ..bar
    };
    assert_eq!("Gus Hansen · drawing dead · 0%", dead.to_string());
}

#[test]
fn outs__two_left_one_card_to_come_lists_the_trailing_seats_outs() {
    let mut game = Fixture::game("mode: replay\n", "tests/hands/three_way_outs.yaml");
    let turn = Fixture::turn_of(&game);
    let frame = Fixture::frame_at(&mut game, turn, View::All, true);
    let bar = frame.odds.unwrap().outs.expect("a bar at the turn");
    assert_eq!(0, bar.seat);
    assert_eq!("Draw", bar.name);
    let cards: Vec<String> = bar.cards.iter().map(ToString::to_string).collect();
    assert_eq!(
        vec!["T♥", "T♦", "T♣", "5♠", "5♥", "5♦", "5♣"],
        cards,
        "T♠ is in the folded hand"
    );
    assert_eq!(
        17, bar.percent,
        "pkcore 0.18.0: Gone's folded T♠ is dead, not a live runout card"
    );
}

#[test]
fn outs__a_pair_of_sixes_behind_quad_fives_has_one_out() {
    // The Hand's turn: Hansen has quad fives, Negreanu sixes full of fives.
    // He is not drawing dead: the 6♣ makes quad sixes, and nobody holds it.
    let mut game = Fixture::the_hand("mode: replay\n");
    let turn = Fixture::turn_of(&game);
    let frame = Fixture::frame_at(&mut game, turn, View::All, true);
    let bar = frame.odds.unwrap().outs.expect("a bar at the turn");
    assert_eq!(NEGREANU, bar.seat);
    assert_eq!("Daniel Negreanu", bar.name);
    assert_eq!(vec![Fixture::card("6♣")], bar.cards);
    assert_eq!(3, bar.percent);
}

#[test]
fn outs__no_bar_on_the_flop_or_the_river() {
    let mut game = Fixture::the_hand("mode: replay\n");
    for moment in ["flop", "river"] {
        let index = Fixture::moment_of(&game, moment);
        assert_eq!(
            None,
            Fixture::frame_at(&mut game, index, View::All, true)
                .odds
                .unwrap()
                .outs,
            "{moment}"
        );
    }
}

#[test]
fn outs__no_bar_with_three_live_seats() {
    // The first preflop moment of The Hand: eight seats live, no bar.
    let mut game = Fixture::the_hand("mode: replay\n");
    assert_eq!(
        None,
        Fixture::frame_at(&mut game, 0, View::All, true)
            .odds
            .unwrap()
            .outs
    );
}
