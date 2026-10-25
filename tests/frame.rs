//! What is on screen: the view applied, the pot with live bets, the caption,
//! and the result.
#![allow(non_snake_case)]

use std::fs;
use std::str::FromStr;

use bowery::check::Checked;
use bowery::config::{Config, View};
use bowery::frame::{Award, CardFace, Frame, SeatStatus};
use bowery::game::{Event, Game, RecordState};
use pkcore::card::Card;
use pkcore::hand_history::HandHistory;

const BRUNSON: u8 = 0;
const HANSEN: u8 = 3;
const NEGREANU: u8 = 4;
/// The Hand: Negreanu bets 65,000 on the river.
const RIVER_BET: usize = 20;

/// Games, frames, seats and cards for the tests.
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

    fn frame_at(game: &mut Game, index: usize, view: View) -> Frame {
        game.seek(index).unwrap();
        game.frame(view)
    }

    fn seat(frame: &Frame, seat: u8) -> &bowery::frame::SeatFrame {
        frame
            .seats
            .iter()
            .find(|candidate| candidate.seat == seat)
            .unwrap()
    }

    fn card(text: &str) -> Card {
        Card::from_str(text).unwrap()
    }
}

#[test]
fn frame__seat_view_hides_other_cards() {
    let mut game = Fixture::the_hand("mode: replay\nseat: 4\n");
    let showdown = (0..game.moments())
        .find(|index| game.event_at(*index) == Some(&Event::Showdown))
        .unwrap();

    for index in 0..showdown {
        let frame = Fixture::frame_at(&mut game, index, View::Seat);
        for seat in &frame.seats {
            let up = seat
                .cards
                .iter()
                .any(|face| matches!(face, CardFace::Up(_)));
            assert_eq!(
                seat.seat == NEGREANU,
                up,
                "moment {index}, seat {}",
                seat.seat
            );
        }
    }
}

#[test]
fn frame__showdown_turns_live_cards_up_in_seat_view() {
    let mut game = Fixture::the_hand("mode: replay\nseat: 4\n");

    let frame = Fixture::frame_at(&mut game, 23, View::Seat);

    assert_eq!(
        [
            CardFace::Up(Fixture::card("5♦")),
            CardFace::Up(Fixture::card("5♣"))
        ],
        Fixture::seat(&frame, HANSEN).cards
    );
    assert_eq!(
        [
            CardFace::Up(Fixture::card("6♠")),
            CardFace::Up(Fixture::card("6♥"))
        ],
        Fixture::seat(&frame, NEGREANU).cards
    );
}

#[test]
fn frame__all_view_shows_every_live_hand() {
    let mut game = Fixture::the_hand("mode: replay\n");

    let frame = Fixture::frame_at(&mut game, 0, View::All);

    assert_eq!(
        [
            CardFace::Up(Fixture::card("T♠")),
            CardFace::Up(Fixture::card("2♥"))
        ],
        Fixture::seat(&frame, BRUNSON).cards
    );
}

#[test]
fn frame__folded_seats_have_no_cards() {
    let mut game = Fixture::the_hand("mode: replay\nstart: flop\n");

    let frame = Fixture::frame_at(&mut game, 10, View::All);

    assert_eq!(SeatStatus::Folded, Fixture::seat(&frame, BRUNSON).status);
    assert_eq!(
        [CardFace::None, CardFace::None],
        Fixture::seat(&frame, BRUNSON).cards
    );
    assert_eq!(SeatStatus::Live, Fixture::seat(&frame, HANSEN).status);
}

#[test]
fn frame__pot_includes_live_bets() {
    let mut game = Fixture::the_hand("mode: replay\n");

    let frame = Fixture::frame_at(&mut game, RIVER_BET, View::All);

    let table = game.table();
    let bets: usize = frame.seats.iter().map(|seat| seat.bet).sum();
    assert_eq!(65_000, Fixture::seat(&frame, NEGREANU).bet);
    assert_eq!(table.pot + bets, frame.pot);
}

