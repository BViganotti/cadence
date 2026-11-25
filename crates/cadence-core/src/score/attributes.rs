//! Key, time and clef attributes.

use super::marks::Accidental;
use super::pitch::Step;
use crate::error::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum KeyMode {
    #[default]
    Major,
    Minor,
}

impl KeyMode {
    pub const fn musicxml_name(self) -> &'static str {
        match self {
            KeyMode::Major => "major",
            KeyMode::Minor => "minor",
        }
    }

    pub fn from_musicxml_name(text: &str) -> KeyMode {
        if text.trim().eq_ignore_ascii_case("minor") {
            KeyMode::Minor
        } else {
            KeyMode::Major
        }
    }
}

/// Circle-of-fifths key signature, from seven flats to seven sharps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Key {
    pub fifths: i8,
    pub mode: KeyMode,
}

impl Default for Key {
    fn default() -> Self {
        Key {
            fifths: 0,
            mode: KeyMode::Major,
        }
    }
}

impl Key {
    pub const MIN_FIFTHS: i8 = -7;
    pub const MAX_FIFTHS: i8 = 7;

    pub const fn new(fifths: i8, mode: KeyMode) -> Key {
        Key { fifths, mode }
    }

    pub fn c_major() -> Key {
        Key::default()
    }

    pub fn validate(self) -> Result<()> {
        if self.fifths < Self::MIN_FIFTHS || self.fifths > Self::MAX_FIFTHS {
            return Err(Error::out_of_range(format!("key fifths {}", self.fifths)));
        }
        Ok(())
    }

    /// Tonic step and alteration implied by the key signature.
    pub fn tonic(self) -> (Step, i8) {
        let major_tonics: [(Step, i8); 15] = [
            (Step::C, -1),
            (Step::G, -1),
            (Step::D, -1),
            (Step::A, -1),
            (Step::E, -1),
            (Step::B, -1),
            (Step::F, 0),
            (Step::C, 0),
            (Step::G, 0),
            (Step::D, 0),
            (Step::A, 0),
            (Step::E, 0),
            (Step::B, 0),
            (Step::F, 1),
            (Step::C, 1),
        ];
        let index = (self.fifths + 7).clamp(0, 14) as usize;
        let (step, alter) = major_tonics[index];
        if self.mode == KeyMode::Minor {
            // Relative minor: down a minor third (three diatonic steps).
            let index = step.index() as i32 - 2;
            let step = Step::from_index(index.rem_euclid(7)).unwrap_or(Step::A);
            (step, alter)
        } else {
            (step, alter)
        }
    }

    /// Ordered accidentals of the key signature.
    pub fn accidentals(self) -> Vec<(Step, Accidental)> {
        const SHARP_ORDER: [Step; 7] = [
            Step::F,
            Step::C,
            Step::G,
            Step::D,
            Step::A,
            Step::E,
            Step::B,
        ];
        const FLAT_ORDER: [Step; 7] = [
            Step::B,
            Step::E,
            Step::A,
            Step::D,
            Step::G,
            Step::C,
            Step::F,
        ];
        let count = self.fifths.unsigned_abs().min(7) as usize;
        if self.fifths >= 0 {
            SHARP_ORDER[..count]
                .iter()
                .map(|step| (*step, Accidental::Sharp))
                .collect()
        } else {
            FLAT_ORDER[..count]
                .iter()
                .map(|step| (*step, Accidental::Flat))
                .collect()
        }
    }

    /// Alteration the key signature applies to a step, in -1/0/1.
    pub fn alter_for(self, step: Step) -> i8 {
        if self.fifths >= 0 {
            const SHARPS: [Step; 7] = [
                Step::F,
                Step::C,
                Step::G,
                Step::D,
                Step::A,
                Step::E,
                Step::B,
            ];
            if SHARPS[..self.fifths.max(0) as usize]
                .iter()
                .any(|candidate| *candidate == step)
            {
                1
            } else {
                0
            }
        } else {
            const FLATS: [Step; 7] = [
                Step::B,
                Step::E,
                Step::A,
                Step::D,
                Step::G,
                Step::C,
                Step::F,
            ];
            if FLATS[..(-self.fifths) as usize]
                .iter()
                .any(|candidate| *candidate == step)
            {
                -1
            } else {
                0
            }
        }
    }

    pub fn prefer_flats(self) -> bool {
        self.fifths < 0
    }

