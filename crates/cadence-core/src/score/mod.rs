//! Score model for Cadence.
//!
//! The model is deliberately explicit: written durations, resolved measure
//! attributes, and note-level ties, slurs and lyrics. Nothing in this module
//! performs I/O or depends on the environment.

pub mod attributes;
pub mod demo;
pub mod duration;
pub mod marks;
pub mod measure;
pub mod note;
pub mod part;
pub mod pitch;
pub mod score;

pub use attributes::{
    Attributes, Clef, ClefSign, Key, KeyMode, TimeSignature, TimeSymbol,
};
pub use duration::{Duration, NoteValue};
pub use marks::{
    Accidental, Articulation, Beam, BeamKind, Dynamic, Lyric, SlurKind, SlurMark, Syllabic, Tie,
};
pub use measure::{
    BarlineStyle, Ending, EndingKind, Measure, Repeat, RepeatDirection, Tempo, Voice,
};
pub use note::{Event, EventKind, Note};
pub use part::{measure_span_ticks, Part};
pub use pitch::{Pitch, Step};
pub use score::{
    event_at, measure_with_attributes, renumber_measures, validate_score, FlatNote, Issue,
    Metadata, Score, Severity, ValidationReport,
};
