//! One table's game: replay, takeover, play, and the end of a hand.
//!
//! A [`Game`] is built from a [`Checked`] config and owns all of its state,
//! so many games can run side by side on one page. It is a line of
//! **moments**. Each moment is one [`Event`] and the pkcore [`Table`] after
//! it. The reader moves between moments with [`Game::seek`] and
//! [`Game::advance`].
//!
//! - **Replay** is the record: the moments are pkcore's replay cursor events,
//!   one for one.
//! - **Interactive** is the record until the reader, who has taken a seat,
//!   acts differently from it. From then on the hand is **off the record**:
//!   pkcore's engine plays on from the cursor's table, whose deck is the
//!   hand's own, so the board is the real board, and the other seats are bots.
//! - **Play** deals hands with pkcore's engine: hand 1 from the config's
//!   `deck` if it gives one, later hands from a seeded shuffle.
//!
//! Every hand ends the same way, on the record or off it: each street dealt as
//! its own moment, then **Return** (an uncalled bet goes back), **Showdown**
//! and **Award** (once per pot share won), each present only when it happens.
//! Off the record the hand is rebuilt as a pkcore hand history when it ends,
//! and pkcore's cursor names those last moments, so bowery computes no
//! settlement of its own.
//!
//! ```
//! use bowery::check::check;
//! use bowery::config::Config;
//! use bowery::game::{Advance, Event, Game};
//!
//! let config = Config::from_yaml("
//! mode: play
//! seat: 0
//! table: { stakes: { small_blind: 1.0, big_blind: 2.0 } }
//! players: [ { seat: 0, stack: 200 }, { seat: 1, stack: 200, bot: abc } ]
//! ").unwrap();
//! let mut game = Game::new(check(&config).unwrap()).unwrap();
//!
//! assert_eq!(Event::Forced, *game.event());
//! assert_eq!(Advance::Waiting, game.advance().unwrap()); // the reader is first to act
//! ```
//!
//! [`Checked`]: crate::check::Checked

use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::str::FromStr;

use pkcore::PKError;
use pkcore::bot::profile::BotProfile;
use pkcore::cards::Cards;
use pkcore::casino::action::PlayerAction;
use pkcore::casino::game::ForcedBets;
use pkcore::casino::session::{PokerSession, SessionStep};
use pkcore::casino::table::{Player, Seat, Seats, Table};
use pkcore::hand_history::{
    HandHistory, HandMeta, HandVariant, PlayerEntry, ReplayEvent, ReplayStreet, Stakes, Streets,
    TableInfo,
};
use rand::SeedableRng;
use rand::rngs::SmallRng;
use uuid::Uuid;

use crate::check::{Checked, chips};
use crate::config::{Config, Mode, View};
use crate::frame::Frame;

/// What happened at a moment. It mirrors pkcore's [`ReplayEvent`], and
/// off the record the game makes the same events itself.
///
/// # Examples
///
/// ```
/// use bowery::game::Event;
/// use pkcore::hand_history::{ReplayEvent, ReplayStreet};
///
/// let event = Event::from_replay(&ReplayEvent::Deal(ReplayStreet::Flop)).unwrap();
/// assert_eq!(Event::Deal(ReplayStreet::Flop), event);
/// assert_eq!("deal the flop", event.to_string());
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// The starting table: blinds and antes posted, hole cards dealt.
    Forced,
    /// A seat acted.
    Action {
        /// The acting seat, 0-based.
        seat: u8,
        /// What it did.
        action: PlayerAction,
        /// The street it acted on.
        street: ReplayStreet,
    },
    /// The bets went into the pot and the street's cards were dealt.
    Deal(ReplayStreet),
    /// Chips nobody matched went back to the seat that bet them. Not a win.
    Return {
        /// The seat the chips went back to.
        seat: u8,
        /// The chips returned.
        amount: usize,
    },
    /// Two or more seats show their cards.
    Showdown,
    /// One pot share was paid to a seat.
    Award {
        /// The seat paid.
        seat: u8,
        /// The chips won.
        amount: usize,
    },
}

impl Event {
    /// The event a pkcore replay event is, or `None` for an event this
    /// version of bowery does not know (`ReplayEvent` is non-exhaustive).
    ///
    /// # Examples
    ///
    /// ```
    /// use bowery::game::Event;
    /// use pkcore::hand_history::ReplayEvent;
    ///
    /// assert_eq!(Some(Event::Showdown), Event::from_replay(&ReplayEvent::Showdown));
    /// ```
    #[must_use]
    pub fn from_replay(event: &ReplayEvent) -> Option<Self> {
        Some(match event {
            ReplayEvent::Forced => Self::Forced,
            ReplayEvent::Action {
                seat,
                action,
                street,
                ..
            } => Self::Action {
                seat: *seat,
                action: *action,
                street: *street,
            },
            ReplayEvent::Deal(street) => Self::Deal(*street),
            ReplayEvent::Return { seat, amount } => Self::Return {
                seat: *seat,
                amount: *amount,
            },
            ReplayEvent::Showdown => Self::Showdown,
            ReplayEvent::Award { seat, amount } => Self::Award {
                seat: *seat,
                amount: *amount,
            },
            _ => return None,
        })
    }

    /// True for the events after the last action: return, showdown, award.
    fn ends_the_hand(&self) -> bool {
        matches!(
            self,
            Self::Return { .. } | Self::Showdown | Self::Award { .. }
        )
    }
}

impl fmt::Display for Event {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Forced => f.write_str("forced bets"),
            Self::Action {
                seat,
                action,
                street,
            } => write!(f, "{street}: seat {seat} {action}"),
            Self::Deal(street) => write!(f, "deal the {street}"),
            Self::Return { seat, amount } => write!(f, "seat {seat} takes back {amount}"),
            Self::Showdown => f.write_str("showdown"),
            Self::Award { seat, amount } => write!(f, "seat {seat} wins {amount}"),
        }
    }
}

/// What [`Game::advance`] did.
///
/// # Examples
///
/// ```
/// use bowery::game::Advance;
///
/// assert_ne!(Advance::Moved, Advance::Ended);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Advance {
    /// One moment on: a step through the record, a bot's action, a deal, or
    /// one of the hand's last moments.
    Moved,
    /// Nothing changed: it is the reader's turn.
    Waiting,
    /// Nothing changed: the hand is over and this is its last moment.
    Ended,
}

/// Where the game stands against the recorded hand.
///
/// # Examples
///
/// ```
/// use bowery::game::RecordState;
///
/// assert_eq!(RecordState::OffRecord { since: 11 }, RecordState::OffRecord { since: 11 });
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecordState {
    /// The moments are the record's.
    OnRecord,
    /// The moments after `since` are the game's own.
    OffRecord {
        /// The last moment on the record: the takeover.
        since: usize,
    },
    /// Play mode: there is no record.
    NotAReplay,
}

