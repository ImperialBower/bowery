//! What `<bowery-table>` asks of a table, in plain Rust.
//!
//! [`Table`] is one element's table: a [`Game`], the layout and the view the
//! reader picked. Every call takes and returns strings and numbers, and the
//! state comes back as JSON, so [`crate::wasm`] only has to wrap it for
//! JavaScript. Behind the `wasm` feature; tested natively.
//!
//! ```
//! use bowery::element::Table;
//!
//! let mut table = Table::new("
//! mode: play
//! seat: 0
//! table: { button: 0, stakes: { small_blind: 1.0, big_blind: 2.0 } }
//! players: [ { seat: 0, stack: 200 }, { seat: 1, stack: 200, bot: abc } ]
//! ").unwrap();
//! assert_eq!("waiting", table.advance().unwrap());
//! table.act(r#"{"action":"fold"}"#).unwrap();
//! assert!(table.svg(680, false).starts_with("<svg"));
//! ```

use std::cell::RefCell;
use std::collections::HashMap;

use pkcore::casino::action::PlayerAction;
use serde::{Deserialize, Serialize};

use crate::check::{Problem, check};
use crate::config::{Config, Layout, Mode, View};
use crate::draw::{Theme, svg};
use crate::frame::{Award, Frame, SeatStatus};
use crate::game::{Advance, Event, Game, RecordState};
use crate::odds::{self, Odds};

/// Bot pacing when the config gives no `delay_ms`.
const DEFAULT_DELAY_MS: u32 = 700;

/// One element's table.
///
/// The win percents (see [`crate::odds`]) are on by default in replay and
/// interactive and off in play, where bots run on a timer and every moment
/// would cost a Monte Carlo run. They show only in
/// the `all` view, where every live card is face up. The element computes them
/// once per moment and keeps them in a cache, so seeking back and forth, or
/// switching the layout or the view, costs nothing the second time. Moments
/// are only ever added at the end, so a step forward or taking a seat leaves
/// the cache alone. An action keeps the moments before it and drops it and
/// what followed (off the record it cuts the line short); the next hand, a
/// shuffle and back to the record empty the cache.
///
/// # Examples
///
/// ```
/// use bowery::element::Table;
///
/// assert!(Table::new("mode: replay\n").is_err()); // replay needs a hand
/// ```
#[derive(Debug)]
pub struct Table {
    game: Game,
    layout: Layout,
    view: View,
    /// The reader's `%` switch; the odds show only when the view is `all`.
    /// Starts on in replay and interactive, off in play (see [`Mode`]).
    odds: bool,
    /// The odds at each moment index already asked for; `None` is a moment
    /// that has none (a face-down card, fewer than two live seats, paid out).
    /// A moment never changes once made, so an entry stays right until the
    /// line is rebuilt under it: [`Table::act`] drops the entries from the
    /// new moment on, and the next hand, a shuffle and back to the record
    /// drop them all.
    cache: RefCell<HashMap<usize, Option<Odds>>>,
}

/// What the controls bar reads after every move.
///
/// # Examples
///
/// ```
/// use bowery::element::Table;
///
/// let table = Table::new("
/// mode: play
/// table: { stakes: { small_blind: 1.0, big_blind: 2.0 } }
/// players: [ { seat: 0, stack: 200 }, { seat: 1, stack: 200, bot: abc } ]
/// ").unwrap();
/// let state = table.state();
/// assert_eq!(1, state.moment);
/// assert!(state.controls.shuffle);
/// assert!(!state.odds); // play opens in the seat view, which hides them
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct State {
    /// Replay, interactive or play.
    pub mode: Mode,
    /// The view the table is drawn in.
    pub view: View,
    /// The layout asked for (ring may draw as ledger when narrow).
    pub layout: Layout,
    /// The moment on screen, from 1.
    pub moment: usize,
    /// How many moments there are.
    pub moments: usize,
    /// The seat whose action comes next.
    pub to_act: Option<u8>,
    /// The reader's seat.
    pub reader: Option<u8>,
    /// The reader's legal actions, when they can act now.
    pub legal: Vec<Legal>,
    /// `on`, `off` (the record) or `none` (play).
    pub record: &'static str,
    /// Off the record: the takeover moment, from 0.
    pub since: Option<usize>,
    /// The pot shares won so far.
    pub result: Option<Vec<Award>>,
    /// The hand is over and this is its last moment.
    pub hand_over: bool,
    /// The hand being played, from 1.
    pub hand_number: u32,
    /// The frame's caption.
    pub caption: String,
    /// Every occupied seat, for **Take this seat**.
    pub seats: Vec<SeatChoice>,
    /// Which controls to show.
    pub controls: Controls,
    /// The win percents are on screen now: the switch is on and the view is
    /// `all`. The switch starts on in replay and interactive and off in play,
    /// where bots run on a timer; [`Table::set_odds`] turns it on.
    pub odds: bool,
    /// Milliseconds between bot moves.
    pub delay_ms: u32,
}

