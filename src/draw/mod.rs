//! The one renderer: a [`Frame`] as SVG.
//!
//! The browser and the printed book draw the same SVG from [`svg`], so they
//! cannot drift apart. The output follows EPIC-18's contract for paper: a
//! deterministic string, no script, no CSS, nothing hidden, every colour a
//! `fill` or `stroke` attribute (Typst knows no CSS). Elements carry classes
//! (`seat`, `card`, `bet`, `pot`, `caption`) so a page can restyle them.
//!
//! The look is the one agreed on 19 October 2026 (`docs/EPIC-20_Bowery_mockup.html`):
//! black on white, 2 px strokes, red only for ♥ and ♦, cards as outlined
//! rectangles with rank over suit, hatched backs, the seat to act filled
//! yellow with a heavy outline, folded seats grey with their cards removed,
//! and bets in reserved slots, inverted for a bet or raise and outlined for a
//! call. Every box comes from [`geometry::plan`] first.
//!
//! ```
//! use bowery::check::check;
//! use bowery::config::{Config, Layout, View};
//! use bowery::draw::{Theme, svg};
//! use bowery::game::Game;
//!
//! let config = Config::from_yaml("
//! mode: play
//! table: { stakes: { small_blind: 1.0, big_blind: 2.0 } }
//! players: [ { seat: 0, stack: 200 }, { seat: 1, stack: 200, bot: abc } ]
//! ").unwrap();
//! let game = Game::new(check(&config).unwrap()).unwrap();
//! let svg = svg(&game.frame(View::All), Layout::Ring, 680, Theme::Light);
//! assert!(svg.starts_with("<svg"));
//! assert!(!svg.contains("<script"));
//! ```
//!
//! [`Frame`]: crate::frame::Frame

pub mod geometry;

use std::fmt;
use std::fmt::Write as _;

use pkcore::casino::action::PlayerAction;

use crate::config::Layout;
use crate::frame::{CardFace, Frame, SeatFrame, SeatStatus, format_chips};
use geometry::{CARD_H, CARD_W, MARGIN, Plan, Rect, SMALL_CARD_H, SMALL_CARD_W};

/// The font stack: the system's monospace.
const FONT: &str = "ui-monospace, Menlo, Consolas, monospace";
/// The longest name drawn; longer ones end in `…`.
const NAME_CHARS: usize = 16;

/// Light (black on white) or dark (white on black).
///
/// # Examples
///
/// ```
/// use bowery::draw::Theme;
///
/// assert_eq!(Theme::Light, Theme::default());
/// assert_eq!("light", Theme::Light.to_string());
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Theme {
    /// Black on white: the page and the book.
    #[default]
    Light,
    /// White on near-black.
    Dark,
}

impl fmt::Display for Theme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Light => "light",
            Self::Dark => "dark",
        })
    }
}

/// A theme's colours.
struct Palette {
    background: &'static str,
    ink: &'static str,
    red: &'static str,
    grey: &'static str,
    to_act: &'static str,
}

impl Theme {
    fn palette(self) -> Palette {
        match self {
            Self::Light => Palette {
                background: "#ffffff",
                ink: "#000000",
                red: "#c00000",
                grey: "#888888",
                to_act: "#ffe94d",
            },
            Self::Dark => Palette {
                background: "#111111",
                ink: "#f2f2f2",
                red: "#ff6b6b",
                grey: "#8a8a8a",
                to_act: "#5e5200",
            },
        }
    }
}

