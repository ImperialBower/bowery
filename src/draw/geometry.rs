//! Every box of a layout, as rectangles, before anything is drawn.
//!
//! [`plan`] places the seat plates, the bet slots, the board, the pot, the
//! caption, and the folded strip or street label, for a layout, a seat count
//! and a width. Nothing is drawn here. Every box is sized for the widest text
//! it can hold (a 16-character name, an 11-character chip count, the longest
//! bet label), so a box that fits on the canvas holds its text, and two boxes
//! that do not intersect cannot cover each other's text. `tests/geometry.rs`
//! checks that for every layout, seat count and width.
//!
//! ```
//! use bowery::config::Layout;
//! use bowery::draw::geometry::plan;
//!
//! let plan = plan(Layout::Ring, &[true; 8], &[], 680);
//! assert_eq!(Layout::Ring, plan.layout);
//! assert_eq!(8, plan.seats.len());
//! ```

use crate::config::Layout;

/// The narrowest canvas; a narrower width draws at this one.
pub const MIN_WIDTH: u32 = 320;
/// Below this width, ring falls back to ledger (EPIC-20 rule 7).
pub const RING_MIN_WIDTH: u32 = 520;
/// From this width, ledger puts a seat on one line instead of two, and
/// spotlight puts two live seats beside the board.
pub const WIDE: u32 = 420;

/// Space between boxes.
pub const GAP: u32 = 6;
/// Inner margin of the canvas.
pub const MARGIN: u32 = 8;
/// A seat plate: name, stack and marks, two small cards.
pub const PLATE_W: u32 = 112;
/// See [`PLATE_W`].
pub const PLATE_H: u32 = 68;
/// A bet slot: an 11-character chip count, or `BET ` and one in ledger.
pub const BET_W: u32 = 82;
/// See [`BET_W`].
pub const BET_H: u32 = 18;
/// Ledger's bet slot, which also names the action: `ALL IN 999,999,999`.
pub const LEDGER_BET_W: u32 = 128;
/// A board card.
pub const CARD_W: u32 = 30;
/// See [`CARD_W`].
pub const CARD_H: u32 = 42;
/// A hole card on a plate.
pub const SMALL_CARD_W: u32 = 24;
/// See [`SMALL_CARD_W`].
pub const SMALL_CARD_H: u32 = 34;
/// Five board cards with gaps.
pub const BOARD_W: u32 = 5 * CARD_W + 4 * 3;
/// `POT 999,999,999` in the pot's font.
pub const POT_W: u32 = 152;
/// See [`POT_W`].
pub const POT_H: u32 = 22;
/// One caption line.
pub const CAPTION_H: u32 = 20;
/// Ledger's street label: `PREFLOP`.
pub const STREET_W: u32 = 60;
/// See [`STREET_W`].
pub const STREET_H: u32 = 14;
/// One line of spotlight's folded strip.
pub const STRIP_LINE_H: u32 = 14;
/// The width of one character of the 11 px text, in tenths of a pixel
/// (monospace: 0.6 em).
pub const SMALL_CHAR_TENTHS: u32 = 66;

/// A box on the canvas, in whole pixels from the top left.
///
/// # Examples
///
/// ```
/// use bowery::draw::geometry::Rect;
///
/// let rect = Rect { x: 10, y: 20, w: 30, h: 40 };
/// assert_eq!(40, rect.right());
/// assert_eq!(60, rect.bottom());
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rect {
    /// Left edge.
    pub x: u32,
    /// Top edge.
    pub y: u32,
    /// Width.
    pub w: u32,
    /// Height.
    pub h: u32,
}

impl Rect {
    /// The right edge: `x + w`.
    ///
    /// # Examples
    ///
    /// ```
    /// use bowery::draw::geometry::Rect;
    ///
    /// assert_eq!(15, Rect { x: 5, y: 0, w: 10, h: 1 }.right());
    /// ```
    #[must_use]
    pub fn right(&self) -> u32 {
        self.x + self.w
    }

    /// The bottom edge: `y + h`.
    ///
    /// # Examples
    ///
    /// ```
    /// use bowery::draw::geometry::Rect;
    ///
    /// assert_eq!(11, Rect { x: 0, y: 10, w: 1, h: 1 }.bottom());
    /// ```
    #[must_use]
    pub fn bottom(&self) -> u32 {
        self.y + self.h
    }

