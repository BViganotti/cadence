//! Beam grouping and beam geometry helpers.

use cadence_core::{BeamKind, Duration, Measure, NoteValue};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BeamGroup {
    pub start_event: usize,
    pub end_event: usize,
}

pub fn beam_groups(measure: &Measure, voice_number: u8) -> Vec<BeamGroup> {
    let Some(voice) = measure.voice(voice_number) else {
        return Vec::new();
    };
    let events = &voice.events;
    let mut used = vec![false; events.len()];
    let mut groups = Vec::new();

    // Explicit beam pairs take priority.
    for (index, event) in events.iter().enumerate() {
        if !event.beams.iter().any(|beam| beam.number == 1 && beam.kind == BeamKind::Begin) {
            continue;
        }
        for end in (index + 1)..events.len() {
            if !is_beamable(&events[end]) {
                break;
            }
            if events[end]
                .beams
                .iter()
                .any(|beam| beam.number == 1 && beam.kind == BeamKind::End)
            {
                if end - index + 1 >= 2 {
                    for flag in used.iter_mut().take(end + 1).skip(index) {
                        *flag = true;
                    }
                    groups.push(BeamGroup {
                        start_event: index,
                        end_event: end,
                    });
                }
                break;
            }
        }
    }

    // Automatic groups, split at time-signature beat boundaries.
    let divisions = measure.attributes.divisions;
    let time = measure.attributes.time;
    let unit = divisions as u64 * 4 / time.beat_type.max(1) as u64;
    let mut boundaries = Vec::new();
    let mut accumulated = 0u64;
    for count in time.beam_groups() {
        accumulated += count as u64 * unit;
        boundaries.push(accumulated);
    }
    let mut starts = Vec::with_capacity(events.len());
    let mut tick = 0u64;
    for event in events {
        starts.push(tick);
        tick += event.duration.ticks(divisions);
    }
    let mut index = 0usize;
    while index < events.len() {
        if used[index] || !is_beamable(&events[index]) {
            index += 1;
            continue;
        }
        let mut end = index;
        let mut boundary = boundary_for(starts[index], &boundaries);
        while end + 1 < events.len()
            && !used[end + 1]
            && is_beamable(&events[end + 1])
            && starts[end + 1] < boundary
        {
            end += 1;
        }
        if end > index {
            for flag in used.iter_mut().take(end + 1).skip(index) {
                *flag = true;
            }
            groups.push(BeamGroup {
                start_event: index,
                end_event: end,
            });
        } else {
            // A single continuous run may still span a beat boundary; start a
            // new attempt at the next event.
            let _ = boundary;
            boundary = boundary_for(starts[index + 1], &boundaries);
            let _ = boundary;
        }
        index = end + 1;
    }
    groups.sort_by_key(|group| group.start_event);
    groups
}

fn is_beamable(event: &cadence_core::Event) -> bool {
    !event.is_rest() && event.duration.is_compound()
}

fn boundary_for(tick: u64, boundaries: &[u64]) -> u64 {
    for boundary in boundaries {
        if tick < *boundary {
            return *boundary;
        }
    }
    u64::MAX
}

pub fn beam_levels(duration: Duration) -> u8 {
    match duration.value {
        NoteValue::Eighth => 1,
        NoteValue::Sixteenth => 2,
        NoteValue::ThirtySecond => 3,
        NoteValue::SixtyFourth => 4,
        NoteValue::HundredTwentyEighth => 5,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadence_core::{Attributes, Beam, Measure, Pitch, TimeSignature};

    fn pitch(name: &str) -> Pitch {
        Pitch::parse(name).unwrap()
    }

    fn measure_with_eighths(count: usize, time: TimeSignature) -> Measure {
        let mut attributes = Attributes::default();
        attributes.time = time;
        let mut measure = Measure::new(1, attributes);
        for index in 0..count {
            measure.push_note(1, pitch(if index % 2 == 0 { "C4" } else { "D4" }), Duration::eighth());
        }
        measure
    }

    #[test]
    fn auto_beams_four_eighths_in_common_time() {
        let measure = measure_with_eighths(8, TimeSignature::new(4, 4));
        let groups = beam_groups(&measure, 1);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].start_event, 0);
        assert_eq!(groups[0].end_event, 3);
        assert_eq!(groups[1].start_event, 4);
        assert_eq!(groups[1].end_event, 7);
    }

    #[test]
    fn compound_time_groups_by_three() {
        let measure = measure_with_eighths(6, TimeSignature::new(6, 8));
        let groups = beam_groups(&measure, 1);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].end_event, 2);
        assert_eq!(groups[1].start_event, 3);
        assert_eq!(groups[1].end_event, 5);
    }

    #[test]
    fn rests_and_quarters_break_runs() {
        let mut measure = Measure::new(1, Attributes::default());
        measure.push_note(1, pitch("C4"), Duration::eighth());
        measure.push_note(1, pitch("D4"), Duration::eighth());
        measure.push_rest(1, Duration::eighth());
        measure.push_note(1, pitch("E4"), Duration::quarter());
        measure.push_note(1, pitch("F4"), Duration::eighth());
        measure.push_note(1, pitch("G4"), Duration::eighth());
        let groups = beam_groups(&measure, 1);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].start_event, 0);
        assert_eq!(groups[0].end_event, 1);
        assert_eq!(groups[1].start_event, 4);
        assert_eq!(groups[1].end_event, 5);
    }

    #[test]
    fn explicit_begin_end_wins() {
        let mut measure = measure_with_eighths(8, TimeSignature::new(4, 4));
        let voice = measure.voice_mut(1);
        voice.events[0].beams = vec![Beam::new(1, BeamKind::Begin)];
        voice.events[1].beams = vec![Beam::new(1, BeamKind::End)];
        let groups = beam_groups(&measure, 1);
        assert!(groups.contains(&BeamGroup {
            start_event: 0,
            end_event: 1
        }));
    }

    #[test]
    fn single_eighth_is_not_beamed() {
        let mut measure = Measure::new(1, Attributes::default());
        measure.push_note(1, pitch("C4"), Duration::eighth());
        measure.push_rest(1, Duration::dotted(NoteValue::Half));
        assert!(beam_groups(&measure, 1).is_empty());
    }
}