/// Why a game refused a move.
///
/// # Examples
///
/// ```
/// use bowery::game::GameError;
///
/// assert_eq!("seat 0 is not in the hand", GameError::SeatNotLive { seat: 0 }.to_string());
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GameError {
    /// Taking a seat, or acting, needs interactive or play mode.
    NotInteractive,
    /// Only play mode deals another hand.
    NotPlay,
    /// The reader has no seat.
    NoReader,
    /// It is not the reader's turn at this moment.
    NotYourTurn,
    /// The seat has nobody in it, or has folded.
    SeatNotLive {
        /// The seat asked for.
        seat: u8,
    },
    /// The hand is over; nothing more can be acted on in it.
    HandIsOver,
    /// The hand has moments still to show.
    HandNotOver,
    /// No such moment.
    OutOfRange {
        /// The moment asked for.
        index: usize,
        /// How many moments there are.
        moments: usize,
    },
    /// The engine refused the reader's action.
    Illegal {
        /// The action refused.
        action: PlayerAction,
        /// The engine's reason.
        reason: PKError,
    },
    /// Fewer than two seats have chips; no hand can be dealt.
    TableBroken,
    /// The engine failed.
    Engine(PKError),
    /// A checked config the engine still cannot set up, or a hand off the
    /// record that pkcore cannot replay. Either is a bowery bug.
    Setup(String),
}

impl fmt::Display for GameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotInteractive => f.write_str("the reader can only watch a replay"),
            Self::NotPlay => f.write_str("only play mode deals another hand"),
            Self::NoReader => f.write_str("take a seat first"),
            Self::NotYourTurn => f.write_str("it is not your turn"),
            Self::SeatNotLive { seat } => write!(f, "seat {seat} is not in the hand"),
            Self::HandIsOver => f.write_str("the hand is over"),
            Self::HandNotOver => f.write_str("the hand is not over"),
            Self::OutOfRange { index, moments } => {
                write!(f, "moment {index} of {moments} does not exist")
            }
            Self::Illegal { action, reason } => write!(f, "{action} is not allowed: {reason}"),
            Self::TableBroken => f.write_str("fewer than two players have chips"),
            Self::Engine(error) => write!(f, "the engine failed: {error}"),
            Self::Setup(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for GameError {}

impl From<PKError> for GameError {
    fn from(error: PKError) -> Self {
        Self::Engine(error)
    }
}

/// One moment: what happened, and the table after it.
#[derive(Clone, Debug, PartialEq)]
struct Step {
    event: Event,
    table: Table,
}

/// A hand the engine is playing: off the record, or in play.
struct Live {
    session: PokerSession,
    /// Where the hand's own entries start in the table's event log.
    log_from: usize,
    /// The street being bet.
    street: ReplayStreet,
    /// What the hand history rebuilt at the end starts from: the record, or
    /// the play hand's players and table. Its streets are replaced.
    base: HandHistory,
    /// Off the record: the takeover moment. `None` in play.
    since: Option<usize>,
    /// The hand's last moments, worked out and not yet shown.
    pending: VecDeque<Step>,
    /// The hand's last moments have been worked out.
    settled: bool,
}

impl Live {
    /// The seat to act now, without dealing anything. `None` when a deal or
    /// the end of the hand comes next.
    fn turn(&self) -> Option<u8> {
        let session = &self.session;
        (!self.settled && !session.is_hand_complete() && !session.table.seats.is_betting_complete())
            .then(|| session.table.next_to_act())
    }
}

/// One table's game. See the [module docs](self).
///
/// # Examples
///
/// ```
/// use bowery::check::check;
/// use bowery::config::Config;
/// use bowery::game::{Advance, Game};
///
/// let config = Config::from_yaml("
/// mode: play
/// seat: 0
/// table: { stakes: { small_blind: 1.0, big_blind: 2.0 } }
/// players: [ { seat: 0, stack: 200 }, { seat: 1, stack: 200, bot: abc } ]
/// ").unwrap();
/// let mut game = Game::new(check(&config).unwrap()).unwrap();
/// game.advance().unwrap();
/// game.act(pkcore::casino::action::PlayerAction::Fold).unwrap();
/// while game.advance().unwrap() != Advance::Ended {}
/// assert_eq!(201, game.table().seats.get_seat(1).unwrap().player.chips);
/// ```
pub struct Game {
    checked: Checked,
    /// The seed the config gave, or the hash of it.
    seed: u64,
    rng: SmallRng,
    bots: HashMap<u8, BotProfile>,
    reader: Option<u8>,
    /// The recorded hand's moments. Empty in play.
    record: Vec<Step>,
    /// The moments the reader moves between.
    line: Vec<Step>,
    live: Option<Live>,
    at: usize,
    hand_number: u32,
}

impl fmt::Debug for Game {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Game")
            .field("mode", &self.config().mode)
            .field("reader", &self.reader)
            .field("at", &self.at)
            .field("moments", &self.line.len())
            .field("record", &self.record_state())
            .field("hand_number", &self.hand_number)
            .finish_non_exhaustive()
    }
}

impl Game {
    /// Builds the game a checked config describes, at its `start` moment.
    ///
    /// Replay and interactive open on the record, with no seat taken. Play
    /// deals hand 1, and the reader is `seat`, or else the first player with
    /// no `bot`. Seats with no `bot` that are not the reader are played by
    /// pkcore's `gto` profile. `seed` (default: a hash of the config) seeds
    /// the bots and the shuffles; it is a `SmallRng`, whose sequence can
    /// differ between 32- and 64-bit targets.
    ///
    /// # Errors
    ///
    /// [`GameError::Engine`] or [`GameError::Setup`] if pkcore cannot replay
    /// the hand or set up the table, which a checked config should rule out.
    ///
    /// # Examples
    ///
    /// ```
    /// use bowery::check::check;
    /// use bowery::config::Config;
    /// use bowery::game::Game;
    ///
    /// let config = Config::from_yaml("
    /// mode: play
    /// table: { stakes: { small_blind: 1.0, big_blind: 2.0 } }
    /// players: [ { seat: 0, stack: 200 }, { seat: 1, stack: 200, bot: abc } ]
    /// ").unwrap();
    /// let game = Game::new(check(&config).unwrap()).unwrap();
    /// assert_eq!(Some(0), game.reader());
    /// ```
    pub fn new(checked: Checked) -> Result<Self, GameError> {
        let config = checked.config();
        let seed = config.seed.unwrap_or_else(|| default_seed(config));
        let profiles = BotProfile::default_profiles();
        let bots = config
            .players
            .iter()
            .filter_map(|player| {
                let name = player.bot.as_ref()?;
                let profile = profiles.iter().find(|profile| profile.name == *name)?;
                Some((player.seat, profile.clone()))
            })
            .collect();
        let mut game = Self {
            checked,
            seed,
            rng: SmallRng::seed_from_u64(seed),
            bots,
            reader: None,
            record: Vec::new(),
            line: Vec::new(),
            live: None,
            at: 0,
            hand_number: 1,
        };
        if game.config().mode == Mode::Play {
            game.open_play()?;
        } else {
            game.open_record()?;
        }
        Ok(game)
    }

