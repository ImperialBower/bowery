//! The one renderer: golden SVGs of The Hand, and what an SVG may never hold.
#![allow(non_snake_case)]

use std::fs;

use bowery::check::check;
use bowery::config::{Config, Layout, Moment, View};
use bowery::draw::{Theme, svg};
use bowery::game::Game;
use pkcore::hand_history::HandHistory;

const WIDTH: u32 = 680;

fn the_hand(head: &str) -> Game {
    let mut config = Config::from_yaml(head).unwrap();
    config.hand = Some(
        HandHistory::from_yaml(&fs::read_to_string("fixtures/the_hand.yaml").unwrap()).unwrap(),
    );
    Game::new(check(&config).unwrap()).unwrap()
}

/// The Hand at `moment`, all cards up.
fn drawn(moment: &str, layout: Layout, theme: Theme) -> String {
    let mut game = the_hand("mode: replay\n");
    let index = match moment {
        "award" => game.moments() - 1,
        name => {
            let moment: Moment = name.parse().unwrap();
            let config = game.config().clone();
            let replay = config.hand.unwrap().cursor().unwrap();
            moment.index_in(&replay).unwrap()
        }
    };
    game.seek(index).unwrap();
    svg(&game.frame(View::All), layout, WIDTH, theme)
}

/// Compares against `tests/golden/<name>.svg` byte for byte. With
/// `BOWERY_BLESS=1` it writes the file instead, for review.
fn golden(name: &str, actual: &str) {
    let path = format!("tests/golden/{name}.svg");
    if std::env::var("BOWERY_BLESS").is_ok_and(|bless| bless == "1") {
        fs::write(&path, actual).unwrap();
        return;
    }
    let expected = fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("{path} is missing; run with BOWERY_BLESS=1"));
    assert!(
        expected == actual,
        "{path} differs; review it and run with BOWERY_BLESS=1"
    );
}

#[test]
fn draw__golden_the_hand() {
    for layout in [Layout::Ring, Layout::Spotlight, Layout::Ledger] {
        for moment in ["preflop", "flop", "river", "award"] {
            for (theme, theme_name) in [(Theme::Light, "light"), (Theme::Dark, "dark")] {
                golden(
                    &format!("the_hand-{layout}-{moment}-{theme_name}"),
                    &drawn(moment, layout, theme),
                );
            }
        }
    }
}

#[test]
fn draw__golden_files_are_the_twenty_four() {
    let count = fs::read_dir("tests/golden")
        .unwrap()
        .filter(|entry| {
            entry
                .as_ref()
                .unwrap()
                .file_name()
                .to_string_lossy()
                .ends_with(".svg")
        })
        .count();

    assert_eq!(24, count);
}

#[test]
fn draw__no_script_no_css_nothing_hidden() {
    for layout in [Layout::Ring, Layout::Spotlight, Layout::Ledger] {
        let svg = drawn("river", layout, Theme::Dark);
        for banned in [
            "<script",
            "<style",
            "style=",
            "display:none",
            "visibility",
            "<foreignObject",
        ] {
            assert!(!svg.contains(banned), "{layout}: {banned}");
        }
    }
}

#[test]
fn draw__is_deterministic() {
    assert_eq!(
        drawn("flop", Layout::Ring, Theme::Light),
        drawn("flop", Layout::Ring, Theme::Light)
    );
}

#[test]
fn draw__seat_view_draws_no_hidden_card() {
    let mut game = the_hand("mode: replay\nseat: 4\nstart: flop\n");

    let svg = svg(&game.frame(View::Seat), Layout::Ring, WIDTH, Theme::Light);

    assert!(
        svg.contains(r#"data-card="6♠""#),
        "Negreanu sees his own cards"
    );
    assert!(!svg.contains(r#"data-card="5♦""#), "Hansen's 5♦ is hidden");
    assert!(!svg.contains(r#"data-card="5♣""#), "Hansen's 5♣ is hidden");
    game.seek(game.moments() - 1).unwrap();
    let shown = svg_of(&game, View::Seat);
    assert!(shown.contains(r#"data-card="5♦""#), "shown at the showdown");
}

fn svg_of(game: &Game, view: View) -> String {
    svg(&game.frame(view), Layout::Ring, WIDTH, Theme::Light)
}

#[test]
fn draw__ring_below_520_draws_ledger() {
    let game = the_hand("mode: replay\n");

    let svg = svg(&game.frame(View::All), Layout::Ring, 400, Theme::Light);

    assert!(svg.contains(r#"data-layout="ledger""#));
    assert!(svg.contains(r#"width="400""#));
}

#[test]
fn draw__names_are_escaped() {
    let config = Config::from_yaml(
        "mode: play\ntable: { stakes: { small_blind: 1.0, big_blind: 2.0 } }\nplayers: [ { seat: 0, name: \"<b>&Co\", stack: 9 }, { seat: 1, stack: 9, bot: abc } ]\n",
    )
    .unwrap();
    let game = Game::new(check(&config).unwrap()).unwrap();

    let svg = svg(&game.frame(View::All), Layout::Ledger, WIDTH, Theme::Light);

    assert!(svg.contains("&lt;b&gt;&amp;Co"));
    assert!(!svg.contains("<b>"));
}

#[test]
fn draw__long_names_are_cut_to_sixteen() {
    let config = Config::from_yaml(
        "mode: play\ntable: { stakes: { small_blind: 1.0, big_blind: 2.0 } }\nplayers: [ { seat: 0, name: Wolfeschlegelsteinhausenberger, stack: 9 }, { seat: 1, stack: 9, bot: abc } ]\n",
    )
    .unwrap();
    let game = Game::new(check(&config).unwrap()).unwrap();

    let svg = svg(&game.frame(View::All), Layout::Ring, WIDTH, Theme::Light);

    assert!(svg.contains(">Wolfeschlegelst…<"), "{svg}");
}

#[test]
fn theme__display() {
    assert_eq!("dark", Theme::Dark.to_string());
}