    pub fn describe(self) -> String {
        let (step, alter) = self.tonic();
        let accidental = match alter {
            1 => "#",
            -1 => "b",
            _ => "",
        };
        format!(
            "{}{} {}",
            step.name(),
            accidental,
            match self.mode {
                KeyMode::Major => "major",
                KeyMode::Minor => "minor",
            }
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TimeSymbol {
    #[default]
    Normal,
    Common,
    Cut,
}

/// Simple time signature. Additive and composite signatures are rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TimeSignature {
    pub beats: u32,
    pub beat_type: u32,
    pub symbol: TimeSymbol,
}

impl Default for TimeSignature {
    fn default() -> Self {
        TimeSignature {
            beats: 4,
            beat_type: 4,
            symbol: TimeSymbol::Normal,
        }
    }
}

impl TimeSignature {
    pub const fn new(beats: u32, beat_type: u32) -> TimeSignature {
        TimeSignature {
            beats,
            beat_type,
            symbol: TimeSymbol::Normal,
        }
    }

    pub fn common_time() -> TimeSignature {
        TimeSignature {
            beats: 4,
            beat_type: 4,
            symbol: TimeSymbol::Common,
        }
    }

    pub fn cut_time() -> TimeSignature {
        TimeSignature {
            beats: 2,
            beat_type: 2,
            symbol: TimeSymbol::Cut,
        }
    }

    pub fn validate(self) -> Result<()> {
        if self.beats == 0 || self.beats > 64 {
            return Err(Error::out_of_range(format!(
                "time signature beats {}",
                self.beats
            )));
        }
        if self.beat_type == 0 || self.beat_type > 64 || !self.beat_type.is_power_of_two() {
            return Err(Error::out_of_range(format!(
                "time signature beat type {}",
                self.beat_type
            )));
        }
        Ok(())
    }

    /// Capacity of one measure in divisions.
    pub fn measure_ticks(self, divisions: u32) -> u64 {
        if self.beat_type == 0 {
            return 0;
        }
        divisions as u64 * 4 * self.beats as u64 / self.beat_type as u64
    }

    pub fn beats_per_measure(self) -> f64 {
        self.beats as f64 * 4.0 / self.beat_type as f64
    }

    pub fn is_compound(self) -> bool {
        self.beat_type >= 8 && self.beats % 3 == 0 && self.beats > 3
    }

    pub fn describe(self) -> String {
        match self.symbol {
            TimeSymbol::Common => "common time".to_string(),
            TimeSymbol::Cut => "cut time".to_string(),
            TimeSymbol::Normal => format!("{}/{}", self.beats, self.beat_type),
        }
    }

    /// Grouping of beats used by automatic beaming. Returns groups of beat
    /// counts, summing to the measure.
    pub fn beam_groups(self) -> Vec<u32> {
        if self.is_compound() {
            let groups = self.beats / 3;
            let mut result = vec![3; groups as usize];
            if self.beats % 3 != 0 {
                result.push(self.beats % 3);
            }
            result
        } else {
            match self.beats {
                4 => vec![2, 2],
                2 => vec![2],
                3 => vec![3],
                6 => vec![3, 3],
                8 => vec![4, 4],
                12 => vec![4, 4, 4],
                other => vec![other.max(1)],
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClefSign {
    G,
    F,
    C,
    Percussion,
}

impl ClefSign {
    pub const fn musicxml_name(self) -> &'static str {
        match self {
            ClefSign::G => "G",
            ClefSign::F => "F",
            ClefSign::C => "C",
            ClefSign::Percussion => "percussion",
        }
    }

    pub fn from_musicxml_name(text: &str) -> Option<ClefSign> {
        Some(match text.trim() {
            "G" | "g" => ClefSign::G,
            "F" | "f" => ClefSign::F,
            "C" | "c" => ClefSign::C,
            "percussion" => ClefSign::Percussion,
            _ => return None,
        })
    }
}

/// Clef with a line number counted from the bottom staff line (1..5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Clef {
    pub sign: ClefSign,
    pub line: u8,
    pub octave_change: i8,
}

impl Default for Clef {
    fn default() -> Self {
        Clef {
            sign: ClefSign::G,
            line: 2,
            octave_change: 0,
        }
    }
}

impl Clef {
    pub const fn new(sign: ClefSign, line: u8, octave_change: i8) -> Clef {
        Clef {
            sign,
            line,
            octave_change,
        }
    }

    pub fn treble() -> Clef {
        Clef::default()
    }

    pub fn bass() -> Clef {
        Clef::new(ClefSign::F, 4, 0)
    }

    pub fn alto() -> Clef {
        Clef::new(ClefSign::C, 3, 0)
    }

    pub fn validate(self) -> Result<()> {
        if !(1..=5).contains(&self.line) {
            return Err(Error::out_of_range(format!("clef line {}", self.line)));
        }
        if !(-2..=2).contains(&self.octave_change) {
            return Err(Error::out_of_range(format!(
                "clef octave change {}",
                self.octave_change
            )));
        }
        Ok(())
    }

    /// Diatonic index (C0 = 0) of the pitch on the clef's reference line.
    pub fn reference_diatonic(self) -> i32 {
        match self.sign {
            ClefSign::G => 4 * 7 + Step::G.index() as i32,
            ClefSign::F => 3 * 7 + Step::F.index() as i32,
            ClefSign::C => 4 * 7 + Step::C.index() as i32,
            ClefSign::Percussion => 3 * 7 + Step::B.index() as i32,
        }
    }

    /// Diatonic index of the bottom staff line.
    pub fn bottom_line_diatonic(self) -> i32 {
        self.reference_diatonic() - 2 * (self.line as i32 - 1)
    }

    /// Diatonic index of the middle staff line.
    pub fn middle_line_diatonic(self) -> i32 {
        self.bottom_line_diatonic() + 4
    }

    /// Vertical staff step for a pitch: 0 is the bottom line, 1 the bottom
    /// space, 2 the second line, and so on. Half-integers are impossible
    /// because diatonic indexes are integers.
    pub fn staff_step(self, pitch_diatonic: i32) -> i32 {
        pitch_diatonic - self.bottom_line_diatonic()
    }
}

/// Resolved attributes at the start of a measure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Attributes {
    pub divisions: u32,
    pub key: Key,
    pub time: TimeSignature,
    pub clef: Clef,
}

impl Default for Attributes {
    fn default() -> Self {
        Attributes {
            divisions: 480,
            key: Key::default(),
            time: TimeSignature::default(),
            clef: Clef::default(),
        }
    }
}

impl Attributes {
    pub fn validate(self) -> Result<()> {
        if self.divisions == 0 || self.divisions > 10_000 {
            return Err(Error::out_of_range(format!(
                "divisions per quarter {}",
                self.divisions
            )));
        }
        self.key.validate()?;
        self.time.validate()?;
        self.clef.validate()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_accidentals_follow_circle_of_fifths() {
        assert!(Key::new(0, KeyMode::Major).accidentals().is_empty());
        let d_major = Key::new(2, KeyMode::Major).accidentals();
        assert_eq!(d_major.len(), 2);
        assert_eq!(d_major[0], (Step::F, Accidental::Sharp));
        assert_eq!(d_major[1], (Step::C, Accidental::Sharp));
        let b_flat = Key::new(-2, KeyMode::Major).accidentals();
        assert_eq!(b_flat[0], (Step::B, Accidental::Flat));
        assert_eq!(b_flat[1], (Step::E, Accidental::Flat));
    }

    #[test]
    fn key_tonics() {
        assert_eq!(Key::new(0, KeyMode::Major).tonic(), (Step::C, 0));
        assert_eq!(Key::new(1, KeyMode::Major).tonic(), (Step::G, 0));
        assert_eq!(Key::new(-1, KeyMode::Major).tonic(), (Step::F, 0));
        assert_eq!(Key::new(0, KeyMode::Minor).tonic(), (Step::A, 0));
    }

    #[test]
    fn clef_staff_positions() {
        let treble = Clef::treble();
        assert_eq!(treble.bottom_line_diatonic(), 30); // E4
        let bass = Clef::bass();
        assert_eq!(bass.bottom_line_diatonic(), 18); // G2
        let alto = Clef::alto();
        assert_eq!(alto.bottom_line_diatonic(), 24); // F3
    }

    #[test]
    fn time_capacity() {
        assert_eq!(TimeSignature::new(4, 4).measure_ticks(480), 1920);
        assert_eq!(TimeSignature::new(6, 8).measure_ticks(480), 1440);
        assert_eq!(TimeSignature::new(3, 4).measure_ticks(96), 288);
    }

    #[test]
    fn invalid_times_are_rejected() {
        assert!(TimeSignature::new(0, 4).validate().is_err());
        assert!(TimeSignature::new(4, 3).validate().is_err());
        assert!(TimeSignature::new(4, 4).validate().is_ok());
    }

    #[test]
    fn beam_groups_cover_measure() {
        for (beats, beat_type) in [(4, 4), (3, 4), (6, 8), (2, 2), (12, 8), (5, 4)] {
            let signature = TimeSignature::new(beats, beat_type);
            let sum: u32 = signature.beam_groups().iter().sum();
            assert_eq!(sum, beats);
        }
    }
}
