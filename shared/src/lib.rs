#[macro_use]
extern crate bitflags;
extern crate csv;
extern crate irc;
extern crate parking_lot;
extern crate regex;
extern crate reqwest;
extern crate rusqlite;
extern crate serde_json;
extern crate serenity;

pub mod error;
pub mod types;

pub mod prelude {
    pub use error::*;
    pub use types::Prefix::*;
    pub use types::Source::*;
    pub use types::*;
}