/// Draws `frame` in `layout` at `width` pixels as an SVG string.
///
/// The height follows from the layout and the seats. Ring falls back to
/// ledger below 520 px, or when its seats do not fit; a width below 320 is
/// drawn at 320. The same frame, layout, width and theme always give the
/// same string.
///
/// # Examples
///
/// ```
/// use bowery::check::check;
/// use bowery::config::{Config, Layout, View};
/// use bowery::draw::{Theme, svg};
/// use bowery::game::Game;
///
/// let config = Config::from_yaml("
/// mode: play
/// table: { stakes: { small_blind: 1.0, big_blind: 2.0 } }
/// players: [ { seat: 0, stack: 200 }, { seat: 1, stack: 200, bot: abc } ]
/// ").unwrap();
/// let frame = Game::new(check(&config).unwrap()).unwrap().frame(View::All);
/// let svg = svg(&frame, Layout::Ring, 400, Theme::Dark);
/// assert!(svg.contains(r#"data-layout="ledger""#));
/// ```
#[must_use]
pub fn svg(frame: &Frame, layout: Layout, width: u32, theme: Theme) -> String {
    let live: Vec<bool> = frame
        .seats
        .iter()
        .map(|seat| in_hand(seat.status))
        .collect();
    let folded = folded_names(frame);
    let plan = geometry::plan(layout, &live, &folded, width);
    let palette = theme.palette();
    let mut out = String::with_capacity(16 * 1024);
    open_svg(&mut out, frame, &plan, &palette);
    if let Some(rect) = plan.strip {
        draw_strip(
            &mut out,
            &geometry::strip_lines(&folded, plan.width),
            rect,
            plan.width,
            &palette,
        );
    }
    draw_board(&mut out, frame, plan.board, &palette);
    draw_pot_and_street(&mut out, frame, &plan, &palette);
    for (seat, boxes) in frame.seats.iter().zip(&plan.seats) {
        match (plan.layout, boxes.plate) {
            (Layout::Ledger, Some(plate)) => draw_row(&mut out, seat, plate, plan.width, &palette),
            (_, Some(plate)) => draw_plate(&mut out, seat, plate, &palette),
            (_, None) => {}
        }
        if let Some(slot) = boxes.bet {
            draw_bet(
                &mut out,
                seat,
                slot,
                plan.layout == Layout::Ledger,
                &palette,
            );
        }
    }
    let caption_chars = usize::try_from((plan.width - 2 * MARGIN) * 10 / 72).unwrap_or(NAME_CHARS);
    draw_text(
        &mut out,
        &Text {
            x: MARGIN,
            y: plan.caption.y + 14,
            size: 12,
            bold: false,
            anchor: "start",
            fill: palette.ink,
            class: "caption",
        },
        &cut(&frame.caption, caption_chars),
    );
    out.push_str("</svg>");
    out
}

/// The `<svg>` tag, the background, and ring's oval.
fn open_svg(out: &mut String, frame: &Frame, plan: &Plan, palette: &Palette) {
    let _ = write!(
        out,
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w} {h}" width="{w}" height="{h}" class="bowery" data-layout="{layout}" data-moment="{at}/{of}" role="img" aria-label="{label}" font-family="{FONT}">"#,
        w = plan.width,
        h = plan.height,
        layout = plan.layout,
        at = frame.moment.0,
        of = frame.moment.1,
        label = escape(&frame.caption),
    );
    let _ = write!(
        out,
        r#"<rect class="bg" x="0" y="0" width="{}" height="{}" fill="{}"/>"#,
        plan.width, plan.height, palette.background
    );
    if let Some(oval) = plan.oval {
        let _ = write!(
            out,
            r#"<rect class="oval" x="{}" y="{}" width="{}" height="{}" rx="{}" fill="none" stroke="{}" stroke-width="3"/>"#,
            oval.x,
            oval.y,
            oval.w,
            oval.h,
            oval.h / 2,
            palette.ink
        );
    }
}

/// The pot, and ledger's street label.
fn draw_pot_and_street(out: &mut String, frame: &Frame, plan: &Plan, palette: &Palette) {
    draw_text(
        out,
        &Text {
            x: plan.pot.x + plan.pot.w / 2,
            y: plan.pot.y + 17,
            size: 16,
            bold: true,
            anchor: "middle",
            fill: palette.ink,
            class: "pot",
        },
        &format!("POT {}", format_chips(frame.pot)),
    );
    if let Some(rect) = plan.street {
        draw_text(
            out,
            &Text {
                x: rect.right(),
                y: rect.y + 11,
                size: 11,
                bold: true,
                anchor: "end",
                fill: palette.ink,
                class: "street",
            },
            &frame.street.to_string().to_uppercase(),
        );
    }
}

fn in_hand(status: SeatStatus) -> bool {
    matches!(status, SeatStatus::Live | SeatStatus::AllIn)
}

/// The seats out of the hand, as spotlight's strip names them:
/// `Eli Elezra (SB)`.
fn folded_names(frame: &Frame) -> Vec<String> {
    frame
        .seats
        .iter()
        .filter(|seat| !in_hand(seat.status))
        .map(|seat| {
            let name = cut(&seat.name, NAME_CHARS);
            let marks = seat.marks.to_string();
            if marks.is_empty() {
                name
            } else {
                format!("{name} ({marks})")
            }
        })
        .collect()
}

/// How one piece of text is set.
struct Text {
    x: u32,
    y: u32,
    size: u32,
    bold: bool,
    anchor: &'static str,
    fill: &'static str,
    class: &'static str,
}

