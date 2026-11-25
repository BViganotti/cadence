//! Deterministic Standard MIDI File (format 1) export from the score model.
//!
//! Cadence exports written pitches, one track per part, a conductor track with
//! tempo/time/key meta events, and merged ties. No external crates or host
//! state are involved.

use cadence_core::{Error, KeyMode, Pitch, Result, Score};
use std::collections::BTreeMap;

pub const PPQ: u16 = 480;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NoteEvent {
    start: u64,
    end: u64,
    pitch: u8,
    velocity: u8,
}

#[derive(Debug, Clone)]
struct TimedMeta {
    tick: u64,
    bytes: Vec<u8>,
}

pub fn render_midi(score: &Score) -> Result<Vec<u8>> {
    score.validate()?;
    if score.parts.is_empty() {
        return Err(Error::msg("cannot export an empty score to MIDI"));
    }
    let mut tracks: Vec<Vec<u8>> = Vec::new();
    tracks.push(conductor_track(score)?);
    for part_index in 0..score.parts.len() {
        tracks.push(part_track(score, part_index)?);
    }
    let mut out = Vec::new();
    out.extend_from_slice(b"MThd");
    out.extend_from_slice(&6u32.to_be_bytes());
    out.extend_from_slice(&1u16.to_be_bytes());
    out.extend_from_slice(&(tracks.len() as u16).to_be_bytes());
    out.extend_from_slice(&PPQ.to_be_bytes());
    for track in tracks {
        out.extend_from_slice(b"MTrk");
        out.extend_from_slice(&(track.len() as u32).to_be_bytes());
        out.extend_from_slice(&track);
    }
    Ok(out)
}

fn conductor_track(score: &Score) -> Result<Vec<u8>> {
    let mut track = Vec::new();
    push_meta(&mut track, 0, 0x03, b"Cadence");
    if !score.metadata.copyright.is_empty() {
        push_meta(&mut track, 0, 0x02, score.metadata.copyright.as_bytes());
    }
    let first = &score.parts[0];
    let mut tempos: BTreeMap<u64, u32> = BTreeMap::new();
    let mut times: BTreeMap<u64, (u32, u32)> = BTreeMap::new();
    let mut keys: BTreeMap<u64, (i8, bool)> = BTreeMap::new();
    let mut start = 0u64;
    let mut previous_tempo: Option<u32> = None;
    let mut previous_time = None;
    let mut previous_key = None;
    for (measure_index, measure) in first.measures.iter().enumerate() {
        let tick = start;
        if let Some(tempo) = measure.tempo {
            if previous_tempo != Some(tempo.bpm) {
                tempos.insert(tick, tempo.bpm);
                previous_tempo = Some(tempo.bpm);
            }
        }
        let time = (measure.attributes.time.beats, measure.attributes.time.beat_type);
        if previous_time != Some(time) {
            times.insert(tick, time);
            previous_time = Some(time);
        }
        let key = (
            measure.attributes.key.fifths,
            measure.attributes.key.mode == KeyMode::Minor,
        );
        if previous_key != Some(key) {
            keys.insert(tick, key);
            previous_key = Some(key);
        }
        let divisions = measure.attributes.divisions;
        let span = cadence_core::measure_span_ticks(measure, measure_index == 0);
        start += span * PPQ as u64 / divisions.max(1) as u64;
    }
    if tempos.is_empty() {
        tempos.insert(0, 120);
    }
    let mut events: Vec<TimedMeta> = Vec::new();
    for (tick, bpm) in tempos {
        let micros = (60_000_000u64 / bpm.max(1) as u64).min(0xFF_FFFF) as u32;
        let bytes = vec![
            0xFF,
            0x51,
            0x03,
            (micros >> 16) as u8,
            (micros >> 8) as u8,
            micros as u8,
        ];
        events.push(TimedMeta { tick, bytes });
    }
    for (tick, (beats, beat_type)) in times {
        let dd = (beat_type.max(1) as f64).log2().round() as u8;
        let bytes = vec![0xFF, 0x58, 0x04, beats.min(255) as u8, dd, 24, 8];
        events.push(TimedMeta { tick, bytes });
    }
    for (tick, (fifths, minor)) in keys {
        let bytes = vec![
            0xFF,
            0x59,
            0x02,
            fifths.clamp(-7, 7) as i8 as u8,
            u8::from(minor),
        ];
        events.push(TimedMeta { tick, bytes });
    }
    events.sort_by_key(|event| (event.tick, event.bytes[1]));
    let mut last = 0u64;
    for event in events {
        let delta = event.tick.saturating_sub(last);
        // The meta event itself already starts with 0xFF.
        write_vlq(&mut track, delta);
        track.extend_from_slice(&event.bytes);
        last = event.tick;
    }
    write_vlq(&mut track, 0);
    track.extend_from_slice(&[0xFF, 0x2F, 0x00]);
    Ok(track)
}

