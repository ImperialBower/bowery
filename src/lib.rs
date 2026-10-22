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
//!
//! The rest of the kernel (`game`, `frame`, `draw`) and the `wasm` feature
//! arrive in the later phases of bower's EPIC-20.
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
