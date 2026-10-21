//! # bowery
//!
//! One poker table control, configured not compiled. `bowery` will put a
//! poker table in any web page as `<bowery-table>`: a strict replay of a
//! recorded hand, an interactive replay where the reader takes over a seat,
//! or a table against pkcore's bots, all driven by a YAML config.
//!
//! The crate is freshly scaffolded. The kernel modules (`config`, `check`,
//! `game`, `frame`, `draw`) and the `wasm` feature arrive in the phases of
//! bower's EPIC-20.
//!
//! ## Quick start
//!
//! ```
//! assert_eq!(env!("CARGO_PKG_VERSION"), bowery::version());
//! ```

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
