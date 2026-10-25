//! Refusing a config that is not true.
//!
//! [`Checked::new`] reads a [`Config`] against every rule and returns every
//! [`Problem`] at once, so an author fixes a page in one pass. The recorded
//! hand is checked by pkcore's [`HandHistory::validate`], never by bowery;
//! its findings come back as [`Problem::Hand`]. The rest are bowery's own
//! [`Problem::Config`] rules:
//!
//! - **mode:** replay and interactive need a `hand` and take no `table` (the
//!   hand brings its own); play needs
//!   `table.stakes`, in whole chips, with each blind at least 1 and the big
//!   blind no smaller than the small; a `table.button` at one of the chairs;
//!   and a `stack` of at least 1 for every player.
//! - **seat:** `seat` has a player in it.
//! - **start:** `start` needs a `hand`, and the hand reaches it.
//! - **players:** with a `hand`, each player's seat is in the hand and a given
//!   `name` or `stack` agrees with it. In play, no seat is taken twice and
//!   every seat is at the table.
//! - **bots:** every `bot` is one of pkcore's default profiles.
//! - **deck:** `deck` parses and holds 52 different cards.
//! - **seats:** 2 to 10 chairs, and 2 to 10 players.
//!
//! A [`Checked`] config is the only kind a game can be built from, so an
//! unchecked config cannot reach the screen.
//!
//! ```
//! use bowery::check::Checked;
//! use bowery::config::Config;
//!
//! let config = Config::from_yaml("mode: replay\n").unwrap();
//! let problems = Checked::new(&config).unwrap_err();
//! assert_eq!("hand: replay needs a recorded hand", problems[0].to_string());
//! ```
//!
//! [`HandHistory::validate`]: pkcore::hand_history::HandHistory::validate

use std::collections::HashSet;
use std::fmt;
use std::str::FromStr;

use pkcore::bot::profile::BotProfile;
use pkcore::card::Card;
use pkcore::cards::Cards;
use pkcore::hand_history::{Finding, HandHistory, Stakes};

use crate::config::{Config, Mode};

/// The fewest seats, and chairs, a table can have.
const MIN_SEATS: usize = 2;
/// The most seats, and chairs, a table can have.
const MAX_SEATS: usize = 10;
/// Cards in a deck (pkcore's `Cards::DECK_SIZE` is private).
const DECK_SIZE: usize = 52;

/// One thing wrong with a config.
///
/// # Examples
///
/// ```
/// use bowery::check::Problem;
///
/// let problem = Problem::Config { path: "deck".to_string(), message: "has 51 cards".to_string() };
/// assert_eq!("deck: has 51 cards", problem.to_string());
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Problem {
    /// A pkcore finding inside `hand`; its path is relative to `hand`, and
    /// [`Display`](fmt::Display) puts `hand.` in front of it.
    Hand(Finding),
    /// A config rule broken at `path`; an empty path is the whole document
    /// (YAML that does not read).
    Config {
        /// YAML path of the offending field, such as `players[1].bot`.
        path: String,
        /// One sentence saying what is wrong.
        message: String,
    },
}

impl Problem {
    fn config(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Config {
            path: path.into(),
            message: message.into(),
        }
    }
}

impl fmt::Display for Problem {
    /// `{path}: {message}`. A hand finding's path starts with `hand.`; a
    /// problem with the whole document is the message alone.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Hand(finding) => write!(f, "hand.{finding}"),
            Self::Config { path, message } if path.is_empty() => f.write_str(message),
            Self::Config { path, message } => write!(f, "{path}: {message}"),
        }
    }
}

impl std::error::Error for Problem {}

/// A config that passed [`Checked::new`]. Only [`Checked::new`] makes one.
///
/// # Examples
///
/// ```
/// use bowery::check::Checked;
/// use bowery::config::{Config, Mode};
///
/// let config = Config::from_yaml("
/// mode: play
/// table: { stakes: { small_blind: 1.0, big_blind: 2.0 } }
/// players: [ { seat: 0, stack: 200 }, { seat: 1, stack: 200, bot: gto } ]
/// ").unwrap();
/// let checked = Checked::new(&config).unwrap();
/// assert_eq!(Mode::Play, checked.config().mode);
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct Checked {
    config: Config,
}