/// One legal action, with its size where it has one.
///
/// # Examples
///
/// ```
/// use bowery::element::Legal;
///
/// let fold = Legal { action: "fold", amount: None, min: None, max: None };
/// assert_eq!(r#"{"action":"fold"}"#, serde_json::to_string(&fold).unwrap());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Legal {
    /// `fold`, `check`, `call`, `bet`, `raise` or `all_in`.
    pub action: &'static str,
    /// Chips a call adds, or an all in's street total.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount: Option<usize>,
    /// The smallest bet or raise, as a street total.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<usize>,
    /// The largest bet or raise: the seat's chips and its bet.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<usize>,
}

/// A seat the reader could take.
///
/// # Examples
///
/// ```
/// use bowery::element::SeatChoice;
///
/// let seat = SeatChoice { seat: 4, name: "Daniel Negreanu".to_string(), live: true };
/// assert!(seat.live);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SeatChoice {
    /// The seat, from 0.
    pub seat: u8,
    /// The player's name.
    pub name: String,
    /// Still in the hand at this moment, so it can be taken.
    pub live: bool,
}

/// Which controls the bar shows.
///
/// # Examples
///
/// ```
/// use bowery::element::Controls;
///
/// assert!(!Controls::default().seek);
/// ```
#[allow(
    clippy::struct_excessive_bools,
    reason = "one flag per control, read by the element as JSON"
)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Controls {
    /// ⏮ ◀ ▶ ⏭ and the i/n readout: replay and interactive.
    pub seek: bool,
    /// **Take this seat**: interactive.
    pub take_seat: bool,
    /// **Back to the record**: interactive, off the record.
    pub back_to_record: bool,
    /// **Next hand**: play, once the hand is over.
    pub next_hand: bool,
    /// **Shuffle**: play.
    pub shuffle: bool,
    /// The 👁 view toggle: the config's `controls.view`, else on outside play.
    pub view: bool,
    /// The `%` odds toggle: on in the `all` view unless the config's
    /// `controls.odds` is `false`. The config drops the button only: the odds
    /// themselves still show ([`State::odds`]), and the reader can still turn
    /// them off with [`Table::set_odds`] when the button is not there. The
    /// switch itself starts on in replay and interactive, off in play.
    ///
    /// ```
    /// use bowery::element::Table;
    ///
    /// let mut table = Table::new("
    /// mode: play
    /// view: all
    /// controls: { odds: false }
    /// table: { stakes: { small_blind: 1.0, big_blind: 2.0 } }
    /// players: [ { seat: 0, stack: 200 }, { seat: 1, stack: 200, bot: abc } ]
    /// ").unwrap();
    /// table.set_odds(true); // play starts with the odds off
    /// assert!(!table.state().controls.odds); // no button
    /// assert!(table.state().odds); // the odds still show
    /// ```
    pub odds: bool,
    /// The ▦ layout toggle: the config's `controls.layout`, else on.
    pub layout: bool,
}

/// What the element sends to [`Table::act`].
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ActRequest {
    action: String,
    #[serde(default)]
    amount: Option<usize>,
}

