//! Cadence core: score model, notation types and validation.

pub mod error;
pub mod notation;
pub mod score;

pub use error::{Error, Result};
pub use score::*;
