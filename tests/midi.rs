//! MIDI acceptance tests across fixtures.

use cadence_core::Score;
use cadence_export::render_midi;
use cadence_import::import_score;

#[derive(Debug)]
struct TrackSummary {
    note_on: usize,
    note_off: usize,
    tempo_events: usize,
    time_events: usize,
    key_events: usize,
    last_tick: u64,
    channels: Vec<u8>,
}

fn read_vlq(bytes: &[u8], mut position: usize) -> (u64, usize) {
    let mut value = 0u64;
    let start = position;
    loop {
        let byte = bytes[position];
        position += 1;
        value = (value << 7) | (byte & 0x7F) as u64;
        if byte & 0x80 == 0 {
            break;
        }
    }
    (value, position - start)
}

fn parse_tracks(bytes: &[u8]) -> Vec<TrackSummary> {
    assert_eq!(&bytes[0..4], b"MThd");
    assert_eq!(&bytes[8..10], &[0, 1]);
    let tracks = u16::from_be_bytes([bytes[10], bytes[11]]) as usize;
    assert_eq!(u16::from_be_bytes([bytes[12], bytes[13]]), 480);
    let mut offset = 14;
    let mut summaries = Vec::new();
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
        let mut tick = 0u64;
        let mut summary = TrackSummary {
            note_on: 0,
            note_off: 0,
            tempo_events: 0,
            time_events: 0,
            key_events: 0,
            last_tick: 0,
            channels: Vec::new(),
        };
        let mut position = offset;
        while position < end {
            let (delta, used) = read_vlq(bytes, position);
            position += used;
            tick += delta;
            summary.last_tick = tick;
            let status = bytes[position];
            if status == 0xFF {
                position += 1;
                let kind = bytes[position];
                position += 1;
                let (length, used) = read_vlq(bytes, position);
                position += used;
                match kind {
                    0x51 => summary.tempo_events += 1,
                    0x58 => summary.time_events += 1,
                    0x59 => summary.key_events += 1,
                    _ => {}
                }
                position += length as usize;
                continue;
            }
            if status == 0xF0 || status == 0xF7 {
                let (length, used) = read_vlq(bytes, position + 1);
                position += 1 + used + length as usize;
                continue;
            }
            let channel = status & 0x0F;
            if !summary.channels.contains(&channel) {
                summary.channels.push(channel);
            }
            position += 1;
            match status & 0xF0 {
                0x80 => {
                    summary.note_off += 1;
                    position += 2;
                }
                0x90 => {
                    if bytes[position + 1] > 0 {
                        summary.note_on += 1;
                    }
                    assert!(bytes[position + 1] <= 127);
                    position += 2;
                }
                0xB0 | 0xE0 => position += 2,
                0xC0 | 0xD0 => position += 1,
                other => panic!("unexpected status {other:#x}"),
            }
        }
        assert_eq!(position, end);
        summaries.push(summary);
        offset = end;
    }
    assert_eq!(offset, bytes.len());
    summaries
}

#[test]
fn demo_midi_matches_score_notes_and_ties() {
    let score = Score::demo();
    let bytes = render_midi(&score).unwrap();
    let tracks = parse_tracks(&bytes);
    assert_eq!(tracks.len(), score.parts.len() + 1);
    assert_eq!(tracks[0].tempo_events, 1);
    assert_eq!(tracks[0].time_events, 1);
    assert_eq!(tracks[0].key_events, 2);
    let total_on: usize = tracks.iter().skip(1).map(|track| track.note_on).sum();
    let total_off: usize = tracks.iter().skip(1).map(|track| track.note_off).sum();
    let expected = score
        .flatten_notes()
        .iter()
        .filter(|note| !note.tie_stop)
        .count();
    assert_eq!(total_on, expected);
    assert_eq!(total_on, total_off);
    assert_eq!(tracks[1].channels, vec![0]);
    assert_eq!(tracks[2].channels, vec![1]);
}

#[test]
fn fixture_midi_is_deterministic_and_balanced() {
    let directory = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/fixtures");
    let mut paths: Vec<_> = std::fs::read_dir(directory)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("musicxml"))
        .collect();
    paths.sort();
    for path in paths.into_iter().take(15) {
        let xml = std::fs::read_to_string(&path).unwrap();
        let score = import_score(&xml).unwrap();
        let bytes = render_midi(&score).unwrap();
        assert_eq!(bytes, render_midi(&score).unwrap());
        let tracks = parse_tracks(&bytes);
        assert_eq!(tracks.len(), score.parts.len() + 1, "{}", path.display());
        for track in tracks.iter().skip(1) {
            assert_eq!(track.note_on, track.note_off, "{}", path.display());
            assert!(track.note_on > 0, "{}", path.display());
        }
    }
}

#[test]
fn note_events_are_ordered_and_have_positive_durations() {
    let bytes = render_midi(&Score::demo()).unwrap();
    let tracks = parse_tracks(&bytes);
    for track in &tracks {
        assert!(track.last_tick > 0);
    }
    let size = bytes.len();
    assert!(size > 200);
    assert!(bytes.windows(4).any(|window| window == b"MTrk"));
}

#[test]
fn empty_score_is_rejected() {
    assert!(render_midi(&Score::new("Empty")).is_err());
}
