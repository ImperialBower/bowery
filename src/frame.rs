//! What is on screen at one moment of a game.
//!
//! A [`Frame`] is built from a [`Game`] and a [`View`]. The view is applied
//! while the frame is built: in a `seat` view the other seats' cards are
//! [`CardFace::Down`] until a showdown, so a hidden card is never in the
//! frame, and nothing drawn from it can leak one.
//!
//! ```
//! use bowery::check::Checked;
//! use bowery::config::{Config, View};
//! use bowery::frame::CardFace;
//! use bowery::game::Game;
//!
//! let config = Config::from_yaml("
//! mode: play
//! seat: 0
//! table: { stakes: { small_blind: 1.0, big_blind: 2.0 } }
//! players: [ { seat: 0, stack: 200 }, { seat: 1, stack: 200, bot: abc } ]
//! ").unwrap();
//! let game = Game::new(Checked::new(&config).unwrap()).unwrap();
//! let frame = game.frame(View::Seat);
//!
//! assert!(matches!(frame.seats[0].cards[0], CardFace::Up(_)));
//! assert_eq!(CardFace::Down, frame.seats[1].cards[0]);
//! assert_eq!(3, frame.pot); // the blinds, still in front of the seats
//! ```
//!
//! [`Game`]: crate::game::Game

use std::fmt;

use pkcore::card::Card;
use pkcore::casino::action::PlayerAction;
use pkcore::casino::state::PlayerState;
use pkcore::casino::table::{Seat, Table};
use pkcore::hand_history::ReplayStreet;

use crate::config::View;
use crate::game::{Event, Game, RecordState};
use crate::odds::Odds;

/// Everything on screen at one moment.
///
/// # Examples
///
/// ```
/// use bowery::check::Checked;
/// use bowery::config::{Config, View};
/// use bowery::game::Game;
///
/// let config = Config::from_yaml("
/// mode: play
/// table: { stakes: { small_blind: 1.0, big_blind: 2.0 } }
/// players: [ { seat: 0, stack: 200 }, { seat: 1, stack: 200, bot: abc } ]
/// ").unwrap();
/// let frame = Game::new(Checked::new(&config).unwrap()).unwrap().frame(View::All);
/// assert_eq!(2, frame.seats.len());
/// assert_eq!((1, 1), frame.moment);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame {
    /// The occupied seats, in seat order. Empty chairs are left out.
    pub seats: Vec<SeatFrame>,
    /// The board in deal order; `None` for a card not dealt yet.
    pub board: [Option<Card>; 5],
    /// The pot plus every bet still in front of a seat, shown as `POT`.
    pub pot: usize,
    /// The seat whose action comes next.
    pub to_act: Option<u8>,
    /// One line on the moment: `River · Daniel Negreanu bets 65,000`.
    pub caption: String,
    /// This moment and the number of moments, counted from 1, for the step
    /// bar: `(21, 25)` is "21 of 25".
    pub moment: (usize, usize),
    /// The pot shares won so far, once the hand pays out. Chips returned to
    /// the seat that bet them are not a result.
    pub result: Option<Vec<Award>>,
    /// Where the game stands against the record.
    pub record: RecordState,
    /// The view the frame was built with.
    pub view: View,
    /// The street being bet, or the last one dealt.
    pub street: ReplayStreet,
    /// Win percents and the outs bar, when odds are on and every live card is up.
    pub odds: Option<Odds>,
}

/// One occupied seat on screen.
///
/// # Examples
///
/// ```
/// use bowery::check::Checked;
/// use bowery::config::{Config, View};
/// use bowery::frame::SeatStatus;
/// use bowery::game::Game;
///
/// let config = Config::from_yaml("
/// mode: play
/// table: { stakes: { small_blind: 1.0, big_blind: 2.0 } }
/// players: [ { seat: 0, name: Ann, stack: 200 }, { seat: 1, stack: 200, bot: abc } ]
/// ").unwrap();
/// let frame = Game::new(Checked::new(&config).unwrap()).unwrap().frame(View::All);
/// assert_eq!("Ann", frame.seats[0].name);
/// assert_eq!(SeatStatus::Live, frame.seats[0].status);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SeatFrame {
    /// The seat number, 0-based.
    pub seat: u8,
    /// The player's name.
    pub name: String,
    /// Chips behind.
    pub stack: usize,
    /// Chips in front of the seat on this street.
    pub bet: usize,
    /// The two hole cards as the view shows them.
    pub cards: [CardFace; 2],
    /// In the hand, folded, all in, or out.
    pub status: SeatStatus,
    /// Dealer button and blinds.
    pub marks: Marks,
    /// The seat's last action on this street, if any.
    pub last: Option<PlayerAction>,
    /// This seat acts next.
    pub to_act: bool,
}

