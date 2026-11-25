//! Higher-level notation analysis shared by engraver, importer and studio.
//!
//! These helpers never mutate the score and never allocate unbounded state:
//! the number of open ties/slurs is capped by the score parser limits.

use crate::score::marks::SlurKind;
use crate::score::note::EventKind;
use crate::score::part::Part;
use crate::score::pitch::Pitch;

/// Stable coordinate of a note inside a part.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NotePos {
    pub measure: usize,
    pub voice: u8,
    pub event: usize,
    pub note: usize,
}

impl NotePos {
    pub const fn new(measure: usize, voice: u8, event: usize, note: usize) -> NotePos {
        NotePos {
            measure,
            voice,
            event,
            note,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlurSpan {
    pub number: u8,
    pub start: NotePos,
    pub end: NotePos,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpanProblemKind {
    StartWithoutStop,
    StopWithoutStart,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpanProblem {
    pub number: u8,
    pub at: NotePos,
    pub kind: SpanProblemKind,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SlurResolution {
    pub spans: Vec<SlurSpan>,
    pub problems: Vec<SpanProblem>,
}

/// Pair slur start/stop marks by number within each voice. Slur numbers are
/// scoped to a voice, which matches how MusicXML slur numbers are used in
/// practice and avoids cross-voice ambiguity.
pub fn resolve_slurs(part: &Part) -> SlurResolution {
    let mut result = SlurResolution::default();
    let mut open: std::collections::BTreeMap<u8, Vec<(u8, NotePos)>> =
        std::collections::BTreeMap::new();
    for (measure_index, measure) in part.measures.iter().enumerate() {
        for voice in &measure.voices {
            for (event_index, event) in voice.events.iter().enumerate() {
                if let EventKind::Pitched(notes) = &event.kind {
                    for (note_index, note) in notes.iter().enumerate() {
                        let pos = NotePos::new(measure_index, voice.number, event_index, note_index);
                        for slur in &note.slurs {
                            let stack = open.entry(voice.number).or_default();
                            match slur.kind {
                                SlurKind::Start => {
                                    if stack.len() < 16 {
                                        stack.push((slur.number, pos));
                                    }
                                }
                                SlurKind::Stop => {
                                    if let Some(index) = stack
                                        .iter()
                                        .rposition(|(number, _)| *number == slur.number)
                                    {
                                        let (number, start) = stack.remove(index);
                                        result.spans.push(SlurSpan {
                                            number,
                                            start,
                                            end: pos,
                                        });
                                    } else {
                                        result.problems.push(SpanProblem {
                                            number: slur.number,
                                            at: pos,
                                            kind: SpanProblemKind::StopWithoutStart,
                                        });
                                    }
                                }
                                SlurKind::Continue => {}
                            }
                        }
                    }
                }
            }
        }
    }
    for stack in open.into_values() {
        for (number, at) in stack {
            result.problems.push(SpanProblem {
                number,
                at,
                kind: SpanProblemKind::StartWithoutStop,
            });
        }
    }
    result.spans.sort_by_key(|span| (span.start, span.end));
    result
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TieSpan {
    pub pitch: Pitch,
    pub start: NotePos,
    pub end: NotePos,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TieResolution {
    pub spans: Vec<TieSpan>,
    pub problems: Vec<SpanProblem>,
}

/// Pair tie start/stop flags by pitch inside a voice.
pub fn resolve_ties(part: &Part) -> TieResolution {
    let mut result = TieResolution::default();
    let mut open: std::collections::BTreeMap<u8, Vec<(Pitch, NotePos)>> =
        std::collections::BTreeMap::new();
    for (measure_index, measure) in part.measures.iter().enumerate() {
        for voice in &measure.voices {
            for (event_index, event) in voice.events.iter().enumerate() {
                if let EventKind::Pitched(notes) = &event.kind {
                    for (note_index, note) in notes.iter().enumerate() {
                        let pos = NotePos::new(measure_index, voice.number, event_index, note_index);
                        let stack = open.entry(voice.number).or_default();
                        if note.tie.stop {
                            if let Some(index) =
                                stack.iter().position(|(pitch, _)| *pitch == note.pitch)
                            {
                                let (pitch, start) = stack.remove(index);
                                result.spans.push(TieSpan {
                                    pitch,
                                    start,
                                    end: pos,
                                });
                            } else {
                                result.problems.push(SpanProblem {
                                    number: 0,
                                    at: pos,
                                    kind: SpanProblemKind::StopWithoutStart,
                                });
                            }
                        }
                        if note.tie.start {
                            stack.push((note.pitch, pos));
                        }
                    }
                }
            }
        }
    }
    for stack in open.into_values() {
        for (_pitch, at) in stack {
            result.problems.push(SpanProblem {
                number: 0,
                at,
                kind: SpanProblemKind::StartWithoutStop,
            });
        }
    }
    result.spans.sort_by_key(|span| (span.start, span.end));
    result
}

/// A run of beamable events inside one voice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BeamRun {
    pub start_event: usize,
    pub end_event: usize,
}

/// Candidate beaming runs: consecutive events shorter than a quarter note,
/// not interrupted by rests. Explicit beam marks are handled by the engraver;
/// this function only proposes natural groups.
pub fn beam_runs(part: &Part, measure_index: usize, voice_number: u8) -> Vec<BeamRun> {
    let Some(measure) = part.measures.get(measure_index) else {
        return Vec::new();
    };
    let Some(voice) = measure.voice(voice_number) else {
        return Vec::new();
    };
    let mut runs = Vec::new();
    let mut start = None;
    for (index, event) in voice.events.iter().enumerate() {
        let beamable = !event.is_rest() && event.duration.is_compound();
        if beamable {
            if start.is_none() {
                start = Some(index);
            }
        } else if let Some(first) = start.take() {
            if index - first >= 2 {
                runs.push(BeamRun {
                    start_event: first,
                    end_event: index - 1,
                });
            }
        }
    }
    if let Some(first) = start {
        if voice.events.len() - first >= 2 {
            runs.push(BeamRun {
                start_event: first,
                end_event: voice.events.len() - 1,
            });
        }
    }
    runs
}

/// First pitched note of an event as a convenience for editing commands.
pub fn first_pitch(part: &Part, measure: usize, voice: u8, event: usize) -> Option<Pitch> {
    part.measures
        .get(measure)?
        .voice(voice)?
        .events
        .get(event)?
        .primary_pitch()
}

/// Count lyric lines across a part, capped at eight.
pub fn lyric_line_count(part: &Part) -> u8 {
    let mut max = 0;
    for measure in &part.measures {
        for voice in &measure.voices {
            for event in &voice.events {
                for note in event.notes() {
                    for lyric in &note.lyrics {
                        max = max.max(lyric.number);
                    }
                }
            }
        }
    }
    max.min(8)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::score::attributes::Attributes;
    use crate::score::duration::Duration;
    use crate::score::marks::{SlurMark, Tie};
    use crate::score::note::Note;
    use crate::score::pitch::Pitch;

    fn part_with_two_notes() -> Part {
        let mut part = Part::new("p", "Piano").with_measures(1, Attributes::default());
        part.measures[0].push_note(1, Pitch::parse("C4").unwrap(), Duration::quarter());
        part.measures[0].push_note(1, Pitch::parse("D4").unwrap(), Duration::quarter());
        part
    }

    #[test]
    fn slur_pairs_are_resolved() {
        let mut part = part_with_two_notes();
        if let Some(note) = part.measures[0].voice_mut(1).events[0].notes_mut().first_mut() {
            note.slurs.push(SlurMark::new(1, SlurKind::Start));
        }
        if let Some(note) = part.measures[0].voice_mut(1).events[1].notes_mut().first_mut() {
            note.slurs.push(SlurMark::new(1, SlurKind::Stop));
        }
        let resolution = resolve_slurs(&part);
        assert_eq!(resolution.spans.len(), 1);
        assert!(resolution.problems.is_empty());
        assert_eq!(resolution.spans[0].start.event, 0);
        assert_eq!(resolution.spans[0].end.event, 1);
    }

    #[test]
    fn unmatched_slur_is_reported() {
        let mut part = part_with_two_notes();
        if let Some(note) = part.measures[0].voice_mut(1).events[0].notes_mut().first_mut() {
            note.slurs.push(SlurMark::new(2, SlurKind::Stop));
        }
        let resolution = resolve_slurs(&part);
        assert_eq!(resolution.problems.len(), 1);
        assert_eq!(
            resolution.problems[0].kind,
            SpanProblemKind::StopWithoutStart
        );
    }

    #[test]
    fn ties_are_paired_by_pitch() {
        let mut part = part_with_two_notes();
        if let Some(note) = part.measures[0].voice_mut(1).events[0].notes_mut().first_mut() {
            note.pitch = Pitch::parse("C4").unwrap();
            note.tie = Tie::new(true, false);
        }
        if let Some(note) = part.measures[0].voice_mut(1).events[1].notes_mut().first_mut() {
            note.pitch = Pitch::parse("C4").unwrap();
            note.tie = Tie::new(false, true);
        }
        let resolution = resolve_ties(&part);
        assert_eq!(resolution.spans.len(), 1);
        assert!(resolution.problems.is_empty());
    }

    #[test]
    fn beam_runs_stop_at_rests() {
        let mut part = Part::new("p", "Piano").with_measures(1, Attributes::default());
        part.measures[0].push_note(1, Pitch::parse("C4").unwrap(), Duration::eighth());
        part.measures[0].push_note(1, Pitch::parse("D4").unwrap(), Duration::eighth());
        part.measures[0].push_rest(1, Duration::eighth());
        part.measures[0].push_note(1, Pitch::parse("E4").unwrap(), Duration::eighth());
        part.measures[0].push_note(1, Pitch::parse("F4").unwrap(), Duration::eighth());
        let runs = beam_runs(&part, 0, 1);
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].start_event, 0);
        assert_eq!(runs[0].end_event, 1);
        assert_eq!(runs[1].start_event, 3);
    }

    #[test]
    fn single_note_is_not_a_beam_run() {
        let mut part = Part::new("p", "Piano").with_measures(1, Attributes::default());
        part.measures[0].push_note(1, Pitch::parse("C4").unwrap(), Duration::eighth());
        part.measures[0].push_rest(1, Duration::eighth());
        assert!(beam_runs(&part, 0, 1).is_empty());
    }

    #[test]
    fn lyric_lines_are_counted() {
        let mut part = part_with_two_notes();
        let mut note = Note::new(Pitch::parse("C4").unwrap());
        note.lyrics.push(crate::score::marks::Lyric {
            number: 2,
            text: "la".to_string(),
            syllabic: crate::score::marks::Syllabic::Single,
            extend: false,
        });
        part.measures[0].voice_mut(1).events[1].add_note(note);
        assert_eq!(lyric_line_count(&part), 2);
    }
}
