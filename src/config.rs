//! What the author can say about a table.
//!
//! A [`Config`] is the YAML an author writes in a page (or a book chapter):
//! the mode, the view, the layout, the recorded hand, the players and the
//! rest. It is only what was written. [`crate::check`] decides whether it is
//! true, and only a checked config can reach the screen.
//!
//! ```
//! use bowery::config::{Config, Layout, Mode};
//!
//! let config = Config::from_yaml("mode: play\nlayout: ledger\n").unwrap();
//! assert_eq!(Mode::Play, config.mode);
//! assert_eq!(Layout::Ledger, config.layout);
//! ```

use std::fmt;

use pkcore::hand_history::{HandHistory, Replay, ReplayEvent, ReplayStreet, Stakes};
use serde::{Deserialize, Serialize};

use crate::check::Problem;

/// Everything an author says about one table.
///
/// Every field but `mode` is optional. Unknown fields are refused, so a typo
/// is a [`Problem`], not a silently ignored line. Defaults that depend on the
/// mode (the view, the controls) stay `None` here and are settled when the
/// game is built.
///
/// Seat numbers (`seat`, `players[].seat`) are the hand-history format's,
/// which are 0-based.
///
/// # Examples
///
/// ```
/// use bowery::config::{Config, Mode, Moment};
///
/// let config = Config::from_yaml("mode: interactive\nseat: 4\nstart: flop\n").unwrap();
/// assert_eq!(Mode::Interactive, config.mode);
/// assert_eq!(Some(4), config.seat);
/// assert_eq!(Some(Moment::Flop), config.start);
/// ```
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Replay, interactive or play.
    pub mode: Mode,
    /// All cards up, or one seat's. Default: seat in play, all otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub view: Option<View>,
    /// The seat for `view: seat`, and the suggested takeover. 0-based.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seat: Option<u8>,
    /// Ring (default), spotlight or ledger.
    #[serde(default)]
    pub layout: Layout,
    /// The recorded hand, inline. Replay and interactive need it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hand: Option<HandHistory>,
    /// Where the table opens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start: Option<Moment>,
    /// Seats, button and stakes for play, or overrides.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table: Option<TableSetup>,
    /// Who sits where: name, stack and bot per seat.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub players: Vec<PlayerSetup>,
    /// Play: a stacked deck for hand 1, 52 cards.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deck: Option<String>,
    /// Seeds the bots and later shuffles. Default: a hash of the config.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,
    /// Which toggles the reader sees. Defaults by mode.
    #[serde(default)]
    pub controls: Controls,
    /// Bot pacing in milliseconds. Default: 700.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delay_ms: Option<u32>,
}

impl Config {
    /// Reads a config from YAML.
    ///
    /// The `hand:` field is a whole pkcore hand history, inline, read with
    /// the same YAML crate pkcore uses.
    ///
    /// # Errors
    ///
    /// Returns a [`Problem::Config`] at the document's root when the text is
    /// not YAML, a field is unknown, a value has the wrong type or a
    /// required field (`mode`) is missing. The message names the field and
    /// the line.
    ///
    /// # Examples
    ///
    /// ```
    /// use bowery::config::{Config, Mode};
    ///
    /// let config = Config::from_yaml("mode: replay\n").unwrap();
    /// assert_eq!(Mode::Replay, config.mode);
    /// assert!(Config::from_yaml("mode: replay\ncolour: red\n").is_err());
    /// ```
    pub fn from_yaml(yaml: &str) -> Result<Self, Problem> {
        serde_yaml_bw::from_str(yaml).map_err(|error| Problem::Config {
            path: String::new(),
            message: error.to_string(),
        })
    }

    /// Writes this config as YAML that [`Config::from_yaml`] reads back
    /// equal.
    ///
    /// # Errors
    ///
    /// Returns a [`Problem::Config`] at the root if the YAML writer fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use bowery::config::Config;
    ///
    /// let config = Config::from_yaml("mode: play\n").unwrap();
    /// let yaml = config.to_yaml().unwrap();
    /// assert_eq!(config, Config::from_yaml(&yaml).unwrap());
    /// ```
    pub fn to_yaml(&self) -> Result<String, Problem> {
        serde_yaml_bw::to_string(self).map_err(|error| Problem::Config {
            path: String::new(),
            message: error.to_string(),
        })
    }
}