/// One hole card as the view shows it.
///
/// # Examples
///
/// ```
/// use bowery::frame::CardFace;
/// use pkcore::card::Card;
/// use std::str::FromStr;
///
/// assert_eq!("A♠", CardFace::Up(Card::from_str("A♠").unwrap()).to_string());
/// assert_eq!("##", CardFace::Down.to_string());
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CardFace {
    /// Face up.
    Up(Card),
    /// Face down: there is a card, and the view hides it.
    Down,
    /// No card: not dealt, or the seat folded.
    None,
}

impl fmt::Display for CardFace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Up(card) => write!(f, "{card}"),
            Self::Down => f.write_str("##"),
            Self::None => f.write_str("--"),
        }
    }
}

/// Where a seat stands in the hand.
///
/// # Examples
///
/// ```
/// use bowery::frame::SeatStatus;
///
/// assert_eq!("all in", SeatStatus::AllIn.to_string());
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SeatStatus {
    /// In the hand, with chips behind.
    Live,
    /// Folded this hand.
    Folded,
    /// In the hand with no chips behind. A seat paid at the end of the hand
    /// has chips again and is live.
    AllIn,
    /// Not in this hand.
    Out,
}

impl fmt::Display for SeatStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Live => "live",
            Self::Folded => "fold",
            Self::AllIn => "all in",
            Self::Out => "out",
        })
    }
}

/// The dealer button and the blinds at a seat. Heads-up, the button is also
/// the small blind.
///
/// # Examples
///
/// ```
/// use bowery::frame::Marks;
///
/// let marks = Marks { dealer: true, small_blind: true, big_blind: false };
/// assert_eq!("D SB", marks.to_string());
/// assert_eq!("", Marks::default().to_string());
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Marks {
    /// The dealer button.
    pub dealer: bool,
    /// The small blind.
    pub small_blind: bool,
    /// The big blind.
    pub big_blind: bool,
}

impl fmt::Display for Marks {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let marks: Vec<&str> = [
            (self.dealer, "D"),
            (self.small_blind, "SB"),
            (self.big_blind, "BB"),
        ]
        .into_iter()
        .filter_map(|(on, mark)| on.then_some(mark))
        .collect();
        f.write_str(&marks.join(" "))
    }
}

/// One pot share won.
///
/// # Examples
///
/// ```
/// use bowery::frame::Award;
///
/// assert_eq!("seat 3 wins 575,700", Award { seat: 3, amount: 575_700 }.to_string());
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub struct Award {
    /// The seat paid.
    pub seat: u8,
    /// The chips won.
    pub amount: usize,
}

impl fmt::Display for Award {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "seat {} wins {}",
            self.seat,
            Frame::format_chips(self.amount)
        )
    }
}