fn draw_text(out: &mut String, text: &Text, content: &str) {
    let weight = if text.bold {
        r#" font-weight="700""#
    } else {
        ""
    };
    let _ = write!(
        out,
        r#"<text class="{}" x="{}" y="{}" font-size="{}"{weight} text-anchor="{}" fill="{}">{}</text>"#,
        text.class,
        text.x,
        text.y,
        text.size,
        text.anchor,
        text.fill,
        escape(content)
    );
}

fn draw_rect(
    out: &mut String,
    rect: Rect,
    fill: &str,
    stroke: &str,
    stroke_width: u32,
    class: &str,
) {
    let _ = write!(
        out,
        r#"<rect class="{class}" x="{}" y="{}" width="{}" height="{}" fill="{fill}" stroke="{stroke}" stroke-width="{stroke_width}"/>"#,
        rect.x, rect.y, rect.w, rect.h
    );
}

/// Spotlight's strip: the folded names, one wrapped line after another.
fn draw_strip(out: &mut String, lines: &[String], rect: Rect, width: u32, palette: &Palette) {
    for (line, text) in (0u32..).zip(lines) {
        draw_text(
            out,
            &Text {
                x: MARGIN,
                y: rect.y + 11 + line * geometry::STRIP_LINE_H,
                size: 11,
                bold: false,
                anchor: "start",
                fill: palette.grey,
                class: "strip",
            },
            text,
        );
    }
    let _ = write!(
        out,
        r#"<line x1="0" y1="{y}" x2="{width}" y2="{y}" stroke="{}" stroke-width="1"/>"#,
        palette.ink,
        y = rect.bottom()
    );
}