/// How the reader meets the table.
///
/// # Examples
///
/// ```
/// use bowery::config::Mode;
///
/// assert_eq!("interactive", Mode::Interactive.to_string());
/// ```
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// A strict replay of the recorded hand: the reader only moves between
    /// its events.
    Replay,
    /// A replay where the reader can take over any live seat.
    Interactive,
    /// A table against pkcore's bots; hands continue.
    Play,
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Replay => "replay",
            Self::Interactive => "interactive",
            Self::Play => "play",
        })
    }
}

/// Whose hole cards are face up.
///
/// # Examples
///
/// ```
/// use bowery::config::View;
///
/// assert_eq!("seat", View::Seat.to_string());
/// ```
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum View {
    /// Every hole card face up, like the TV hole-card cams.
    All,
    /// One seat's cards; the rest stay face down until showdown.
    Seat,
}

impl fmt::Display for View {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::All => "all",
            Self::Seat => "seat",
        })
    }
}

/// How the table is drawn.
///
/// # Examples
///
/// ```
/// use bowery::config::Layout;
///
/// assert_eq!(Layout::Ring, Layout::default());
/// assert_eq!("spotlight", Layout::Spotlight.to_string());
/// ```
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Layout {
    /// Seats on fixed slots outside an outlined oval. The default.
    #[default]
    Ring,
    /// Folded seats collapse into one strip of names on top; live seats
    /// split the width around the board. Suits a hand that comes down to
    /// two players.
    Spotlight,
    /// One row per seat, with the board and pot on top. The narrow-width
    /// fallback.
    Ledger,
}

impl fmt::Display for Layout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Ring => "ring",
            Self::Spotlight => "spotlight",
            Self::Ledger => "ledger",
        })
    }
}

/// A moment in a recorded hand, where a table opens.
///
/// In YAML it is a street name (`preflop`, `flop`, `turn`, `river`,
/// `showdown`) or an event index into pkcore's replay cursor.
///
/// # Examples
///
/// ```
/// use bowery::config::{Config, Moment};
///
/// let config = Config::from_yaml("mode: replay\nstart: 7\n").unwrap();
/// assert_eq!(Some(Moment::Event(7)), config.start);
/// assert_eq!("river", Moment::River.to_string());
/// ```
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(try_from = "MomentRepr", into = "MomentRepr")]
pub enum Moment {
    /// Before the first action: blinds and antes posted, hole cards dealt.
    Preflop,
    /// The flop is dealt.
    Flop,
    /// The turn is dealt.
    Turn,
    /// The river is dealt.
    River,
    /// Two or more seats show down.
    Showdown,
    /// The cursor's event at this index; 0 is the starting table.
    Event(usize),
}

impl Moment {
    /// The index of this moment among `replay`'s events, or `None` if the
    /// hand never gets there.
    ///
    /// A street is its `Deal` event; `Preflop` is the starting table (event
    /// 0); `Showdown` is the `Showdown` event, present only when two or more
    /// seats reach the end; `Event(i)` is `i` itself while it is in range.
    ///
    /// # Examples
    ///
    /// ```
    /// use bowery::config::Moment;
    /// use pkcore::hand_history::HandHistory;
    ///
    /// let yaml = "
    /// hand: { id: fold, game: holdem }
    /// table: { seats: 2, button: 0, stakes: { small_blind: 1.0, big_blind: 2.0 } }
    /// players:
    ///   - { seat: 0, name: A, stack: 100.0 }
    ///   - { seat: 1, name: B, stack: 100.0 }
    /// streets: { preflop: { actions: [ { seat: 0, action: fold } ] } }
    /// ";
    /// let replay = HandHistory::from_yaml(yaml).unwrap().cursor().unwrap();
    /// assert_eq!(Some(0), Moment::Preflop.index_in(&replay));
    /// assert_eq!(None, Moment::Flop.index_in(&replay));
    /// ```
    #[must_use]
    pub fn index_in(self, replay: &Replay) -> Option<usize> {
        let wanted = |event: &ReplayEvent| match self {
            Self::Preflop => matches!(event, ReplayEvent::Forced),
            Self::Flop => matches!(event, ReplayEvent::Deal(ReplayStreet::Flop)),
            Self::Turn => matches!(event, ReplayEvent::Deal(ReplayStreet::Turn)),
            Self::River => matches!(event, ReplayEvent::Deal(ReplayStreet::River)),
            Self::Showdown => matches!(event, ReplayEvent::Showdown),
            Self::Event(_) => false,
        };
        match self {
            Self::Event(index) => (index < replay.len()).then_some(index),
            _ => (0..replay.len()).find(|&index| replay.event(index).is_some_and(wanted)),
        }
    }
}

