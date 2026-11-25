//! Measures, voices, barlines, repeats, endings and tempo.

use super::attributes::Attributes;
use super::duration::{Duration, NoteValue};
use super::note::Event;
use super::pitch::Pitch;
use crate::error::{Error, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Voice {
    pub number: u8,
    pub events: Vec<Event>,
}

impl Voice {
    pub fn new(number: u8) -> Voice {
        Voice {
            number,
            events: Vec::new(),
        }
    }

    pub fn duration_ticks(&self, divisions: u32) -> u64 {
        self.events.iter().map(|e| e.duration.ticks(divisions)).sum()
    }

    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub fn validate(&self) -> Result<()> {
        if self.number == 0 || self.number > 16 {
            return Err(Error::out_of_range(format!("voice number {}", self.number)));
        }
        for event in &self.events {
            event.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum BarlineStyle {
    #[default]
    Regular,
    Double,
    Final,
    Dashed,
    Dotted,
    Invisible,
}

impl BarlineStyle {
    pub const fn musicxml_name(self) -> &'static str {
        match self {
            BarlineStyle::Regular => "regular",
            BarlineStyle::Double => "light-light",
            BarlineStyle::Final => "light-heavy",
            BarlineStyle::Dashed => "dashed",
            BarlineStyle::Dotted => "dotted",
            BarlineStyle::Invisible => "none",
        }
    }

    pub fn from_musicxml_name(text: &str) -> Option<BarlineStyle> {
        Some(match text.trim() {
            "regular" => BarlineStyle::Regular,
            "light-light" => BarlineStyle::Double,
            "light-heavy" => BarlineStyle::Final,
            "dashed" => BarlineStyle::Dashed,
            "dotted" => BarlineStyle::Dotted,
            "none" => BarlineStyle::Invisible,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RepeatDirection {
    Forward,
    Backward,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Repeat {
    pub direction: RepeatDirection,
    pub times: u8,
}

impl Repeat {
    pub fn forward() -> Repeat {
        Repeat {
            direction: RepeatDirection::Forward,
            times: 2,
        }
    }

    pub fn backward() -> Repeat {
        Repeat {
            direction: RepeatDirection::Backward,
            times: 2,
        }
    }

    pub fn validate(self) -> Result<()> {
        if self.times < 2 || self.times > 9 {
            return Err(Error::out_of_range(format!("repeat times {}", self.times)));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EndingKind {
    Start,
    Stop,
    Discontinue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Ending {
    pub number: u8,
    pub kind: EndingKind,
}

impl Ending {
    pub fn validate(self) -> Result<()> {
        if self.number == 0 || self.number > 9 {
            return Err(Error::out_of_range(format!("ending number {}", self.number)));
        }
        Ok(())
    }

    pub fn label(self) -> String {
        format!("{}.", self.number)
    }
}

/// Tempo marking attached to a measure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Tempo {
    pub bpm: u32,
    pub beat: NoteValue,
    pub text: Option<&'static str>,
}

impl Tempo {
    pub fn new(bpm: u32, beat: NoteValue) -> Tempo {
        Tempo {
            bpm,
            beat,
            text: None,
        }
    }

    pub fn validate(self) -> Result<()> {
        if self.bpm < 10 || self.bpm > 400 {
            return Err(Error::out_of_range(format!("tempo {}", self.bpm)));
        }
        Ok(())
    }

    pub fn label(self) -> String {
        format!("{} = {}", self.beat.label(), self.bpm)
    }
}

/// One measure in a part. Attributes are resolved (copied forward) so the
/// engraver and exporter always see a complete measure context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Measure {
    pub number: usize,
    pub voices: Vec<Voice>,
    pub attributes: Attributes,
    /// True when this measure carried explicit `<attributes>` in the source.
    pub explicit_attributes: bool,
    pub barline: BarlineStyle,
    pub repeat: Option<Repeat>,
    pub ending: Option<Ending>,
    pub rehearsal: Option<String>,
    pub tempo: Option<Tempo>,
    pub system_break: bool,
    pub page_break: bool,
}

impl Measure {
    pub fn new(number: usize, attributes: Attributes) -> Measure {
        Measure {
            number,
            voices: Vec::new(),
            attributes,
            explicit_attributes: false,
            barline: BarlineStyle::Regular,
            repeat: None,
            ending: None,
            rehearsal: None,
            tempo: None,
            system_break: false,
            page_break: false,
        }
    }

    pub fn voice(&self, number: u8) -> Option<&Voice> {
        self.voices.iter().find(|v| v.number == number)
    }

    pub fn voice_mut(&mut self, number: u8) -> &mut Voice {
        if let Some(index) = self.voices.iter().position(|v| v.number == number) {
            return &mut self.voices[index];
        }
        self.voices.push(Voice::new(number));
        self.voices.sort_by_key(|v| v.number);
        let index = self
            .voices
            .iter()
            .position(|v| v.number == number)
            .unwrap_or(0);
        &mut self.voices[index]
    }

    pub fn push_event(&mut self, voice: u8, event: Event) -> &mut Event {
        let voice = self.voice_mut(voice);
        voice.events.push(event);
        let index = voice.events.len() - 1;
        &mut voice.events[index]
    }

    pub fn push_note(&mut self, voice: u8, pitch: Pitch, duration: Duration) {
        self.push_event(voice, Event::note(pitch, duration));
    }

    pub fn push_rest(&mut self, voice: u8, duration: Duration) {
        self.push_event(voice, Event::rest(duration));
    }

    pub fn event_count(&self) -> usize {
        self.voices.iter().map(|v| v.events.len()).sum()
    }

    pub fn note_count(&self) -> usize {
        self.voices
            .iter()
            .flat_map(|v| v.events.iter())
            .map(|e| e.notes().len())
            .sum()
    }

    pub fn is_empty(&self) -> bool {
        self.voices.iter().all(|v| v.events.is_empty())
    }

    pub fn capacity_ticks(&self) -> u64 {
        self.attributes
            .time
            .measure_ticks(self.attributes.divisions)
    }

    /// Longest voice duration in ticks.
    pub fn longest_voice_ticks(&self) -> u64 {
        self.voices
            .iter()
            .map(|v| v.duration_ticks(self.attributes.divisions))
            .max()
            .unwrap_or(0)
    }

    pub fn remove_event(&mut self, voice: u8, event: usize) -> Option<Event> {
        let voice_index = self.voices.iter().position(|v| v.number == voice)?;
        if event >= self.voices[voice_index].events.len() {
            return None;
        }
        let removed = self.voices[voice_index].events.remove(event);
        if self.voices[voice_index].events.is_empty() {
            self.voices.remove(voice_index);
        }
        Some(removed)
    }

    pub fn validate(&self) -> Result<()> {
        if self.number == 0 {
            return Err(Error::out_of_range("measure number 0"));
        }
        if self.voices.len() > 16 {
            return Err(Error::Limit("voices per measure"));
        }
        self.attributes.validate()?;
        let mut numbers = Vec::new();
        for voice in &self.voices {
            voice.validate()?;
            if numbers.contains(&voice.number) {
                return Err(Error::msg(format!(
                    "duplicate voice {} in measure {}",
                    voice.number, self.number
                )));
            }
            numbers.push(voice.number);
        }
        if let Some(repeat) = self.repeat {
            repeat.validate()?;
        }
        if let Some(ending) = self.ending {
            ending.validate()?;
        }
        if let Some(tempo) = self.tempo {
            tempo.validate()?;
        }
        if let Some(rehearsal) = &self.rehearsal {
            if rehearsal.chars().count() > 128 {
                return Err(Error::Limit("rehearsal text"));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::score::pitch::Pitch;

    fn pitch(name: &str) -> Pitch {
        Pitch::parse(name).unwrap()
    }

    #[test]
    fn voices_are_created_sorted_and_unique() {
        let mut measure = Measure::new(1, Attributes::default());
        measure.push_note(2, pitch("E4"), Duration::quarter());
        measure.push_note(1, pitch("C4"), Duration::quarter());
        assert_eq!(measure.voices.len(), 2);
        assert_eq!(measure.voices[0].number, 1);
        assert_eq!(measure.voices[1].number, 2);
        assert_eq!(measure.event_count(), 2);
    }

    #[test]
    fn voice_duration_accumulates() {
        let mut measure = Measure::new(1, Attributes::default());
        measure.push_note(1, pitch("C4"), Duration::half());
        measure.push_note(1, pitch("D4"), Duration::quarter());
        assert_eq!(measure.voice(1).unwrap().duration_ticks(480), 1440);
        assert_eq!(measure.capacity_ticks(), 1920);
    }

    #[test]
    fn removing_the_last_event_removes_the_voice() {
        let mut measure = Measure::new(1, Attributes::default());
        measure.push_note(1, pitch("C4"), Duration::quarter());
        assert!(measure.remove_event(1, 0).is_some());
        assert!(measure.voices.is_empty());
        assert!(measure.remove_event(1, 0).is_none());
    }

    #[test]
    fn invalid_repeat_is_rejected() {
        let mut measure = Measure::new(1, Attributes::default());
        measure.repeat = Some(Repeat {
            direction: RepeatDirection::Forward,
            times: 1,
        });
        assert!(measure.validate().is_err());
        measure.repeat = Some(Repeat::forward());
        assert!(measure.validate().is_ok());
    }
}