fn draw_board(out: &mut String, frame: &Frame, rect: Rect, palette: &Palette) {
    out.push_str(r#"<g class="board">"#);
    for (index, card) in (0u32..).zip(frame.board) {
        let x = rect.x + index * (CARD_W + 3);
        match card {
            Some(card) => draw_card(out, x, rect.y, false, CardFace::Up(card), palette),
            None => {
                let _ = write!(
                    out,
                    r#"<rect class="slot" x="{x}" y="{}" width="{CARD_W}" height="{CARD_H}" rx="4" fill="none" stroke="{}" stroke-width="2" stroke-dasharray="4 3"/>"#,
                    rect.y, palette.grey
                );
            }
        }
    }
    out.push_str("</g>");
}

/// One card at `(x, y)`: board size, or small for a hole card.
fn draw_card(out: &mut String, x: u32, y: u32, small: bool, face: CardFace, palette: &Palette) {
    let (w, h, size) = if small {
        (SMALL_CARD_W, SMALL_CARD_H, 12)
    } else {
        (CARD_W, CARD_H, 15)
    };
    match face {
        CardFace::Up(card) => {
            let text = card.to_string();
            let suit: String = text.chars().last().map(String::from).unwrap_or_default();
            let rank = match text.trim_end_matches(suit.as_str()) {
                "T" => "10".to_string(),
                rank => rank.to_string(),
            };
            let red = matches!(suit.as_str(), "♥" | "♦");
            let colour = if red { palette.red } else { palette.ink };
            let _ = write!(
                out,
                r#"<g class="card{}" data-card="{text}">"#,
                if red { " red" } else { "" }
            );
            let _ = write!(
                out,
                r#"<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="3" fill="{}" stroke="{}" stroke-width="2"/>"#,
                palette.background, palette.ink
            );
            for (line, glyph, bold) in [(0, rank.as_str(), true), (1, suit.as_str(), false)] {
                let weight = if bold { r#" font-weight="700""# } else { "" };
                let _ = write!(
                    out,
                    r#"<text x="{}" y="{}" font-size="{size}"{weight} text-anchor="middle" fill="{colour}">{glyph}</text>"#,
                    x + w / 2,
                    y + h / 2 - 3 + line * (size + 1)
                );
            }
            out.push_str("</g>");
        }
        CardFace::Down => {
            out.push_str(r#"<g class="card back">"#);
            let _ = write!(
                out,
                r#"<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="3" fill="{}" stroke="{}" stroke-width="2"/>"#,
                palette.background, palette.ink
            );
            out.push_str(&hatch(
                Rect {
                    x: x + 3,
                    y: y + 3,
                    w: w - 6,
                    h: h - 6,
                },
                palette.ink,
            ));
            out.push_str("</g>");
        }
        CardFace::None => {}
    }
}

/// Diagonal lines filling `rect`, every 5 px: a card back.
fn hatch(rect: Rect, ink: &str) -> String {
    let mut path = String::new();
    let mut offset = 5;
    while offset < rect.w + rect.h {
        let (x1, y1) = (
            rect.x + offset.saturating_sub(rect.h),
            rect.y + offset.min(rect.h),
        );
        let (x2, y2) = (
            rect.x + offset.min(rect.w),
            rect.y + offset.saturating_sub(rect.w),
        );
        let _ = write!(path, "M{x1} {y1}L{x2} {y2}");
        offset += 5;
    }
    format!(r#"<path d="{path}" stroke="{ink}" stroke-width="1.5" fill="none"/>"#)
}

/// The classes of a seat: `seat`, and its status or turn.
fn seat_class(seat: &SeatFrame, base: &str) -> String {
    let mut class = base.to_string();
    match seat.status {
        SeatStatus::Folded => class.push_str(" folded"),
        SeatStatus::AllIn => class.push_str(" all-in"),
        SeatStatus::Out => class.push_str(" out"),
        SeatStatus::Live => {}
    }
    if seat.to_act {
        class.push_str(" to-act");
    }
    class
}

/// What the stack line says: the chips, or the seat's state when it has
/// none to show.
fn stack_text(seat: &SeatFrame) -> String {
    match seat.status {
        SeatStatus::Live => format_chips(seat.stack),
        status => status.to_string(),
    }
}

/// A ring or spotlight plate: name, stack and marks, two small cards.
fn draw_plate(out: &mut String, seat: &SeatFrame, plate: Rect, palette: &Palette) {
    let ink = if in_hand(seat.status) {
        palette.ink
    } else {
        palette.grey
    };
    let (fill, stroke_width) = if seat.to_act {
        (palette.to_act, 3)
    } else {
        (palette.background, 2)
    };
    let _ = write!(
        out,
        r#"<g class="{}" data-seat="{}">"#,
        seat_class(seat, "seat"),
        seat.seat
    );
    draw_rect(out, plate, fill, ink, stroke_width, "plate");
    let line = |x: u32, y: u32, bold: bool, anchor: &'static str, class: &'static str| Text {
        x,
        y,
        size: 11,
        bold,
        anchor,
        fill: ink,
        class,
    };
    draw_text(
        out,
        &line(plate.x + 4, plate.y + 14, true, "start", "name"),
        &cut(&seat.name, NAME_CHARS),
    );
    draw_text(
        out,
        &line(plate.x + 4, plate.y + 28, false, "start", "stack"),
        &stack_text(seat),
    );
    draw_text(
        out,
        &line(plate.right() - 4, plate.y + 28, true, "end", "marks"),
        &seat.marks.to_string(),
    );
    for (index, face) in (0u32..).zip(seat.cards) {
        draw_card(
            out,
            plate.x + 4 + index * (SMALL_CARD_W + 3),
            plate.y + 31,
            true,
            face,
            palette,
        );
    }
    out.push_str("</g>");
}

/// A ledger row: mark, name, cards and stack; a rule underneath.
fn draw_row(out: &mut String, seat: &SeatFrame, plate: Rect, width: u32, palette: &Palette) {
    let ink = if in_hand(seat.status) {
        palette.ink
    } else {
        palette.grey
    };
    let wide = width >= geometry::WIDE;
    let _ = write!(
        out,
        r#"<g class="{}" data-seat="{}">"#,
        seat_class(seat, "seat row"),
        seat.seat
    );
    if seat.to_act {
        let _ = write!(
            out,
            r#"<rect class="plate" x="0" y="{}" width="{width}" height="{}" fill="{}"/>"#,
            plate.y, plate.h, palette.to_act
        );
    }
    let mark = [
        (seat.marks.dealer, "D"),
        (seat.marks.small_blind, "SB"),
        (seat.marks.big_blind, "BB"),
    ]
    .into_iter()
    .find_map(|(on, mark)| on.then_some(mark))
    .unwrap_or("");
    let (name_y, cards_y, stack_y) = if wide {
        (plate.y + 24, plate.y + 3, plate.y + 24)
    } else {
        (plate.y + 15, plate.y + 20, plate.y + 44)
    };
    let text = |x: u32, y: u32, bold: bool, anchor: &'static str, class: &'static str| Text {
        x,
        y,
        size: 11,
        bold,
        anchor,
        fill: ink,
        class,
    };
    draw_text(
        out,
        &text(plate.x + 6, name_y, true, "start", "marks"),
        mark,
    );
    draw_text(
        out,
        &text(plate.x + 32, name_y, true, "start", "name"),
        &cut(&seat.name, NAME_CHARS),
    );
    let cards_x = if wide {
        plate.x + 32 + 112 + 4
    } else {
        plate.x + 32
    };
    for (index, face) in (0u32..).zip(seat.cards) {
        draw_card(
            out,
            cards_x + index * (SMALL_CARD_W + 3),
            cards_y,
            true,
            face,
            palette,
        );
    }
    let stack = if seat.status == SeatStatus::AllIn {
        stack_text(seat)
    } else {
        format_chips(seat.stack)
    };
    draw_text(
        out,
        &text(plate.right() - 4, stack_y, false, "end", "stack"),
        &stack,
    );
    let _ = write!(
        out,
        r#"<line x1="0" y1="{y}" x2="{width}" y2="{y}" stroke="{ink}" stroke-width="1"/>"#,
        y = plate.bottom()
    );
    out.push_str("</g>");
}

/// The bet slot: chips inverted for a bet or raise, outlined for a call or
/// a blind; `check` outlined. Ledger names the action too, and marks the
/// seat to act and a fold.
fn draw_bet(out: &mut String, seat: &SeatFrame, slot: Rect, ledger: bool, palette: &Palette) {
    let raised = matches!(
        seat.last,
        Some(PlayerAction::Bet(_) | PlayerAction::Raise(_) | PlayerAction::AllIn)
    );
    let verb = match seat.last {
        Some(PlayerAction::Bet(_)) => "BET",
        Some(PlayerAction::Raise(_)) => "RAISE",
        Some(PlayerAction::AllIn) => "ALL IN",
        Some(PlayerAction::Call) => "CALL",
        _ => "POST",
    };
    let (label, inverted, class) = if seat.bet > 0 {
        let chips = format_chips(seat.bet);
        let label = if ledger {
            format!("{verb} {chips}")
        } else {
            chips
        };
        (label, raised, if raised { "bet raise" } else { "bet call" })
    } else if seat.last == Some(PlayerAction::Check) {
        ("check".to_string(), false, "bet check")
    } else if ledger && seat.status == SeatStatus::Folded {
        draw_text(
            out,
            &Text {
                x: slot.right(),
                y: slot.y + 13,
                size: 11,
                bold: false,
                anchor: "end",
                fill: palette.grey,
                class: "bet fold",
            },
            "fold",
        );
        return;
    } else if ledger && seat.to_act {
        ("TO ACT".to_string(), false, "bet to-act")
    } else {
        return;
    };
    let (fill, ink) = if inverted {
        (palette.ink, palette.background)
    } else {
        (palette.background, palette.ink)
    };
    let _ = write!(out, r#"<g class="{class}" data-seat="{}">"#, seat.seat);
    draw_rect(out, slot, fill, palette.ink, 2, "slot");
    draw_text(
        out,
        &Text {
            x: slot.x + slot.w / 2,
            y: slot.y + 13,
            size: 11,
            bold: true,
            anchor: "middle",
            fill: ink,
            class: "chips",
        },
        &label,
    );
    out.push_str("</g>");
}

/// `text` cut to `chars` characters, the last one `…` when cut.
fn cut(text: &str, chars: usize) -> String {
    if text.chars().count() <= chars {
        return text.to_string();
    }
    let mut cut: String = text.chars().take(chars.saturating_sub(1)).collect();
    cut.push('…');
    cut
}

/// `text` safe inside XML text and attributes.
fn escape(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            other => escaped.push(other),
        }
    }
    escaped
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use super::*;

    #[test]
    fn cut__keeps_short_text() {
        assert_eq!("Gus Hansen", cut("Gus Hansen", 16));
    }

    #[test]
    fn cut__ends_long_text_in_an_ellipsis() {
        assert_eq!("abcd…", cut("abcdefgh", 5));
    }

    #[test]
    fn escape__every_special_character() {
        assert_eq!(
            "&lt;a href=&quot;x&quot;&gt;&amp;&apos;",
            escape(r#"<a href="x">&'"#)
        );
    }

    #[test]
    fn hatch__stays_inside_its_box() {
        let path = hatch(
            Rect {
                x: 10,
                y: 10,
                w: 18,
                h: 28,
            },
            "#000",
        );
        for number in path
            .split(|character: char| !character.is_ascii_digit())
            .filter(|part| !part.is_empty())
            .filter_map(|part| part.parse::<u32>().ok())
            .filter(|number| *number != 1 && *number != 5 && *number != 0)
        {
            assert!((10..=38).contains(&number), "{number} in {path}");
        }
    }

    #[test]
    fn palette__dark_differs_from_light() {
        assert_ne!(
            Theme::Light.palette().background,
            Theme::Dark.palette().background
        );
    }
}
