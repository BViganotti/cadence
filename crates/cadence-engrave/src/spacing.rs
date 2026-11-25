//! Horizontal spacing: duration-aware columns shared by all voices.
//!
//! A measure is divided at every event start/end tick. Each segment receives a
//! width from its duration and from the widest glyph starting there. Voices
//! share the resulting columns, which keeps vertical time alignment and makes
//! horizontal collisions impossible by construction.

use crate::glyphs;
use crate::options::Options;
use crate::text;
use cadence_core::{Attributes, Event, Measure};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Column {
    pub start_tick: u64,
    pub end_tick: u64,
    pub x: f64,
    pub width: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeasureSpacing {
    pub width: f64,
    pub columns: Vec<Column>,
    pub capacity: u64,
}

impl MeasureSpacing {
    pub fn column_for_tick(&self, tick: u64) -> usize {
        match self
            .columns
            .binary_search_by(|column| column.start_tick.cmp(&tick))
        {
            Ok(index) => index,
            Err(index) => index.saturating_sub(1),
        }
        .min(self.columns.len().saturating_sub(1))
    }

    pub fn x_for_tick(&self, tick: u64) -> f64 {
        let index = self.column_for_tick(tick);
        self.columns
            .get(index)
            .map(|column| column.x)
            .unwrap_or(0.0)
    }
}

pub fn duration_width(beats: f64, options: &Options) -> f64 {
    let beats = beats.max(0.05);
    options.min_event_width + options.duration_scale * beats.powf(0.58)
}

/// Required width from the event anchor to the right edge plus left padding.
pub fn event_required_width(event: &Event, divisions: u32, options: &Options) -> f64 {
    let s = options.staff_space;
    let mut left = 2.0 * s;
    if event.is_chord() {
        left += event.notes().len().saturating_sub(1) as f64 * 0.95 * s;
    }
    let mut right = 0.75 * s;
    if event.is_rest() {
        // Multi-voice rests are offset to the right; reserve room for that.
        right = right.max(2.8 * s);
    } else {
        let dots = event.duration.dots as f64 * 0.55 * s;
        let note_offsets = event.notes().len().saturating_sub(1) as f64 * 0.55 * s;
        let chord_width = if event.is_chord() { 1.6 * s } else { 0.0 };
        right = right.max(1.75 * s + note_offsets + dots + 0.25 * s);
        right = right.max(chord_width + 0.95 * s + note_offsets + dots + 0.25 * s);
        if event.duration.is_compound() && event.beams.is_empty() {
            right += 0.9 * s;
        }
        for note in event.notes() {
            for lyric in &note.lyrics {
                let hyphen = lyric.syllabic.needs_hyphen();
                let width = text::lyric_width(&lyric.text, options.lyric_size, hyphen);
                right = right.max(width + 0.35 * s);
            }
        }
    }
    if let Some(dynamic) = event.dynamics {
        let width = text::width(glyphs::dynamic_text(dynamic), options.text_size * 1.05);
        right = right.max(width + 0.3 * s);
    }
    // Duration only affects the amount of space after the event; the glyph
    // itself is compact.
    let _ = divisions;
    left + right
}

pub fn space_measure(measure: &Measure, options: &Options) -> MeasureSpacing {
    let divisions = measure.attributes.divisions;
    let capacity = measure.capacity_ticks();
    let mut boundaries: Vec<u64> = vec![0, capacity];
    let mut starts: Vec<(u64, f64, bool)> = Vec::new();
    for (voice_index, voice) in measure.voices.iter().enumerate() {
        let mut tick = 0u64;
        for event in &voice.events {
            boundaries.push(tick);
            let next = tick + event.duration.ticks(divisions);
            boundaries.push(next);
            starts.push((
                tick,
                event_required_width(event, divisions, options),
                voice_index > 0,
            ));
            tick = next;
        }
    }
    boundaries.sort_unstable();
    boundaries.dedup();
    if boundaries.len() < 2 {
        boundaries = vec![0, capacity.max(1)];
    }
    let mut columns = Vec::with_capacity(boundaries.len() - 1);
    let mut x = 0.0;
    for window in boundaries.windows(2) {
        let (start, end) = (window[0], window[1]);
        let beats = (end - start) as f64 / divisions as f64;
        let mut width = 4.0f64;
        let mut has_start = false;
        for (start_tick, required, lower_voice) in &starts {
            if *start_tick == start {
                has_start = true;
                width = width.max(*required + if *lower_voice { 4.0 * options.staff_space } else { 0.0 });
            }
        }
        if has_start {
            width = width.max(duration_width(beats, options));
        } else {
            width = width.max(4.0);
        }
        columns.push(Column {
            start_tick: start,
            end_tick: end,
            x,
            width,
        });
        x += width;
    }
    let width = (x + options.barline_width + 0.35 * options.staff_space).max(52.0);
    MeasureSpacing {
        width,
        columns,
        capacity,
    }
}

/// Clef, key and time signature prefix used at the start of a system.
pub fn prefix_width(attributes: Attributes, options: &Options) -> f64 {
    let s = options.staff_space;
    let clef = glyphs::clef_width(attributes.clef, s);
    let key = attributes.key.accidentals().len() as f64 * 1.35 * s
        + if attributes.key.accidentals().is_empty() {
            0.0
        } else {
            0.5 * s
        };
    let time = glyphs::time_signature_width(attributes.time, s);
    clef + key + time + 1.1 * s
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadence_core::{Attributes, Duration, Event, Measure, Pitch};

    fn simple_measure() -> Measure {
        let mut measure = Measure::new(1, Attributes::default());
        measure.push_note(1, Pitch::parse("C4").unwrap(), Duration::quarter());
        measure.push_note(1, Pitch::parse("D4").unwrap(), Duration::quarter());
        measure.push_rest(1, Duration::half());
        measure
    }

    #[test]
    fn columns_cover_measure_and_align_voices() {
        let options = Options::default();
        let measure = simple_measure();
        let spacing = space_measure(&measure, &options);
        assert_eq!(spacing.capacity, 1920);
        assert_eq!(spacing.columns.first().unwrap().start_tick, 0);
        assert_eq!(spacing.columns.last().unwrap().end_tick, 1920);
        assert!(spacing.width > 52.0);
        let x0 = spacing.x_for_tick(0);
        let x1 = spacing.x_for_tick(480);
        assert!(x1 > x0);
    }

    #[test]
    fn longer_durations_get_more_space() {
        let options = Options::default();
        assert!(
            duration_width(2.0, &options) > duration_width(1.0, &options)
        );
        assert!(
            duration_width(1.0, &options) > duration_width(0.5, &options)
        );
    }

    #[test]
    fn lyrics_widen_the_slot() {
        let options = Options::default();
        let mut measure = Measure::new(1, Attributes::default());
        let mut event = Event::note(Pitch::parse("C4").unwrap(), Duration::eighth());
        event.notes_mut()[0]
            .lyrics
            .push(cadence_core::Lyric::new("extraordinary"));
        measure.push_event(1, event);
        let with_lyric = space_measure(&measure, &options).width;
        let mut measure2 = Measure::new(1, Attributes::default());
        measure2.push_note(1, Pitch::parse("C4").unwrap(), Duration::eighth());
        let without = space_measure(&measure2, &options).width;
        assert!(with_lyric > without);
    }

    #[test]
    fn prefix_includes_clef_key_and_time() {
        let options = Options::default();
        let plain = prefix_width(Attributes::default(), &options);
        let mut keyed = Attributes::default();
        keyed.key = cadence_core::Key::new(4, cadence_core::KeyMode::Major);
        assert!(prefix_width(keyed, &options) > plain);
    }
}