impl Checked {
    /// The config that passed.
    ///
    /// # Examples
    ///
    /// ```
    /// use bowery::check::Checked;
    /// use bowery::config::Config;
    ///
    /// let config = Config::from_yaml("
    /// mode: play
    /// table: { stakes: { small_blind: 1.0, big_blind: 2.0 } }
    /// players: [ { seat: 0, stack: 200 }, { seat: 1, stack: 200, bot: abc } ]
    /// ").unwrap();
    /// assert_eq!(&config, Checked::new(&config).unwrap().config());
    /// ```
    #[must_use]
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Checks a config against every rule and returns it as [`Checked`].
    ///
    /// The rules are listed in the [module docs](self). They all run, and every
    /// problem comes back, except that `start` is looked for in the hand only
    /// when pkcore finds nothing wrong with the hand.
    ///
    /// # Errors
    ///
    /// Returns every [`Problem`] found, in rule order, when there is at least
    /// one.
    ///
    /// # Examples
    ///
    /// ```
    /// use bowery::check::Checked;
    /// use bowery::config::Config;
    ///
    /// let config = Config::from_yaml("
    /// mode: play
    /// table: { stakes: { small_blind: 1.0, big_blind: 2.0 } }
    /// players: [ { seat: 0, stack: 200 }, { seat: 1, stack: 200, bot: shark } ]
    /// ").unwrap();
    /// let problems = Checked::new(&config).unwrap_err();
    /// assert_eq!(1, problems.len());
    /// assert!(problems[0].to_string().starts_with("players[1].bot:"));
    /// ```
    pub fn new(config: &Config) -> Result<Self, Vec<Problem>> {
        let mut problems = Vec::new();
        Self::mode(config, &mut problems);
        let hand_is_true = Self::hand(config, &mut problems);
        Self::seat(config, &mut problems);
        Self::start(config, hand_is_true, &mut problems);
        Self::players(config, &mut problems);
        Self::bots(config, &mut problems);
        Self::deck(config, &mut problems);
        Self::seats(config, &mut problems);
        if problems.is_empty() {
            Ok(Self {
                config: config.clone(),
            })
        } else {
            Err(problems)
        }
    }

    /// The mode has what it needs.
    fn mode(config: &Config, problems: &mut Vec<Problem>) {
        match config.mode {
            Mode::Replay | Mode::Interactive if config.hand.is_none() => {
                problems.push(Problem::config(
                    "hand",
                    format!("{} needs a recorded hand", config.mode),
                ));
            }
            Mode::Play => {
                match config
                    .table
                    .as_ref()
                    .and_then(|table| table.stakes.as_ref())
                {
                    None => problems.push(Problem::config("table.stakes", "play needs the blinds")),
                    Some(stakes) => Self::play_stakes(stakes, problems),
                }
                Self::play_button(config, problems);
                for (index, player) in config.players.iter().enumerate() {
                    let message = match player.stack {
                        None => format!("seat {} needs a stack in play", player.seat),
                        Some(0) => format!("seat {} needs chips to play", player.seat),
                        Some(_) => continue,
                    };
                    problems.push(Problem::config(format!("players[{index}].stack"), message));
                }
            }
            Mode::Replay | Mode::Interactive if config.table.is_some() => {
                problems.push(Problem::config(
                    "table",
                    format!(
                        "{} plays the recorded hand's own table; drop `table`",
                        config.mode
                    ),
                ));
            }
            Mode::Replay | Mode::Interactive => {}
        }
    }

