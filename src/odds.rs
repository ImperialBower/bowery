//! TV-style odds: each live seat's win percent, asked of pkcore's equity
//! engine.
//!
//! [`compute`] reads the hole cards and the board off a frame's seats and
//! returns an [`Odds`], or `None` when there is nothing honest to show: the
//! hand has paid out, fewer than two seats are in it, or a live seat's card
//! is face down. Equity is exact when pkcore can enumerate the runouts and
//! seeded Monte Carlo before that, so the same seed always gives the same
//! percents.
//!
//! ```
//! use bowery::check::check;
//! use bowery::config::{Config, View};
//! use bowery::game::Game;
//!
//! let config = Config::from_yaml("
//! mode: play
//! table: { stakes: { small_blind: 1.0, big_blind: 2.0 } }
//! players: [ { seat: 0, stack: 200 }, { seat: 1, stack: 200, bot: abc } ]
//! ").unwrap();
//! let game = Game::new(check(&config).unwrap()).unwrap();
//! let frame = game.frame_with_odds(View::All);
//! let odds = frame.odds.unwrap();
//! assert_eq!(2, odds.percents.len());
//! ```

use std::fmt;

use pkcore::analysis::equity::{EquityOptions, EquityRequest, PlayerSpec};
use pkcore::arrays::two::Two;
use pkcore::card::Card;
use pkcore::cards::Cards;
use pkcore::casino::table::Table;
use pkcore::play::board::Board;

use crate::frame::{CardFace, SeatFrame, SeatStatus};

/// The odds at one moment.
///
/// # Examples
///
/// ```
/// use bowery::odds::Odds;
///
/// let odds = Odds { percents: vec![Some(62), None, Some(38)], outs: None };
/// assert_eq!(2, odds.percents.iter().flatten().count());
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Odds {
    /// Each seat's win percent, 0 to 100, aligned with the frame's `seats`.
    /// `None` for a seat that is not live (folded or out).
    pub percents: Vec<Option<u8>>,
    /// The outs bar, when a live seat is behind on the flop or the turn.
    pub outs: Option<OutsBar>,
}

/// The seat that is behind, and the cards that would put it ahead.
///
/// # Examples
///
/// ```
/// use std::str::FromStr;
/// use bowery::odds::OutsBar;
/// use pkcore::card::Card;
///
/// let bar = OutsBar {
///     seat: 3,
///     name: "Gus Hansen".into(),
///     percent: 18,
///     cards: vec![Card::from_str("T♠").unwrap(), Card::from_str("4♣").unwrap()],
/// };
/// assert_eq!("Gus Hansen · 2 outs · 18%", bar.to_string());
///
/// let dead = OutsBar { cards: Vec::new(), percent: 0, ..bar };
/// assert_eq!("Gus Hansen · drawing dead · 0%", dead.to_string());
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutsBar {
    /// The seat number of the player who is behind.
    pub seat: u8,
    /// That player's name.
    pub name: String,
    /// That player's win percent, 0 to 100.
    pub percent: u8,
    /// The cards that put the player ahead; empty means drawing dead.
    pub cards: Vec<Card>,
}

impl fmt::Display for OutsBar {
    /// `Gus Hansen · 8 outs · 18%`, or `Gus Hansen · drawing dead · 0%`.
    /// One out reads `1 out`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.cards.len() {
            0 => write!(f, "{} · drawing dead · {}%", self.name, self.percent),
            1 => write!(f, "{} · 1 out · {}%", self.name, self.percent),
            count => write!(f, "{} · {count} outs · {}%", self.name, self.percent),
        }
    }
}

/// The odds for the seats of a frame, or `None` when there is nothing honest
/// to show.
///
/// It is `None` once the hand `paid` out, when fewer than two seats are live
/// or all in, and when any such seat has a hole card that is not face up.
/// The percents come from pkcore's equity engine, seeded with `seed`.
///
/// `table` is the pkcore table the frame was built from; `seats` and `board`
/// are the frame's.
///
/// # Examples
///
/// ```
/// use bowery::check::check;
/// use bowery::config::{Config, View};
/// use bowery::game::Game;
/// use bowery::odds::compute;
///
/// let config = Config::from_yaml("
/// mode: play
/// table: { stakes: { small_blind: 1.0, big_blind: 2.0 } }
/// players: [ { seat: 0, stack: 200 }, { seat: 1, stack: 200, bot: abc } ]
/// ").unwrap();
/// let game = Game::new(check(&config).unwrap()).unwrap();
/// let frame = game.frame(View::All);
///
/// // Both seats are dealt and face up, so the odds are there.
/// let odds = compute(game.table(), &frame.seats, &frame.board, false, game.seed()).unwrap();
/// assert_eq!(2, odds.percents.len());
/// // The hand has paid out: no odds.
/// assert_eq!(None, compute(game.table(), &frame.seats, &frame.board, true, game.seed()));
/// ```
#[must_use]
pub fn compute(
    _table: &Table,
    seats: &[SeatFrame],
    board: &[Option<Card>; 5],
    paid: bool,
    seed: u64,
) -> Option<Odds> {
    if paid {
        return None;
    }
    let mut players = Vec::with_capacity(seats.len());
    for seat in seats.iter().filter(|seat| is_live(seat)) {
        players.push(PlayerSpec::Exact(hole_cards(seat)?));
    }
    if players.len() < 2 {
        return None;
    }
    let cards: Cards = board.iter().flatten().copied().collect();
    let mut request = EquityRequest::new(players);
    request.board = if cards.is_empty() {
        Board::default()
    } else {
        Board::try_from(cards).ok()?
    };
    request.opts = EquityOptions {
        seed: Some(seed),
        ..EquityOptions::default()
    };
    let report = request.compute().ok()?;
    let mut shares = report.players.iter();
    let mut percents = Vec::with_capacity(seats.len());
    for seat in seats {
        percents.push(if is_live(seat) {
            Some(whole_percent(shares.next()?.equity_pct()))
        } else {
            None
        });
    }
    Some(Odds {
        percents,
        outs: None,
    })
}

/// A seat still in the hand: live or all in.
fn is_live(seat: &SeatFrame) -> bool {
    matches!(seat.status, SeatStatus::Live | SeatStatus::AllIn)
}

/// The seat's two face-up hole cards, or `None` if either is not up.
fn hole_cards(seat: &SeatFrame) -> Option<Two> {
    let [CardFace::Up(first), CardFace::Up(second)] = seat.cards else {
        return None;
    };
    Two::new(first, second).ok()
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "clamped to 0..=100 first"
)]
fn whole_percent(percent: f64) -> u8 {
    percent.round().clamp(0.0, 100.0) as u8
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use super::*;

    #[test]
    fn whole_percent__rounds_and_clamps() {
        assert_eq!(0, whole_percent(-0.4));
        assert_eq!(38, whole_percent(37.5));
        assert_eq!(37, whole_percent(37.49));
        assert_eq!(100, whole_percent(100.4));
        assert_eq!(0, whole_percent(f64::NAN));
    }

    #[test]
    fn outs_bar__one_out_reads_singular() {
        let bar = OutsBar {
            seat: 0,
            name: "Ann".into(),
            percent: 2,
            cards: vec![Card::BLANK],
        };
        assert_eq!("Ann · 1 out · 2%", bar.to_string());
    }

    #[test]
    fn odds__clone_equals() {
        let odds = Odds {
            percents: vec![Some(1), None],
            outs: None,
        };
        assert_eq!(odds, odds.clone());
    }
}
