//! Marking types shared by the score model, MusicXML and engraving.

use std::fmt;

/// Engraved accidental.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Accidental {
    DoubleFlat,
    Flat,
    Natural,
    Sharp,
    DoubleSharp,
}

impl Accidental {
    pub const ALL: [Accidental; 5] = [
        Accidental::DoubleFlat,
        Accidental::Flat,
        Accidental::Natural,
        Accidental::Sharp,
        Accidental::DoubleSharp,
    ];

    pub const fn alter(self) -> i8 {
        match self {
            Accidental::DoubleFlat => -2,
            Accidental::Flat => -1,
            Accidental::Natural => 0,
            Accidental::Sharp => 1,
            Accidental::DoubleSharp => 2,
        }
    }

    pub const fn from_alter(alter: i8) -> Option<Accidental> {
        Some(match alter {
            -2 => Accidental::DoubleFlat,
            -1 => Accidental::Flat,
            0 => Accidental::Natural,
            1 => Accidental::Sharp,
            2 => Accidental::DoubleSharp,
            _ => return None,
        })
    }

    pub const fn musicxml_name(self) -> &'static str {
        match self {
            Accidental::DoubleFlat => "flat-flat",
            Accidental::Flat => "flat",
            Accidental::Natural => "natural",
            Accidental::Sharp => "sharp",
            Accidental::DoubleSharp => "double-sharp",
        }
    }

    pub fn from_musicxml_name(text: &str) -> Option<Accidental> {
        Some(match text.trim() {
            "flat-flat" | "double-flat" => Accidental::DoubleFlat,
            "flat" => Accidental::Flat,
            "natural" => Accidental::Natural,
            "sharp" => Accidental::Sharp,
            "double-sharp" | "sharp-sharp" => Accidental::DoubleSharp,
            _ => return None,
        })
    }

    pub const fn symbol(self) -> &'static str {
        match self {
            Accidental::DoubleFlat => "\u{1d12b}",
            Accidental::Flat => "\u{266d}",
            Accidental::Natural => "\u{266e}",
            Accidental::Sharp => "\u{266f}",
            Accidental::DoubleSharp => "\u{1d12a}",
        }
    }
}

/// Articulation marks attached to a note.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Articulation {
    Accent,
    Staccato,
    Staccatissimo,
    Tenuto,
    Marcato,
    Fermata,
    BreathMark,
    UpBow,
    DownBow,
    Harmonic,
    OpenString,
    Pizzicato,
    SnapPizzicato,
}

impl Articulation {
    pub const ALL: [Articulation; 13] = [
        Articulation::Accent,
        Articulation::Staccato,
        Articulation::Staccatissimo,
        Articulation::Tenuto,
        Articulation::Marcato,
        Articulation::Fermata,
        Articulation::BreathMark,
        Articulation::UpBow,
        Articulation::DownBow,
        Articulation::Harmonic,
        Articulation::OpenString,
        Articulation::Pizzicato,
        Articulation::SnapPizzicato,
    ];

    pub const fn musicxml_name(self) -> &'static str {
        match self {
            Articulation::Accent => "accent",
            Articulation::Staccato => "staccato",
            Articulation::Staccatissimo => "staccatissimo",
            Articulation::Tenuto => "tenuto",
            Articulation::Marcato => "strong-accent",
            Articulation::Fermata => "fermata",
            Articulation::BreathMark => "breath-mark",
            Articulation::UpBow => "up-bow",
            Articulation::DownBow => "down-bow",
            Articulation::Harmonic => "harmonic",
            Articulation::OpenString => "open-string",
            Articulation::Pizzicato => "pizzicato",
            Articulation::SnapPizzicato => "snap-pizzicato",
        }
    }

    pub fn from_musicxml_name(text: &str) -> Option<Articulation> {
        Some(match text.trim() {
            "accent" => Articulation::Accent,
            "staccato" => Articulation::Staccato,
            "staccatissimo" => Articulation::Staccatissimo,
            "tenuto" => Articulation::Tenuto,
            "strong-accent" => Articulation::Marcato,
            "fermata" => Articulation::Fermata,
            "breath-mark" => Articulation::BreathMark,
            "up-bow" => Articulation::UpBow,
            "down-bow" => Articulation::DownBow,
            "harmonic" => Articulation::Harmonic,
            "open-string" => Articulation::OpenString,
            "pizzicato" => Articulation::Pizzicato,
            "snap-pizzicato" => Articulation::SnapPizzicato,
            _ => return None,
        })
    }

    /// Marks drawn close above/below the notehead by default.
    pub const fn is_near_note(self) -> bool {
        matches!(
            self,
            Articulation::Accent
                | Articulation::Staccato
                | Articulation::Staccatissimo
                | Articulation::Tenuto
                | Articulation::Marcato
                | Articulation::Fermata
        )
    }
}