    /// The config this game was built from.
    ///
    /// # Examples
    ///
    /// ```
    /// use bowery::check::check;
    /// use bowery::config::{Config, Mode};
    /// use bowery::game::Game;
    ///
    /// let config = Config::from_yaml("
    /// mode: play
    /// table: { stakes: { small_blind: 1.0, big_blind: 2.0 } }
    /// players: [ { seat: 0, stack: 200 }, { seat: 1, stack: 200, bot: abc } ]
    /// ").unwrap();
    /// let game = Game::new(check(&config).unwrap()).unwrap();
    /// assert_eq!(Mode::Play, game.config().mode);
    /// ```
    #[must_use]
    pub fn config(&self) -> &Config {
        self.checked.config()
    }

    /// The number of moments. On the record it is the replay's length; off
    /// the record and in play it grows as the hand goes on.
    ///
    /// # Examples
    ///
    /// ```
    /// use bowery::check::check;
    /// use bowery::config::Config;
    /// use bowery::game::Game;
    ///
    /// let config = Config::from_yaml("
    /// mode: play
    /// table: { stakes: { small_blind: 1.0, big_blind: 2.0 } }
    /// players: [ { seat: 0, stack: 200 }, { seat: 1, stack: 200, bot: abc } ]
    /// ").unwrap();
    /// let game = Game::new(check(&config).unwrap()).unwrap();
    /// assert_eq!(1, game.moments()); // the forced bets
    /// ```
    #[must_use]
    pub fn moments(&self) -> usize {
        self.line.len()
    }

    /// The index of the moment on screen.
    ///
    /// # Examples
    ///
    /// ```
    /// use bowery::check::check;
    /// use bowery::config::Config;
    /// use bowery::game::Game;
    ///
    /// let config = Config::from_yaml("
    /// mode: play
    /// table: { stakes: { small_blind: 1.0, big_blind: 2.0 } }
    /// players: [ { seat: 0, stack: 200 }, { seat: 1, stack: 200, bot: abc } ]
    /// ").unwrap();
    /// let game = Game::new(check(&config).unwrap()).unwrap();
    /// assert_eq!(0, game.at());
    /// ```
    #[must_use]
    pub fn at(&self) -> usize {
        self.at
    }

    /// What happened at the moment on screen.
    ///
    /// # Examples
    ///
    /// ```
    /// use bowery::check::check;
    /// use bowery::config::Config;
    /// use bowery::game::{Event, Game};
    ///
    /// let config = Config::from_yaml("
    /// mode: play
    /// table: { stakes: { small_blind: 1.0, big_blind: 2.0 } }
    /// players: [ { seat: 0, stack: 200 }, { seat: 1, stack: 200, bot: abc } ]
    /// ").unwrap();
    /// let game = Game::new(check(&config).unwrap()).unwrap();
    /// assert_eq!(Event::Forced, *game.event());
    /// ```
    #[must_use]
    pub fn event(&self) -> &Event {
        &self.step().event
    }

    /// What happened at moment `index`, or `None` past the last moment.
    ///
    /// # Examples
    ///
    /// ```
    /// use bowery::check::check;
    /// use bowery::config::Config;
    /// use bowery::game::{Event, Game};
    ///
    /// let config = Config::from_yaml("
    /// mode: play
    /// table: { stakes: { small_blind: 1.0, big_blind: 2.0 } }
    /// players: [ { seat: 0, stack: 200 }, { seat: 1, stack: 200, bot: abc } ]
    /// ").unwrap();
    /// let game = Game::new(check(&config).unwrap()).unwrap();
    /// assert_eq!(Some(&Event::Forced), game.event_at(0));
    /// assert_eq!(None, game.event_at(1));
    /// ```
    #[must_use]
    pub fn event_at(&self, index: usize) -> Option<&Event> {
        self.line.get(index).map(|step| &step.event)
    }

    /// The view the config asks for: its `view`, or else `seat` in play and
    /// `all` in replay and interactive.
    ///
    /// # Examples
    ///
    /// ```
    /// use bowery::check::check;
    /// use bowery::config::{Config, View};
    /// use bowery::game::Game;
    ///
    /// let config = Config::from_yaml("
    /// mode: play
    /// table: { stakes: { small_blind: 1.0, big_blind: 2.0 } }
    /// players: [ { seat: 0, stack: 200 }, { seat: 1, stack: 200, bot: abc } ]
    /// ").unwrap();
    /// assert_eq!(View::Seat, Game::new(check(&config).unwrap()).unwrap().view());
    /// ```
    #[must_use]
    pub fn view(&self) -> View {
        let config = self.config();
        config.view.unwrap_or(if config.mode == Mode::Play {
            View::Seat
        } else {
            View::All
        })
    }

    /// What is on screen at this moment, through `view`. See [`Frame::new`].
    ///
    /// # Examples
    ///
    /// ```
    /// use bowery::check::check;
    /// use bowery::config::{Config, View};
    /// use bowery::game::Game;
    ///
    /// let config = Config::from_yaml("
    /// mode: play
    /// table: { stakes: { small_blind: 1.0, big_blind: 2.0 } }
    /// players: [ { seat: 0, stack: 200 }, { seat: 1, stack: 200, bot: abc } ]
    /// ").unwrap();
    /// let game = Game::new(check(&config).unwrap()).unwrap();
    /// assert_eq!(3, game.frame(View::All).pot);
    /// ```
    #[must_use]
    pub fn frame(&self, view: View) -> Frame {
        Frame::new(self, view)
    }

    /// The seed this game runs on: the config's `seed`, or else a hash of
    /// the config. The odds draw their Monte Carlo samples from it, so the
    /// same config always shows the same percents.
    ///
    /// # Examples
    ///
    /// ```
    /// use bowery::check::check;
    /// use bowery::config::Config;
    /// use bowery::game::Game;
    ///
    /// let config = Config::from_yaml("
    /// mode: play
    /// seed: 7
    /// table: { stakes: { small_blind: 1.0, big_blind: 2.0 } }
    /// players: [ { seat: 0, stack: 200 }, { seat: 1, stack: 200, bot: abc } ]
    /// ").unwrap();
    /// let game = Game::new(check(&config).unwrap()).unwrap();
    /// assert_eq!(7, game.seed());
    /// ```
    #[must_use]
    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// The frame for the moment on screen with the win percents on it: see
    /// [`Frame::with_odds`].
    ///
    /// # Examples
    ///
    /// ```
    /// use bowery::check::check;
    /// use bowery::config::{Config, View};
    /// use bowery::game::Game;
    ///
    /// let config = Config::from_yaml("
    /// mode: play
    /// table: { stakes: { small_blind: 1.0, big_blind: 2.0 } }
    /// players: [ { seat: 0, stack: 200 }, { seat: 1, stack: 200, bot: abc } ]
    /// ").unwrap();
    /// let game = Game::new(check(&config).unwrap()).unwrap();
    /// assert!(game.frame_with_odds(View::All).odds.is_some());
    /// assert!(game.frame(View::All).odds.is_none());
    /// ```
    #[must_use]
    pub fn frame_with_odds(&self, view: View) -> Frame {
        Frame::with_odds(self, view)
    }

