//! The one renderer: golden SVGs of The Hand, and what an SVG may never hold.
#![allow(non_snake_case)]

use std::fs;

use bowery::check::check;
use bowery::config::{Config, Layout, Moment, View};
use bowery::draw::{Theme, geometry, svg};
use bowery::game::{Event, Game};
use pkcore::hand_history::{HandHistory, ReplayStreet};

const WIDTH: u32 = 680;

fn game_of(head: &str, hand_path: &str) -> Game {
    let mut config = Config::from_yaml(head).unwrap();
    config.hand = Some(HandHistory::from_yaml(&fs::read_to_string(hand_path).unwrap()).unwrap());
    Game::new(check(&config).unwrap()).unwrap()
}

fn the_hand(head: &str) -> Game {
    game_of(head, "fixtures/the_hand.yaml")
}

/// The index of the moment the turn is dealt.
fn turn_index(game: &Game) -> usize {
    (0..game.moments())
        .find(|index| game.event_at(*index) == Some(&Event::Deal(ReplayStreet::Turn)))
        .unwrap()
}

/// A hand at its turn, odds on, all cards up.
fn drawn_with_odds(hand: &str, layout: Layout, theme: Theme) -> String {
    let mut game = game_of("mode: replay\n", hand);
    game.seek(turn_index(&game)).unwrap();
    svg(&game.frame_with_odds(View::All), layout, WIDTH, theme)
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
fn draw__golden_files_are_the_thirty_six() {
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

    assert_eq!(36, count);
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

#[test]
fn draw__board_is_one_group() {
    let svg = drawn("turn", Layout::Ring, Theme::Light);

    let board = svg
        .split(r#"<g class="board">"#)
        .nth(1)
        .unwrap()
        .split(r#"<text class="pot""#)
        .next()
        .unwrap();
    assert_eq!(4, board.matches(r#"data-card=""#).count(), "{board}");
    assert_eq!(1, board.matches(r#"class="slot""#).count(), "{board}");
}

#[test]
fn draw__golden_turn_with_odds() {
    for (file, hand) in [
        ("the_hand", "fixtures/the_hand.yaml"),
        ("three_way_outs", "tests/hands/three_way_outs.yaml"),
    ] {
        for layout in [Layout::Ring, Layout::Spotlight, Layout::Ledger] {
            for (theme, theme_name) in [(Theme::Light, "light"), (Theme::Dark, "dark")] {
                golden(
                    &format!("{file}-{layout}-turn-odds-{theme_name}"),
                    &drawn_with_odds(hand, layout, theme),
                );
            }
        }
    }
}

#[test]
fn draw__odds_on_every_live_plate_and_the_bar_lists_the_outs() {
    let svg = drawn_with_odds(
        "tests/hands/three_way_outs.yaml",
        Layout::Ring,
        Theme::Light,
    );
    assert_eq!(2, svg.matches(r#"class="odds""#).count());
    assert!(svg.contains(r#"<g class="outs" data-seat="0">"#));
    assert!(svg.contains("DRAW · 7 OUTS · 18%"));
    assert!(svg.contains(r#"data-card="T♥""#) && !svg.contains(r#"data-card="T♠""#));
}

#[test]
fn draw__drawing_dead_is_text_not_cards() {
    // No fixture hand is drawing dead at the turn (in The Hand Negreanu
    // has one out, the 6♣), so empty the bar by hand before drawing.
    let mut game = the_hand("mode: replay\n");
    game.seek(turn_index(&game)).unwrap();
    let mut frame = game.frame_with_odds(View::All);
    let bar = frame.odds.as_mut().unwrap().outs.as_mut().unwrap();
    bar.cards.clear();
    bar.percent = 0;
    let svg = svg(&frame, Layout::Ledger, WIDTH, Theme::Light);
    assert!(svg.contains(r#"class="dead""#));
    assert!(svg.contains("drawing dead"));
    assert!(svg.contains("DANIEL NEGREANU · DRAWING DEAD · 0%"));
    let bar_svg = svg.split(r#"<g class="outs""#).nth(1).unwrap();
    let bar_svg = bar_svg.split("</g>").next().unwrap();
    assert!(!bar_svg.contains("data-card"), "no cards inside the bar");
}

#[test]
fn draw__one_out_is_one_card_and_a_singular_label() {
    let svg = drawn_with_odds("fixtures/the_hand.yaml", Layout::Ledger, Theme::Light);
    assert!(svg.contains(r#"<g class="outs" data-seat="4">"#));
    assert!(svg.contains("DANIEL NEGREANU · 1 OUT · 2%"));
    assert!(svg.contains(r#"data-card="6♣""#));
}

#[test]
fn draw__seat_view_draws_no_odds_and_no_bar() {
    let mut game = the_hand("mode: replay\nseat: 4\n");
    game.seek(turn_index(&game)).unwrap();
    let svg = svg(
        &game.frame_with_odds(View::Seat),
        Layout::Ring,
        WIDTH,
        Theme::Light,
    );
    assert!(!svg.contains(r#"class="odds""#));
    assert!(!svg.contains(r#"class="outs""#));
}

/// A text element's attribute inside `group`, by class and name.
fn text_attr<'a>(group: &'a str, class: &str, name: &str) -> Option<&'a str> {
    group
        .split(&format!(r#"<text class="{class}" "#))
        .nth(1)?
        .split('>')
        .next()?
        .split(&format!(r#"{name}=""#))
        .nth(1)?
        .split('"')
        .next()
}

/// A text element's numeric attribute.
fn text_number(group: &str, class: &str, name: &str) -> Option<u32> {
    text_attr(group, class, name)?.parse().ok()
}

/// The text of the element of `class` inside `group`.
fn text_of<'a>(group: &'a str, class: &str) -> Option<&'a str> {
    group
        .split(&format!(r#"<text class="{class}" "#))
        .nth(1)?
        .split_once('>')?
        .1
        .split('<')
        .next()
}

/// The ledger rows of The Hand at the river, all cards up, with odds: each
/// string runs from one row's start to the next's (cards are groups too).
fn ledger_rows_at_the_river(width: u32) -> Vec<String> {
    let mut game = the_hand("mode: replay\n");
    let moment: Moment = "river".parse().unwrap();
    let replay = game.config().hand.clone().unwrap().cursor().unwrap();
    game.seek(moment.index_in(&replay).unwrap()).unwrap();
    svg(
        &game.frame_with_odds(View::All),
        Layout::Ledger,
        width,
        Theme::Light,
    )
    .split(r#"<g class="seat row"#)
    .skip(1)
    .map(str::to_string)
    .collect()
}

#[test]
fn draw__ledger_row_with_a_percent_is_two_lines_below_ledger_wide() {
    let width = geometry::LEDGER_WIDE;
    let mut with_percent = 0;

    for row in ledger_rows_at_the_river(width - 8) {
        if text_attr(&row, "odds", "x").is_none() {
            continue;
        }
        with_percent += 1;
        assert_eq!(
            Some("end"),
            text_attr(&row, "odds", "text-anchor"),
            "the percent ends the name line: {row}"
        );
        assert_ne!(
            text_number(&row, "name", "y"),
            text_number(&row, "stack", "y"),
            "the stack has its own line: {row}"
        );
    }
    assert!(with_percent >= 2, "both live seats show a percent");

    with_percent = 0;
    for row in ledger_rows_at_the_river(width) {
        let Some(percent_x) = text_number(&row, "odds", "x") else {
            continue;
        };
        with_percent += 1;
        assert_eq!(
            text_number(&row, "name", "y"),
            text_number(&row, "stack", "y"),
            "one line: {row}"
        );
        // The stack is end-anchored; 11 px monospace is 6.6 px a character.
        let stack_x = text_number(&row, "stack", "x").unwrap();
        let chars = u32::try_from(text_of(&row, "stack").unwrap().chars().count()).unwrap();
        let stack_left = stack_x - (chars * geometry::SMALL_CHAR_TENTHS).div_ceil(10);
        assert!(
            percent_x + 27 + geometry::GAP <= stack_left,
            "{percent_x} + 27 + gap <= {stack_left}: {row}"
        );
    }
    assert!(with_percent >= 2, "both live seats show a percent");
}
