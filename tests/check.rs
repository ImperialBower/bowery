//! Every config rule fires alone, from its own fixture under `tests/config/`,
//! and The Hand checks clean.
#![allow(non_snake_case)]

use std::collections::HashSet;
use std::fs;

use bowery::check::{Problem, check};
use bowery::config::{Config, Mode};
use pkcore::hand_history::{HandHistory, Rule};

fn load(path: &str) -> Config {
    let yaml = fs::read_to_string(path).unwrap();
    Config::from_yaml(&yaml).unwrap_or_else(|problem| panic!("{path}: {problem}"))
}

/// The config in `head` with the hand history at `hand_path` as its `hand`.
fn with_hand(head: &str, hand_path: &str) -> Config {
    let mut config = Config::from_yaml(head).unwrap();
    config.hand = Some(HandHistory::from_yaml(&fs::read_to_string(hand_path).unwrap()).unwrap());
    config
}

fn problems(config: &Config) -> Vec<Problem> {
    check(config).err().unwrap_or_default()
}

/// The fixture breaks exactly one rule, and it is at `path`.
fn assert_one_config_problem(fixture: &str, path: &str) {
    let found = problems(&load(&format!("tests/config/{fixture}.yaml")));
    assert_eq!(1, found.len(), "{fixture}: {found:#?}");
    match &found[0] {
        Problem::Config { path: actual, .. } => assert_eq!(path, actual, "{fixture}: {}", found[0]),
        Problem::Hand(finding) => {
            panic!("{fixture}: expected a config problem, got hand.{finding}")
        }
    }
}

#[test]
fn check__replay_without_hand() {
    assert_one_config_problem("replay_without_hand", "hand");
}

#[test]
fn check__interactive_without_hand() {
    assert_one_config_problem("interactive_without_hand", "hand");
}

#[test]
fn check__hand_breaks_a_rule() {
    let found = problems(&load("tests/config/hand_breaks_a_rule.yaml"));

    assert_ne!(Vec::<Problem>::new(), found);
    for problem in &found {
        match problem {
            Problem::Hand(finding) => assert_eq!(Rule::Cards, finding.rule, "{problem}"),
            Problem::Config { .. } => panic!("expected only hand findings, got {problem}"),
        }
    }
}

#[test]
fn check__seat_not_seated() {
    assert_one_config_problem("seat_not_seated", "seat");
}

#[test]
fn check__start_not_in_hand() {
    assert_one_config_problem("start_not_in_hand", "start");
}

#[test]
fn check__start_past_the_end() {
    assert_one_config_problem("start_past_the_end", "start");
}

#[test]
fn check__start_without_hand() {
    assert_one_config_problem("start_without_hand", "start");
}

#[test]
fn check__player_name_disagrees() {
    assert_one_config_problem("player_name_disagrees", "players[0].name");
}

#[test]
fn check__player_stack_disagrees() {
    assert_one_config_problem("player_stack_disagrees", "players[0].stack");
}

#[test]
fn check__player_not_in_hand() {
    assert_one_config_problem("player_not_in_hand", "players[0].seat");
}

#[test]
fn check__player_seat_twice() {
    assert_one_config_problem("player_seat_twice", "players[2].seat");
}

#[test]
fn check__player_without_stack() {
    assert_one_config_problem("player_without_stack", "players[1].stack");
}

#[test]
fn check__play_without_stakes() {
    assert_one_config_problem("play_without_stakes", "table.stakes");
}

#[test]
fn check__stakes_not_whole() {
    assert_one_config_problem("stakes_not_whole", "table.stakes.small_blind");
}

#[test]
fn check__bot_unknown() {
    assert_one_config_problem("bot_unknown", "players[1].bot");
}

#[test]
fn check__deck_does_not_parse() {
    assert_one_config_problem("deck_does_not_parse", "deck");
}

#[test]
fn check__deck_short() {
    assert_one_config_problem("deck_short", "deck");
}

#[test]
fn check__deck_repeats_a_card() {
    assert_one_config_problem("deck_repeats_a_card", "deck");
}

#[test]
fn check__too_few_players() {
    assert_one_config_problem("too_few_players", "players");
}

#[test]
fn check__too_many_chairs() {
    assert_one_config_problem("too_many_chairs", "table.seats");
}

#[test]
fn check__player_seat_off_the_table() {
    assert_one_config_problem("player_seat_off_the_table", "players[1].seat");
}

#[test]
fn check__every_fixture_has_a_test() {
    let source = include_str!("check.rs");
    for entry in fs::read_dir("tests/config").unwrap() {
        let name = entry.unwrap().file_name().into_string().unwrap();
        let stem = name.trim_end_matches(".yaml");
        assert!(
            source.contains(&format!("fn check__{stem}()")),
            "tests/config/{name} has no test"
        );
    }
}

#[test]
fn check__good_play_config() {
    let config = Config::from_yaml(
        "\
mode: play
seat: 0
table: { seats: 6, button: 0, stakes: { small_blind: 1.0, big_blind: 2.0, ante: 0.0 } }
players:
  - { seat: 0, name: Reader, stack: 200 }
  - { seat: 3, name: Robo, stack: 200, bot: tight_aggressive }
",
    )
    .unwrap();

    let checked = check(&config).unwrap();

    assert_eq!(&config, checked.config());
}

#[test]
fn check__every_start_moment_of_a_full_hand() {
    for start in ["preflop", "flop", "turn", "river", "showdown", "0"] {
        let config = with_hand(
            &format!("mode: replay\nstart: {start}\n"),
            "tests/hands/heads_up_all_in.yaml",
        );
        assert!(
            check(&config).is_ok(),
            "start: {start}: {:?}",
            check(&config).err()
        );
    }
}

#[test]
fn check__interactive_players_agree_with_hand() {
    let config = with_hand(
        "mode: interactive\nseat: 1\nplayers:\n  - { seat: 0, name: Deep, stack: 1000, bot: maniac }\n",
        "tests/hands/heads_up_all_in.yaml",
    );

    assert!(check(&config).is_ok(), "{:?}", check(&config).err());
}

#[test]
fn check__the_hand_checks_clean() {
    let config = with_hand(
        "mode: interactive\nseat: 4\nstart: flop\n",
        "fixtures/the_hand.yaml",
    );

    let checked = check(&config).unwrap();

    assert_eq!(Mode::Interactive, checked.config().mode);
}

/// The pre-`DEFECT_040` copy of The Hand fails with hand findings whose rules
/// are exactly {Seats, Result, Money}. The walk stops at the file's first,
/// out-of-order action, so its amount, pot and legality faults are not
/// reported (pkcore EPIC-93 corrigendum).
#[test]
fn check__the_hand_before_defect_040() {
    let config = with_hand(
        "mode: replay\n",
        "tests/hands/DEFECT_040_the_hand_before.yaml",
    );

    let rules: HashSet<Rule> = problems(&config)
        .iter()
        .map(|problem| match problem {
            Problem::Hand(finding) => finding.rule,
            Problem::Config { .. } => panic!("expected only hand findings, got {problem}"),
        })
        .collect();

    assert_eq!(
        HashSet::from([Rule::Seats, Rule::Result, Rule::Money]),
        rules
    );
}
