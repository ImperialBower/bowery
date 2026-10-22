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

use pkcore::casino::action::PlayerAction;
use serde::{Deserialize, Serialize};

use crate::check::{Problem, check};
use crate::config::{Config, Layout, Mode, View};
use crate::draw::{Theme, svg};
use crate::frame::{Award, SeatStatus};
use crate::game::{Advance, Event, Game, RecordState};

/// Bot pacing when the config gives no `delay_ms`.
const DEFAULT_DELAY_MS: u32 = 700;

/// One element's table.
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
        svg(&self.game.frame(self.view), self.layout, width, theme)
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
                layout: config.controls.layout.unwrap_or(true),
            },
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
        self.game.act(action).map_err(|error| error.to_string())
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
