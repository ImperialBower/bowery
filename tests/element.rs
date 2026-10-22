//! The element's surface in plain Rust: what `<bowery-table>` asks of a
//! table, and the state it reads back as JSON.
#![cfg(feature = "wasm")]
#![allow(non_snake_case)]

use std::fs;

use bowery::element::{Table, problems_json};
use serde_json::Value;

/// `head` with The Hand inline as `hand:`.
fn the_hand(head: &str) -> String {
    let hand: String = fs::read_to_string("fixtures/the_hand.yaml")
        .unwrap()
        .lines()
        .flat_map(|line| ["  ", line, "\n"])
        .collect();
    format!("{head}hand:\n{hand}")
}

const HEADS_UP_PLAY: &str = "\
mode: play
seat: 0
table: { seats: 2, button: 0, stakes: { small_blind: 50.0, big_blind: 100.0 } }
players:
  - { seat: 0, name: Reader, stack: 1000 }
  - { seat: 1, name: Robo, stack: 1000, bot: abc }
";

fn state(table: &Table) -> Value {
    serde_json::from_str(&table.state_json()).unwrap()
}

/// Advances until the reader's turn or the end; returns the last answer.
fn run(table: &mut Table) -> &'static str {
    for _ in 0..500 {
        let answer = table.advance().unwrap();
        if answer == "waiting" || answer == "over" {
            return answer;
        }
    }
    panic!("no end in 500 steps");
}

#[test]
fn element__new_reports_problems_as_json() {
    let problems = Table::new("mode: replay\n").unwrap_err();

    let json: Value = serde_json::from_str(&problems_json(&problems)).unwrap();
    assert_eq!("hand", json[0]["path"]);
    assert_eq!("replay needs a recorded hand", json[0]["message"]);
}

#[test]
fn element__yaml_that_does_not_read_is_a_problem() {
    let problems = Table::new("mode: [").unwrap_err();

    assert_eq!(1, problems.len());
}

#[test]
fn element__hand_findings_carry_their_rule() {
    let yaml = the_hand("mode: replay\n").replace("hole_cards: 6♠ 6♥", "hole_cards: 5♦ 6♥");

    let json: Value =
        serde_json::from_str(&problems_json(&Table::new(&yaml).unwrap_err())).unwrap();

    assert_eq!("cards", json[0]["rule"]);
    assert!(json[0]["path"].as_str().unwrap().starts_with("hand."));
}

#[test]
fn element__replay_state() {
    let table = Table::new(&the_hand("mode: replay\nstart: flop\n")).unwrap();

    let state = state(&table);

    assert_eq!("replay", state["mode"]);
    assert_eq!(11, state["moment"]);
    assert_eq!(25, state["moments"]);
    assert_eq!("on", state["record"]);
    assert_eq!("Flop · 9♣ 6♦ 5♥", state["caption"]);
    assert_eq!(true, state["controls"]["seek"]);
    assert_eq!(false, state["controls"]["take_seat"]);
    assert_eq!(false, state["hand_over"]);
    assert_eq!(8, state["seats"].as_array().unwrap().len());
}