    /// Play's stakes are whole chips, each blind is at least one chip, and the
    /// big blind is no smaller than the small blind. pkcore's engine refuses to
    /// post a blind of nothing.
    fn play_stakes(stakes: &Stakes, problems: &mut Vec<Problem>) {
        let named = [
            ("small_blind", Some(stakes.small_blind)),
            ("big_blind", Some(stakes.big_blind)),
            ("ante", stakes.ante),
        ];
        for (name, value) in named {
            if let Some(value) = value
                && Self::chips(value).is_none()
            {
                problems.push(Problem::config(
                    format!("table.stakes.{name}"),
                    format!("{value} is not a whole number of chips"),
                ));
            }
        }
        for (name, value) in [
            ("small_blind", stakes.small_blind),
            ("big_blind", stakes.big_blind),
        ] {
            if Self::chips(value) == Some(0) {
                problems.push(Problem::config(
                    format!("table.stakes.{name}"),
                    "a blind is at least 1 chip",
                ));
            }
        }
        if stakes.big_blind < stakes.small_blind {
            problems.push(Problem::config(
                "table.stakes.big_blind",
                format!(
                    "{} is smaller than the small blind, {}",
                    stakes.big_blind, stakes.small_blind
                ),
            ));
        }
    }

    /// Play's button is at one of the table's chairs: `table.seats` of them, or
    /// one past the highest seat (at least two) when `seats` is not given, as
    /// the game builds the table.
    fn play_button(config: &Config, problems: &mut Vec<Problem>) {
        let Some(table) = &config.table else {
            return;
        };
        let Some(button) = table.button else {
            return;
        };
        let highest = config
            .players
            .iter()
            .map(|player| player.seat)
            .max()
            .unwrap_or(0);
        let chairs = table.seats.unwrap_or(highest.saturating_add(1)).max(2);
        if button >= chairs {
            problems.push(Problem::config(
                "table.button",
                format!(
                    "the button is on seat {button}, off a {chairs}-chair table (seats 0 to {})",
                    chairs - 1
                ),
            ));
        }
    }

    /// pkcore's findings on the hand. Returns whether there were none.
    fn hand(config: &Config, problems: &mut Vec<Problem>) -> bool {
        let Some(hand) = &config.hand else {
            return true;
        };
        let findings = hand.validate();
        let clean = findings.is_empty();
        problems.extend(findings.into_iter().map(Problem::Hand));
        clean
    }

    /// The seats with a player in them, from `players` in play and from the
    /// hand otherwise. `None` when there is nothing to seat them from.
    fn seated(config: &Config) -> Option<Vec<u8>> {
        match (config.mode, &config.hand) {
            (Mode::Play, _) => Some(config.players.iter().map(|player| player.seat).collect()),
            (_, Some(hand)) => Some(hand.players.iter().map(|player| player.seat).collect()),
            (_, None) => None,
        }
    }

    /// `seat` has a player in it.
    fn seat(config: &Config, problems: &mut Vec<Problem>) {
        let (Some(seat), Some(seated)) = (config.seat, Self::seated(config)) else {
            return;
        };
        if !seated.contains(&seat) {
            problems.push(Problem::config(
                "seat",
                format!("nobody sits in seat {seat}"),
            ));
        }
    }

    /// `start` needs a hand, and the hand reaches it.
    fn start(config: &Config, hand_is_true: bool, problems: &mut Vec<Problem>) {
        let Some(moment) = config.start else {
            return;
        };
        let Some(hand) = &config.hand else {
            problems.push(Problem::config(
                "start",
                format!("{moment} needs a recorded hand"),
            ));
            return;
        };
        if !hand_is_true {
            return;
        }
        match hand.cursor() {
            Ok(replay) if moment.index_in(&replay).is_none() => {
                problems.push(Problem::config(
                    "start",
                    format!(
                        "the hand never reaches {moment}; it has {} events",
                        replay.len()
                    ),
                ));
            }
            Ok(_) => {}
            Err(error) => problems.push(Problem::config(
                "start",
                format!("the hand does not replay: {error}"),
            )),
        }
    }