impl Table {
    /// Reads, checks and builds the table a YAML config describes.
    ///
    /// # Errors
    ///
    /// Every [`Problem`] in the config, or one problem at the root if the
    /// YAML does not read or the engine cannot set the game up.
    ///
    /// # Examples
    ///
    /// ```
    /// use bowery::element::Table;
    ///
    /// let table = Table::new("
    /// mode: play
    /// table: { stakes: { small_blind: 1.0, big_blind: 2.0 } }
    /// players: [ { seat: 0, stack: 200 }, { seat: 1, stack: 200, bot: abc } ]
    /// ").unwrap();
    /// assert_eq!(1, table.state().hand_number);
    /// ```
    pub fn new(yaml: &str) -> Result<Self, Vec<Problem>> {
        let config = Config::from_yaml(yaml).map_err(|problem| vec![problem])?;
        Self::from_config(&config)
    }

    fn from_config(config: &Config) -> Result<Self, Vec<Problem>> {
        let game = Game::new(check(config)?).map_err(|error| {
            vec![Problem::Config {
                path: String::new(),
                message: error.to_string(),
            }]
        })?;
        Ok(Self {
            layout: config.layout,
            view: game.view(),
            odds: config.mode != Mode::Play,
            cache: RefCell::new(HashMap::new()),
            game,
        })
    }

    /// The table on screen, as SVG, `width` pixels wide.
    ///
    /// # Examples
    ///
    /// ```
    /// use bowery::element::Table;
    ///
    /// let table = Table::new("
    /// mode: play
    /// table: { stakes: { small_blind: 1.0, big_blind: 2.0 } }
    /// players: [ { seat: 0, stack: 200 }, { seat: 1, stack: 200, bot: abc } ]
    /// ").unwrap();
    /// assert!(table.svg(680, true).contains("#111111"));
    /// ```
    #[must_use]
    pub fn svg(&self, width: u32, dark: bool) -> String {
        let theme = if dark { Theme::Dark } else { Theme::Light };
        svg(&self.frame(), self.layout, width, theme)
    }

    /// Whether the win percents are on screen: the switch is on and the view
    /// is `all`, where every live card is face up.
    fn odds_on(&self) -> bool {
        self.odds && self.view == View::All
    }

    /// The frame on screen, with the win percents when [`Table::odds_on`].
    /// The percents of a moment are computed once and cached by its index.
    fn frame(&self) -> Frame {
        let mut frame = self.game.frame(self.view);
        if self.odds_on() {
            // The cache is only borrowed here, never across a call back into
            // `frame`, so the borrow cannot overlap.
            let mut cache = self.cache.borrow_mut();
            frame.odds = cache
                .entry(self.game.at())
                .or_insert_with(|| {
                    odds::compute(
                        self.game.table(),
                        &frame.seats,
                        &frame.board,
                        frame.result.is_some(),
                        self.game.seed(),
                    )
                })
                .clone();
        }
        frame
    }

    /// What the controls bar reads now.
    ///
    /// # Examples
    ///
    /// ```
    /// use bowery::element::Table;
    ///
    /// let table = Table::new("
    /// mode: play
    /// table: { stakes: { small_blind: 1.0, big_blind: 2.0 } }
    /// players: [ { seat: 0, stack: 200 }, { seat: 1, stack: 200, bot: abc } ]
    /// ").unwrap();
    /// assert_eq!("none", table.state().record);
    /// ```
    #[must_use]
    pub fn state(&self) -> State {
        let game = &self.game;
        let config = game.config();
        // Not `self.frame()`: the state needs no percents, and computing them
        // here would cost a Monte Carlo run nobody asked for.
        let frame = game.frame(self.view);
        let (record, since) = match game.record_state() {
            RecordState::OnRecord => ("on", None),
            RecordState::OffRecord { since } => ("off", Some(since)),
            RecordState::NotAReplay => ("none", None),
        };
        let hand_over = game.is_hand_over();
        let play = config.mode == Mode::Play;
        State {
            mode: config.mode,
            view: self.view,
            layout: self.layout,
            moment: frame.moment.0,
            moments: frame.moment.1,
            to_act: frame.to_act,
            reader: game.reader(),
            legal: self.legal(),
            record,
            since,
            result: frame.result,
            hand_over,
            hand_number: game.hand_number(),
            caption: frame.caption,
            seats: frame
                .seats
                .iter()
                .map(|seat| SeatChoice {
                    seat: seat.seat,
                    name: seat.name.clone(),
                    live: matches!(seat.status, SeatStatus::Live | SeatStatus::AllIn),
                })
                .collect(),
            controls: Controls {
                seek: !play,
                take_seat: config.mode == Mode::Interactive,
                back_to_record: since.is_some(),
                next_hand: play && hand_over,
                shuffle: play,
                view: config.controls.view.unwrap_or(!play),
                odds: config.controls.odds.unwrap_or(true) && self.view == View::All,
                layout: config.controls.layout.unwrap_or(true),
            },
            odds: self.odds_on(),
            delay_ms: config.delay_ms.unwrap_or(DEFAULT_DELAY_MS),
        }
    }

