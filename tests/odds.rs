//! Win percents per live seat: when they are there, and when they are not.
#![allow(non_snake_case)]

use std::fs;
use std::str::FromStr;

use bowery::check::check;
use bowery::config::{Config, View};
use bowery::frame::{CardFace, Frame, SeatStatus};
use bowery::game::{Event, Game};
use bowery::odds::{self, OutsBar};
use pkcore::card::Card;
use pkcore::casino::action::PlayerAction;
use pkcore::hand_history::HandHistory;

const BRUNSON: u8 = 0;
const HANSEN: u8 = 3;
const NEGREANU: u8 = 4;

fn game(head: &str, hand_path: &str) -> Game {
    let mut config = Config::from_yaml(head).unwrap();
    config.hand = Some(HandHistory::from_yaml(&fs::read_to_string(hand_path).unwrap()).unwrap());
    Game::new(check(&config).unwrap()).unwrap()
}

fn the_hand(head: &str) -> Game {
    game(head, "fixtures/the_hand.yaml")
}

fn frame_at(game: &mut Game, index: usize, view: View, odds: bool) -> Frame {
    game.seek(index).unwrap();
    if odds {
        Frame::with_odds(game, view)
    } else {
        game.frame(view)
    }
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

#[test]
fn odds__every_live_seat_has_a_percent_on_the_flop() {
    let game = the_hand("mode: replay\nstart: flop\n");
    let frame = Frame::with_odds(&game, View::All);
    let odds = frame.odds.as_ref().expect("odds on");
    let hansen = odds.percents[index_of(&frame, HANSEN)].unwrap();
    let negreanu = odds.percents[index_of(&frame, NEGREANU)].unwrap();
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
    let mut game = the_hand("mode: replay\n");
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
    let before = frame_at(&mut game, folds - 1, View::All, true);
    let after = frame_at(&mut game, folds, View::All, true);
    let negreanu = index_of(&after, NEGREANU);
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
    let first = Frame::with_odds(&the_hand("mode: replay\n"), View::All).odds;
    let second = Frame::with_odds(&the_hand("mode: replay\n"), View::All).odds;
    assert_eq!(first, second);
}

#[test]
fn odds__river_is_decided() {
    let game = the_hand("mode: replay\nstart: river\n");
    let frame = Frame::with_odds(&game, View::All);
    let odds = frame.odds.as_ref().unwrap();
    assert_eq!(Some(100), odds.percents[index_of(&frame, HANSEN)]);
    assert_eq!(Some(0), odds.percents[index_of(&frame, NEGREANU)]);
}

#[test]
fn odds__none_after_the_award() {
    let mut game = the_hand("mode: replay\n");
    let last = game.moments() - 1;
    assert_eq!(None, frame_at(&mut game, last, View::All, true).odds);
}

#[test]
fn odds__none_when_a_card_is_hidden() {
    let game = the_hand("mode: replay\nseat: 4\nstart: flop\n");
    assert_eq!(None, Frame::with_odds(&game, View::Seat).odds);
}

#[test]
fn odds__one_live_seat_gives_none() {
    // heads_up_fold: one seat folds, the other takes the pot back.
    let mut game = game("mode: replay\n", "tests/hands/heads_up_fold.yaml");
    let returned = (0..game.moments())
        .find(|index| matches!(game.event_at(*index), Some(Event::Return { .. })))
        .unwrap();
    assert_eq!(None, frame_at(&mut game, returned, View::All, true).odds);
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
    let game = Game::new(check(&config).unwrap()).unwrap();
    let frame = Frame::with_odds(&game, View::All);
    // Both seats are dealt, so this is Some; the None case is pinned below.
    assert!(frame.odds.is_some());
    let mut blank = frame.clone();
    blank.seats[0].cards = [CardFace::None, CardFace::None];
    assert_eq!(
        None,
        odds::compute(game.table(), &blank.seats, &blank.board, false, game.seed())
    );
}

#[test]
fn frame__new_has_no_odds() {
    assert_eq!(None, the_hand("mode: replay\n").frame(View::All).odds);
}

#[test]
fn outs_bar__display() {
    let bar = OutsBar {
        seat: 3,
        name: "Gus Hansen".into(),
        percent: 18,
        cards: vec![card("T♠"), card("4♣")],
    };
    assert_eq!("Gus Hansen · 2 outs · 18%", bar.to_string());
    let dead = OutsBar {
        cards: Vec::new(),
        percent: 0,
        ..bar
    };
    assert_eq!("Gus Hansen · drawing dead · 0%", dead.to_string());
}