fn part_track(score: &Score, part_index: usize) -> Result<Vec<u8>> {
    let part = &score.parts[part_index];
    let channel = part.midi_channel.min(15);
    let mut notes: Vec<NoteEvent> = Vec::new();
    let mut measure_start = 0u64;
    let mut open_ties: Vec<(Pitch, usize)> = Vec::new();
    for (measure_index, measure) in part.measures.iter().enumerate() {
        let divisions = measure.attributes.divisions.max(1);
        for voice in &measure.voices {
            let mut offset = 0u64;
            for event in &voice.events {
                let duration = event.duration.ticks(divisions) * PPQ as u64 / divisions as u64;
                let start = measure_start + offset * PPQ as u64 / divisions as u64;
                for note in event.notes() {
                    let velocity = event
                        .dynamics
                        .map(|dynamic| dynamic.velocity())
                        .unwrap_or_else(|| note.effective_velocity())
                        .clamp(1, 127);
                    let Some(pitch) = note.pitch.midi() else {
                        return Err(Error::out_of_range(format!(
                            "pitch {} has no MIDI number",
                            note.pitch.name()
                        )));
                    };
                    if note.tie.stop {
                        if let Some(position) = open_ties
                            .iter()
                            .position(|(pitch_value, _)| *pitch_value == note.pitch)
                        {
                            let (_, note_index) = open_ties.remove(position);
                            if let Some(existing) = notes.get_mut(note_index) {
                                existing.end = start + duration;
                            }
                            if note.tie.start {
                                open_ties.push((note.pitch, note_index));
                            }
                            continue;
                        }
                    }
                    let note_event = NoteEvent {
                        start,
                        end: start + duration.max(1),
                        pitch,
                        velocity,
                    };
                    notes.push(note_event);
                    if note.tie.start {
                        open_ties.push((note.pitch, notes.len() - 1));
                    }
                }
                offset += event.duration.ticks(divisions);
            }
        }
        let span = cadence_core::measure_span_ticks(measure, measure_index == 0);
        measure_start += span * PPQ as u64 / divisions as u64;
    }
    let track_name = if part.name.is_empty() {
        format!("Part {}", part_index + 1)
    } else {
        part.name.clone()
    };
    let mut track = Vec::new();
    push_meta(&mut track, 0, 0x03, track_name.as_bytes());
    write_vlq(&mut track, 0);
    track.extend_from_slice(&[0xC0 | channel, part.midi_program.min(127)]);
    // Encode note-on and note-off events; at a shared tick note-offs sort
    // before note-ons to avoid stuck notes.
    let mut encoded: Vec<(u64, u8, [u8; 3], usize)> = Vec::new();
    for note in &notes {
        if note.start >= note.end {
            continue;
        }
        encoded.push((
            note.start,
            1,
            [0x90 | channel, note.pitch, note.velocity],
            3,
        ));
        encoded.push((note.end, 0, [0x80 | channel, note.pitch, 0], 3));
    }
    encoded.sort_by_key(|(tick, priority, bytes, _)| (*tick, *priority, bytes[1]));
    let mut last = 0u64;
    for (tick, _priority, bytes, len) in encoded {
        write_vlq(&mut track, tick.saturating_sub(last));
        track.extend_from_slice(&bytes[..len]);
        last = tick;
    }
    write_vlq(&mut track, 0);
    track.extend_from_slice(&[0xFF, 0x2F, 0x00]);
    Ok(track)
}

fn push_meta(track: &mut Vec<u8>, delta: u64, kind: u8, data: &[u8]) {
    write_vlq(track, delta);
    track.push(0xFF);
    track.push(kind);
    let len = data.len().min(0x0FFF_FFFF);
    write_vlq(track, len as u64);
    track.extend_from_slice(&data[..len]);
}

