//! Parts: an ordered list of measures plus instrument metadata.

use super::attributes::Attributes;
use super::measure::{Measure, Voice};
use super::note::Event;
use crate::error::{Error, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Part {
    pub id: String,
    pub name: String,
    pub abbreviation: String,
    pub measures: Vec<Measure>,
    pub midi_program: u8,
    pub midi_channel: u8,
}

impl Part {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Part {
        Part {
            id: id.into(),
            name: name.into(),
            abbreviation: String::new(),
            measures: Vec::new(),
            midi_program: 0,
            midi_channel: 0,
        }
    }

    pub fn with_measures(mut self, count: usize, attributes: Attributes) -> Part {
        for number in 1..=count {
            self.measures.push(Measure::new(number, attributes));
        }
        self
    }

    pub fn measure(&self, index: usize) -> Option<&Measure> {
        self.measures.get(index)
    }

    pub fn measure_mut(&mut self, index: usize) -> Option<&mut Measure> {
        self.measures.get_mut(index)
    }

    pub fn push_measure(&mut self, attributes: Attributes) -> &mut Measure {
        let number = self.measures.len() + 1;
        self.measures.push(Measure::new(number, attributes));
        let index = self.measures.len() - 1;
        &mut self.measures[index]
    }

    pub fn note_count(&self) -> usize {
        self.measures.iter().map(|m| m.note_count()).sum()
    }

    pub fn event_count(&self) -> usize {
        self.measures.iter().map(|m| m.event_count()).sum()
    }

    pub fn voice(&self, measure: usize, voice: u8) -> Option<&Voice> {
        self.measures.get(measure)?.voice(voice)
    }

    pub fn event(&self, measure: usize, voice: u8, event: usize) -> Option<&Event> {
        self.voice(measure, voice)?.events.get(event)
    }

    /// Start tick of a measure. The first measure may be a pickup; later
    /// underfull measures still occupy a full bar so the timeline stays stable.
    pub fn measure_start_ticks(&self, index: usize) -> u64 {
        let mut start = 0;
        for measure in self.measures.iter().take(index) {
            start += measure_span_ticks(measure, measure.number == 1);
        }
        start
    }

    pub fn total_ticks(&self) -> u64 {
        self.measures
            .iter()
            .map(|m| measure_span_ticks(m, m.number == 1))
            .sum()
    }

    pub fn validate(&self) -> Result<()> {
        if self.id.is_empty() || self.id.chars().count() > 64 {
            return Err(Error::msg("part id must be 1..=64 characters"));
        }
        if self.name.chars().count() > 256 {
            return Err(Error::Limit("part name"));
        }
        if self.abbreviation.chars().count() > 32 {
            return Err(Error::Limit("part abbreviation"));
        }
        if self.measures.len() > 10_000 {
            return Err(Error::Limit("measures per part"));
        }
        if self.midi_channel > 15 {
            return Err(Error::out_of_range(format!(
                "MIDI channel {}",
                self.midi_channel
            )));
        }
        for measure in &self.measures {
            measure.validate()?;
        }
        Ok(())
    }
}

/// Actual span used for timeline computation. A pickup measure is as long as
/// its longest voice; every other measure occupies its time signature.
pub fn measure_span_ticks(measure: &Measure, is_first: bool) -> u64 {
    let capacity = measure.capacity_ticks();
    let longest = measure.longest_voice_ticks();
    if is_first && longest > 0 && longest < capacity {
        longest
    } else {
        capacity
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::score::duration::Duration;
    use crate::score::pitch::Pitch;

    #[test]
    fn measure_starts_account_for_pickup() {
        let mut part = Part::new("p1", "Piano").with_measures(3, Attributes::default());
        part.measures[0].push_note(1, Pitch::parse("C4").unwrap(), Duration::quarter());
        assert_eq!(part.measure_start_ticks(0), 0);
        assert_eq!(part.measure_start_ticks(1), 480);
        assert_eq!(part.measure_start_ticks(2), 480 + 1920);
        assert_eq!(part.total_ticks(), 480 + 1920 + 1920);
    }

    #[test]
    fn invalid_channel_is_rejected() {
        let mut part = Part::new("p1", "Piano");
        part.midi_channel = 16;
        assert!(part.validate().is_err());
        part.midi_channel = 15;
        assert!(part.validate().is_ok());
    }
}