impl Frame {
    /// The frame for the moment `game` is on, seen through `view`.
    ///
    /// In [`View::Seat`] the seat seen from is the config's `seat`, or else
    /// the reader's; with neither, every hole card is down. From a showdown
    /// on, every seat still in the hand shows its cards in either view.
    /// Folded seats show no cards.
    ///
    /// # Examples
    ///
    /// ```
    /// use bowery::check::Checked;
    /// use bowery::config::{Config, View};
    /// use bowery::frame::Frame;
    /// use bowery::game::Game;
    ///
    /// let config = Config::from_yaml("
    /// mode: play
    /// table: { stakes: { small_blind: 1.0, big_blind: 2.0 } }
    /// players: [ { seat: 0, stack: 200 }, { seat: 1, stack: 200, bot: abc } ]
    /// ").unwrap();
    /// let game = Game::new(Checked::new(&config).unwrap()).unwrap();
    /// assert_eq!(game.frame(View::All), Frame::new(&game, View::All));
    /// ```
    #[must_use]
    pub fn new(game: &Game, view: View) -> Self {
        let table = game.table();
        let at = game.at();
        let history: Vec<&Event> = (0..=at).filter_map(|index| game.event_at(index)).collect();
        let shown = history.contains(&&Event::Showdown);
        let street = history
            .iter()
            .rev()
            .find_map(|event| match event {
                Event::Deal(street) => Some(*street),
                _ => None,
            })
            .unwrap_or(ReplayStreet::Preflop);
        let viewer = game.config().seat.or(game.reader());
        let to_act = game.to_act();
        let seats: Vec<SeatFrame> = Self::occupied(table)
            .map(|(seat, chair)| {
                let status = Self::status(chair);
                let visible = view == View::All || viewer == Some(seat) || shown;
                SeatFrame {
                    seat,
                    name: chair.player.handle.clone(),
                    stack: chair.player.chips,
                    bet: chair.player.bet,
                    cards: Self::faces(chair, status, visible),
                    status,
                    marks: Marks {
                        dealer: table.button == seat,
                        small_blind: table.determine_small_blind() == seat,
                        big_blind: table.determine_big_blind() == seat,
                    },
                    last: Self::last_action(&history, seat),
                    to_act: to_act == Some(seat),
                }
            })
            .collect();
        let mut board = [None; 5];
        for (slot, card) in board.iter_mut().zip(table.board.iter()) {
            *slot = Some(*card);
        }
        let awards: Vec<Award> = history
            .iter()
            .filter_map(|event| match event {
                Event::Award { seat, amount } => Some(Award {
                    seat: *seat,
                    amount: *amount,
                }),
                _ => None,
            })
            .collect();
        Self {
            pot: table.pot + seats.iter().map(|seat| seat.bet).sum::<usize>(),
            caption: Self::caption(game.event(), street, &seats, table),
            seats,
            board,
            to_act,
            moment: (at + 1, game.moments()),
            result: (!awards.is_empty()).then_some(awards),
            record: game.record_state(),
            view,
            street,
            odds: None,
        }
    }

    /// The frame for the moment `game` is on, with the win percents worked
    /// out: [`Frame::new`], then [`Odds::compute`] on what it shows.
    ///
    /// `odds` stays `None` when any live seat's card is face down, when
    /// fewer than two seats are live, and once the hand has paid out.
    ///
    /// # Examples
    ///
    /// ```
    /// use bowery::check::Checked;
    /// use bowery::config::{Config, View};
    /// use bowery::frame::Frame;
    /// use bowery::game::Game;
    ///
    /// let config = Config::from_yaml("
    /// mode: play
    /// table: { stakes: { small_blind: 1.0, big_blind: 2.0 } }
    /// players: [ { seat: 0, stack: 200 }, { seat: 1, stack: 200, bot: abc } ]
    /// ").unwrap();
    /// let game = Game::new(Checked::new(&config).unwrap()).unwrap();
    /// assert!(Frame::with_odds(&game, View::All).odds.is_some());
    /// // Seen from seat 0, the bot's cards are down: nothing to count.
    /// assert!(Frame::with_odds(&game, View::Seat).odds.is_none());
    /// ```
    #[must_use]
    pub fn with_odds(game: &Game, view: View) -> Self {
        let mut frame = Self::new(game, view);
        frame.odds = Odds::compute(
            game.table(),
            &frame.seats,
            &frame.board,
            frame.result.is_some(),
            game.seed(),
        );
        frame
    }
}