    /// Players agree with the hand; in play, each seat is taken once and is at
    /// the table.
    fn players(config: &Config, problems: &mut Vec<Problem>) {
        if let Some(hand) = &config.hand
            && config.mode != Mode::Play
        {
            Self::players_agree_with(hand, config, problems);
        }
        let chairs = config.table.as_ref().and_then(|table| table.seats);
        let mut taken = HashSet::new();
        for (index, player) in config.players.iter().enumerate() {
            let path = format!("players[{index}].seat");
            if config.mode == Mode::Play && !taken.insert(player.seat) {
                problems.push(Problem::config(
                    path,
                    format!("seat {} is taken twice", player.seat),
                ));
            } else if let Some(chairs) = chairs
                && player.seat >= chairs
            {
                problems.push(Problem::config(
                    path,
                    format!(
                        "seat {} is not at a {chairs}-chair table (seats 0 to {})",
                        player.seat,
                        chairs - 1
                    ),
                ));
            }
        }
    }

    fn players_agree_with(hand: &HandHistory, config: &Config, problems: &mut Vec<Problem>) {
        for (index, player) in config.players.iter().enumerate() {
            let Some(entry) = hand.players.iter().find(|entry| entry.seat == player.seat) else {
                problems.push(Problem::config(
                    format!("players[{index}].seat"),
                    format!("the hand has nobody in seat {}", player.seat),
                ));
                continue;
            };
            if let Some(name) = &player.name
                && *name != entry.name
            {
                problems.push(Problem::config(
                    format!("players[{index}].name"),
                    format!(
                        "the hand has {} in seat {}, not {name}",
                        entry.name, player.seat
                    ),
                ));
            }
            if let Some(stack) = player.stack
                && (f64::from(stack) - entry.stack).abs() > f64::EPSILON
            {
                problems.push(Problem::config(
                    format!("players[{index}].stack"),
                    format!(
                        "the hand has seat {} starting with {}, not {stack}",
                        player.seat, entry.stack
                    ),
                ));
            }
        }
    }

    /// Every bot is one of pkcore's default profiles.
    fn bots(config: &Config, problems: &mut Vec<Problem>) {
        let profiles = BotProfile::default_profiles();
        for (index, player) in config.players.iter().enumerate() {
            let Some(bot) = &player.bot else {
                continue;
            };
            if !profiles.iter().any(|profile| profile.name == *bot) {
                let names: Vec<&str> = profiles
                    .iter()
                    .map(|profile| profile.name.as_str())
                    .collect();
                problems.push(Problem::config(
                    format!("players[{index}].bot"),
                    format!("`{bot}` is not a bot; the bots are {}", names.join(", ")),
                ));
            }
        }
    }

    /// `deck` parses and holds 52 different cards.
    fn deck(config: &Config, problems: &mut Vec<Problem>) {
        let Some(deck) = &config.deck else {
            return;
        };
        if let Some(message) = Self::deck_fault(deck) {
            problems.push(Problem::config("deck", message));
        }
    }

    /// What is wrong with a deck string, if anything. The separators are
    /// pkcore's: whitespace, commas and hyphens.
    fn deck_fault(deck: &str) -> Option<String> {
        let tokens: Vec<&str> = deck
            .split(|character: char| {
                character.is_whitespace() || character == ',' || character == '-'
            })
            .filter(|token| !token.is_empty())
            .collect();
        if Cards::from_str(deck).is_err() {
            let bad = tokens
                .iter()
                .find(|token| Card::from_str(token).is_err())
                .unwrap_or(&"");
            return Some(format!("`{bad}` is not a card"));
        }
        if tokens.len() != DECK_SIZE {
            return Some(format!(
                "has {} cards; a deck has {}",
                tokens.len(),
                DECK_SIZE
            ));
        }
        let mut seen = HashSet::new();
        tokens
            .iter()
            .find(|token| Card::from_str(token).is_ok_and(|card| !seen.insert(card)))
            .map(|token| format!("{token} appears twice"))
    }

