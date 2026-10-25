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

use std::cmp::{Ordering, Reverse};
use std::fmt;

use pkcore::analysis::case_eval::CaseEval;
use pkcore::analysis::equity::{EquityOptions, EquityRequest, PlayerSpec};
use pkcore::arrays::two::Two;
use pkcore::card::Card;
use pkcore::cards::Cards;
use pkcore::casino::table::Table;
use pkcore::play::board::Board;
use pkcore::play::game::Game;
use pkcore::play::hole_cards::HoleCards;

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
    ///
    /// A folded seat's cards count as unknown, still in the deck, until pkcore
    /// ships dead cards (pkcore EPIC-123, `EquityRequest::dead`, in 0.18.0), so
    /// the percents are a little off wherever a seat has folded. The outs bar
    /// removes those cards by hand.
    pub percents: Vec<Option<u8>>,
    /// The outs bar: only at the turn (a four-card board), with exactly two
    /// live seats, and `None` when they are tied.
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
    /// The cards that put the player ahead; empty means drawing dead. In
    /// order: rank high to low, then ♠ ♥ ♦ ♣. A card a folded seat holds is left
    /// out.
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
/// or all in, when any such seat has a hole card that is not face up, and
/// when pkcore's equity engine returns an error (the engine's reason is not
/// kept). The percents come from that engine, seeded with `seed`.
///
/// `seats` and `board` are the frame's. `table` is the pkcore table the frame
/// was built from; its `dealt_hole_cards` give the cards of folded seats, which the
/// outs bar leaves out. The equity report's players are taken to be in request
/// order, which is the order of the live seats.
///
/// The percents treat a folded seat's cards as unknown, still in the deck,
/// until pkcore ships dead cards (pkcore EPIC-123, `EquityRequest::dead`, in
/// 0.18.0); the outs bar removes them by hand.
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
    table: &Table,
    seats: &[SeatFrame],
    board: &[Option<Card>; 5],
    paid: bool,
    seed: u64,
) -> Option<Odds> {
    if paid {
        return None;
    }
    let mut hands = Vec::with_capacity(seats.len());
    for seat in seats.iter().filter(|seat| is_live(seat)) {
        hands.push(hole_cards(seat)?);
    }
    let players: Vec<PlayerSpec> = hands.iter().copied().map(PlayerSpec::Exact).collect();
    if players.len() < 2 {
        return None;
    }
    let cards: Cards = board.iter().flatten().copied().collect();
    let mut request = EquityRequest::new(players);
    request.board = if cards.is_empty() {
        Board::default()
    } else {
        Board::try_from(cards.clone()).ok()?
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
    let outs = if hands.len() == 2 && cards.len() == 4 {
        outs_bar(table, seats, &percents, hands, cards)
    } else {
        None
    };
    Some(Odds { percents, outs })
}

/// The index of the lower of exactly two `Some` percents, or `None` when
/// there are not exactly two or they are equal.
fn trailing(percents: &[Option<u8>]) -> Option<usize> {
    let mut live = percents
        .iter()
        .enumerate()
        .filter_map(|(index, percent)| percent.map(|percent| (index, percent)));
    let (first, second) = (live.next()?, live.next()?);
    if live.next().is_some() {
        return None;
    }
    match first.1.cmp(&second.1) {
        Ordering::Less => Some(first.0),
        Ordering::Greater => Some(second.0),
        Ordering::Equal => None,
    }
}

/// The outs bar for the trailing seat of two live seats at the turn: the river
/// cards that make it the sole winner, less any card a folded seat holds.
///
/// `hands` are the two live seats' hole cards in live-seat order, and `board`
/// is the four turn cards. `None` when the seats are tied, the board is not a
/// turn board, or the trailing seat has no entry in `seats`.
fn outs_bar(
    table: &Table,
    seats: &[SeatFrame],
    percents: &[Option<u8>],
    hands: Vec<Two>,
    board: Cards,
) -> Option<OutsBar> {
    let behind = trailing(percents)?;
    let position = percents[..behind].iter().flatten().count();
    let percent = percents[behind]?;
    let trailing_seat = seats.get(behind)?;
    let held = folded_cards(table, seats);
    let game = Game::new(HoleCards::from(hands), Board::try_from(board).ok()?);
    let mut cards: Vec<Card> = game
        .turn_case_evals()
        .iter()
        .filter(|case| {
            case.win_count() == 1
                && case.winning_seats().first() == u8::try_from(position).ok().as_ref()
        })
        .map(CaseEval::card)
        .filter(|card| !held.contains(card))
        .collect();
    cards.sort_by_key(|card| (Reverse(card.get_rank()), Reverse(card.get_suit() as u8)));
    Some(OutsBar {
        seat: trailing_seat.seat,
        name: trailing_seat.name.clone(),
        percent,
        cards,
    })
}

/// The hole cards held by seats that have folded, from the pkcore table.
fn folded_cards(table: &Table, seats: &[SeatFrame]) -> Vec<Card> {
    seats
        .iter()
        .filter(|seat| seat.status == SeatStatus::Folded)
        .filter_map(|seat| table.dealt_hole_cards.get(&seat.seat))
        .flat_map(|cards| cards.as_slice().iter().copied())
        .filter(|card| *card != Card::BLANK)
        .collect()
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
    use std::str::FromStr;

    use pkcore::arrays::sliced::BoxedCards;

    use super::*;
    use crate::frame::Marks;

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

    fn seat(number: u8, name: &str, status: SeatStatus) -> SeatFrame {
        SeatFrame {
            seat: number,
            name: name.into(),
            stack: 0,
            bet: 0,
            cards: [CardFace::None; 2],
            status,
            marks: Marks::default(),
            last: None,
            to_act: false,
        }
    }

    fn card(text: &str) -> Card {
        Card::from_str(text).unwrap()
    }

    fn two(text: &str) -> Two {
        Two::from_str(text).unwrap()
    }

    fn board(text: &str) -> Cards {
        Cards::from_str(text).unwrap()
    }

    #[test]
    fn compute__three_live_seats_at_the_turn_have_percents_but_no_bar() {
        let up =
            |first: &str, second: &str| [CardFace::Up(card(first)), CardFace::Up(card(second))];
        let mut seats = [
            seat(0, "Ann", SeatStatus::Live),
            seat(1, "Bob", SeatStatus::Live),
            seat(2, "Cid", SeatStatus::Live),
        ];
        seats[0].cards = up("A♠", "A♥");
        seats[1].cards = up("K♦", "K♣");
        seats[2].cards = up("Q♥", "J♥");
        let board = [
            Some(card("9♣")),
            Some(card("6♦")),
            Some(card("2♣")),
            Some(card("3♥")),
            None,
        ];

        let odds = compute(&Table::default(), &seats, &board, false, 7).unwrap();

        assert_eq!(3, odds.percents.len());
        assert!(odds.percents.iter().all(Option::is_some));
        assert_eq!(None, odds.outs, "a bar is for exactly two live seats");
    }

    #[test]
    fn trailing__lower_of_two_live_seats() {
        assert_eq!(Some(1), trailing(&[Some(62), Some(38)]));
        assert_eq!(Some(2), trailing(&[None, Some(70), Some(30), None]));
    }

    #[test]
    fn trailing__none_when_tied() {
        assert_eq!(None, trailing(&[Some(50), Some(50)]));
    }

    #[test]
    fn trailing__none_unless_exactly_two_live() {
        assert_eq!(None, trailing(&[Some(100), None]));
        assert_eq!(None, trailing(&[Some(20), Some(30), Some(50)]));
        assert_eq!(None, trailing(&[None, None]));
        assert_eq!(None, trailing(&[]));
    }

    #[test]
    fn outs_bar__lists_the_trailing_seats_outs_in_deck_order() {
        // A nut-less flush draw against top pair: the nine spades left, high
        // to low.
        let seats = [
            seat(0, "Ann", SeatStatus::Live),
            seat(1, "Bob", SeatStatus::Live),
        ];
        let bar = outs_bar(
            &Table::default(),
            &seats,
            &[Some(80), Some(20)],
            vec![two("A♣ K♦"), two("9♠ 8♠")],
            board("A♠ 7♠ 2♥ Q♦"),
        )
        .unwrap();
        assert_eq!(1, bar.seat);
        assert_eq!("Bob", bar.name);
        assert_eq!(20, bar.percent);
        let listed: Vec<String> = bar.cards.iter().map(ToString::to_string).collect();
        assert_eq!(
            vec!["K♠", "Q♠", "J♠", "T♠", "6♠", "5♠", "4♠", "3♠", "2♠"],
            listed
        );
    }

    #[test]
    fn outs_bar__trailing_seat_after_a_folded_seat_maps_to_its_live_position() {
        // Seat 0 folded, so the trailing seat 2 is the second live hand: its
        // position among the live seats (1) is not its index in `percents` (2).
        let seats = [
            seat(0, "Ann", SeatStatus::Folded),
            seat(1, "Bob", SeatStatus::Live),
            seat(2, "Cid", SeatStatus::Live),
        ];
        let mut table = Table::default();
        table
            .dealt_hole_cards
            .insert(0, BoxedCards::from(vec![card("3♣"), card("4♦")]));
        let bar = outs_bar(
            &table,
            &seats,
            &[None, Some(80), Some(20)],
            vec![two("A♣ K♦"), two("9♠ 8♠")],
            board("A♠ 7♠ 2♥ Q♦"),
        )
        .unwrap();
        assert_eq!(2, bar.seat);
        assert_eq!("Cid", bar.name);
        assert_eq!(20, bar.percent);
        let listed: Vec<String> = bar.cards.iter().map(ToString::to_string).collect();
        assert_eq!(
            vec!["K♠", "Q♠", "J♠", "T♠", "6♠", "5♠", "4♠", "3♠", "2♠"],
            listed
        );
    }

    #[test]
    fn outs_bar__drawing_dead_is_empty() {
        // Quad kings against aces full: no river card helps the aces.
        let seats = [
            seat(0, "Ann", SeatStatus::Live),
            seat(1, "Bob", SeatStatus::Live),
        ];
        let bar = outs_bar(
            &Table::default(),
            &seats,
            &[Some(0), Some(100)],
            vec![two("A♦ A♣"), two("K♥ K♣")],
            board("K♠ K♦ 9♥ 4♣"),
        )
        .unwrap();
        assert_eq!(0, bar.seat);
        assert_eq!(Vec::<Card>::new(), bar.cards);
        assert_eq!(0, bar.percent);
    }

    #[test]
    fn outs_bar__none_when_tied() {
        let seats = [
            seat(0, "Ann", SeatStatus::Live),
            seat(1, "Bob", SeatStatus::Live),
        ];
        assert_eq!(
            None,
            outs_bar(
                &Table::default(),
                &seats,
                &[Some(50), Some(50)],
                vec![two("A♦ A♣"), two("K♥ K♣")],
                board("K♠ K♦ 9♥ 4♣"),
            )
        );
    }

    #[test]
    fn outs_bar__leaves_out_a_folded_seats_cards() {
        let seats = [
            seat(0, "Ann", SeatStatus::Live),
            seat(1, "Bob", SeatStatus::Live),
            seat(2, "Cid", SeatStatus::Folded),
        ];
        let mut table = Table::default();
        table
            .dealt_hole_cards
            .insert(2, BoxedCards::from(vec![card("6♣"), card("2♣")]));
        let hands = vec![two("6♠ 6♥"), two("5♦ 5♣")];
        let board = board("9♣ 6♦ 5♥ 5♠");
        let with_fold = outs_bar(
            &table,
            &seats,
            &[Some(2), Some(98), None],
            hands.clone(),
            board.clone(),
        )
        .unwrap();
        assert_eq!(
            Vec::<Card>::new(),
            with_fold.cards,
            "the 6♣ is in the folded hand"
        );
        let without = outs_bar(
            &Table::default(),
            &seats,
            &[Some(2), Some(98), None],
            hands,
            board,
        )
        .unwrap();
        assert_eq!(vec![card("6♣")], without.cards);
    }

    #[test]
    fn folded_cards__reads_only_folded_seats_and_skips_blanks() {
        let seats = [
            seat(0, "Ann", SeatStatus::Live),
            seat(1, "Bob", SeatStatus::Folded),
            seat(2, "Cid", SeatStatus::Folded),
        ];
        let mut table = Table::default();
        table
            .dealt_hole_cards
            .insert(0, BoxedCards::from(vec![card("A♠"), card("A♥")]));
        table
            .dealt_hole_cards
            .insert(1, BoxedCards::from(vec![card("7♣"), Card::BLANK]));
        assert_eq!(vec![card("7♣")], folded_cards(&table, &seats));
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