    /// The pkcore table after the moment on screen.
    ///
    /// # Examples
    ///
    /// ```
    /// use bowery::check::check;
    /// use bowery::config::Config;
    /// use bowery::game::Game;
    ///
    /// let config = Config::from_yaml("
    /// mode: play
    /// table: { stakes: { small_blind: 1.0, big_blind: 2.0 } }
    /// players: [ { seat: 0, stack: 200 }, { seat: 1, stack: 200, bot: abc } ]
    /// ").unwrap();
    /// let game = Game::new(check(&config).unwrap()).unwrap();
    /// assert_eq!(3, game.table().seats.get_seat(0).unwrap().player.bet
    ///     + game.table().seats.get_seat(1).unwrap().player.bet);
    /// ```
    #[must_use]
    pub fn table(&self) -> &Table {
        &self.step().table
    }

    /// The reader's seat, if they have one.
    ///
    /// # Examples
    ///
    /// ```
    /// use bowery::check::check;
    /// use bowery::config::Config;
    /// use bowery::game::Game;
    ///
    /// let config = Config::from_yaml("
    /// mode: play
    /// seat: 1
    /// table: { stakes: { small_blind: 1.0, big_blind: 2.0 } }
    /// players: [ { seat: 0, stack: 200, bot: gto }, { seat: 1, stack: 200 } ]
    /// ").unwrap();
    /// assert_eq!(Some(1), Game::new(check(&config).unwrap()).unwrap().reader());
    /// ```
    #[must_use]
    pub fn reader(&self) -> Option<u8> {
        self.reader
    }

    /// The hand being played: 1, then one more per [`Game::next_hand`].
    ///
    /// # Examples
    ///
    /// ```
    /// use bowery::check::check;
    /// use bowery::config::Config;
    /// use bowery::game::Game;
    ///
    /// let config = Config::from_yaml("
    /// mode: play
    /// table: { stakes: { small_blind: 1.0, big_blind: 2.0 } }
    /// players: [ { seat: 0, stack: 200 }, { seat: 1, stack: 200, bot: abc } ]
    /// ").unwrap();
    /// assert_eq!(1, Game::new(check(&config).unwrap()).unwrap().hand_number());
    /// ```
    #[must_use]
    pub fn hand_number(&self) -> u32 {
        self.hand_number
    }

    /// True on the hand's last moment once the hand is over: nothing is
    /// left to act, deal or pay. Play mode can then deal the next hand.
    ///
    /// # Examples
    ///
    /// ```
    /// use bowery::check::check;
    /// use bowery::config::Config;
    /// use bowery::game::{Advance, Game};
    /// use pkcore::casino::action::PlayerAction;
    ///
    /// let config = Config::from_yaml("
    /// mode: play
    /// table: { button: 0, stakes: { small_blind: 1.0, big_blind: 2.0 } }
    /// players: [ { seat: 0, stack: 200 }, { seat: 1, stack: 200, bot: abc } ]
    /// ").unwrap();
    /// let mut game = Game::new(check(&config).unwrap()).unwrap();
    /// assert!(!game.is_hand_over());
    /// game.act(PlayerAction::Fold).unwrap();
    /// while game.advance().unwrap() != Advance::Ended {}
    /// assert!(game.is_hand_over());
    /// ```
    #[must_use]
    pub fn is_hand_over(&self) -> bool {
        let last = self.at + 1 == self.line.len();
        let settled = self
            .live
            .as_ref()
            .is_none_or(|live| live.settled && live.pending.is_empty());
        last && settled && self.event().ends_the_hand()
    }

    /// Where the game stands against the record.
    ///
    /// # Examples
    ///
    /// ```
    /// use bowery::check::check;
    /// use bowery::config::Config;
    /// use bowery::game::{Game, RecordState};
    ///
    /// let config = Config::from_yaml("
    /// mode: play
    /// table: { stakes: { small_blind: 1.0, big_blind: 2.0 } }
    /// players: [ { seat: 0, stack: 200 }, { seat: 1, stack: 200, bot: abc } ]
    /// ").unwrap();
    /// let game = Game::new(check(&config).unwrap()).unwrap();
    /// assert_eq!(RecordState::NotAReplay, game.record_state());
    /// ```
    #[must_use]
    pub fn record_state(&self) -> RecordState {
        match (
            self.config().mode,
            self.live.as_ref().and_then(|live| live.since),
        ) {
            (Mode::Play, _) => RecordState::NotAReplay,
            (_, Some(since)) => RecordState::OffRecord { since },
            (_, None) => RecordState::OnRecord,
        }
    }

    /// Moves to moment `index`, forwards or back. Nothing is played: this
    /// only changes which moment is on screen.
    ///
    /// # Errors
    ///
    /// [`GameError::OutOfRange`] when `index >= moments()`; the game does not
    /// move.
    ///
    /// # Examples
    ///
    /// ```
    /// use bowery::check::check;
    /// use bowery::config::Config;
    /// use bowery::game::Game;
    ///
    /// let config = Config::from_yaml("
    /// mode: play
    /// table: { stakes: { small_blind: 1.0, big_blind: 2.0 } }
    /// players: [ { seat: 0, stack: 200 }, { seat: 1, stack: 200, bot: abc } ]
    /// ").unwrap();
    /// let mut game = Game::new(check(&config).unwrap()).unwrap();
    /// assert!(game.seek(0).is_ok());
    /// assert!(game.seek(1).is_err());
    /// ```
    pub fn seek(&mut self, index: usize) -> Result<(), GameError> {
        if index >= self.line.len() {
            return Err(GameError::OutOfRange {
                index,
                moments: self.line.len(),
            });
        }
        self.at = index;
        Ok(())
    }