pub fn write_vlq(out: &mut Vec<u8>, mut value: u64) {
    let mut buffer = [0u8; 10];
    let mut index = 9;
    buffer[index] = (value & 0x7F) as u8;
    value >>= 7;
    while value > 0 {
        index -= 1;
        buffer[index] = ((value & 0x7F) as u8) | 0x80;
        value >>= 7;
    }
    out.extend_from_slice(&buffer[index..]);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_header(bytes: &[u8]) -> (u16, u16) {
        assert_eq!(&bytes[0..4], b"MThd");
        assert_eq!(&bytes[4..8], &6u32.to_be_bytes());
        let format = u16::from_be_bytes([bytes[8], bytes[9]]);
        let tracks = u16::from_be_bytes([bytes[10], bytes[11]]);
        let division = u16::from_be_bytes([bytes[12], bytes[13]]);
        assert_eq!(format, 1);
        assert_eq!(division, PPQ);
        (tracks, division)
    }

    #[test]
    fn header_and_tracks_are_well_formed() {
        let bytes = render_midi(&Score::demo()).unwrap();
        let (tracks, _) = parse_header(&bytes);
        assert_eq!(tracks, 3);
        let mut offset = 14;
        for _ in 0..tracks {
            assert_eq!(&bytes[offset..offset + 4], b"MTrk");
            let length = u32::from_be_bytes([
                bytes[offset + 4],
                bytes[offset + 5],
                bytes[offset + 6],
                bytes[offset + 7],
            ]) as usize;
            offset += 8 + length;
        }
        assert_eq!(offset, bytes.len());
    }

    #[test]
    fn note_events_and_ties_are_exported() {
        let bytes = render_midi(&Score::demo()).unwrap();
        let (tracks, _) = parse_header(&bytes);
        let mut offset = 14;
        let mut note_on = 0;
        let mut note_off = 0;
        let mut tempo = false;
        for _ in 0..tracks {
            assert_eq!(&bytes[offset..offset + 4], b"MTrk");
            let length = u32::from_be_bytes([
                bytes[offset + 4],
                bytes[offset + 5],
                bytes[offset + 6],
                bytes[offset + 7],
            ]) as usize;
            offset += 8;
            let end = offset + length;
            let mut pos = offset;
            while pos < end {
                let (_, used) = read_vlq(&bytes[pos..]);
                pos += used;
                let status = bytes[pos];
                if status == 0xFF {
                    pos += 1;
                    let kind = bytes[pos];
                    pos += 1;
                    let (len, used) = read_vlq(&bytes[pos..]);
                    pos += used + len as usize;
                    if kind == 0x51 {
                        tempo = true;
                    }
                    continue;
                }
                pos += 1;
                match status & 0xF0 {
                    0x80 => {
                        note_off += 1;
                        pos += 2;
                    }
                    0x90 => {
                        if bytes[pos + 1] > 0 {
                            note_on += 1;
                        }
                        pos += 2;
                    }
                    0xB0 | 0xE0 => pos += 2,
                    0xC0 | 0xD0 => pos += 1,
                    other => panic!("unexpected status {other:#x}"),
                }
            }
            offset = end;
        }
        assert!(note_on > 0);
        assert_eq!(note_on, note_off);
        assert!(tempo);
    }

    fn read_vlq(bytes: &[u8]) -> (u64, usize) {
        let mut value = 0u64;
        let mut used = 0;
        for byte in bytes {
            used += 1;
            value = (value << 7) | (byte & 0x7F) as u64;
            if byte & 0x80 == 0 {
                break;
            }
        }
        (value, used)
    }

    #[test]
    fn export_is_deterministic() {
        let score = Score::demo();
        assert_eq!(render_midi(&score).unwrap(), render_midi(&score).unwrap());
    }

    #[test]
    fn vlq_round_trips() {
        let values = [0u64, 1, 127, 128, 255, 16383, 16384, 0x0FFF_FFFF];
        for value in values {
            let mut out = Vec::new();
            write_vlq(&mut out, value);
            assert!(!out.is_empty());
            let mut decoded = 0u64;
            for byte in out {
                decoded = (decoded << 7) | (byte & 0x7F) as u64;
            }
            assert_eq!(decoded, value);
        }
    }

    #[test]
    fn key_and_time_meta_are_present() {
        let bytes = render_midi(&Score::demo()).unwrap();
        assert!(bytes.windows(2).any(|window| window == [0xFF, 0x58]));
        assert!(bytes.windows(2).any(|window| window == [0xFF, 0x59]));
    }

    #[test]
    fn empty_score_is_rejected() {
        assert!(render_midi(&Score::new("empty")).is_err());
    }
}
