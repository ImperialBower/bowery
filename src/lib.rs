//! # bowery
//!
//! One poker table control, configured not compiled. `bowery` will put a
//! poker table in any web page as `<bowery-table>`: a strict replay of a
//! recorded hand, an interactive replay where the reader takes over a seat,
//! or a table against pkcore's bots, all driven by a YAML config.
//!
//! ## Modules
//!
//! - [`config`]: what the author can say about a table, read from YAML.
//! - [`check`]: refusing a config that is not true. The recorded hand is
//!   checked by pkcore; a [`check::Checked`] config is the only kind a game
//!   can be built from.
//! - [`game`]: one table's game, built from a checked config: the record,
//!   taking a seat and going off it, play against pkcore's bots, and every
//!   hand's end as separate moments.
//! - [`frame`]: what is on screen at one moment, with the view applied, so a
//!   hidden card is never in it.
//! - [`draw`]: the one renderer, a frame as SVG in three layouts, with every
//!   box placed by [`draw::geometry`] before anything is drawn.
//!
//! Behind the `wasm` feature (off by default, so the rlib has no
//! wasm-bindgen in its tree):
//!
//! - `element`: what `<bowery-table>` asks of a table, in plain Rust, with
//!   its state as JSON.
//! - `wasm`: the `BoweryTable` class JavaScript constructs, one per element.
//!
//! `js/bowery.js` and `js/bowery.css` are the element; `make site` builds the
//! demo (`demo/index.html`) and `make serve` serves it.
//!
//! ## Quick start
//!
//! ```
//! use bowery::check::check;
//! use bowery::config::{Config, Mode};
//!
//! let config = Config::from_yaml("
//! mode: play
//! table: { seats: 6, stakes: { small_blind: 1.0, big_blind: 2.0 } }
//! players:
//!   - { seat: 0, name: Reader, stack: 200 }
//!   - { seat: 3, name: Robo, stack: 200, bot: tight_aggressive }
//! ").unwrap();
//!
//! let checked = check(&config).unwrap();
//! assert_eq!(Mode::Play, checked.config().mode);
//!
//! // A config that is not true never gets that far.
//! let typo = Config::from_yaml("mode: play\nplayers: [ { seat: 0, stack: 200, bot: shark } ]\n").unwrap();
//! assert!(check(&typo).is_err());
//! ```

pub mod check;
pub mod config;
pub mod draw;
#[cfg(feature = "wasm")]
pub mod element;
pub mod frame;
pub mod game;
#[cfg(feature = "wasm")]
pub mod wasm;

/// Returns the version of this crate, as `Cargo.toml` declares it.
///
/// # Examples
///
/// ```
/// let version = bowery::version();
/// assert!(!version.is_empty());
/// ```
#[must_use]
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_matches_the_manifest() {
        assert_eq!(env!("CARGO_PKG_VERSION"), version());
    }
}
