//! Score, metadata and validation.

use super::attributes::Attributes;
use super::measure::Measure;
use super::note::Event;
use super::part::{measure_span_ticks, Part};
use super::pitch::Pitch;
use crate::error::{Error, Result};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Metadata {
    pub title: String,
    pub subtitle: String,
    pub composer: String,
    pub lyricist: String,
    pub arranger: String,
    pub copyright: String,
    pub source: String,
}

impl Metadata {
    pub fn new(title: impl Into<String>) -> Metadata {
        Metadata {
            title: title.into(),
            ..Metadata::default()
        }
    }

    pub fn validate(&self) -> Result<()> {
        for (name, value, limit) in [
            ("title", &self.title, 512),
            ("subtitle", &self.subtitle, 512),
            ("composer", &self.composer, 512),
            ("lyricist", &self.lyricist, 512),
            ("arranger", &self.arranger, 512),
            ("copyright", &self.copyright, 1024),
            ("source", &self.source, 1024),
        ] {
            if value.chars().count() > limit {
                return Err(Error::Limit(match name {
                    "title" => "title",
                    "subtitle" => "subtitle",
                    "composer" => "composer",
                    "lyricist" => "lyricist",
                    "arranger" => "arranger",
                    "copyright" => "copyright",
                    _ => "source",
                }));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Score {
    pub metadata: Metadata,
    pub parts: Vec<Part>,
}

impl Default for Score {
    fn default() -> Self {
        Score {
            metadata: Metadata::new("Untitled"),
            parts: Vec::new(),
        }
    }
}

impl Score {
    pub const MAX_PARTS: usize = 64;
    pub const MAX_TOTAL_EVENTS: usize = 1_000_000;

    pub fn new(title: impl Into<String>) -> Score {
        Score {
            metadata: Metadata::new(title),
            parts: Vec::new(),
        }
    }

    pub fn add_part(&mut self, part: Part) -> usize {
        self.parts.push(part);
        self.parts.len() - 1
    }

    pub fn part(&self, index: usize) -> Option<&Part> {
        self.parts.get(index)
    }

    pub fn part_mut(&mut self, index: usize) -> Option<&mut Part> {
        self.parts.get_mut(index)
    }

    pub fn part_index_by_id(&self, id: &str) -> Option<usize> {
        self.parts.iter().position(|part| part.id == id)
    }

    pub fn measure_count(&self) -> usize {
        self.parts
            .iter()
            .map(|part| part.measures.len())
            .max()
            .unwrap_or(0)
    }

    pub fn note_count(&self) -> usize {
        self.parts.iter().map(|part| part.note_count()).sum()
    }

    pub fn event_count(&self) -> usize {
        self.parts.iter().map(|part| part.event_count()).sum()
    }

    pub fn total_ticks(&self) -> u64 {
        self.parts
            .iter()
            .map(|part| part.total_ticks())
            .max()
            .unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.parts.is_empty()
    }

    /// Structural validation. Returns the first hard error, if any.
    pub fn validate(&self) -> Result<()> {
        self.metadata.validate()?;
        if self.parts.len() > Self::MAX_PARTS {
            return Err(Error::Limit("parts per score"));
        }
        if self.event_count() > Self::MAX_TOTAL_EVENTS {
            return Err(Error::Limit("events per score"));
        }
        let mut ids = Vec::new();
        for part in &self.parts {
            part.validate()?;
            if ids.contains(&part.id) {
                return Err(Error::msg(format!("duplicate part id {:?}", part.id)));
            }
            ids.push(part.id.clone());
        }
        Ok(())
    }

    /// Musical inspection: returns warnings and errors that do not make the
    /// score structurally invalid (underfull measures, unmatched ties, ...).
    pub fn inspect(&self) -> ValidationReport {
        let mut issues = Vec::new();
        if self.parts.is_empty() {
            issues.push(Issue::warning("score has no parts"));
        }
        for (part_index, part) in self.parts.iter().enumerate() {
            if part.measures.is_empty() {
                issues.push(Issue::warning(format!(
                    "part {} ({}) has no measures",
                    part_index + 1,
                    part.name
                )));
                continue;
            }
            for (measure_index, measure) in part.measures.iter().enumerate() {
                if measure.number != measure_index + 1 {
                    issues.push(Issue::warning(format!(
                        "part {} measure {} is numbered {}",
                        part_index + 1,
                        measure_index + 1,
                        measure.number
                    )));
                }
                let capacity = measure.capacity_ticks();
                for voice in &measure.voices {
                    let fill = voice.duration_ticks(measure.attributes.divisions);
                    if fill > capacity {
                        issues.push(Issue::error(format!(
                            "part {} measure {} voice {} overfull: {fill} > {capacity} ticks",
                            part_index + 1,
                            measure.number,
                            voice.number
                        )));
                    } else if fill < capacity && !(measure_index == 0 && !voice.is_empty()) {
                        issues.push(Issue::warning(format!(
                            "part {} measure {} voice {} underfull: {fill} < {capacity} ticks",
                            part_index + 1,
                            measure.number,
                            voice.number
                        )));
                    }
                }
                if measure.voices.is_empty() {
                    issues.push(Issue::warning(format!(
                        "part {} measure {} has no voices",
                        part_index + 1,
                        measure.number
                    )));
                }
            }
            issues.extend(inspect_ties_and_slurs(part_index, part));
        }
        for measure_index in 0..self.measure_count() {
            let lengths: Vec<usize> = self
                .parts
                .iter()
                .map(|part| part.measures.len())
                .collect();
            if lengths.iter().any(|&len| len != lengths[0]) && measure_index == 0 {
                issues.push(Issue::warning(format!(
                    "parts have different measure counts: {lengths:?}"
                )));
            }
        }
        ValidationReport { issues }
    }

    /// A copy containing only one part, with the same metadata.
    pub fn extract_part(&self, index: usize) -> Option<Score> {
        let part = self.parts.get(index)?.clone();
        Some(Score {
            metadata: self.metadata.clone(),
            parts: vec![part],
        })
    }

    /// Transpose one part chromatically, spelling with the local key.
    pub fn transpose_part(&mut self, index: usize, semitones: i8) -> Result<()> {
        let part = self
            .parts
            .get_mut(index)
            .ok_or_else(|| Error::out_of_range(format!("part index {index}")))?;
        for measure in &mut part.measures {
            let prefer_flats = measure.attributes.key.prefer_flats();
            for voice in &mut measure.voices {
                for event in &mut voice.events {
                    for note in event.notes_mut() {
                        let midi = note.pitch.midi().ok_or_else(|| {
                            Error::out_of_range(format!("pitch {}", note.pitch.name()))
                        })? as i32;
                        let shifted = midi + semitones as i32;
                        if !(0..=127).contains(&shifted) {
                            return Err(Error::out_of_range(format!(
                                "transposed pitch from {}",
                                note.pitch.name()
                            )));
                        }
                        note.pitch = Pitch::spell_midi(shifted as u8, prefer_flats);
                    }
                }
            }
        }
        Ok(())
    }

    /// Start tick of a measure in a part.
    pub fn measure_start_ticks(&self, part: usize, measure: usize) -> u64 {
        self.parts
            .get(part)
            .map(|part| part.measure_start_ticks(measure))
            .unwrap_or(0)
    }

    /// Flatten all pitched notes for MIDI export and analysis.
    pub fn flatten_notes(&self) -> Vec<FlatNote> {
        let mut notes = Vec::new();
        for (part_index, part) in self.parts.iter().enumerate() {
            let mut measure_start = 0u64;
            for (measure_index, measure) in part.measures.iter().enumerate() {
                let divisions = measure.attributes.divisions;
                for (voice_index, voice) in measure.voices.iter().enumerate() {
                    let mut offset = 0u64;
                    for (event_index, event) in voice.events.iter().enumerate() {
                        let ticks = event.duration.ticks(divisions);
                        if let super::note::EventKind::Pitched(notes_in_event) = &event.kind {
                            for (note_index, note) in notes_in_event.iter().enumerate() {
                                notes.push(FlatNote {
                                    part: part_index,
                                    measure: measure_index,
                                    voice: voice_index,
                                    event: event_index,
                                    note: note_index,
                                    pitch: note.pitch,
                                    start_ticks: measure_start + offset,
                                    duration_ticks: ticks,
                                    velocity: event
                                        .dynamics
                                        .map(|d| d.velocity())
                                        .unwrap_or_else(|| note.effective_velocity()),
                                    tie_start: note.tie.start,
                                    tie_stop: note.tie.stop,
                                });
                            }
                        }
                        offset += ticks;
                    }
                }
                measure_start += measure_span_ticks(measure, measure_index == 0);
            }
        }
        notes
    }

    /// Build a small, musical demonstration score.
    pub fn demo() -> Score {
        super::demo::demo_score()
    }
}

/// Flattened note used by MIDI export and tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlatNote {
    pub part: usize,
    pub measure: usize,
    pub voice: usize,
    pub event: usize,
    pub note: usize,
    pub pitch: Pitch,
    pub start_ticks: u64,
    pub duration_ticks: u64,
    pub velocity: u8,
    pub tie_start: bool,
    pub tie_stop: bool,
}

impl FlatNote {
    pub fn end_ticks(self) -> u64 {
        self.start_ticks + self.duration_ticks
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Issue {
    pub severity: Severity,
    pub message: String,
}

impl Issue {
    pub fn warning(message: impl Into<String>) -> Issue {
        Issue {
            severity: Severity::Warning,
            message: message.into(),
        }
    }

    pub fn error(message: impl Into<String>) -> Issue {
        Issue {
            severity: Severity::Error,
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ValidationReport {
    pub issues: Vec<Issue>,
}

impl ValidationReport {
    pub fn is_ok(&self) -> bool {
        !self.issues.iter().any(|i| i.severity == Severity::Error)
    }

    pub fn error_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|i| i.severity == Severity::Error)
            .count()
    }

    pub fn warning_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|i| i.severity == Severity::Warning)
            .count()
    }

    pub fn summary(&self) -> String {
        format!(
            "{} error(s), {} warning(s)",
            self.error_count(),
            self.warning_count()
        )
    }
}

fn inspect_ties_and_slurs(part_index: usize, part: &Part) -> Vec<Issue> {
    let mut issues = Vec::new();
    // Ties are tracked per voice across measure boundaries.
    let mut open: std::collections::BTreeMap<u8, Vec<(Pitch, usize)>> =
        std::collections::BTreeMap::new();
    for measure in &part.measures {
        for voice in &measure.voices {
            for event in &voice.events {
                for note in event.notes() {
                    let ties = open.entry(voice.number).or_default();
                    if note.tie.stop {
                        if let Some(index) = ties
                            .iter()
                            .rposition(|(pitch, _)| *pitch == note.pitch)
                        {
                            ties.remove(index);
                        } else {
                            issues.push(Issue::warning(format!(
                                "part {} measure {} voice {} tie stop without matching start on {}",
                                part_index + 1,
                                measure.number,
                                voice.number,
                                note.pitch
                            )));
                        }
                    }
                    if note.tie.start {
                        ties.push((note.pitch, measure.number));
                    }
                }
            }
        }
    }
    for (voice, ties) in open {
        for (pitch, measure_number) in ties {
            issues.push(Issue::warning(format!(
                "part {} measure {} voice {} tie start on {} has no continuation",
                part_index + 1,
                measure_number,
                voice,
                pitch
            )));
        }
    }
    // Slur balance per part, scanning all voices in order.
    let mut open: Vec<u8> = Vec::new();
    for measure in &part.measures {
        for voice in &measure.voices {
            for event in &voice.events {
                for note in event.notes() {
                    for slur in &note.slurs {
                        match slur.kind {
                            super::marks::SlurKind::Start => {
                                if open.len() >= 8 {
                                    issues.push(Issue::warning(format!(
                                        "part {} has too many nested slurs",
                                        part_index + 1
                                    )));
                                } else {
                                    open.push(slur.number);
                                }
                            }
                            super::marks::SlurKind::Stop => {
                                if let Some(pos) =
                                    open.iter().rposition(|number| *number == slur.number)
                                {
                                    open.remove(pos);
                                } else {
                                    issues.push(Issue::warning(format!(
                                        "part {} measure {} slur stop {} without start",
                                        part_index + 1,
                                        measure.number,
                                        slur.number
                                    )));
                                }
                            }
                            super::marks::SlurKind::Continue => {}
                        }
                    }
                }
            }
        }
    }
    issues
}

/// Convenience: structural check plus inspection errors.
pub fn validate_score(score: &Score) -> Result<ValidationReport> {
    score.validate()?;
    Ok(score.inspect())
}

/// Recompute measure numbers to be sequential within every part.
pub fn renumber_measures(score: &mut Score) {
    for part in &mut score.parts {
        for (index, measure) in part.measures.iter_mut().enumerate() {
            measure.number = index + 1;
        }
    }
}

/// Copy the resolved attributes forward into a new measure.
pub fn measure_with_attributes(number: usize, attributes: Attributes) -> Measure {
    Measure::new(number, attributes)
}

/// Event lookup used by the studio editor.
pub fn event_at(score: &Score, part: usize, measure: usize, voice: u8, event: usize) -> Option<&Event> {
    score.parts.get(part)?.measures.get(measure)?.voice(voice)?.events.get(event)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::score::duration::Duration;
    use crate::score::measure::Repeat;

    fn simple_score() -> Score {
        let mut part = Part::new("p1", "Piano").with_measures(2, Attributes::default());
        part.measures[0].push_note(1, Pitch::parse("C4").unwrap(), Duration::whole());
        part.measures[1].push_note(1, Pitch::parse("D4").unwrap(), Duration::whole());
        let mut score = Score::new("Test");
        score.add_part(part);
        score
    }

    #[test]
    fn validate_and_inspect_clean_score() {
        let score = simple_score();
        assert!(score.validate().is_ok());
        let report = score.inspect();
        assert!(report.is_ok(), "{:?}", report.issues);
        assert_eq!(score.note_count(), 2);
        assert_eq!(score.measure_count(), 2);
        assert_eq!(score.total_ticks(), 3840);
    }

    #[test]
    fn overfull_measure_is_reported() {
        let mut score = simple_score();
        score.parts[0].measures[0].push_note(
            1,
            Pitch::parse("E4").unwrap(),
            Duration::quarter(),
        );
        let report = score.inspect();
        assert!(!report.is_ok());
        assert!(report.error_count() >= 1);
    }

    #[test]
    fn pickup_measure_is_not_underfull_warning() {
        let mut part = Part::new("p1", "Piano").with_measures(2, Attributes::default());
        part.measures[0].push_note(1, Pitch::parse("C4").unwrap(), Duration::quarter());
        part.measures[1].push_note(1, Pitch::parse("D4").unwrap(), Duration::whole());
        let mut score = Score::new("Pickup");
        score.add_part(part);
        let report = score.inspect();
        assert!(report.is_ok(), "{:?}", report.issues);
        assert!(!report
            .issues
            .iter()
            .any(|issue| issue.message.contains("measure 1 voice 1 underfull")));
    }

    #[test]
    fn extract_part_keeps_metadata() {
        let score = simple_score();
        let extracted = score.extract_part(0).unwrap();
        assert_eq!(extracted.metadata.title, "Test");
        assert_eq!(extracted.parts.len(), 1);
        assert_eq!(extracted.note_count(), 2);
        assert!(score.extract_part(9).is_none());
    }

    #[test]
    fn transpose_updates_pitches() {
        let mut score = simple_score();
        score.transpose_part(0, 2).unwrap();
        assert_eq!(score.parts[0].measures[0].voices[0].events[0].primary_pitch().unwrap().midi(), Some(62));
    }

    #[test]
    fn flatten_notes_has_starts() {
        let score = simple_score();
        let notes = score.flatten_notes();
        assert_eq!(notes.len(), 2);
        assert_eq!(notes[0].start_ticks, 0);
        assert_eq!(notes[1].start_ticks, 1920);
    }

    #[test]
    fn duplicate_part_ids_are_rejected() {
        let mut score = simple_score();
        score.add_part(Part::new("p1", "Other"));
        assert!(score.validate().is_err());
    }

    #[test]
    fn invalid_repeat_is_structural() {
        let mut score = simple_score();
        score.parts[0].measures[0].repeat = Some(Repeat {
            direction: crate::score::measure::RepeatDirection::Backward,
            times: 0,
        });
        assert!(score.validate().is_err());
    }
}