    /// True when the two boxes share any area. Boxes that only touch along
    /// an edge do not intersect.
    ///
    /// # Examples
    ///
    /// ```
    /// use bowery::draw::geometry::Rect;
    ///
    /// let first = Rect { x: 0, y: 0, w: 10, h: 10 };
    /// assert!(first.intersects(&Rect { x: 5, y: 5, w: 10, h: 10 }));
    /// assert!(!first.intersects(&Rect { x: 10, y: 0, w: 10, h: 10 }));
    /// ```
    #[must_use]
    pub fn intersects(&self, other: &Rect) -> bool {
        self.x < other.right()
            && other.x < self.right()
            && self.y < other.bottom()
            && other.y < self.bottom()
    }
}

/// One seat's boxes. In spotlight a folded seat has neither: it is a name in
/// the folded strip.
///
/// # Examples
///
/// ```
/// use bowery::draw::geometry::SeatBoxes;
///
/// assert_eq!(None, SeatBoxes::default().plate);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SeatBoxes {
    /// The seat plate (in ledger, the row less its bet slot).
    pub plate: Option<Rect>,
    /// The reserved bet slot: the same place every moment.
    pub bet: Option<Rect>,
}

/// Every box of one layout at one width.
///
/// # Examples
///
/// ```
/// use bowery::config::Layout;
/// use bowery::draw::geometry::plan;
///
/// let plan = plan(Layout::Ledger, &[true, false], &[], 680);
/// assert_eq!(2, plan.seats.len());
/// assert!(plan.boxes().len() >= 7);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
    /// The layout drawn: ring falls back to ledger when it does not fit.
    pub layout: Layout,
    /// Canvas width, at least [`MIN_WIDTH`].
    pub width: u32,
    /// Canvas height.
    pub height: u32,
    /// One entry per seat, in the order the seats were given.
    pub seats: Vec<SeatBoxes>,
    /// The five board cards.
    pub board: Rect,
    /// The pot.
    pub pot: Rect,
    /// The caption line.
    pub caption: Rect,
    /// Spotlight: the folded seats' names.
    pub strip: Option<Rect>,
    /// Ledger: the street's name.
    pub street: Option<Rect>,
    /// Ring: the outlined oval, drawn under everything. Not a box.
    pub oval: Option<Rect>,
}

impl Plan {
    /// Every box, for the overlap check: plates, bet slots, board, pot,
    /// caption, strip and street label. The oval is not a box.
    ///
    /// # Examples
    ///
    /// ```
    /// use bowery::config::Layout;
    /// use bowery::draw::geometry::plan;
    ///
    /// let plan = plan(Layout::Ring, &[true, true], &[], 680);
    /// assert_eq!(2 * 2 + 3, plan.boxes().len());
    /// ```
    #[must_use]
    pub fn boxes(&self) -> Vec<Rect> {
        let mut boxes: Vec<Rect> = self
            .seats
            .iter()
            .flat_map(|seat| seat.plate.into_iter().chain(seat.bet))
            .collect();
        boxes.extend([self.board, self.pot, self.caption]);
        boxes.extend(self.strip);
        boxes.extend(self.street);
        boxes
    }
}

/// Places every box of `layout` for seats whose liveness is `live` (in seat
/// order), with `folded` the names spotlight lists in its strip, at `width`
/// pixels.
///
/// Ring needs [`RING_MIN_WIDTH`], and room for its seats: two rows of plates,
/// and a column on each side from 574 px. Where it does not fit it falls back
/// to ledger, which fits every count from [`MIN_WIDTH`]. A width below
/// [`MIN_WIDTH`] is drawn at [`MIN_WIDTH`].
///
/// # Examples
///
/// ```
/// use bowery::config::Layout;
/// use bowery::draw::geometry::plan;
///
/// assert_eq!(Layout::Ledger, plan(Layout::Ring, &[true; 6], &[], 400).layout);
/// ```
#[must_use]
pub fn plan(layout: Layout, live: &[bool], folded: &[String], width: u32) -> Plan {
    let width = width.max(MIN_WIDTH);
    match layout {
        Layout::Ring => ring(live.len(), width).unwrap_or_else(|| ledger(live.len(), width)),
        Layout::Spotlight => spotlight(live, folded, width),
        Layout::Ledger => ledger(live.len(), width),
    }
}