    /// [`Table::state`] as JSON.
    ///
    /// # Examples
    ///
    /// ```
    /// use bowery::element::Table;
    ///
    /// let table = Table::new("
    /// mode: play
    /// table: { stakes: { small_blind: 1.0, big_blind: 2.0 } }
    /// players: [ { seat: 0, stack: 200 }, { seat: 1, stack: 200, bot: abc } ]
    /// ").unwrap();
    /// assert!(table.state_json().contains(r#""mode":"play""#));
    /// ```
    #[must_use]
    pub fn state_json(&self) -> String {
        serde_json::to_string(&self.state()).unwrap_or_else(|_| "{}".to_string())
    }

    /// The reader's legal actions with their sizes.
    fn legal(&self) -> Vec<Legal> {
        let Some(reader) = self.game.reader() else {
            return Vec::new();
        };
        let table = self.game.table();
        let (chips, bet) = table
            .seats
            .get_seat(reader)
            .map_or((0, 0), |seat| (seat.player.chips, seat.player.bet));
        let max = Some(chips + bet);
        self.game
            .legal()
            .into_iter()
            .map(|action| match action {
                PlayerAction::Fold => Legal {
                    action: "fold",
                    amount: None,
                    min: None,
                    max: None,
                },
                PlayerAction::Check => Legal {
                    action: "check",
                    amount: None,
                    min: None,
                    max: None,
                },
                PlayerAction::Call => Legal {
                    action: "call",
                    amount: Some(table.to_call(reader)),
                    min: None,
                    max: None,
                },
                PlayerAction::Bet(min) => Legal {
                    action: "bet",
                    amount: None,
                    min: Some(min),
                    max,
                },
                PlayerAction::Raise(min) => Legal {
                    action: "raise",
                    amount: None,
                    min: Some(min),
                    max,
                },
                PlayerAction::AllIn => Legal {
                    action: "all_in",
                    amount: max,
                    min: None,
                    max: None,
                },
            })
            .collect()
    }

    /// Moves to moment `index`, from 0; past the end it moves to the last.
    ///
    /// # Examples
    ///
    /// ```
    /// use bowery::element::Table;
    ///
    /// let mut table = Table::new("
    /// mode: play
    /// table: { stakes: { small_blind: 1.0, big_blind: 2.0 } }
    /// players: [ { seat: 0, stack: 200 }, { seat: 1, stack: 200, bot: abc } ]
    /// ").unwrap();
    /// table.seek(9);
    /// assert_eq!(1, table.state().moment);
    /// ```
    pub fn seek(&mut self, index: usize) {
        let last = self.game.moments().saturating_sub(1);
        // In range by construction, so the seek cannot fail.
        let _ = self.game.seek(index.min(last));
    }

    /// The reader takes `seat` (interactive).
    ///
    /// # Errors
    ///
    /// The game's reason, as text: not interactive, the hand is over, or the
    /// seat is not in the hand.
    ///
    /// # Examples
    ///
    /// ```
    /// use bowery::element::Table;
    ///
    /// let mut table = Table::new("
    /// mode: play
    /// table: { stakes: { small_blind: 1.0, big_blind: 2.0 } }
    /// players: [ { seat: 0, stack: 200 }, { seat: 1, stack: 200, bot: abc } ]
    /// ").unwrap();
    /// assert!(table.take_seat(1).is_err()); // only in interactive
    /// ```
    pub fn take_seat(&mut self, seat: u8) -> Result<(), String> {
        self.game.take_seat(seat).map_err(|error| error.to_string())
    }