    /// 2 to 10 chairs, and 2 to 10 players.
    fn seats(config: &Config, problems: &mut Vec<Problem>) {
        let table_seats = config.table.as_ref().and_then(|table| table.seats);
        let hand_seats = config.hand.as_ref().and_then(|hand| hand.table.seats);
        let chairs = match (table_seats, hand_seats, config.mode) {
            (Some(seats), _, _) => Some(("table.seats", seats)),
            (None, Some(seats), Mode::Replay | Mode::Interactive) => {
                Some(("hand.table.seats", seats))
            }
            _ => None,
        };
        if let Some((path, chairs)) = chairs
            && !(MIN_SEATS..=MAX_SEATS).contains(&usize::from(chairs))
        {
            problems.push(Problem::config(
                path,
                format!(
                    "{}; a table has {MIN_SEATS} to {MAX_SEATS} chairs",
                    Self::count_of(usize::from(chairs), "chair")
                ),
            ));
        }
        let Some(seated) = Self::seated(config) else {
            return;
        };
        let count = seated.iter().collect::<HashSet<_>>().len();
        if !(MIN_SEATS..=MAX_SEATS).contains(&count) {
            let path = if config.mode == Mode::Play {
                "players"
            } else {
                "hand.players"
            };
            problems.push(Problem::config(
                path,
                format!(
                    "{}; a table seats {MIN_SEATS} to {MAX_SEATS}",
                    Self::count_of(count, "player")
                ),
            ));
        }
    }

    /// A hand-history amount as whole chips: `None` unless it is finite,
    /// non-negative, whole and no more than `u32::MAX`.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the value is checked to be a whole number in 0..=u32::MAX first"
    )]
    pub(crate) fn chips(value: f64) -> Option<usize> {
        (value.is_finite() && value >= 0.0 && value.fract() == 0.0 && value <= f64::from(u32::MAX))
            .then_some(value as usize)
    }

    /// `1 player`, `3 players`.
    fn count_of(count: usize, noun: &str) -> String {
        if count == 1 {
            format!("1 {noun}")
        } else {
            format!("{count} {noun}s")
        }
    }
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use super::*;

    use pkcore::hand_history::Rule;

    #[test]
    fn problem__display_config() {
        let problem = Problem::config("players[1].bot", "`shark` is not a bot");

        assert_eq!("players[1].bot: `shark` is not a bot", problem.to_string());
    }

    #[test]
    fn problem__display_root() {
        let problem = Problem::config("", "unknown field `colour`");

        assert_eq!("unknown field `colour`", problem.to_string());
    }

    #[test]
    fn problem__display_hand() {
        let problem = Problem::Hand(Finding {
            path: "board".to_string(),
            rule: Rule::Cards,
            message: "A♠ appears twice".to_string(),
        });

        assert_eq!("hand.board: A♠ appears twice (Cards)", problem.to_string());
    }

    #[test]
    fn deck_fault__full_deck_is_fine() {
        let deck = Cards::deck().to_string();

        assert_eq!(None, Checked::deck_fault(&deck));
    }

    #[test]
    fn deck_fault__names_the_bad_card() {
        assert_eq!(
            Some("`Z♠` is not a card".to_string()),
            Checked::deck_fault("A♠ Z♠")
        );
    }

    #[test]
    fn deck_fault__counts_cards() {
        assert_eq!(
            Some("has 2 cards; a deck has 52".to_string()),
            Checked::deck_fault("A♠ K♠")
        );
    }

    #[test]
    fn chips__whole_numbers_only() {
        assert_eq!(Some(0), Checked::chips(0.0));
        assert_eq!(Some(600), Checked::chips(600.0));
        for bad in [0.5, -1.0, f64::NAN, f64::INFINITY, 1e12] {
            assert_eq!(None, Checked::chips(bad), "{bad}");
        }
    }

    #[test]
    fn count_of__singular_and_plural() {
        assert_eq!("1 player", Checked::count_of(1, "player"));
        assert_eq!("11 chairs", Checked::count_of(11, "chair"));
    }

    #[test]
    fn check__collects_every_problem() {
        let config =
            Config::from_yaml("mode: play\ndeck: A♠\nplayers: [ { seat: 0, bot: shark } ]\n")
                .unwrap();

        let paths: Vec<String> = Checked::new(&config)
            .unwrap_err()
            .into_iter()
            .map(|problem| match problem {
                Problem::Config { path, .. } => path,
                Problem::Hand(finding) => panic!("unexpected hand.{finding}"),
            })
            .collect();

        assert_eq!(
            vec![
                "table.stakes",
                "players[0].stack",
                "players[0].bot",
                "deck",
                "players"
            ],
            paths
        );
    }
}