/// Spotlight's strip, `folded: A · B (SB) · C`, wrapped between names to
/// fit `width`; a name too long for a line is cut across lines. Empty when
/// nothing is folded.
///
/// # Examples
///
/// ```
/// use bowery::draw::geometry::strip_lines;
///
/// let names: Vec<String> = ["Doyle Brunson (D)", "Eli Elezra (SB)", "Cory Zeidman"]
///     .map(String::from)
///     .to_vec();
/// assert_eq!(vec!["folded: Doyle Brunson (D) · Eli Elezra (SB) · Cory Zeidman"], strip_lines(&names, 680));
/// assert_eq!(
///     vec!["folded: Doyle Brunson (D) · Eli Elezra (SB)", "Cory Zeidman"],
///     strip_lines(&names, 320)
/// );
/// assert!(strip_lines(&[], 680).is_empty());
/// ```
#[must_use]
pub fn strip_lines(folded: &[String], width: u32) -> Vec<String> {
    let per_line =
        usize::try_from(((width.max(MIN_WIDTH) - 2 * MARGIN) * 10 / SMALL_CHAR_TENTHS).max(1))
            .unwrap_or(1);
    let mut lines: Vec<String> = Vec::new();
    let mut line = String::new();
    for (index, name) in folded.iter().enumerate() {
        let piece = if index == 0 {
            format!("folded: {name}")
        } else {
            format!(" · {name}")
        };
        if !line.is_empty() && line.chars().count() + piece.chars().count() > per_line {
            lines.push(std::mem::take(&mut line));
            line.push_str(piece.trim_start_matches(" · "));
        } else {
            line.push_str(&piece);
        }
        while line.chars().count() > per_line {
            let rest: String = line.chars().skip(per_line).collect();
            lines.push(line.chars().take(per_line).collect());
            line = rest;
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

/// How many plates fit in one row across `width`.
fn per_row(width: u32) -> u32 {
    (width + GAP) / (PLATE_W + GAP)
}

/// `count` plates spread evenly across `width` at height `y`.
fn row(count: u32, width: u32, y: u32) -> Vec<Rect> {
    (0..count)
        .map(|index| {
            let x = if count == 1 {
                (width - PLATE_W) / 2
            } else {
                index * (width - PLATE_W) / (count - 1)
            };
            Rect {
                x,
                y,
                w: PLATE_W,
                h: PLATE_H,
            }
        })
        .collect()
}

/// A bet slot centred on `plate` at height `y`.
fn bet_under(plate: Rect, y: u32) -> Rect {
    Rect {
        x: plate.x + (PLATE_W - BET_W) / 2,
        y,
        w: BET_W,
        h: BET_H,
    }
}

fn centred(width: u32, w: u32, y: u32, h: u32) -> Rect {
    Rect {
        x: (width - w) / 2,
        y,
        w,
        h,
    }
}

/// Ring: plates on fixed slots outside an oval, bets in reserved slots
/// between each plate and the centre. Rows only, or rows and two side
/// columns from 574 px. `None` when the seats do not fit.
fn ring(seats: usize, width: u32) -> Option<Plan> {
    let count = u32::try_from(seats).ok()?;
    if width < RING_MIN_WIDTH || count == 0 {
        return None;
    }
    let sides_need = 2 * (PLATE_W + GAP + BET_W + GAP) + BOARD_W;
    let sides = width >= sides_need && count >= 2;
    let (left_right, rest) = if sides { (2, count - 2) } else { (0, count) };
    let (top, bottom) = (rest.div_ceil(2), rest / 2);
    if top > per_row(width) {
        return None;
    }

    let top_bets = PLATE_H + GAP;
    let middle = top_bets + BET_H + GAP;
    let pot_y = middle + CARD_H + GAP;
    let bottom_bets = middle + PLATE_H + GAP;
    let bottom_y = bottom_bets + BET_H + GAP;
    let caption_y = bottom_y + PLATE_H + GAP;
    let height = caption_y + CAPTION_H;

    let top_plates = row(top, width, 0);
    let bottom_plates = row(bottom, width, bottom_y);
    let mut slots: Vec<SeatBoxes> = Vec::with_capacity(seats);
    let side_bet_y = middle + (PLATE_H - BET_H) / 2;
    if left_right > 0 {
        let plate = Rect {
            x: 0,
            y: middle,
            w: PLATE_W,
            h: PLATE_H,
        };
        let bet = Rect {
            x: PLATE_W + GAP,
            y: side_bet_y,
            w: BET_W,
            h: BET_H,
        };
        slots.push(SeatBoxes {
            plate: Some(plate),
            bet: Some(bet),
        });
    }
    slots.extend(top_plates.into_iter().map(|plate| SeatBoxes {
        plate: Some(plate),
        bet: Some(bet_under(plate, top_bets)),
    }));
    if left_right > 0 {
        let plate = Rect {
            x: width - PLATE_W,
            y: middle,
            w: PLATE_W,
            h: PLATE_H,
        };
        let bet = Rect {
            x: width - PLATE_W - GAP - BET_W,
            y: side_bet_y,
            w: BET_W,
            h: BET_H,
        };
        slots.push(SeatBoxes {
            plate: Some(plate),
            bet: Some(bet),
        });
    }
    slots.extend(bottom_plates.into_iter().rev().map(|plate| SeatBoxes {
        plate: Some(plate),
        bet: Some(bet_under(plate, bottom_bets)),
    }));

    let oval_inset = PLATE_W / 2;
    Some(Plan {
        layout: Layout::Ring,
        width,
        height,
        seats: slots,
        board: centred(width, BOARD_W, middle, CARD_H),
        pot: centred(width, POT_W, pot_y, POT_H),
        caption: Rect {
            x: 0,
            y: caption_y,
            w: width,
            h: CAPTION_H,
        },
        strip: None,
        street: None,
        oval: Some(Rect {
            x: oval_inset,
            y: PLATE_H / 2,
            w: width - 2 * oval_inset,
            h: bottom_y,
        }),
    })
}

/// Ledger: the board, pot and street on top, then one row per seat. A row
/// is one line from [`WIDE`], two below it.
fn ledger(seats: usize, width: u32) -> Plan {
    let wide = width >= WIDE;
    let board = Rect {
        x: MARGIN,
        y: MARGIN,
        w: BOARD_W,
        h: CARD_H,
    };
    let pot = if wide {
        Rect {
            x: board.right() + 2 * GAP,
            y: board.y + (CARD_H - POT_H) / 2,
            w: POT_W,
            h: POT_H,
        }
    } else {
        Rect {
            x: MARGIN,
            y: board.bottom() + GAP,
            w: POT_W,
            h: POT_H,
        }
    };
    let street = Rect {
        x: width - MARGIN - STREET_W,
        y: MARGIN,
        w: STREET_W,
        h: STREET_H,
    };
    let header = pot.bottom().max(board.bottom()) + GAP;
    let row_h = if wide {
        SMALL_CARD_H + 6
    } else {
        2 * SMALL_CARD_H - 10
    };
    let slot_x = width - MARGIN - LEDGER_BET_W;
    let rows: Vec<SeatBoxes> = (0..seats)
        .map(|index| {
            let y = header + u32::try_from(index).unwrap_or(0) * row_h;
            SeatBoxes {
                plate: Some(Rect {
                    x: 0,
                    y,
                    w: slot_x - GAP,
                    h: row_h,
                }),
                bet: Some(Rect {
                    x: slot_x,
                    y: y + (row_h - BET_H) / 2,
                    w: LEDGER_BET_W,
                    h: BET_H,
                }),
            }
        })
        .collect();
    let caption_y = header + u32::try_from(seats).unwrap_or(0) * row_h + GAP;
    Plan {
        layout: Layout::Ledger,
        width,
        height: caption_y + CAPTION_H,
        seats: rows,
        board,
        pot,
        caption: Rect {
            x: 0,
            y: caption_y,
            w: width,
            h: CAPTION_H,
        },
        strip: None,
        street: Some(street),
        oval: None,
    }
}

/// Spotlight: folded seats as a strip of names on top, live seats large.
/// Two live seats sit either side of the board from [`WIDE`]; otherwise the
/// board and pot come first and live seats follow in a grid.
fn spotlight(live: &[bool], folded: &[String], width: u32) -> Plan {
    let lines = u32::try_from(strip_lines(folded, width).len()).unwrap_or(u32::MAX);
    let strip = (lines > 0).then(|| Rect {
        x: 0,
        y: 0,
        w: width,
        h: lines.saturating_mul(STRIP_LINE_H) + GAP,
    });
    let top = strip.map_or(0, |strip| strip.bottom() + GAP);
    let live_count = live.iter().filter(|alive| **alive).count();
    let mut seats = vec![SeatBoxes::default(); live.len()];
    let live_indexes: Vec<usize> = (0..live.len()).filter(|index| live[*index]).collect();

    let (board, pot, bottom) = if live_count == 2 && width >= WIDE {
        let left = Rect {
            x: MARGIN,
            y: top,
            w: PLATE_W,
            h: PLATE_H,
        };
        let right = Rect {
            x: width - MARGIN - PLATE_W,
            ..left
        };
        for (index, plate) in live_indexes.iter().zip([left, right]) {
            seats[*index] = SeatBoxes {
                plate: Some(plate),
                bet: Some(bet_under(plate, plate.bottom() + GAP)),
            };
        }
        let board = centred(width, BOARD_W, top, CARD_H);
        let pot = centred(width, POT_W, board.bottom() + GAP, POT_H);
        (board, pot, (top + PLATE_H + GAP + BET_H).max(pot.bottom()))
    } else {
        let board = centred(width, BOARD_W, top, CARD_H);
        let pot = centred(width, POT_W, board.bottom() + GAP, POT_H);
        let columns = ((width - 2 * MARGIN + GAP) / (PLATE_W + GAP)).max(1);
        let cell_h = PLATE_H + GAP + BET_H + 2 * GAP;
        let grid_y = pot.bottom() + 2 * GAP;
        let mut bottom = pot.bottom();
        for (slot, index) in live_indexes.iter().enumerate() {
            let slot = u32::try_from(slot).unwrap_or(0);
            let plate = Rect {
                x: MARGIN + slot % columns * (PLATE_W + GAP),
                y: grid_y + slot / columns * cell_h,
                w: PLATE_W,
                h: PLATE_H,
            };
            let bet = bet_under(plate, plate.bottom() + GAP);
            bottom = bottom.max(bet.bottom());
            seats[*index] = SeatBoxes {
                plate: Some(plate),
                bet: Some(bet),
            };
        }
        (board, pot, bottom)
    };
    let caption_y = bottom + GAP;
    Plan {
        layout: Layout::Spotlight,
        width,
        height: caption_y + CAPTION_H,
        seats,
        board,
        pot,
        caption: Rect {
            x: 0,
            y: caption_y,
            w: width,
            h: CAPTION_H,
        },
        strip,
        street: None,
        oval: None,
    }
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use super::*;

    /// Text of `chars` characters at `tenths_per_char`, with `padding`.
    fn fits(chars: u32, tenths_per_char: u32, padding: u32, width: u32) -> bool {
        (chars * tenths_per_char).div_ceil(10) + padding <= width
    }

    #[test]
    fn plate__holds_a_sixteen_character_name() {
        assert!(fits(16, SMALL_CHAR_TENTHS, 6, PLATE_W));
    }

    #[test]
    fn plate__holds_the_widest_stack_and_its_marks() {
        // `999,999,999 D SB` is 16 characters.
        assert!(fits(16, SMALL_CHAR_TENTHS, 6, PLATE_W));
        const { assert!(2 * SMALL_CARD_W + 3 + 6 <= PLATE_W) };
        const { assert!(2 * 13 + SMALL_CARD_H + 6 <= PLATE_H) };
    }

    #[test]
    fn bet_slot__holds_the_widest_chip_count() {
        assert!(fits(11, SMALL_CHAR_TENTHS, 8, BET_W));
        assert!(fits(18, SMALL_CHAR_TENTHS, 8, LEDGER_BET_W));
    }

    #[test]
    fn pot__holds_the_widest_pot() {
        // `POT 999,999,999` at 16 px bold, 9.6 px a character.
        assert!(fits(15, 96, 8, POT_W));
    }

    #[test]
    fn per_row__counts_plates() {
        assert_eq!(4, per_row(520));
        assert_eq!(5, per_row(680));
    }

    #[test]
    fn row__spreads_plates_edge_to_edge() {
        let plates = row(3, 680, 0);

        assert_eq!(0, plates[0].x);
        assert_eq!(680, plates[2].right());
    }
}
