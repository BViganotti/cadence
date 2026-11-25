//! Notes, chords, rests and events.

use super::duration::Duration;
use super::marks::{Articulation, Beam, Dynamic, Lyric, SlurMark, Tie};
use super::pitch::Pitch;
use crate::error::Result;

/// A single written note inside an event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    pub pitch: Pitch,
    pub tie: Tie,
    pub accidental: Option<super::marks::Accidental>,
    pub articulations: Vec<Articulation>,
    pub lyrics: Vec<Lyric>,
    pub slurs: Vec<SlurMark>,
    /// MIDI velocity override, 1..=127.
    pub velocity: u8,
}

impl Note {
    pub fn new(pitch: Pitch) -> Note {
        Note {
            pitch,
            tie: Tie::NONE,
            accidental: None,
            articulations: Vec::new(),
            lyrics: Vec::new(),
            slurs: Vec::new(),
            velocity: 80,
        }
    }

    pub fn with_velocity(mut self, velocity: u8) -> Note {
        self.velocity = velocity.clamp(1, 127);
        self
    }

    pub fn validate(&self) -> Result<()> {
        self.pitch.validate()?;
        if self.velocity == 0 || self.velocity > 127 {
            return Err(crate::error::Error::out_of_range(format!(
                "velocity {}",
                self.velocity
            )));
        }
        if self.articulations.len() > 16 {
            return Err(crate::error::Error::Limit("articulations per note"));
        }
        if self.lyrics.len() > 8 {
            return Err(crate::error::Error::Limit("lyrics per note"));
        }
        if self.slurs.len() > 8 {
            return Err(crate::error::Error::Limit("slurs per note"));
        }
        for lyric in &self.lyrics {
            if !lyric.validate() {
                return Err(crate::error::Error::Limit("lyric text"));
            }
        }
        Ok(())
    }

    pub fn effective_velocity(&self) -> u8 {
        self.velocity.clamp(1, 127)
    }
}

/// Event contents: one or more simultaneous notes, or a rest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventKind {
    Pitched(Vec<Note>),
    Rest,
}

/// A rhythmic event in a voice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub kind: EventKind,
    pub duration: Duration,
    pub beams: Vec<Beam>,
    pub dynamics: Option<Dynamic>,
}

impl Event {
    pub fn note(pitch: Pitch, duration: Duration) -> Event {
        Event {
            kind: EventKind::Pitched(vec![Note::new(pitch)]),
            duration,
            beams: Vec::new(),
            dynamics: None,
        }
    }

    pub fn chord(notes: Vec<Note>, duration: Duration) -> Event {
        Event {
            kind: EventKind::Pitched(notes),
            duration,
            beams: Vec::new(),
            dynamics: None,
        }
    }

    pub fn rest(duration: Duration) -> Event {
        Event {
            kind: EventKind::Rest,
            duration,
            beams: Vec::new(),
            dynamics: None,
        }
    }

    pub fn is_rest(&self) -> bool {
        matches!(self.kind, EventKind::Rest)
    }

    pub fn is_chord(&self) -> bool {
        matches!(&self.kind, EventKind::Pitched(notes) if notes.len() > 1)
    }

    pub fn notes(&self) -> &[Note] {
        match &self.kind {
            EventKind::Pitched(notes) => notes,
            EventKind::Rest => &[],
        }
    }

    pub fn notes_mut(&mut self) -> &mut [Note] {
        match &mut self.kind {
            EventKind::Pitched(notes) => notes,
            EventKind::Rest => &mut [],
        }
    }

    /// Add a note, keeping chord notes sorted from low to high.
    pub fn add_note(&mut self, note: Note) -> bool {
        match &mut self.kind {
            EventKind::Pitched(notes) => {
                notes.push(note);
                notes.sort_by_key(|n| (n.pitch.diatonic_index(), n.pitch.chromatic_index()));
                notes.dedup_by(|a, b| a.pitch == b.pitch);
                true
            }
            EventKind::Rest => false,
        }
    }

    pub fn primary_pitch(&self) -> Option<Pitch> {
        self.notes().first().map(|n| n.pitch)
    }

    pub fn lowest_pitch(&self) -> Option<Pitch> {
        self.notes().iter().map(|n| n.pitch).min()
    }

    pub fn highest_pitch(&self) -> Option<Pitch> {
        self.notes().iter().map(|n| n.pitch).max()
    }

    pub fn validate(&self) -> Result<()> {
        self.duration.validate()?;
        if self.beams.len() > 6 {
            return Err(crate::error::Error::Limit("beams per event"));
        }
        match &self.kind {
            EventKind::Pitched(notes) => {
                if notes.is_empty() {
                    return Err(crate::error::Error::msg("pitched event has no notes"));
                }
                if notes.len() > 24 {
                    return Err(crate::error::Error::Limit("notes per chord"));
                }
                for note in notes {
                    note.validate()?;
                }
            }
            EventKind::Rest => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::score::pitch::Step;

    fn pitch(name: &str) -> Pitch {
        Pitch::parse(name).unwrap()
    }

    #[test]
    fn chords_sort_low_to_high() {
        let mut event = Event::note(pitch("C4"), Duration::quarter());
        assert!(event.add_note(Note::new(pitch("E4"))));
        assert!(event.add_note(Note::new(pitch("G4"))));
        let pitches: Vec<_> = event.notes().iter().map(|n| n.pitch).collect();
        assert_eq!(
            pitches,
            vec![pitch("C4"), pitch("E4"), pitch("G4")]
        );
        assert!(event.is_chord());
    }

    #[test]
    fn duplicate_chord_pitch_is_dropped() {
        let mut event = Event::note(pitch("C4"), Duration::quarter());
        event.add_note(Note::new(pitch("C4")));
        assert_eq!(event.notes().len(), 1);
        assert!(event.add_note(Note::new(pitch("E4"))));
        assert_eq!(event.notes().len(), 2);
    }

    #[test]
    fn rest_has_no_pitches() {
        let event = Event::rest(Duration::half());
        assert!(event.is_rest());
        assert!(event.primary_pitch().is_none());
        assert!(!event.is_chord());
    }

    #[test]
    fn note_velocity_is_bounded() {
        let note = Note::new(Pitch::new(Step::C, 0, 4)).with_velocity(200);
        assert_eq!(note.velocity, 127);
        assert!(note.validate().is_ok());
    }
}
