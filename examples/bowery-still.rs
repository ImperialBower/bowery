//! A still: a config (or a bare hand history) in, one SVG out, for eyeballing.
//!
//! ```text
//! cargo run --example bowery-still -- fixtures/the_hand.yaml --layout spotlight --at river > river.svg
//! ```
//!
//! Options: `--layout ring|spotlight|ledger` (default: the config's),
//! `--at preflop|flop|turn|river|showdown|last|<index>` (default: the config's
//! `start`), `--view all|seat` (default: the config's), `--width <px>`
//! (default 680), `--dark`, `--odds` (every live seat's win percent, and the
//! outs bar when two seats are left with one card to come).
//!
//! ```text
//! cargo run --example bowery-still -- fixtures/the_hand.yaml --at turn --odds > turn.svg
//! ```
//!
//! A preflop percent is sampled, and 32-bit wasm and 64-bit native sample
//! differently, so a still's preflop figure may differ from the browser's.

use std::error::Error;
use std::{env, fs};

use bowery::check::check;
use bowery::config::{Config, Layout, Mode, Moment, View};
use bowery::draw::{Theme, svg};
use bowery::game::Game;
use pkcore::hand_history::HandHistory;

/// The config in `yaml`, or a replay of it when it is a bare hand history.
fn config(yaml: &str) -> Result<Config, Box<dyn Error>> {
    if let Ok(config) = Config::from_yaml(yaml) {
        return Ok(config);
    }
    let hand = HandHistory::from_yaml(yaml)
        .map_err(|error| format!("neither a config nor a hand history: {error:?}"))?;
    let mut config = Config::from_yaml("mode: replay\n")?;
    config.mode = Mode::Replay;
    config.hand = Some(hand);
    Ok(config)
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    let path = args.first().ok_or(
        "usage: bowery-still <config.yaml> [--layout L] [--at M] [--view V] [--width N] [--dark] [--odds]",
    )?;
    let option = |name: &str| {
        args.iter()
            .position(|arg| arg == name)
            .and_then(|index| args.get(index + 1))
            .map(String::as_str)
    };
    let config = config(&fs::read_to_string(path)?)?;
    let layout: Layout = option("--layout").map_or(Ok(config.layout), str::parse)?;
    let width: u32 = option("--width").map_or(Ok(680), str::parse)?;
    let theme = if args.iter().any(|arg| arg == "--dark") {
        Theme::Dark
    } else {
        Theme::Light
    };
    let checked = check(&config).map_err(|problems| {
        problems
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n")
    })?;
    let mut game = Game::new(checked)?;
    let view: View = option("--view").map_or(Ok(game.view()), str::parse)?;
    match option("--at") {
        Some("last") => game.seek(game.moments() - 1)?,
        Some(moment) => {
            let moment: Moment = moment.parse()?;
            let replay = config
                .hand
                .as_ref()
                .ok_or("--at needs a recorded hand")?
                .cursor()?;
            let index = moment
                .index_in(&replay)
                .ok_or(format!("the hand never reaches {moment}"))?;
            game.seek(index)?;
        }
        None => {}
    }
    let frame = if args.iter().any(|arg| arg == "--odds") {
        game.frame_with_odds(view)
    } else {
        game.frame(view)
    };
    println!("{}", svg(&frame, layout, width, theme));
    Ok(())
}