    /// The seat whose action comes next, or `None` when a deal, the end of
    /// the hand, or nothing comes next.
    ///
    /// # Examples
    ///
    /// ```
    /// use bowery::check::check;
    /// use bowery::config::Config;
    /// use bowery::game::Game;
    ///
    /// let config = Config::from_yaml("
    /// mode: play
    /// table: { button: 0, stakes: { small_blind: 1.0, big_blind: 2.0 } }
    /// players: [ { seat: 0, stack: 200 }, { seat: 1, stack: 200, bot: abc } ]
    /// ").unwrap();
    /// let game = Game::new(check(&config).unwrap()).unwrap();
    /// assert_eq!(Some(0), game.to_act()); // heads-up, the button acts first
    /// ```
    #[must_use]
    pub fn to_act(&self) -> Option<u8> {
        match self.line.get(self.at + 1) {
            Some(Step {
                event: Event::Action { seat, .. },
                ..
            }) => Some(*seat),
            Some(_) => None,
            None => self.live.as_ref().and_then(Live::turn),
        }
    }

    /// The reader's legal actions, when it is their turn and they can act
    /// now; empty otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use bowery::check::check;
    /// use bowery::config::Config;
    /// use bowery::game::Game;
    /// use pkcore::casino::action::PlayerAction;
    ///
    /// let config = Config::from_yaml("
    /// mode: play
    /// table: { button: 0, stakes: { small_blind: 1.0, big_blind: 2.0 } }
    /// players: [ { seat: 0, stack: 200 }, { seat: 1, stack: 200, bot: abc } ]
    /// ").unwrap();
    /// let game = Game::new(check(&config).unwrap()).unwrap();
    /// assert!(game.legal().contains(&PlayerAction::Fold));
    /// ```
    #[must_use]
    pub fn legal(&self) -> Vec<PlayerAction> {
        match (self.reader, self.acting_table()) {
            (Some(reader), Some(table)) => table.legal_actions(reader),
            _ => Vec::new(),
        }
    }

    /// The reader takes `seat` at the moment on screen. While their actions
    /// match the record, the hand stays on it.
    ///
    /// # Errors
    ///
    /// - [`GameError::NotInteractive`] outside interactive mode.
    /// - [`GameError::HandIsOver`] at the hand's return, showdown or award.
    /// - [`GameError::SeatNotLive`] when nobody sits there or they folded.
    ///
    /// # Examples
    ///
    /// ```
    /// use bowery::check::check;
    /// use bowery::config::Config;
    /// use bowery::game::Game;
    ///
    /// let config = Config::from_yaml("
    /// mode: interactive
    /// hand:
    ///   hand: { id: ex, game: holdem }
    ///   table: { seats: 2, button: 0, stakes: { small_blind: 1.0, big_blind: 2.0 } }
    ///   players: [ { seat: 0, name: A, stack: 100.0 }, { seat: 1, name: B, stack: 100.0 } ]
    ///   streets: { preflop: { actions: [ { seat: 0, action: fold } ] } }
    /// ").unwrap();
    /// let mut game = Game::new(check(&config).unwrap()).unwrap();
    /// game.take_seat(0).unwrap();
    /// assert_eq!(Some(0), game.reader());
    /// ```
    pub fn take_seat(&mut self, seat: u8) -> Result<(), GameError> {
        if self.config().mode != Mode::Interactive {
            return Err(GameError::NotInteractive);
        }
        if self.event().ends_the_hand() {
            return Err(GameError::HandIsOver);
        }
        let live = self
            .table()
            .seats
            .get_seat(seat)
            .is_some_and(|chair| !chair.is_empty() && chair.player.is_active());
        if !live {
            return Err(GameError::SeatNotLive { seat });
        }
        self.reader = Some(seat);
        Ok(())
    }

    /// Off the record, returns to the record at the takeover moment; the
    /// moments played off it are dropped. Does nothing on the record or in
    /// play.
    ///
    /// # Examples
    ///
    /// ```
    /// use bowery::check::check;
    /// use bowery::config::Config;
    /// use bowery::game::{Game, RecordState};
    /// use pkcore::casino::action::PlayerAction;
    ///
    /// let config = Config::from_yaml("
    /// mode: interactive
    /// hand:
    ///   hand: { id: ex, game: holdem }
    ///   table: { seats: 2, button: 0, stakes: { small_blind: 1.0, big_blind: 2.0 } }
    ///   players: [ { seat: 0, name: A, stack: 100.0 }, { seat: 1, name: B, stack: 100.0 } ]
    ///   streets: { preflop: { actions: [ { seat: 0, action: fold } ] } }
    /// ").unwrap();
    /// let mut game = Game::new(check(&config).unwrap()).unwrap();
    /// game.take_seat(0).unwrap();
    /// game.act(PlayerAction::Call).unwrap(); // the record folds
    /// assert_eq!(RecordState::OffRecord { since: 0 }, game.record_state());
    /// game.back_to_record();
    /// assert_eq!(RecordState::OnRecord, game.record_state());
    /// ```
    pub fn back_to_record(&mut self) {
        let Some(since) = self.live.as_ref().and_then(|live| live.since) else {
            return;
        };
        self.live = None;
        self.line = self.record.clone();
        self.at = since;
    }

    /// The reader acts. On the record, an action that matches the record
    /// moves one moment on and stays on it; any other takes the hand off the
    /// record.
    ///
    /// # Errors
    ///
    /// - [`GameError::NotInteractive`] in replay mode.
    /// - [`GameError::NoReader`] when the reader has no seat.
    /// - [`GameError::NotYourTurn`] when the next action is not the reader's,
    ///   or, off the record or in play, when the moment on screen is not the
    ///   latest.
    /// - [`GameError::Illegal`] when the engine refuses the action.
    ///
    /// # Examples
    ///
    /// ```
    /// use bowery::check::check;
    /// use bowery::config::Config;
    /// use bowery::game::Game;
    /// use pkcore::casino::action::PlayerAction;
    ///
    /// let config = Config::from_yaml("
    /// mode: play
    /// table: { button: 0, stakes: { small_blind: 1.0, big_blind: 2.0 } }
    /// players: [ { seat: 0, stack: 200 }, { seat: 1, stack: 200, bot: abc } ]
    /// ").unwrap();
    /// let mut game = Game::new(check(&config).unwrap()).unwrap();
    /// game.act(PlayerAction::Fold).unwrap();
    /// assert_eq!(2, game.moments());
    /// ```
    pub fn act(&mut self, action: PlayerAction) -> Result<(), GameError> {
        if self.config().mode == Mode::Replay {
            return Err(GameError::NotInteractive);
        }
        let reader = self.reader.ok_or(GameError::NoReader)?;
        if self.to_act() != Some(reader) || self.acting_table().is_none() {
            return Err(GameError::NotYourTurn);
        }
        if self.live.is_none() {
            return self.act_on_record(reader, action);
        }
        let live = self.live.as_mut().ok_or(GameError::NotYourTurn)?;
        live.session
            .apply_action(reader, action)
            .map_err(|reason| GameError::Illegal { action, reason })?;
        let step = Step {
            event: Event::Action {
                seat: reader,
                action,
                street: live.street,
            },
            table: live.session.table.clone(),
        };
        self.push(step);
        Ok(())
    }