impl fmt::Display for Moment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Preflop => f.write_str("preflop"),
            Self::Flop => f.write_str("flop"),
            Self::Turn => f.write_str("turn"),
            Self::River => f.write_str("river"),
            Self::Showdown => f.write_str("showdown"),
            Self::Event(index) => write!(f, "event {index}"),
        }
    }
}

/// The YAML shape of a [`Moment`]: an index or a name.
#[derive(Deserialize, Serialize)]
#[serde(untagged)]
enum MomentRepr {
    Index(usize),
    Name(String),
}

impl TryFrom<MomentRepr> for Moment {
    type Error = String;

    fn try_from(repr: MomentRepr) -> Result<Self, Self::Error> {
        match repr {
            MomentRepr::Index(index) => Ok(Self::Event(index)),
            MomentRepr::Name(name) => match name.as_str() {
                "preflop" => Ok(Self::Preflop),
                "flop" => Ok(Self::Flop),
                "turn" => Ok(Self::Turn),
                "river" => Ok(Self::River),
                "showdown" => Ok(Self::Showdown),
                _ => Err(format!(
                    "unknown moment `{name}`: expected preflop, flop, turn, river, showdown or an event index"
                )),
            },
        }
    }
}

impl From<Moment> for MomentRepr {
    fn from(moment: Moment) -> Self {
        match moment {
            Moment::Event(index) => Self::Index(index),
            named => Self::Name(named.to_string()),
        }
    }
}

/// The table itself: how many seats, where the button is, the stakes.
///
/// In play it sets the table up; with a recorded hand it overrides the
/// hand's own `table`.
///
/// # Examples
///
/// ```
/// use bowery::config::Config;
///
/// let config = Config::from_yaml("mode: play\ntable: { seats: 6, button: 2 }\n").unwrap();
/// let table = config.table.unwrap();
/// assert_eq!(Some(6), table.seats);
/// assert_eq!(Some(2), table.button);
/// ```
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TableSetup {
    /// Chairs at the table, 2 to 10.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seats: Option<u8>,
    /// The button's seat, 0-based.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub button: Option<u8>,
    /// Blinds and ante, in pkcore's hand-history shape.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stakes: Option<Stakes>,
}

/// One seat: who sits there, with how many chips, and which bot plays it.
///
/// With a recorded hand, a given `name` or `stack` must agree with the
/// hand's player in that seat.
///
/// # Examples
///
/// ```
/// use bowery::config::Config;
///
/// let config = Config::from_yaml("mode: play\nplayers: [{ seat: 1, stack: 500, bot: abc }]\n").unwrap();
/// let player = &config.players[0];
/// assert_eq!(1, player.seat);
/// assert_eq!(Some(500), player.stack);
/// assert_eq!(Some("abc"), player.bot.as_deref());
/// ```
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PlayerSetup {
    /// The seat, 0-based.
    pub seat: u8,
    /// The name readers see.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Starting chips.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stack: Option<u32>,
    /// A pkcore bot profile name, such as `gto` or `maniac`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bot: Option<String>,
}

