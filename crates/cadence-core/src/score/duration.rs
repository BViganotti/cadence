//! Written durations, dots and tick conversion.

use crate::error::{Error, Result};
use std::fmt;

/// Written note value. The denominator is relative to a whole note.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum NoteValue {
    Whole,
    Half,
    Quarter,
    Eighth,
    Sixteenth,
    ThirtySecond,
    SixtyFourth,
    HundredTwentyEighth,
}

impl NoteValue {
    pub const ALL: [NoteValue; 8] = [
        NoteValue::Whole,
        NoteValue::Half,
        NoteValue::Quarter,
        NoteValue::Eighth,
        NoteValue::Sixteenth,
        NoteValue::ThirtySecond,
        NoteValue::SixtyFourth,
        NoteValue::HundredTwentyEighth,
    ];

    pub const fn denominator(self) -> u32 {
        match self {
            NoteValue::Whole => 1,
            NoteValue::Half => 2,
            NoteValue::Quarter => 4,
            NoteValue::Eighth => 8,
            NoteValue::Sixteenth => 16,
            NoteValue::ThirtySecond => 32,
            NoteValue::SixtyFourth => 64,
            NoteValue::HundredTwentyEighth => 128,
        }
    }

    pub fn from_denominator(denominator: u32) -> Option<NoteValue> {
        Some(match denominator {
            1 => NoteValue::Whole,
            2 => NoteValue::Half,
            4 => NoteValue::Quarter,
            8 => NoteValue::Eighth,
            16 => NoteValue::Sixteenth,
            32 => NoteValue::ThirtySecond,
            64 => NoteValue::SixtyFourth,
            128 => NoteValue::HundredTwentyEighth,
            _ => return None,
        })
    }

    /// Number of flags used by an unbeamed, non-whole note.
    pub const fn flags(self) -> u8 {
        match self {
            NoteValue::Whole | NoteValue::Half => 0,
            NoteValue::Quarter => 0,
            NoteValue::Eighth => 1,
            NoteValue::Sixteenth => 2,
            NoteValue::ThirtySecond => 3,
            NoteValue::SixtyFourth => 4,
            NoteValue::HundredTwentyEighth => 5,
        }
    }

    pub const fn has_stem(self) -> bool {
        !matches!(self, NoteValue::Whole)
    }

    pub fn musicxml_type(self) -> &'static str {
        match self {
            NoteValue::Whole => "whole",
            NoteValue::Half => "half",
            NoteValue::Quarter => "quarter",
            NoteValue::Eighth => "eighth",
            NoteValue::Sixteenth => "16th",
            NoteValue::ThirtySecond => "32nd",
            NoteValue::SixtyFourth => "64th",
            NoteValue::HundredTwentyEighth => "128th",
        }
    }

    pub fn from_musicxml_type(text: &str) -> Option<NoteValue> {
        Some(match text.trim() {
            "whole" => NoteValue::Whole,
            "half" => NoteValue::Half,
            "quarter" => NoteValue::Quarter,
            "eighth" => NoteValue::Eighth,
            "16th" => NoteValue::Sixteenth,
            "32nd" => NoteValue::ThirtySecond,
            "64th" => NoteValue::SixtyFourth,
            "128th" => NoteValue::HundredTwentyEighth,
            _ => return None,
        })
    }

    pub fn label(self) -> &'static str {
        match self {
            NoteValue::Whole => "whole",
            NoteValue::Half => "half",
            NoteValue::Quarter => "quarter",
            NoteValue::Eighth => "eighth",
            NoteValue::Sixteenth => "16th",
            NoteValue::ThirtySecond => "32nd",
            NoteValue::SixtyFourth => "64th",
            NoteValue::HundredTwentyEighth => "128th",
        }
    }
}

/// A written duration: note value plus zero to three augmentation dots.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Duration {
    pub value: NoteValue,
    pub dots: u8,
}

impl Duration {
    pub const MAX_DOTS: u8 = 3;

    pub const fn new(value: NoteValue, dots: u8) -> Duration {
        Duration { value, dots }
    }

    pub const fn whole() -> Duration {
        Duration::new(NoteValue::Whole, 0)
    }

    pub const fn half() -> Duration {
        Duration::new(NoteValue::Half, 0)
    }

    pub const fn quarter() -> Duration {
        Duration::new(NoteValue::Quarter, 0)
    }

    pub const fn eighth() -> Duration {
        Duration::new(NoteValue::Eighth, 0)
    }

    pub const fn sixteenth() -> Duration {
        Duration::new(NoteValue::Sixteenth, 0)
    }

    pub const fn dotted(value: NoteValue) -> Duration {
        Duration::new(value, 1)
    }