impl fmt::Display for Frame {
    /// The caption, the board and pot, then one line per seat, ending in the
    /// seat's win percent when the frame has odds.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}", self.caption)?;
        let board: Vec<String> = self
            .board
            .iter()
            .flatten()
            .map(ToString::to_string)
            .collect();
        writeln!(
            f,
            "[{}] POT {}",
            board.join(" "),
            Self::format_chips(self.pot)
        )?;
        for (index, seat) in self.seats.iter().enumerate() {
            write!(
                f,
                "{:>2} {:<16} {:>11} {} {} {}",
                seat.seat,
                seat.name,
                Self::format_chips(seat.stack),
                seat.cards[0],
                seat.cards[1],
                seat.status
            )?;
            let percent = self
                .odds
                .as_ref()
                .and_then(|odds| odds.percents.get(index).copied().flatten());
            if let Some(percent) = percent {
                write!(f, " {percent:>3}%")?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

impl Frame {
    /// Chips as people read them: `260,200`. From a billion they are shortened
    /// to `1.25B`, from a trillion to `1.25T`, and from a thousand trillion to
    /// whole trillions, so the text is never longer than 11 characters.
    ///
    /// # Examples
    ///
    /// ```
    /// use bowery::frame::Frame;
    ///
    /// assert_eq!("575,700", Frame::format_chips(575_700));
    /// assert_eq!("999,999,999", Frame::format_chips(999_999_999));
    /// assert_eq!("4.29B", Frame::format_chips(4_294_967_295));
    /// ```
    #[must_use]
    pub fn format_chips(chips: usize) -> String {
        // u64, not usize: usize is 32 bits on wasm32, where a trillion does not fit.
        const BILLION: u64 = 1_000_000_000;
        const TRILLION: u64 = 1_000_000_000_000;
        let chips = u64::try_from(chips).unwrap_or(u64::MAX);
        let shorten = |unit: u64, suffix: char| {
            let whole = chips / unit;
            let hundredths = chips % unit / (unit / 100);
            if whole >= 1_000 {
                format!("{whole}{suffix}")
            } else {
                format!("{whole}.{hundredths:02}{suffix}")
            }
        };
        if chips >= TRILLION {
            return shorten(TRILLION, 'T');
        }
        if chips >= BILLION {
            return shorten(BILLION, 'B');
        }
        let digits = chips.to_string();
        let mut grouped = String::with_capacity(digits.len() + digits.len() / 3);
        for (index, digit) in digits.chars().enumerate() {
            if index > 0 && (digits.len() - index).is_multiple_of(3) {
                grouped.push(',');
            }
            grouped.push(digit);
        }
        grouped
    }

    /// The occupied chairs of `table`, with their seat numbers.
    fn occupied(table: &Table) -> impl Iterator<Item = (u8, &Seat)> {
        (0..=u8::MAX)
            .map_while(|seat| table.seats.get_seat(seat).map(|chair| (seat, chair)))
            .filter(|(_, chair)| !chair.is_empty())
    }

    fn status(chair: &Seat) -> SeatStatus {
        match chair.player.state {
            PlayerState::Fold => SeatStatus::Folded,
            PlayerState::Out => SeatStatus::Out,
            _ if chair.player.chips == 0
                && (chair.player.bet > 0
                    || matches!(chair.player.state, PlayerState::AllIn(_))) =>
            {
                SeatStatus::AllIn
            }
            _ => SeatStatus::Live,
        }
    }

    /// The two hole cards as the view shows them: none for a folded seat or one
    /// out of the hand, up when `visible`, down otherwise.
    fn faces(chair: &Seat, status: SeatStatus, visible: bool) -> [CardFace; 2] {
        let mut faces = [CardFace::None; 2];
        if matches!(status, SeatStatus::Folded | SeatStatus::Out) {
            return faces;
        }
        for (face, card) in faces.iter_mut().zip(chair.cards.as_slice()) {
            *face = match (*card == Card::BLANK, visible) {
                (true, _) => CardFace::None,
                (false, true) => CardFace::Up(*card),
                (false, false) => CardFace::Down,
            };
        }
        faces
    }

    /// The seat's last action since the last deal.
    fn last_action(history: &[&Event], seat: u8) -> Option<PlayerAction> {
        history
            .iter()
            .rev()
            .take_while(|event| !matches!(event, Event::Deal(_)))
            .find_map(|event| match event {
                Event::Action {
                    seat: actor,
                    action,
                    ..
                } if *actor == seat => Some(*action),
                _ => None,
            })
    }

    fn street_name(street: ReplayStreet) -> &'static str {
        match street {
            ReplayStreet::Preflop => "Preflop",
            ReplayStreet::Flop => "Flop",
            ReplayStreet::Turn => "Turn",
            ReplayStreet::River => "River",
        }
    }

    /// One line on the moment's event.
    fn caption(event: &Event, street: ReplayStreet, seats: &[SeatFrame], table: &Table) -> String {
        let name = |seat: u8| {
            seats
                .iter()
                .find(|candidate| candidate.seat == seat)
                .map_or_else(
                    || format!("Seat {seat}"),
                    |candidate| candidate.name.clone(),
                )
        };
        let bet = |seat: u8| {
            seats
                .iter()
                .find(|candidate| candidate.seat == seat)
                .map_or(0, |candidate| candidate.bet)
        };
        match event {
            Event::Forced => "Preflop · blinds posted, cards dealt".to_string(),
            Event::Action { seat, action, .. } => {
                let verb = match action {
                    PlayerAction::Fold => "folds".to_string(),
                    PlayerAction::Check => "checks".to_string(),
                    PlayerAction::Call => format!("calls {}", Self::format_chips(bet(*seat))),
                    PlayerAction::Bet(amount) => format!("bets {}", Self::format_chips(*amount)),
                    PlayerAction::Raise(amount) => {
                        format!("raises to {}", Self::format_chips(*amount))
                    }
                    PlayerAction::AllIn => {
                        format!("is all in for {}", Self::format_chips(bet(*seat)))
                    }
                };
                format!("{} · {} {verb}", Self::street_name(street), name(*seat))
            }
            Event::Deal(street) => {
                let cards: Vec<String> = match street {
                    ReplayStreet::Flop => table
                        .board
                        .iter()
                        .take(3)
                        .map(ToString::to_string)
                        .collect(),
                    _ => table
                        .board
                        .iter()
                        .last()
                        .map(ToString::to_string)
                        .into_iter()
                        .collect(),
                };
                format!("{} · {}", Self::street_name(*street), cards.join(" "))
            }
            Event::Return { seat, amount } => {
                format!("{} takes back {}", name(*seat), Self::format_chips(*amount))
            }
            Event::Showdown => "Showdown".to_string(),
            Event::Award { seat, amount } => {
                format!("{} wins {}", name(*seat), Self::format_chips(*amount))
            }
        }
    }
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use super::*;

    #[test]
    fn format_chips__groups_thousands() {
        assert_eq!("0", Frame::format_chips(0));
        assert_eq!("999", Frame::format_chips(999));
        assert_eq!("1,000", Frame::format_chips(1_000));
        assert_eq!("287,100", Frame::format_chips(287_100));
        assert_eq!("12,345,678", Frame::format_chips(12_345_678));
    }

    #[test]
    fn format_chips__shortens_from_a_billion() {
        assert_eq!("1.00B", Frame::format_chips(1_000_000_000));
        assert_eq!("42.94B", Frame::format_chips(42_949_672_950));
        assert_eq!("1.50T", Frame::format_chips(1_500_000_000_000));
    }

    #[test]
    fn format_chips__never_longer_than_eleven() {
        for chips in [999_999_999, 999_999_999_999, usize::MAX] {
            assert!(Frame::format_chips(chips).chars().count() <= 11, "{chips}");
        }
    }

    #[test]
    fn card_face__display() {
        assert_eq!("--", CardFace::None.to_string());
    }

    #[test]
    fn seat_status__display() {
        assert_eq!("fold", SeatStatus::Folded.to_string());
        assert_eq!("out", SeatStatus::Out.to_string());
    }

    #[test]
    fn marks__display_all() {
        let marks = Marks {
            dealer: true,
            small_blind: true,
            big_blind: true,
        };

        assert_eq!("D SB BB", marks.to_string());
    }

    #[test]
    fn street_name__capitalised() {
        assert_eq!("River", Frame::street_name(ReplayStreet::River));
    }
}
