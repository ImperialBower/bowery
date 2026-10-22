//! The JavaScript surface: one `BoweryTable` per `<bowery-table>` element.
//!
//! The only wasm-bindgen code in the crate. Every method wraps
//! [`crate::element::Table`] and turns its errors into JavaScript strings;
//! a config's problems come back as the JSON of
//! [`crate::element::problems_json`]. Each element constructs its own
//! table from the one shared module, and a table owns all of its state, so
//! many tables on a page are independent.
//!
//! ```
//! use bowery::wasm::BoweryTable;
//!
//! let table = BoweryTable::new("
//! mode: play
//! table: { stakes: { small_blind: 1.0, big_blind: 2.0 } }
//! players: [ { seat: 0, stack: 200 }, { seat: 1, stack: 200, bot: abc } ]
//! ").unwrap();
//! assert!(table.svg(680, false).starts_with("<svg"));
//! ```

use wasm_bindgen::prelude::*;

use crate::element::{Table, problems_json};

/// One element's table, for JavaScript.
///
/// # Examples
///
/// ```
/// use bowery::wasm::BoweryTable;
///
/// let table = BoweryTable::new("
/// mode: play
/// table: { stakes: { small_blind: 1.0, big_blind: 2.0 } }
/// players: [ { seat: 0, stack: 200 }, { seat: 1, stack: 200, bot: abc } ]
/// ").unwrap();
/// assert!(table.state().contains(r#""mode":"play""#));
/// ```
#[wasm_bindgen]
#[derive(Debug)]
pub struct BoweryTable {
    table: Table,
}

#[wasm_bindgen]
impl BoweryTable {
    /// Reads, checks and builds a table from a YAML config.
    ///
    /// # Errors
    ///
    /// The config's problems as a JSON string (see
    /// [`crate::element::problems_json`]).
    ///
    /// # Examples
    ///
    /// ```
    /// use bowery::wasm::BoweryTable;
    ///
    /// let table = BoweryTable::new("
    /// mode: play
    /// table: { stakes: { small_blind: 1.0, big_blind: 2.0 } }
    /// players: [ { seat: 0, stack: 200 }, { seat: 1, stack: 200, bot: abc } ]
    /// ");
    /// assert!(table.is_ok());
    /// ```
    #[wasm_bindgen(constructor)]
    pub fn new(config_yaml: &str) -> Result<BoweryTable, JsValue> {
        Table::new(config_yaml)
            .map(|table| Self { table })
            .map_err(|problems| JsValue::from_str(&problems_json(&problems)))
    }

    /// The table on screen as SVG, `width` pixels wide, dark or light.
    ///
    /// # Examples
    ///
    /// ```
    /// use bowery::wasm::BoweryTable;
    ///
    /// let table = BoweryTable::new("
    /// mode: play
    /// table: { stakes: { small_blind: 1.0, big_blind: 2.0 } }
    /// players: [ { seat: 0, stack: 200 }, { seat: 1, stack: 200, bot: abc } ]
    /// ").unwrap();
    /// assert!(table.svg(400, true).contains(r#"width="400""#));
    /// ```
    #[must_use]
    pub fn svg(&self, width: u32, dark: bool) -> String {
        self.table.svg(width, dark)
    }

    /// The controls' state as JSON (see [`crate::element::State`]).
    ///
    /// # Examples
    ///
    /// ```
    /// use bowery::wasm::BoweryTable;
    ///
    /// let table = BoweryTable::new("
    /// mode: play
    /// table: { stakes: { small_blind: 1.0, big_blind: 2.0 } }
    /// players: [ { seat: 0, stack: 200 }, { seat: 1, stack: 200, bot: abc } ]
    /// ").unwrap();
    /// assert!(table.state().contains(r#""moment":1"#));
    /// ```
    #[must_use]
    pub fn state(&self) -> String {
        self.table.state_json()
    }

    /// Moves to moment `index`, from 0; past the end, to the last.
    ///
    /// # Examples
    ///
    /// ```
    /// use bowery::wasm::BoweryTable;
    ///
    /// let mut table = BoweryTable::new("
    /// mode: play
    /// table: { stakes: { small_blind: 1.0, big_blind: 2.0 } }
    /// players: [ { seat: 0, stack: 200 }, { seat: 1, stack: 200, bot: abc } ]
    /// ").unwrap();
    /// table.seek(0);
    /// assert!(table.state().contains(r#""moment":1"#));
    /// ```
    pub fn seek(&mut self, index: usize) {
        self.table.seek(index);
    }

    /// The reader takes `seat` (interactive).
    ///
    /// # Errors
    ///
    /// The game's reason, as a string.
    ///
    /// # Examples
    ///
    /// ```
    /// use bowery::wasm::BoweryTable;
    ///
    /// let mut table = BoweryTable::new("
    /// mode: interactive
    /// hand:
    ///   hand: { id: ex, game: holdem }
    ///   table: { seats: 2, button: 0, stakes: { small_blind: 1.0, big_blind: 2.0 } }
    ///   players: [ { seat: 0, name: A, stack: 100.0 }, { seat: 1, name: B, stack: 100.0 } ]
    ///   streets: { preflop: { actions: [ { seat: 0, action: fold } ] } }
    /// ").unwrap();
    /// assert!(table.take_seat(0).is_ok());
    /// ```
    pub fn take_seat(&mut self, seat: u8) -> Result<(), JsValue> {
        self.table
            .take_seat(seat)
            .map_err(|error| JsValue::from_str(&error))
    }