    /// Back to the record at the takeover moment.
    ///
    /// # Examples
    ///
    /// ```
    /// use bowery::element::Table;
    ///
    /// let mut table = Table::new("
    /// mode: play
    /// table: { stakes: { small_blind: 1.0, big_blind: 2.0 } }
    /// players: [ { seat: 0, stack: 200 }, { seat: 1, stack: 200, bot: abc } ]
    /// ").unwrap();
    /// table.back_to_record(); // nothing to go back to in play
    /// assert_eq!("none", table.state().record);
    /// ```
    pub fn back_to_record(&mut self) {
        self.cache.get_mut().clear();
        self.game.back_to_record();
    }

    /// The reader acts. `json` is `{"action":"fold"}`, or `{"action":"bet",
    /// "amount":600}` for a bet or a raise (the street total).
    ///
    /// # Errors
    ///
    /// The JSON does not read, names no action, a bet or raise has no
    /// amount, or the game refuses the action.
    ///
    /// # Examples
    ///
    /// ```
    /// use bowery::element::Table;
    ///
    /// let mut table = Table::new("
    /// mode: play
    /// table: { button: 0, stakes: { small_blind: 1.0, big_blind: 2.0 } }
    /// players: [ { seat: 0, stack: 200 }, { seat: 1, stack: 200, bot: abc } ]
    /// ").unwrap();
    /// table.act(r#"{"action":"call"}"#).unwrap();
    /// assert_eq!(2, table.state().moment);
    /// ```
    pub fn act(&mut self, json: &str) -> Result<(), String> {
        let request: ActRequest = serde_json::from_str(json).map_err(|error| error.to_string())?;
        let amount = || {
            request
                .amount
                .ok_or_else(|| format!("{} needs an amount", request.action))
        };
        let action = match request.action.as_str() {
            "fold" => PlayerAction::Fold,
            "check" => PlayerAction::Check,
            "call" => PlayerAction::Call,
            "bet" => PlayerAction::Bet(amount()?),
            "raise" => PlayerAction::Raise(amount()?),
            "all_in" => PlayerAction::AllIn,
            other => return Err(format!("unknown action `{other}`")),
        };
        self.game.act(action).map_err(|error| error.to_string())?;
        // The action is the newest moment, and it may have cut the line short
        // (off the record, from the moment it was made on): every moment
        // before it is as it was, and it and what followed are not.
        let at = self.game.at();
        self.cache.get_mut().retain(|index, _| *index < at);
        Ok(())
    }