    /// Moves one moment on: a step through the record, one bot action, one
    /// deal, or one of the hand's last moments. Never a whole run-out.
    ///
    /// # Errors
    ///
    /// [`GameError::Engine`] if the engine fails, or [`GameError::Setup`] if
    /// a hand off the record cannot be rebuilt at its end.
    ///
    /// # Examples
    ///
    /// ```
    /// use bowery::check::check;
    /// use bowery::config::Config;
    /// use bowery::game::{Advance, Game};
    ///
    /// let config = Config::from_yaml("
    /// mode: play
    /// seat: 1
    /// table: { button: 0, stakes: { small_blind: 1.0, big_blind: 2.0 } }
    /// players: [ { seat: 0, stack: 200, bot: abc }, { seat: 1, stack: 200 } ]
    /// ").unwrap();
    /// let mut game = Game::new(check(&config).unwrap()).unwrap();
    /// assert_eq!(Advance::Moved, game.advance().unwrap()); // the bot acts first
    /// ```
    pub fn advance(&mut self) -> Result<Advance, GameError> {
        if self.at + 1 < self.line.len() {
            if self.live.is_none()
                && let Some(reader) = self.reader
                && self.to_act() == Some(reader)
            {
                return Ok(Advance::Waiting);
            }
            self.at += 1;
            return Ok(Advance::Moved);
        }
        let Some(live) = self.live.as_mut() else {
            return Ok(Advance::Ended);
        };
        if let Some(step) = live.pending.pop_front() {
            self.push(step);
            return Ok(Advance::Moved);
        }
        if live.settled {
            return Ok(Advance::Ended);
        }
        match live.session.next_step() {
            SessionStep::PlayerToAct(seat) if Some(seat) == self.reader => Ok(Advance::Waiting),
            SessionStep::PlayerToAct(seat) => {
                let profile = self
                    .bots
                    .get(&seat)
                    .cloned()
                    .unwrap_or_else(BotProfile::gto);
                let action = bot_act(&mut live.session, &profile, seat, &mut self.rng)?;
                let step = Step {
                    event: Event::Action {
                        seat,
                        action,
                        street: live.street,
                    },
                    table: live.session.table.clone(),
                };
                self.push(step);
                Ok(Advance::Moved)
            }
            SessionStep::StreetAdvanced => {
                live.street = next_street(live.street);
                let step = Step {
                    event: Event::Deal(live.street),
                    table: live.session.table.clone(),
                };
                self.push(step);
                Ok(Advance::Moved)
            }
            SessionStep::HandComplete => {
                let ending = settle(live, self.line.len())?;
                live.pending = ending.into();
                live.settled = true;
                match live.pending.pop_front() {
                    Some(step) => {
                        self.push(step);
                        Ok(Advance::Moved)
                    }
                    None => Ok(Advance::Ended),
                }
            }
            SessionStep::Failed(error) => Err(GameError::Engine(error)),
        }
    }

    /// Play mode: pays the hand, moves the button to the next seated player,
    /// and deals the next hand from a seeded shuffle. Stacks carry over;
    /// players with no chips leave.
    ///
    /// # Errors
    ///
    /// - [`GameError::NotPlay`] outside play mode.
    /// - [`GameError::HandNotOver`] before the hand's last moment is on
    ///   screen.
    /// - [`GameError::TableBroken`] when fewer than two players have chips.
    /// - [`GameError::Engine`] if the engine fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use bowery::check::check;
    /// use bowery::config::Config;
    /// use bowery::game::{Advance, Game};
    /// use pkcore::casino::action::PlayerAction;
    ///
    /// let config = Config::from_yaml("
    /// mode: play
    /// table: { button: 0, stakes: { small_blind: 1.0, big_blind: 2.0 } }
    /// players: [ { seat: 0, stack: 200 }, { seat: 1, stack: 200, bot: abc } ]
    /// ").unwrap();
    /// let mut game = Game::new(check(&config).unwrap()).unwrap();
    /// game.act(PlayerAction::Fold).unwrap();
    /// while game.advance().unwrap() != Advance::Ended {}
    /// game.next_hand().unwrap();
    /// assert_eq!(2, game.hand_number());
    /// ```
    pub fn next_hand(&mut self) -> Result<(), GameError> {
        if self.config().mode != Mode::Play {
            return Err(GameError::NotPlay);
        }
        let over = self
            .live
            .as_ref()
            .is_some_and(|live| live.settled && live.pending.is_empty());
        if !over || self.at + 1 != self.line.len() {
            return Err(GameError::HandNotOver);
        }
        let live = self.live.as_mut().ok_or(GameError::HandNotOver)?;
        live.session.end_hand()?;
        live.session.eliminate_busted();
        if live.session.count_funded() < 2 {
            return Err(GameError::TableBroken);
        }
        move_button(&mut live.session.table);
        self.hand_number += 1;
        let session = &mut live.session;
        let (base, log_from) = deal(
            session,
            self.hand_number,
            self.checked.config(),
            None,
            &mut self.rng,
        )?;
        live.log_from = log_from;
        live.base = base;
        live.street = ReplayStreet::Preflop;
        live.settled = false;
        live.pending.clear();
        self.line = vec![Step {
            event: Event::Forced,
            table: live.session.table.clone(),
        }];
        self.at = 0;
        Ok(())
    }

    fn step(&self) -> &Step {
        // `line` always holds at least the forced bets, and `at` is in range.
        &self.line[self.at.min(self.line.len().saturating_sub(1))]
    }

    /// The table the reader would act on now, if they can act now: the
    /// moment on screen on the record, the engine's table at the latest
    /// moment off it.
    fn acting_table(&self) -> Option<&Table> {
        let reader = self.reader?;
        if self.to_act() != Some(reader) {
            return None;
        }
        match &self.live {
            None => Some(self.table()),
            Some(live) if self.at + 1 == self.line.len() => Some(&live.session.table),
            Some(_) => None,
        }
    }

    /// Appends a moment and moves to it.
    fn push(&mut self, step: Step) {
        self.line.push(step);
        self.at = self.line.len() - 1;
    }

    fn act_on_record(&mut self, reader: u8, action: PlayerAction) -> Result<(), GameError> {
        let recorded = match self.line.get(self.at + 1) {
            Some(Step {
                event: Event::Action { action, .. },
                ..
            }) => *action,
            _ => return Err(GameError::NotYourTurn),
        };
        if action == recorded {
            self.at += 1;
            return Ok(());
        }
        let since = self.at;
        let hand = self
            .config()
            .hand
            .as_ref()
            .ok_or_else(|| GameError::Setup("an interactive game has no hand".to_string()))?;
        let mut base = hand.clone();
        base.results = None;
        let street = street_at(&self.line, since);
        let table = self.table().clone();
        let mut session = PokerSession::new(table);
        session
            .apply_action(reader, action)
            .map_err(|reason| GameError::Illegal { action, reason })?;
        let step = Step {
            event: Event::Action {
                seat: reader,
                action,
                street,
            },
            table: session.table.clone(),
        };
        self.live = Some(Live {
            session,
            log_from: 0,
            street,
            base,
            since: Some(since),
            pending: VecDeque::new(),
            settled: false,
        });
        self.line.truncate(since + 1);
        self.push(step);
        Ok(())
    }