    /// Back to the record at the takeover moment.
    ///
    /// # Examples
    ///
    /// ```
    /// use bowery::wasm::BoweryTable;
    ///
    /// let mut table = BoweryTable::new("
    /// mode: play
    /// table: { stakes: { small_blind: 1.0, big_blind: 2.0 } }
    /// players: [ { seat: 0, stack: 200 }, { seat: 1, stack: 200, bot: abc } ]
    /// ").unwrap();
    /// table.back_to_record();
    /// assert!(table.state().contains(r#""record":"none""#));
    /// ```
    pub fn back_to_record(&mut self) {
        self.table.back_to_record();
    }

    /// The reader acts: `{"action":"raise","amount":600}`.
    ///
    /// # Errors
    ///
    /// The JSON does not read, or the game refuses the action.
    ///
    /// # Examples
    ///
    /// ```
    /// use bowery::wasm::BoweryTable;
    ///
    /// let mut table = BoweryTable::new("
    /// mode: play
    /// table: { button: 0, stakes: { small_blind: 1.0, big_blind: 2.0 } }
    /// players: [ { seat: 0, stack: 200 }, { seat: 1, stack: 200, bot: abc } ]
    /// ").unwrap();
    /// assert!(table.act(r#"{"action":"call"}"#).is_ok());
    /// ```
    pub fn act(&mut self, action_json: &str) -> Result<(), JsValue> {
        self.table
            .act(action_json)
            .map_err(|error| JsValue::from_str(&error))
    }

    /// One step: `"acted"`, `"dealt"`, `"waiting"` or `"over"`.
    ///
    /// # Errors
    ///
    /// The engine failed.
    ///
    /// # Examples
    ///
    /// ```
    /// use bowery::wasm::BoweryTable;
    ///
    /// let mut table = BoweryTable::new("
    /// mode: play
    /// table: { button: 0, stakes: { small_blind: 1.0, big_blind: 2.0 } }
    /// players: [ { seat: 0, stack: 200 }, { seat: 1, stack: 200, bot: abc } ]
    /// ").unwrap();
    /// assert_eq!("waiting", table.advance().unwrap());
    /// ```
    pub fn advance(&mut self) -> Result<String, JsValue> {
        self.table
            .advance()
            .map(String::from)
            .map_err(|error| JsValue::from_str(&error))
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
    /// use bowery::wasm::BoweryTable;
    ///
    /// let mut table = BoweryTable::new("
    /// mode: play
    /// table: { button: 0, stakes: { small_blind: 1.0, big_blind: 2.0 } }
    /// players: [ { seat: 0, stack: 200 }, { seat: 1, stack: 200, bot: abc } ]
    /// ").unwrap();
    /// table.act(r#"{"action":"fold"}"#).unwrap();
    /// while table.advance().unwrap() != "over" {}
    /// assert!(table.next_hand().is_ok());
    /// ```
    pub fn next_hand(&mut self) -> Result<(), JsValue> {
        self.table
            .next_hand()
            .map_err(|error| JsValue::from_str(&error))
    }

    /// Play: a new game seeded with `seed`.
    ///
    /// # Errors
    ///
    /// Not play mode.
    ///
    /// # Examples
    ///
    /// ```
    /// use bowery::wasm::BoweryTable;
    ///
    /// let mut table = BoweryTable::new("
    /// mode: play
    /// table: { stakes: { small_blind: 1.0, big_blind: 2.0 } }
    /// players: [ { seat: 0, stack: 200 }, { seat: 1, stack: 200, bot: abc } ]
    /// ").unwrap();
    /// assert!(table.shuffle(9).is_ok());
    /// ```
    pub fn shuffle(&mut self, seed: u32) -> Result<(), JsValue> {
        self.table
            .shuffle(u64::from(seed))
            .map_err(|error| JsValue::from_str(&error))
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
    /// use bowery::wasm::BoweryTable;
    ///
    /// let mut table = BoweryTable::new("
    /// mode: play
    /// table: { stakes: { small_blind: 1.0, big_blind: 2.0 } }
    /// players: [ { seat: 0, stack: 200 }, { seat: 1, stack: 200, bot: abc } ]
    /// ").unwrap();
    /// assert!(table.set_view("all").is_ok());
    /// ```
    pub fn set_view(&mut self, view: &str) -> Result<(), JsValue> {
        self.table
            .set_view(view)
            .map_err(|error| JsValue::from_str(&error))
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
    /// use bowery::wasm::BoweryTable;
    ///
    /// let mut table = BoweryTable::new("
    /// mode: play
    /// table: { stakes: { small_blind: 1.0, big_blind: 2.0 } }
    /// players: [ { seat: 0, stack: 200 }, { seat: 1, stack: 200, bot: abc } ]
    /// ").unwrap();
    /// assert!(table.set_layout("ledger").is_ok());
    /// ```
    pub fn set_layout(&mut self, layout: &str) -> Result<(), JsValue> {
        self.table
            .set_layout(layout)
            .map_err(|error| JsValue::from_str(&error))
    }

    /// This crate's version.
    ///
    /// # Examples
    ///
    /// ```
    /// use bowery::wasm::BoweryTable;
    ///
    /// assert_eq!(env!("CARGO_PKG_VERSION"), BoweryTable::version());
    /// ```
    #[must_use]
    pub fn version() -> String {
        crate::element::version().to_string()
    }
}