/// Which toggles the reader sees under the table. `None` means the mode's
/// default.
///
/// # Examples
///
/// ```
/// use bowery::config::Controls;
///
/// let controls = Controls::default();
/// assert_eq!(None, controls.view);
/// assert_eq!(None, controls.layout);
/// ```
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Controls {
    /// The 👁 view toggle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub view: Option<bool>,
    /// The ▦ layout toggle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout: Option<bool>,
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use super::*;

    const HEADS_UP: &str = include_str!("../tests/hands/heads_up_all_in.yaml");

    fn replay_yaml() -> String {
        let hand: String = HEADS_UP
            .lines()
            .flat_map(|line| ["  ", line, "\n"])
            .collect();
        format!("mode: replay\nstart: flop\nhand:\n{hand}")
    }

    #[test]
    fn config__from_yaml_reads_a_replay() {
        let config = Config::from_yaml(&replay_yaml()).unwrap();

        assert_eq!(Mode::Replay, config.mode);
        assert_eq!(Some(Moment::Flop), config.start);
        assert_eq!(2, config.hand.unwrap().players.len());
    }

    #[test]
    fn config__round_trips_through_yaml() {
        let config = Config::from_yaml(&replay_yaml()).unwrap();

        let again = Config::from_yaml(&config.to_yaml().unwrap()).unwrap();

        assert_eq!(config, again);
    }

    #[test]
    fn config__unknown_field_is_a_problem() {
        let problem = Config::from_yaml("mode: play\ncolour: red\n").unwrap_err();

        assert!(problem.to_string().contains("colour"), "{problem}");
    }

    #[test]
    fn config__defaults() {
        let config = Config::from_yaml("mode: play\n").unwrap();

        assert_eq!(Layout::Ring, config.layout);
        assert_eq!(Controls::default(), config.controls);
        assert_eq!(Vec::<PlayerSetup>::new(), config.players);
        assert_eq!(None, config.view);
    }

    #[test]
    fn config__reads_play_setup() {
        let yaml = "\
mode: play
view: seat
seat: 0
layout: ledger
table: { seats: 3, button: 2, stakes: { small_blind: 1.0, big_blind: 2.0 } }
players:
  - { seat: 0, name: Reader, stack: 200 }
  - { seat: 1, name: Bot, stack: 200, bot: maniac }
deck: A♠ K♠
seed: 7
controls: { view: true, layout: false }
delay_ms: 300
";
        let config = Config::from_yaml(yaml).unwrap();

        assert_eq!(Some(View::Seat), config.view);
        assert_eq!(Some(0), config.seat);
        assert_eq!(Layout::Ledger, config.layout);
        let table = config.table.unwrap();
        assert_eq!(Some(3), table.seats);
        assert_eq!(Some(2), table.button);
        assert!((table.stakes.unwrap().big_blind - 2.0).abs() < f64::EPSILON);
        assert_eq!(Some("maniac"), config.players[1].bot.as_deref());
        assert_eq!(Some(200), config.players[0].stack);
        assert_eq!(Some("A♠ K♠"), config.deck.as_deref());
        assert_eq!(Some(7), config.seed);
        assert_eq!(Some(true), config.controls.view);
        assert_eq!(Some(false), config.controls.layout);
        assert_eq!(Some(300), config.delay_ms);
    }

    #[test]
    fn moment__reads_names_and_indexes() {
        for (yaml, moment) in [
            ("preflop", Moment::Preflop),
            ("flop", Moment::Flop),
            ("turn", Moment::Turn),
            ("river", Moment::River),
            ("showdown", Moment::Showdown),
            ("12", Moment::Event(12)),
        ] {
            let config = Config::from_yaml(&format!("mode: play\nstart: {yaml}\n")).unwrap();
            assert_eq!(Some(moment), config.start, "{yaml}");
        }
    }

    #[test]
    fn moment__unknown_name_is_a_problem() {
        let problem = Config::from_yaml("mode: play\nstart: dawn\n").unwrap_err();

        assert!(problem.to_string().contains("dawn"), "{problem}");
    }

    #[test]
    fn moment__index_in_a_full_hand() {
        let replay = HandHistory::from_yaml(HEADS_UP).unwrap().cursor().unwrap();
        let last = replay.len() - 1;

        assert_eq!(Some(0), Moment::Preflop.index_in(&replay));
        assert_eq!(Some(3), Moment::Flop.index_in(&replay));
        assert_eq!(Some(4), Moment::Turn.index_in(&replay));
        assert_eq!(Some(5), Moment::River.index_in(&replay));
        assert!(Moment::Showdown.index_in(&replay).is_some());
        assert_eq!(Some(last), Moment::Event(last).index_in(&replay));
        assert_eq!(None, Moment::Event(last + 1).index_in(&replay));
    }

    #[test]
    fn moment__index_in_a_folded_hand() {
        let yaml = include_str!("../tests/hands/heads_up_fold.yaml");
        let replay = HandHistory::from_yaml(yaml).unwrap().cursor().unwrap();

        assert_eq!(Some(0), Moment::Preflop.index_in(&replay));
        for moment in [Moment::Flop, Moment::Turn, Moment::River, Moment::Showdown] {
            assert_eq!(None, moment.index_in(&replay), "{moment}");
        }
    }

    #[test]
    fn moment__display() {
        assert_eq!("flop", Moment::Flop.to_string());
        assert_eq!("event 3", Moment::Event(3).to_string());
    }
}