    fn open_record(&mut self) -> Result<(), GameError> {
        let config = self.checked.config();
        let hand = config
            .hand
            .as_ref()
            .ok_or_else(|| GameError::Setup(format!("{} needs a recorded hand", config.mode)))?;
        let mut replay = hand.cursor()?;
        let start = config
            .start
            .and_then(|moment| moment.index_in(&replay))
            .unwrap_or(0);
        let events: Vec<ReplayEvent> = (0..replay.len())
            .filter_map(|index| replay.event(index).cloned())
            .collect();
        let mut record = Vec::with_capacity(events.len());
        for (index, replay_event) in events.iter().enumerate() {
            if index > 0 {
                replay.step()?;
            }
            let event = Event::from_replay(replay_event).ok_or_else(|| {
                GameError::Setup(format!("unknown replay event `{replay_event}`"))
            })?;
            record.push(Step {
                event,
                table: replay.table().clone(),
            });
        }
        self.line.clone_from(&record);
        self.record = record;
        self.at = start;
        Ok(())
    }

    fn open_play(&mut self) -> Result<(), GameError> {
        let config = self.checked.config();
        self.reader = config.seat.or_else(|| {
            config
                .players
                .iter()
                .find(|player| player.bot.is_none())
                .map(|player| player.seat)
        });
        let mut session = PokerSession::new(play_table(config)?);
        let deck = config.deck.as_deref().map(Cards::from_str).transpose()?;
        let (base, log_from) = deal(&mut session, self.hand_number, config, deck, &mut self.rng)?;
        self.line = vec![Step {
            event: Event::Forced,
            table: session.table.clone(),
        }];
        self.live = Some(Live {
            session,
            log_from,
            street: ReplayStreet::Preflop,
            base,
            since: None,
            pending: VecDeque::new(),
            settled: false,
        });
        Ok(())
    }
}

/// The seed when the config gives none: FNV-1a over the config's YAML, so a
/// page plays the same way every time it loads.
fn default_seed(config: &Config) -> u64 {
    let yaml = config.to_yaml().unwrap_or_default();
    yaml.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// The street after `street`.
fn next_street(street: ReplayStreet) -> ReplayStreet {
    match street {
        ReplayStreet::Preflop => ReplayStreet::Flop,
        ReplayStreet::Flop => ReplayStreet::Turn,
        ReplayStreet::Turn | ReplayStreet::River => ReplayStreet::River,
    }
}

/// A bot acts for `seat`. If the engine refuses its choice, it checks, else
/// calls, else folds.
fn bot_act(
    session: &mut PokerSession,
    profile: &BotProfile,
    seat: u8,
    rng: &mut SmallRng,
) -> Result<PlayerAction, GameError> {
    let decided = profile.decide(&session.table, seat, rng);
    let mut last = PKError::InvalidAction;
    for action in [
        decided,
        PlayerAction::Check,
        PlayerAction::Call,
        PlayerAction::Fold,
    ] {
        match session.apply_action(seat, action) {
            Ok(()) => return Ok(action),
            Err(error) => last = error,
        }
    }
    Err(GameError::Engine(last))
}

/// The play table: every chair up to `table.seats`, the players in theirs,
/// the stakes and the button.
fn play_table(config: &Config) -> Result<Table, GameError> {
    let setup = config.table.clone().unwrap_or_default();
    let stakes = setup
        .stakes
        .as_ref()
        .ok_or_else(|| GameError::Setup("play needs the blinds".to_string()))?;
    let whole = |value: f64| {
        chips(value).ok_or_else(|| GameError::Setup(format!("{value} is not whole chips")))
    };
    let forced = ForcedBets::new_with_ante(
        whole(stakes.small_blind)?,
        whole(stakes.big_blind)?,
        whole(stakes.ante.unwrap_or(0.0))?,
    );
    let highest = config
        .players
        .iter()
        .map(|player| player.seat)
        .max()
        .unwrap_or(0);
    let chairs = setup.seats.unwrap_or(highest.saturating_add(1)).max(2);
    let mut seats: Vec<Seat> = (0..chairs).map(|_| Seat::new(Player::default())).collect();
    for player in &config.players {
        let stack = usize::try_from(player.stack.unwrap_or(0))
            .map_err(|_| GameError::Setup(format!("seat {}'s stack is too large", player.seat)))?;
        let name = player
            .name
            .clone()
            .or_else(|| player.bot.clone())
            .unwrap_or_else(|| format!("Seat {}", player.seat));
        let chair = seats
            .get_mut(usize::from(player.seat))
            .ok_or_else(|| GameError::Setup(format!("seat {} is not at the table", player.seat)))?;
        *chair = Seat::new(Player::with_id(
            Uuid::from_u128(u128::from(player.seat) + 1),
            name,
            stack,
        ));
    }
    let mut table =
        Table::nlh_from_seats_with_id(Seats::new(seats), forced, Uuid::from_u128(0xB0E5));
    table.button = setup.button.unwrap_or(0);
    Ok(table)
}

/// Deals a play hand: from `deck` if given, else from a shuffle on `rng`.
/// Returns the hand history the end of the hand is rebuilt from (players,
/// table, stakes; no streets) and where the hand starts in the event log.
fn deal(
    session: &mut PokerSession,
    hand_number: u32,
    config: &Config,
    deck: Option<Cards>,
    rng: &mut SmallRng,
) -> Result<(HandHistory, usize), GameError> {
    let starting: Vec<(u8, String, usize)> = seated(&session.table)
        .map(|(seat, chair)| (seat, chair.player.handle.clone(), chair.player.chips))
        .collect();
    let log_from = session.table.event_log.len();
    match deck {
        Some(deck) => session.start_hand_with_deck(deck)?,
        None => session.start_hand_with(rng)?,
    }
    let table = &session.table;
    let players = starting
        .into_iter()
        .map(|(seat, name, stack)| PlayerEntry {
            seat,
            name,
            stack: amount(stack),
            player_id: None,
            hole_cards: table
                .seats
                .get_seat(seat)
                .map(|chair| chair.cards.to_string()),
            posted: None,
            hole_cards_visibility: None,
            withdrawn: None,
        })
        .collect();
    let stakes = config
        .table
        .as_ref()
        .and_then(|setup| setup.stakes.clone())
        .unwrap_or(Stakes {
            small_blind: amount(table.forced.small_blind),
            big_blind: amount(table.forced.big_blind),
            ante: None,
            straddle: None,
            bring_in: None,
        });
    let base = HandHistory {
        pkcore_version: None,
        format_version: 1,
        hand: HandMeta {
            id: format!("bowery-play-{hand_number}"),
            game: HandVariant::Holdem,
            timestamp: None,
            source: Some("bowery play".to_string()),
            description: None,
        },
        table: TableInfo {
            name: None,
            seats: Some(table.seats.size()),
            button: Some(table.button),
            stakes,
            betting_structure: table.betting,
        },
        players,
        board: None,
        streets: None,
        results: None,
        analysis: None,
        shuffled_deck: None,
    };
    Ok((base, log_from))
}

/// The occupied seats of `table`, with their numbers.
fn seated(table: &Table) -> impl Iterator<Item = (u8, &Seat)> {
    (0..=u8::MAX)
        .map_while(|seat| table.seats.get_seat(seat).map(|chair| (seat, chair)))
        .filter(|(_, chair)| !chair.is_empty())
}

/// Chips as a hand-history amount.
#[allow(
    clippy::cast_precision_loss,
    reason = "chip counts are far below 2^52, where f64 stops being exact"
)]
fn amount(chips: usize) -> f64 {
    chips as f64
}