    /// One step: `dealt` for a deal, `acted` for any other move, `waiting`
    /// when it is the reader's turn, `over` at the hand's last moment.
    ///
    /// # Errors
    ///
    /// The engine failed.
    ///
    /// # Examples
    ///
    /// ```
    /// use bowery::element::Table;
    ///
    /// let mut table = Table::new("
    /// mode: play
    /// seat: 1
    /// table: { button: 0, stakes: { small_blind: 1.0, big_blind: 2.0 } }
    /// players: [ { seat: 0, stack: 200, bot: abc }, { seat: 1, stack: 200 } ]
    /// ").unwrap();
    /// assert_eq!("acted", table.advance().unwrap());
    /// ```
    pub fn advance(&mut self) -> Result<&'static str, String> {
        match self.game.advance().map_err(|error| error.to_string())? {
            Advance::Moved if matches!(self.game.event(), Event::Deal(_)) => Ok("dealt"),
            Advance::Moved => Ok("acted"),
            Advance::Waiting => Ok("waiting"),
            Advance::Ended => Ok("over"),
        }
    }

    /// Play: deals the next hand.
    ///
    /// # Errors
    ///
    /// Not play mode, the hand is not over, or the table is broken.
    ///
    /// # Examples
    ///
    /// ```
    /// use bowery::element::Table;
    ///
    /// let mut table = Table::new("
    /// mode: play
    /// table: { stakes: { small_blind: 1.0, big_blind: 2.0 } }
    /// players: [ { seat: 0, stack: 200 }, { seat: 1, stack: 200, bot: abc } ]
    /// ").unwrap();
    /// assert!(table.next_hand().is_err()); // the hand has not started
    /// ```
    pub fn next_hand(&mut self) -> Result<(), String> {
        self.cache.get_mut().clear();
        self.game.next_hand().map_err(|error| error.to_string())
    }

    /// Play: a new game from hand 1, seeded with `seed` (the element passes
    /// a random one). The layout and view stay.
    ///
    /// # Errors
    ///
    /// Not play mode, or the engine cannot set the game up.
    ///
    /// # Examples
    ///
    /// ```
    /// use bowery::element::Table;
    ///
    /// let mut table = Table::new("
    /// mode: play
    /// table: { stakes: { small_blind: 1.0, big_blind: 2.0 } }
    /// players: [ { seat: 0, stack: 200 }, { seat: 1, stack: 200, bot: abc } ]
    /// ").unwrap();
    /// table.shuffle(42).unwrap();
    /// assert_eq!(1, table.state().hand_number);
    /// ```
    pub fn shuffle(&mut self, seed: u64) -> Result<(), String> {
        let mut config = self.game.config().clone();
        if config.mode != Mode::Play {
            return Err("only play mode shuffles".to_string());
        }
        config.seed = Some(seed);
        config.deck = None;
        let fresh = Self::from_config(&config).map_err(|problems| problems_json(&problems))?;
        self.game = fresh.game;
        self.cache.get_mut().clear();
        Ok(())
    }

    /// Draws in `view` from now on: `all` or `seat`.
    ///
    /// # Errors
    ///
    /// Any other name.
    ///
    /// # Examples
    ///
    /// ```
    /// use bowery::element::Table;
    /// use bowery::config::View;
    ///
    /// let mut table = Table::new("
    /// mode: play
    /// table: { stakes: { small_blind: 1.0, big_blind: 2.0 } }
    /// players: [ { seat: 0, stack: 200 }, { seat: 1, stack: 200, bot: abc } ]
    /// ").unwrap();
    /// table.set_view("all").unwrap();
    /// assert_eq!(View::All, table.state().view);
    /// ```
    pub fn set_view(&mut self, view: &str) -> Result<(), String> {
        self.view = view.parse()?;
        Ok(())
    }

    /// Turns the win percents on or off. They show only in the `all` view
    /// whatever the switch says, and the cache keeps what it has: turning
    /// them back on costs nothing.
    ///
    /// # Examples
    ///
    /// ```
    /// use bowery::element::Table;
    ///
    /// let mut table = Table::new("
    /// mode: play
    /// view: all
    /// table: { stakes: { small_blind: 1.0, big_blind: 2.0 } }
    /// players: [ { seat: 0, stack: 200 }, { seat: 1, stack: 200, bot: abc } ]
    /// ").unwrap();
    /// assert!(!table.state().odds); // play starts with the odds off
    /// table.set_odds(true);
    /// assert!(table.state().odds);
    /// ```
    pub fn set_odds(&mut self, on: bool) {
        self.odds = on;
    }

    /// Draws in `layout` from now on: `ring`, `spotlight` or `ledger`.
    ///
    /// # Errors
    ///
    /// Any other name.
    ///
    /// # Examples
    ///
    /// ```
    /// use bowery::element::Table;
    /// use bowery::config::Layout;
    ///
    /// let mut table = Table::new("
    /// mode: play
    /// table: { stakes: { small_blind: 1.0, big_blind: 2.0 } }
    /// players: [ { seat: 0, stack: 200 }, { seat: 1, stack: 200, bot: abc } ]
    /// ").unwrap();
    /// table.set_layout("spotlight").unwrap();
    /// assert_eq!(Layout::Spotlight, table.state().layout);
    /// ```
    pub fn set_layout(&mut self, layout: &str) -> Result<(), String> {
        self.layout = layout.parse()?;
        Ok(())
    }
}

/// The problems as JSON for the element to print under the picture:
/// `[{"path":"hand","message":"…","rule":null}]`; a hand finding's path
/// starts with `hand.` and carries its rule.
///
/// # Examples
///
/// ```
/// use bowery::element::{Table, problems_json};
///
/// let problems = Table::new("mode: replay\n").unwrap_err();
/// assert!(problems_json(&problems).contains(r#""path":"hand""#));
/// ```
#[must_use]
pub fn problems_json(problems: &[Problem]) -> String {
    #[derive(Serialize)]
    struct Shown {
        path: String,
        message: String,
        rule: Option<String>,
    }
    let shown: Vec<Shown> = problems
        .iter()
        .map(|problem| match problem {
            Problem::Hand(finding) => Shown {
                path: format!("hand.{}", finding.path),
                message: finding.message.clone(),
                rule: Some(finding.rule.to_string()),
            },
            Problem::Config { path, message } => Shown {
                path: path.clone(),
                message: message.clone(),
                rule: None,
            },
        })
        .collect();
    serde_json::to_string(&shown).unwrap_or_else(|_| "[]".to_string())
}

