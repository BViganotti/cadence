//! Cadence MusicXML parser and writer.
//!
//! The parser is original, dependency free and bounded: it rejects input that
//! exceeds configured limits instead of recursing without bound.

pub mod musicxml;
pub mod xml;