#[test]
fn frame__caption_names_the_street_the_seat_and_the_amount() {
    let mut game = Fixture::the_hand("mode: replay\n");

    assert_eq!(
        "River · Daniel Negreanu bets 65,000",
        Fixture::frame_at(&mut game, RIVER_BET, View::All).caption
    );
    assert_eq!(
        "Flop · 9♣ 6♦ 5♥",
        Fixture::frame_at(&mut game, 10, View::All).caption
    );
    assert_eq!(
        "Gus Hansen wins 575,700",
        Fixture::frame_at(&mut game, 24, View::All).caption
    );
}

#[test]
fn frame__board_in_deal_order() {
    let mut game = Fixture::the_hand("mode: replay\n");

    let frame = Fixture::frame_at(&mut game, 15, View::All);

    assert_eq!(
        [
            Some(Fixture::card("9♣")),
            Some(Fixture::card("6♦")),
            Some(Fixture::card("5♥")),
            Some(Fixture::card("5♠")),
            None
        ],
        frame.board
    );
}

#[test]
fn frame__moment_is_one_based_of_all() {
    let mut game = Fixture::the_hand("mode: replay\n");

    assert_eq!(
        (21, 25),
        Fixture::frame_at(&mut game, RIVER_BET, View::All).moment
    );
}

#[test]
fn frame__marks_the_button_and_blinds() {
    let mut game = Fixture::the_hand("mode: replay\n");

    let frame = Fixture::frame_at(&mut game, 0, View::All);

    assert!(Fixture::seat(&frame, 0).marks.dealer);
    assert!(Fixture::seat(&frame, 1).marks.small_blind);
    assert!(Fixture::seat(&frame, 2).marks.big_blind);
    assert!(!Fixture::seat(&frame, HANSEN).marks.dealer);
}

#[test]
fn frame__to_act_is_marked() {
    let mut game = Fixture::the_hand("mode: replay\n");

    let frame = Fixture::frame_at(&mut game, RIVER_BET, View::All);

    assert_eq!(Some(HANSEN), frame.to_act);
    assert!(Fixture::seat(&frame, HANSEN).to_act);
    assert!(!Fixture::seat(&frame, NEGREANU).to_act);
}

#[test]
fn frame__result_is_won_shares_only() {
    let mut game = Fixture::game("mode: replay\n", "tests/hands/heads_up_all_in.yaml");
    let last = game.moments() - 1;

    let frame = Fixture::frame_at(&mut game, last, View::All);

    assert_eq!(
        Some(vec![Award {
            seat: 1,
            amount: 400
        }]),
        frame.result
    );
    assert_eq!(None, Fixture::frame_at(&mut game, 0, View::All).result);
}

#[test]
fn frame__record_state_and_view_carry_over() {
    let mut game = Fixture::the_hand("mode: replay\n");

    let frame = Fixture::frame_at(&mut game, 0, View::Seat);

    assert_eq!(RecordState::OnRecord, frame.record);
    assert_eq!(View::Seat, frame.view);
}

#[test]
fn frame__all_in_status() {
    let mut game = Fixture::game("mode: replay\n", "tests/hands/heads_up_all_in.yaml");

    let frame = Fixture::frame_at(&mut game, 2, View::All);

    assert_eq!(SeatStatus::AllIn, Fixture::seat(&frame, 0).status);
    assert_eq!(SeatStatus::AllIn, Fixture::seat(&frame, 1).status);
}

#[test]
fn game__view_defaults_by_mode() {
    assert_eq!(View::All, Fixture::the_hand("mode: replay\n").view());
    assert_eq!(
        View::Seat,
        Fixture::the_hand("mode: replay\nview: seat\nseat: 4\n").view()
    );
    let play = Config::from_yaml(
        "mode: play\ntable: { stakes: { small_blind: 1.0, big_blind: 2.0 } }\nplayers: [ { seat: 0, stack: 9 }, { seat: 1, stack: 9, bot: abc } ]\n",
    )
    .unwrap();
    assert_eq!(
        View::Seat,
        Game::new(Checked::new(&play).unwrap()).unwrap().view()
    );
}