/// Dynamic marking. Values follow the MusicXML `<dynamics>` children.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Dynamic {
    Ppp,
    Pp,
    P,
    Mp,
    Mf,
    F,
    Ff,
    Fff,
    Sf,
    Sfp,
    Fp,
    Rf,
    Rfz,
    Sfz,
    Sffz,
    Fz,
}

impl Dynamic {
    pub const ALL: [Dynamic; 16] = [
        Dynamic::Ppp,
        Dynamic::Pp,
        Dynamic::P,
        Dynamic::Mp,
        Dynamic::Mf,
        Dynamic::F,
        Dynamic::Ff,
        Dynamic::Fff,
        Dynamic::Sf,
        Dynamic::Sfp,
        Dynamic::Fp,
        Dynamic::Rf,
        Dynamic::Rfz,
        Dynamic::Sfz,
        Dynamic::Sffz,
        Dynamic::Fz,
    ];

    pub const fn musicxml_name(self) -> &'static str {
        match self {
            Dynamic::Ppp => "ppp",
            Dynamic::Pp => "pp",
            Dynamic::P => "p",
            Dynamic::Mp => "mp",
            Dynamic::Mf => "mf",
            Dynamic::F => "f",
            Dynamic::Ff => "ff",
            Dynamic::Fff => "fff",
            Dynamic::Sf => "sf",
            Dynamic::Sfp => "sfp",
            Dynamic::Fp => "fp",
            Dynamic::Rf => "rf",
            Dynamic::Rfz => "rfz",
            Dynamic::Sfz => "sfz",
            Dynamic::Sffz => "sffz",
            Dynamic::Fz => "fz",
        }
    }

    pub fn from_musicxml_name(text: &str) -> Option<Dynamic> {
        Some(match text.trim() {
            "ppp" => Dynamic::Ppp,
            "pp" => Dynamic::Pp,
            "p" => Dynamic::P,
            "mp" => Dynamic::Mp,
            "mf" => Dynamic::Mf,
            "f" => Dynamic::F,
            "ff" => Dynamic::Ff,
            "fff" => Dynamic::Fff,
            "sf" => Dynamic::Sf,
            "sfp" => Dynamic::Sfp,
            "fp" => Dynamic::Fp,
            "rf" => Dynamic::Rf,
            "rfz" => Dynamic::Rfz,
            "sfz" => Dynamic::Sfz,
            "sffz" => Dynamic::Sffz,
            "fz" => Dynamic::Fz,
            _ => return None,
        })
    }

    /// Rough loudness rank used to choose MIDI velocity.
    pub const fn level(self) -> u8 {
        match self {
            Dynamic::Ppp => 1,
            Dynamic::Pp => 2,
            Dynamic::P => 3,
            Dynamic::Mp => 4,
            Dynamic::Mf => 5,
            Dynamic::F => 6,
            Dynamic::Ff => 7,
            Dynamic::Fff => 8,
            Dynamic::Sf | Dynamic::Fp | Dynamic::Rf | Dynamic::Fz => 7,
            Dynamic::Sfp | Dynamic::Rfz | Dynamic::Sfz => 6,
            Dynamic::Sffz => 8,
        }
    }

    pub fn velocity(self) -> u8 {
        32 + self.level().saturating_sub(1) * 10
    }
}

impl fmt::Display for Dynamic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.musicxml_name())
    }
}

/// Lyric syllable position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Syllabic {
    #[default]
    Single,
    Begin,
    Middle,
    End,
}