/// This crate's version, for the element's badge and `bowery:ready`.
///
/// # Examples
///
/// ```
/// assert_eq!(env!("CARGO_PKG_VERSION"), bowery::element::version());
/// ```
#[must_use]
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use super::*;

    const ALL_IN: &str = include_str!("../tests/hands/heads_up_all_in.yaml");

    fn all_in(head: &str) -> Table {
        let hand: String = ALL_IN.lines().flat_map(|line| ["  ", line, "\n"]).collect();
        Table::new(&format!("{head}hand:\n{hand}")).unwrap()
    }

    fn cached(table: &Table) -> usize {
        table.cache.borrow().len()
    }

    #[test]
    fn frame__fills_the_cache_once_per_moment() {
        let table = all_in("mode: replay\nview: all\nstart: flop\n");
        assert_eq!(0, cached(&table));

        let first = table.frame();
        let second = table.frame();

        assert_eq!(1, cached(&table));
        assert!(first.odds.is_some());
        assert_eq!(first.odds, second.odds);
    }

    #[test]
    fn frame__hits_the_cache_after_a_seek_back() {
        let mut table = all_in("mode: replay\nview: all\nstart: flop\n");
        let flop = table.frame().odds;
        table.seek(table.state().moment); // the next moment
        assert!(table.frame().odds.is_some());
        assert_eq!(2, cached(&table));

        table.seek(table.state().moment.saturating_sub(2)); // back to the flop
        let again = table.frame().odds;

        assert_eq!(2, cached(&table), "a seek leaves the cache alone");
        assert_eq!(flop, again);
    }

    #[test]
    fn frame__caches_a_moment_that_has_no_odds() {
        // The last moment has paid out, so it holds no percents.
        let mut table = all_in("mode: replay\nview: all\n");
        table.seek(usize::MAX);

        assert!(table.frame().odds.is_none());
        assert_eq!(1, cached(&table));
        assert_eq!(Some(&None), table.cache.borrow().values().next());
    }

    #[test]
    fn frame__off_or_in_the_seat_view_leaves_the_cache_empty() {
        let mut table = all_in("mode: replay\nview: all\nstart: flop\n");
        table.set_odds(false);
        assert!(table.frame().odds.is_none());
        assert_eq!(0, cached(&table));

        table.set_odds(true);
        table.set_view("seat").unwrap();
        assert!(table.frame().odds.is_none());
        assert_eq!(0, cached(&table));
    }

    #[test]
    fn set_odds__keeps_the_cache() {
        let mut table = all_in("mode: replay\nview: all\nstart: flop\n");
        let _ = table.frame();

        table.set_odds(false);
        table.set_odds(true);
        table.set_view("seat").unwrap();
        table.set_layout("ledger").unwrap();

        assert_eq!(1, cached(&table));
    }

    #[test]
    fn take_seat__keeps_the_cache() {
        let mut table = all_in("mode: interactive\nview: all\nseat: 1\nstart: 1\n");
        let _ = table.frame();
        assert_eq!(1, cached(&table));

        table.take_seat(1).unwrap();

        assert_eq!(1, cached(&table), "the reader is not in a cached moment");
    }

    #[test]
    fn act__drops_cached_moments_from_the_action_on_and_keeps_earlier_ones() {
        let mut table = all_in("mode: interactive\nview: all\nseat: 1\nstart: 1\n");
        table.take_seat(1).unwrap();
        let acting = table.state().moment - 1; // the index; `moment` counts from 1
        for index in [0, acting, acting + 1, acting + 2, acting + 7] {
            table.cache.get_mut().insert(index, None);
        }

        // Off the record: the line is cut after the acting moment and the
        // action is the moment that follows.
        table.act(r#"{"action":"fold"}"#).unwrap();

        assert_eq!(acting + 2, table.state().moment);
        let mut kept: Vec<usize> = table.cache.borrow().keys().copied().collect();
        kept.sort_unstable();
        assert_eq!(vec![0, acting], kept);
    }

    #[test]
    fn act__in_play_drops_only_the_moment_it_appends() {
        let mut table = Table::new(
            "mode: play\nview: all\nseat: 0\ntable: { button: 0, stakes: { small_blind: 1.0, big_blind: 2.0 } }\nplayers: [ { seat: 0, stack: 200 }, { seat: 1, stack: 200, bot: abc } ]\n",
        )
        .unwrap();
        assert_eq!(1, table.state().moment, "the hand has just been dealt");
        for index in [0, 1, 4] {
            table.cache.get_mut().insert(index, None);
        }

        table.act(r#"{"action":"fold"}"#).unwrap();

        assert_eq!(2, table.state().moment, "the fold is index 1");
        let mut kept: Vec<usize> = table.cache.borrow().keys().copied().collect();
        kept.sort_unstable();
        assert_eq!(vec![0], kept);
    }

    #[test]
    fn act__refused_keeps_the_cache() {
        let mut table = all_in("mode: interactive\nview: all\nseat: 1\nstart: 1\n");
        let _ = table.frame();
        table.cache.get_mut().insert(5, None);

        // No reader yet: the game refuses the action.
        assert!(table.act(r#"{"action":"fold"}"#).is_err());
        // The request does not read, or names no action.
        assert!(table.act("not json").is_err());
        assert!(table.act(r#"{"action":"dance"}"#).is_err());

        assert_eq!(2, cached(&table));
    }

    #[test]
    fn back_to_record__clears_the_cache() {
        let mut table = all_in("mode: interactive\nview: all\nseat: 1\nstart: 1\n");
        table.take_seat(1).unwrap();
        table.act(r#"{"action":"fold"}"#).unwrap();
        let _ = table.frame();

        table.back_to_record();

        assert_eq!(0, cached(&table));
    }

    #[test]
    fn advance__keeps_the_cache_so_a_seek_back_hits_it() {
        let mut table = Table::new(
            "mode: play\nview: all\nseat: 1\ntable: { button: 0, stakes: { small_blind: 1.0, big_blind: 2.0 } }\nplayers: [ { seat: 0, stack: 200, bot: abc }, { seat: 1, stack: 200 } ]\n",
        )
        .unwrap();
        table.set_odds(true);
        // Preflop with every card up has percents; the seeded `None` says the
        // cache answers, so no run is made.
        table.cache.get_mut().insert(0, None);

        assert_eq!("acted", table.advance().unwrap());
        table.seek(0);

        assert_eq!(Some(&None), table.cache.borrow().get(&0));
        assert!(table.frame().odds.is_none(), "answered from the cache");
        assert_eq!(1, cached(&table));
    }

    #[test]
    fn next_hand__clears_the_cache() {
        let mut table = Table::new(
            "mode: play\nseat: 0\ntable: { button: 0, stakes: { small_blind: 1.0, big_blind: 2.0 } }\nplayers: [ { seat: 0, stack: 200 }, { seat: 1, stack: 200, bot: abc } ]\n",
        )
        .unwrap();
        table.act(r#"{"action":"fold"}"#).unwrap();
        while table.advance().unwrap() != "over" {}
        table.cache.get_mut().insert(0, None);

        table.next_hand().unwrap();

        assert_eq!(0, cached(&table));
    }

    #[test]
    fn shuffle__clears_the_cache() {
        let mut table = Table::new(
            "mode: play\ntable: { stakes: { small_blind: 1.0, big_blind: 2.0 } }\nplayers: [ { seat: 0, stack: 200 }, { seat: 1, stack: 200, bot: abc } ]\n",
        )
        .unwrap();
        table.cache.get_mut().insert(0, None);

        table.shuffle(5).unwrap();

        assert_eq!(0, cached(&table));
    }

    #[test]
    fn controls__odds_needs_the_all_view() {
        let table = Table::new(
            "mode: play\ntable: { stakes: { small_blind: 1.0, big_blind: 2.0 } }\nplayers: [ { seat: 0, stack: 200 }, { seat: 1, stack: 200, bot: abc } ]\n",
        )
        .unwrap();
        assert!(!table.state().controls.odds);
        assert!(!table.state().odds);
    }
}