#[test]
fn element__seek_moves_and_svg_follows() {
    let mut table = Table::new(&the_hand("mode: replay\n")).unwrap();

    table.seek(20);

    assert!(table.svg(680, false).contains(r#"data-moment="21/25""#));
    assert!(table.svg(680, true).contains("#111111"));
    table.seek(999);
    assert_eq!(25, state(&table)["moment"]);
}

#[test]
fn element__advance_names_what_happened() {
    let mut table = Table::new(&the_hand("mode: replay\nstart: 9\n")).unwrap();

    assert_eq!("dealt", table.advance().unwrap());
    assert_eq!("acted", table.advance().unwrap());
    table.seek(24);
    assert_eq!("over", table.advance().unwrap());
}

#[test]
fn element__interactive_takes_a_seat_and_goes_off_the_record() {
    let mut table = Table::new(&the_hand("mode: interactive\nstart: flop\n")).unwrap();
    assert_eq!(true, state(&table)["controls"]["take_seat"]);

    table.take_seat(4).unwrap();
    assert_eq!("acted", table.advance().unwrap());
    assert_eq!("waiting", table.advance().unwrap());
    table.act(r#"{"action":"check"}"#).unwrap();

    let state = state(&table);
    assert_eq!("off", state["record"]);
    assert_eq!(11, state["since"]);
    assert_eq!(4, state["reader"]);
    assert_eq!(true, state["controls"]["back_to_record"]);
    table.back_to_record();
    assert_eq!("on", self::state(&table)["record"]);
}

#[test]
fn element__legal_actions_carry_their_sizes() {
    let mut table = Table::new(HEADS_UP_PLAY).unwrap();
    assert_eq!("waiting", table.advance().unwrap());

    let state = state(&table);
    let legal = state["legal"].as_array().unwrap();
    let raise = legal
        .iter()
        .find(|action| action["action"] == "raise")
        .unwrap();
    let call = legal
        .iter()
        .find(|action| action["action"] == "call")
        .unwrap();

    assert_eq!(1_000, raise["max"]);
    assert!(raise["min"].as_u64().unwrap() > 100);
    assert_eq!(50, call["amount"]);
    assert!(legal.iter().any(|action| action["action"] == "fold"));
}

#[test]
fn element__act_reads_json() {
    let mut table = Table::new(HEADS_UP_PLAY).unwrap();
    table.advance().unwrap();

    assert!(table.act("not json").is_err());
    assert!(table.act(r#"{"action":"dance"}"#).is_err());
    assert!(
        table.act(r#"{"action":"raise"}"#).is_err(),
        "a raise needs an amount"
    );
    table.act(r#"{"action":"raise","amount":300}"#).unwrap();

    assert_eq!(2, state(&table)["moment"]);
}

#[test]
fn element__play_hands_go_on() {
    let mut table = Table::new(HEADS_UP_PLAY).unwrap();
    table.advance().unwrap();
    table.act(r#"{"action":"fold"}"#).unwrap();

    assert_eq!("over", run(&mut table));
    let over = state(&table);
    assert_eq!(true, over["hand_over"]);
    assert_eq!(true, over["controls"]["next_hand"]);
    assert_eq!(1, over["result"].as_array().unwrap().len());

    table.next_hand().unwrap();
    assert_eq!(2, state(&table)["hand_number"]);
}

#[test]
fn element__shuffle_starts_a_new_game() {
    let mut table = Table::new(HEADS_UP_PLAY).unwrap();
    table.advance().unwrap();
    table.act(r#"{"action":"fold"}"#).unwrap();
    run(&mut table);
    table.next_hand().unwrap();

    table.shuffle(7).unwrap();

    let state = state(&table);
    assert_eq!(1, state["hand_number"]);
    assert_eq!(1, state["moment"]);
}

#[test]
fn element__shuffle_only_in_play() {
    let mut table = Table::new(&the_hand("mode: replay\n")).unwrap();

    assert!(table.shuffle(7).is_err());
}

#[test]
fn element__view_and_layout_switch() {
    let mut table = Table::new(&the_hand("mode: replay\nseat: 4\n")).unwrap();

    table.set_layout("ledger").unwrap();
    table.set_view("seat").unwrap();

    let state = state(&table);
    assert_eq!("ledger", state["layout"]);
    assert_eq!("seat", state["view"]);
    assert!(table.svg(680, false).contains(r#"data-layout="ledger""#));
    assert!(table.set_layout("oval").is_err());
    assert!(table.set_view("none").is_err());
}

#[test]
fn element__controls_follow_the_config() {
    let table = Table::new(&the_hand(
        "mode: replay\ncontrols: { view: false, layout: false }\n",
    ))
    .unwrap();

    let state = state(&table);

    assert_eq!(false, state["controls"]["view"]);
    assert_eq!(false, state["controls"]["layout"]);
}

#[test]
fn element__play_state_names_its_delay() {
    let table = Table::new(&format!("{HEADS_UP_PLAY}delay_ms: 250\n")).unwrap();

    let state = state(&table);

    assert_eq!(250, state["delay_ms"]);
    assert_eq!(false, state["controls"]["seek"]);
    assert_eq!(true, state["controls"]["shuffle"]);
}

#[test]
fn element__version_is_the_crate_version() {
    assert_eq!(env!("CARGO_PKG_VERSION"), bowery::element::version());
}