impl Syllabic {
    pub const fn musicxml_name(self) -> &'static str {
        match self {
            Syllabic::Single => "single",
            Syllabic::Begin => "begin",
            Syllabic::Middle => "middle",
            Syllabic::End => "end",
        }
    }

    pub fn from_musicxml_name(text: &str) -> Syllabic {
        match text.trim() {
            "begin" => Syllabic::Begin,
            "middle" => Syllabic::Middle,
            "end" => Syllabic::End,
            _ => Syllabic::Single,
        }
    }

    pub const fn needs_hyphen(self) -> bool {
        matches!(self, Syllabic::Begin | Syllabic::Middle)
    }

    pub const fn needs_underscore(self) -> bool {
        matches!(self, Syllabic::Begin | Syllabic::Middle)
    }
}

/// One lyric line attached to a note.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Lyric {
    pub number: u8,
    pub text: String,
    pub syllabic: Syllabic,
    pub extend: bool,
}

impl Lyric {
    pub fn new(text: impl Into<String>) -> Lyric {
        Lyric {
            number: 1,
            text: text.into(),
            syllabic: Syllabic::Single,
            extend: false,
        }
    }

    pub fn validate(&self) -> bool {
        self.number > 0 && self.text.chars().count() <= 512
    }
}

/// Tie flags on a note. Both may be true for a middle note of a chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Tie {
    pub start: bool,
    pub stop: bool,
}

impl Tie {
    pub const NONE: Tie = Tie {
        start: false,
        stop: false,
    };

    pub const fn new(start: bool, stop: bool) -> Tie {
        Tie { start, stop }
    }

    pub const fn is_tied(self) -> bool {
        self.start || self.stop
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SlurKind {
    Start,
    Stop,
    Continue,
}

/// A slur mark on a note. Start/stop marks with the same number are paired by
/// the engraver.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SlurMark {
    pub number: u8,
    pub kind: SlurKind,
}

impl SlurMark {
    pub const fn new(number: u8, kind: SlurKind) -> SlurMark {
        SlurMark { number, kind }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BeamKind {
    Begin,
    Continue,
    End,
    ForwardHook,
    BackwardHook,
}

impl BeamKind {
    pub fn from_musicxml_name(text: &str) -> Option<BeamKind> {
        Some(match text.trim() {
            "begin" => BeamKind::Begin,
            "continue" => BeamKind::Continue,
            "end" => BeamKind::End,
            "forward hook" => BeamKind::ForwardHook,
            "backward hook" => BeamKind::BackwardHook,
            _ => return None,
        })
    }

    pub const fn musicxml_name(self) -> &'static str {
        match self {
            BeamKind::Begin => "begin",
            BeamKind::Continue => "continue",
            BeamKind::End => "end",
            BeamKind::ForwardHook => "forward hook",
            BeamKind::BackwardHook => "backward hook",
        }
    }
}

/// A beam mark carried on an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Beam {
    pub number: u8,
    pub kind: BeamKind,
}

impl Beam {
    pub const fn new(number: u8, kind: BeamKind) -> Beam {
        Beam { number, kind }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accidental_mapping() {
        for accidental in Accidental::ALL {
            assert_eq!(Accidental::from_alter(accidental.alter()), Some(accidental));
            assert_eq!(
                Accidental::from_musicxml_name(accidental.musicxml_name()),
                Some(accidental)
            );
        }
        assert_eq!(Accidental::from_alter(3), None);
    }

    #[test]
    fn dynamic_mapping() {
        for dynamic in Dynamic::ALL {
            assert_eq!(
                Dynamic::from_musicxml_name(dynamic.musicxml_name()),
                Some(dynamic)
            );
            assert!(dynamic.velocity() > 0);
        }
    }

    #[test]
    fn articulation_mapping() {
        for articulation in Articulation::ALL {
            assert_eq!(
                Articulation::from_musicxml_name(articulation.musicxml_name()),
                Some(articulation)
            );
        }
    }

    #[test]
    fn lyric_validation() {
        assert!(Lyric::new("la").validate());
        assert!(!Lyric::new("x".repeat(513)).validate());
    }
}