/// Moves the button to the next occupied chair.
fn move_button(table: &mut Table) {
    for _ in 0..table.seats.size() {
        table.button_up();
        if table
            .seats
            .get_seat(table.button)
            .is_some_and(|chair| !chair.is_empty())
        {
            return;
        }
    }
}

/// The street being bet at moment `index` of `line`: the last street dealt
/// by then, or the preflop.
fn street_at(line: &[Step], index: usize) -> ReplayStreet {
    line.iter()
        .take(index + 1)
        .rev()
        .find_map(|step| match step.event {
            Event::Deal(street) => Some(street),
            _ => None,
        })
        .unwrap_or(ReplayStreet::Preflop)
}

/// The hand's last moments: the hand is rebuilt as a hand history (the base,
/// with the streets the table logged since `log_from`) and pkcore's cursor
/// names the return, showdown and awards. `played` is the number of moments
/// so far, which the rebuilt hand must have before its last ones.
///
/// Off the record the whole log is read: since pkcore 0.17.3 a replayed
/// table logs the streets the record laid (`DEFECT_041`), so the log holds
/// the record up to the takeover and the engine's play after it.
fn settle(live: &Live, played: usize) -> Result<Vec<Step>, GameError> {
    let log = live
        .session
        .table
        .event_log
        .get(live.log_from..)
        .unwrap_or_default();
    let mut streets = Streets::from_event_log(log).unwrap_or_default();
    // The log holds a flop as a set, which loses the deal order; the table's
    // board keeps it, so the rebuilt flop reads the same as the moments before.
    if let Some(flop) = streets.flop.as_mut() {
        let dealt: Vec<String> = live
            .session
            .table
            .board
            .iter()
            .take(3)
            .map(ToString::to_string)
            .collect();
        flop.cards = dealt.join(" ");
    }
    let mut hand = live.base.clone();
    hand.streets = Some(streets);
    // The streets hold every dealt card; the record's `board` may be shorter
    // than the hand played off it.
    hand.board = None;
    let findings = hand.validate();
    if !findings.is_empty() {
        let found: Vec<String> = findings.iter().map(ToString::to_string).collect();
        return Err(GameError::Setup(format!(
            "the hand off the record does not replay: {}",
            found.join("; ")
        )));
    }
    let mut replay = hand.cursor()?;
    let events: Vec<ReplayEvent> = (0..replay.len())
        .filter_map(|index| replay.event(index).cloned())
        .collect();
    let first_last = events.len()
        - events
            .iter()
            .rev()
            .take_while(|event| {
                Event::from_replay(event).is_some_and(|event| event.ends_the_hand())
            })
            .count();
    if first_last != played {
        return Err(GameError::Setup(format!(
            "the rebuilt hand has {first_last} moments before its end; the game played {played}"
        )));
    }
    let mut ending = Vec::with_capacity(events.len() - first_last);
    for (index, replay_event) in events.iter().enumerate().skip(1) {
        replay.step()?;
        if index >= first_last {
            let event = Event::from_replay(replay_event).ok_or_else(|| {
                GameError::Setup(format!("unknown replay event `{replay_event}`"))
            })?;
            ending.push(Step {
                event,
                table: replay.table().clone(),
            });
        }
    }
    Ok(ending)
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use super::*;

    #[test]
    fn event__display() {
        assert_eq!("forced bets", Event::Forced.to_string());
        assert_eq!(
            "flop: seat 4 Bet(8000)",
            Event::Action {
                seat: 4,
                action: PlayerAction::Bet(8_000),
                street: ReplayStreet::Flop
            }
            .to_string()
        );
        assert_eq!(
            "seat 0 takes back 800",
            Event::Return {
                seat: 0,
                amount: 800
            }
            .to_string()
        );
        assert_eq!(
            "seat 1 wins 400",
            Event::Award {
                seat: 1,
                amount: 400
            }
            .to_string()
        );
    }

    #[test]
    fn event__ends_the_hand() {
        assert!(Event::Showdown.ends_the_hand());
        assert!(Event::Return { seat: 0, amount: 1 }.ends_the_hand());
        assert!(!Event::Deal(ReplayStreet::River).ends_the_hand());
    }

    #[test]
    fn next_street__stops_at_the_river() {
        assert_eq!(ReplayStreet::Flop, next_street(ReplayStreet::Preflop));
        assert_eq!(ReplayStreet::River, next_street(ReplayStreet::River));
    }

    #[test]
    fn game_error__display() {
        assert_eq!(
            "moment 9 of 4 does not exist",
            GameError::OutOfRange {
                index: 9,
                moments: 4
            }
            .to_string()
        );
    }

    #[test]
    fn default_seed__is_stable() {
        let config = Config::from_yaml("mode: play\n").unwrap();

        assert_eq!(default_seed(&config), default_seed(&config.clone()));
        assert_ne!(
            default_seed(&config),
            default_seed(&Config::from_yaml("mode: replay\n").unwrap())
        );
    }

    #[test]
    fn seed__hashes_the_config_unless_it_gives_one() {
        let play = |extra: &str| {
            let config = Config::from_yaml(&format!(
                "mode: play\n{extra}table: {{ stakes: {{ small_blind: 1.0, big_blind: 2.0 }} }}\nplayers: [ {{ seat: 0, stack: 200 }}, {{ seat: 1, stack: 200, bot: abc }} ]\n"
            ))
            .unwrap();
            Game::new(crate::check::check(&config).unwrap()).unwrap()
        };

        assert_eq!(
            play("").seed(),
            play("").seed(),
            "the same config, the same seed"
        );
        assert_eq!(7, play("seed: 7\n").seed());
        assert_ne!(play("").seed(), play("seed: 7\n").seed());
    }
}