    pub fn validate(self) -> Result<()> {
        if self.dots > Self::MAX_DOTS {
            return Err(Error::out_of_range(format!(
                "{} dots on {:?}",
                self.dots,
                self.value
            )));
        }
        Ok(())
    }

    /// Dot multiplier numerator/denominator: 1, 3/2, 7/4, 15/8.
    pub const fn dot_factor(self) -> (u64, u64) {
        match self.dots {
            0 => (1, 1),
            1 => (3, 2),
            2 => (7, 4),
            3 => (15, 8),
            _ => (1, 1),
        }
    }

    /// Duration in divisions per quarter note, rounded to nearest integer.
    pub fn ticks(self, divisions: u32) -> u64 {
        let denominator = self.value.denominator() as u64;
        let (num, den) = self.dot_factor();
        let numerator = divisions as u64 * 4 * num;
        let divisor = denominator * den;
        if divisor == 0 {
            return 0;
        }
        (numerator + divisor / 2) / divisor
    }

    /// Exact tick duration, or `None` when the divisions do not represent the
    /// written value exactly. Import uses this to reject inconsistent files.
    pub fn ticks_exact(self, divisions: u32) -> Option<u64> {
        let denominator = self.value.denominator() as u64;
        let (num, den) = self.dot_factor();
        let numerator = divisions as u64 * 4 * num;
        let divisor = denominator * den;
        if divisor == 0 || numerator % divisor != 0 {
            None
        } else {
            Some(numerator / divisor)
        }
    }

    /// Length in quarter-note beats (1.0 = quarter).
    pub fn beats(self) -> f64 {
        let (num, den) = self.dot_factor();
        (4.0 / self.value.denominator() as f64) * (num as f64 / den as f64)
    }

    /// Choose the simplest written duration for an exact tick count.
    ///
    /// Returns `None` when the tick count is not representable with supported
    /// values and up to three dots.
    pub fn from_ticks(divisions: u32, ticks: u64) -> Option<Duration> {
        if divisions == 0 || ticks == 0 {
            return None;
        }
        for value in NoteValue::ALL {
            for dots in 0..=Self::MAX_DOTS {
                let duration = Duration::new(value, dots);
                if duration.ticks_exact(divisions) == Some(ticks) {
                    return Some(duration);
                }
            }
        }
        None
    }

    pub fn is_dotted(self) -> bool {
        self.dots > 0
    }

    pub fn is_compound(self) -> bool {
        matches!(
            self.value,
            NoteValue::Eighth
                | NoteValue::Sixteenth
                | NoteValue::ThirtySecond
                | NoteValue::SixtyFourth
                | NoteValue::HundredTwentyEighth
        )
    }
}

impl fmt::Display for Duration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.value.label())?;
        for _ in 0..self.dots {
            f.write_str(".")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_conversion_is_exact_for_common_divisions() {
        let d = 480;
        assert_eq!(Duration::quarter().ticks(d), 480);
        assert_eq!(Duration::half().ticks(d), 960);
        assert_eq!(Duration::whole().ticks(d), 1920);
        assert_eq!(Duration::eighth().ticks(d), 240);
        assert_eq!(Duration::dotted(NoteValue::Quarter).ticks(d), 720);
        assert_eq!(
            Duration::new(NoteValue::HundredTwentyEighth, 0).ticks(d),
            15
        );
    }

    #[test]
    fn exact_ticks_reject_inexact_divisions() {
        assert_eq!(Duration::quarter().ticks_exact(3), Some(3));
        assert_eq!(
            Duration::new(NoteValue::HundredTwentyEighth, 0).ticks_exact(3),
            None
        );
    }

    #[test]
    fn from_ticks_round_trips() {
        let d = 480;
        for value in NoteValue::ALL {
            for dots in 0..=3 {
                let duration = Duration::new(value, dots);
                if duration.ticks_exact(d).is_none() {
                    continue;
                }
                assert_eq!(
                    Duration::from_ticks(d, duration.ticks(d)),
                    Some(duration),
                    "{duration}"
                );
            }
        }
    }

    #[test]
    fn beats_are_quarter_relative() {
        assert_eq!(Duration::quarter().beats(), 1.0);
        assert_eq!(Duration::half().beats(), 2.0);
        assert_eq!(Duration::dotted(NoteValue::Half).beats(), 3.0);
    }

    #[test]
    fn musicxml_names_round_trip() {
        for value in NoteValue::ALL {
            assert_eq!(
                NoteValue::from_musicxml_type(value.musicxml_type()),
                Some(value)
            );
        }
        assert_eq!(NoteValue::from_musicxml_type("breve"), None);
    }
}
