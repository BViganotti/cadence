//! MusicXML import for the supported Cadence subset.
//!
//! The importer is strict by design. Unsupported constructs that would change
//! the meaning of the score (tuplets, grace notes, multiple staves per part,
//! composite time signatures) are rejected with a clear error rather than
//! silently dropped. Purely presentational elements that Cadence engraves
//! itself (stem direction, default x/y, page layout) are ignored.

use cadence_core::notation::{resolve_slurs, resolve_ties};
use cadence_core::score::attributes::{
    Attributes, Clef, ClefSign, Key, KeyMode, TimeSignature, TimeSymbol,
};
use cadence_core::score::duration::{Duration, NoteValue};
use cadence_core::score::marks::{
    Accidental, Articulation, Beam, BeamKind, Dynamic, Lyric, SlurKind, SlurMark, Syllabic,
};
use cadence_core::score::measure::{
    BarlineStyle, Ending, EndingKind, Measure, Repeat, RepeatDirection, Tempo,
};
use cadence_core::score::note::{Event, EventKind, Note};
use cadence_core::score::part::Part;
use cadence_core::score::pitch::{Pitch, Step};
use cadence_core::score::score::{Metadata, Score};
use cadence_core::{Error, Result};

use crate::xml::{self, Element};

pub const MAX_MEASURES: usize = 10_000;
pub const MAX_PARTS: usize = 64;

#[derive(Debug, Clone, Default)]
struct PartInfo {
    id: String,
    name: String,
    abbreviation: String,
    program: Option<u8>,
    channel: Option<u8>,
}

#[derive(Debug)]
struct ParsedNote {
    note: Note,
    is_rest: bool,
    chord: bool,
    duration: Duration,
    ticks: u64,
    voice: u8,
    beams: Vec<Beam>,
}

#[derive(Debug, Clone, Copy)]
struct PendingDynamic {
    cursor: u64,
    dynamic: Dynamic,
    voice: u8,
}

pub fn import_score(input: &str) -> Result<Score> {
    if input.len() > xml::MAX_XML_BYTES {
        return Err(Error::Limit("MusicXML input bytes"));
    }
    let document = xml::parse(input)?;
    let root = &document.root;
    if root.local_name() != "score-partwise" {
        return Err(Error::msg(format!(
            "unsupported MusicXML root <{}>; only score-partwise is supported",
            root.local_name()
        )));
    }
    let metadata = parse_metadata(root);
    let infos = parse_part_list(root)?;
    let part_elements: Vec<&Element> = root.children_named("part").collect();
    if part_elements.len() > MAX_PARTS {
        return Err(Error::Limit("parts in MusicXML"));
    }
    if part_elements.is_empty() {
        return Err(Error::msg("MusicXML contains no parts"));
    }
    let mut score = Score {
        metadata,
        parts: Vec::new(),
    };
    let mut used_ids = Vec::new();
    for (index, element) in part_elements.iter().enumerate() {
        let id = element
            .attr("id")
            .map(|value| value.to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| format!("P{}", index + 1));
        if used_ids.contains(&id) {
            return Err(Error::msg(format!("duplicate part id {id:?}")));
        }
        used_ids.push(id.clone());
        let info = infos.iter().find(|info| info.id == id);
        let mut part = Part::new(
            id,
            info.map(|info| info.name.clone())
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| format!("Part {}", index + 1)),
        );
        if let Some(info) = info {
            part.abbreviation = info.abbreviation.clone();
            part.midi_program = info.program.unwrap_or(0);
            part.midi_channel = info.channel.unwrap_or((index % 16) as u8);
        } else {
            part.midi_channel = (index % 16) as u8;
        }
        let measures = element.children_named("measure").count();
        if measures > MAX_MEASURES {
            return Err(Error::Limit("measures in MusicXML part"));
        }
        let mut attributes = Attributes::default();
        for (measure_index, measure_element) in element.children_named("measure").enumerate() {
            let number = measure_element
                .attr("number")
                .and_then(|text| text.parse::<usize>().ok())
                .filter(|number| *number > 0)
                .unwrap_or(measure_index + 1);
            let measure =
                parse_measure(measure_element, number, measure_index, &mut attributes)?;
            part.measures.push(measure);
        }
        score.parts.push(part);
    }
    score.validate()?;
    // Reject scores whose musical invariants are structurally broken. Warnings
    // (for example an underfull final measure) remain allowed.
    let report = score.inspect();
    if !report.is_ok() {
        let first = report
            .issues
            .iter()
            .find(|issue| issue.severity == cadence_core::Severity::Error)
            .map(|issue| issue.message.clone())
            .unwrap_or_else(|| "invalid score".to_string());
        return Err(Error::msg(first));
    }
    // Validate tie/slur pairing but do not reject: malformed pairing is a
    // notation warning and the engraver can still draw the rest of the score.
    for part in &score.parts {
        let _ = resolve_slurs(part);
        let _ = resolve_ties(part);
    }
    Ok(score)
}

fn parse_metadata(root: &Element) -> Metadata {
    let mut metadata = Metadata::default();
    metadata.title = root
        .child("work")
        .and_then(|work| work.child_text("work-title"))
        .or_else(|| root.child_text("movement-title"))
        .unwrap_or("Untitled")
        .to_string();
    if let Some(movement) = root.child_text("movement-title") {
        if !movement.is_empty() && movement != metadata.title {
            metadata.subtitle = movement.to_string();
        }
    }
    if let Some(identification) = root.child("identification") {
        for creator in identification.children_named("creator") {
            let text = creator.text_trimmed();
            if text.is_empty() {
                continue;
            }
            match creator.attr("type").unwrap_or("") {
                "composer" => metadata.composer = text.to_string(),
                "lyricist" | "poet" => metadata.lyricist = text.to_string(),
                "arranger" => metadata.arranger = text.to_string(),
                _ => {}
            }
        }
        if let Some(rights) = identification.child_text("rights") {
            metadata.copyright = rights.to_string();
        }
        if let Some(source) = identification.child_text("source") {
            metadata.source = source.to_string();
        }
    }
    if metadata.title == "Untitled" {
        if let Some(credit) = root.child("credit") {
            if let Some(words) = credit.child_text("credit-words") {
                if !words.is_empty() {
                    metadata.title = words.to_string();
                }
            }
        }
    }
    metadata
}

fn parse_part_list(root: &Element) -> Result<Vec<PartInfo>> {
    let mut infos = Vec::new();
    let Some(part_list) = root.child("part-list") else {
        return Ok(infos);
    };
    for score_part in part_list.children_named("score-part") {
        let id = score_part
            .attr("id")
            .ok_or_else(|| Error::msg("score-part missing id"))?
            .to_string();
        if id.is_empty() {
            return Err(Error::msg("score-part has empty id"));
        }
        if infos.iter().any(|info: &PartInfo| info.id == id) {
            return Err(Error::msg(format!("duplicate score-part id {id:?}")));
        }
        let name = score_part
            .child_text("part-name")
            .unwrap_or("")
            .to_string();
        let abbreviation = score_part
            .child_text("part-abbreviation")
            .unwrap_or("")
            .to_string();
        let mut program = None;
        let mut channel = None;
        if let Some(instrument) = score_part.child("midi-instrument") {
            if let Some(text) = instrument.child_text("midi-program") {
                let value = parse_u32(text, "midi-program")?;
                if !(1..=128).contains(&value) {
                    return Err(Error::out_of_range(format!("midi-program {value}")));
                }
                program = Some((value - 1) as u8);
            }
            if let Some(text) = instrument.child_text("midi-channel") {
                let value = parse_u32(text, "midi-channel")?;
                if !(1..=16).contains(&value) {
                    return Err(Error::out_of_range(format!("midi-channel {value}")));
                }
                channel = Some((value - 1) as u8);
            }
        }
        infos.push(PartInfo {
            id,
            name,
            abbreviation,
            program,
            channel,
        });
    }
    Ok(infos)
}

fn parse_measure(
    element: &Element,
    number: usize,
    measure_index: usize,
    attributes: &mut Attributes,
) -> Result<Measure> {
    let mut measure = Measure::new(number, *attributes);
    let mut cursor: i64 = 0;
    let mut voice_fill: Vec<(u8, u64)> = Vec::new();
    let mut last_event: Vec<(u8, usize)> = Vec::new();
    let mut pending_dynamics: Vec<PendingDynamic> = Vec::new();
    let mut attributes_seen = false;
    let mut last_note_voice: Option<u8> = None;

    for child in &element.children {
        match child.local_name() {
            "attributes" => {
                if attributes_seen && (cursor > 0 || measure.event_count() > 0) {
                    return Err(Error::msg(format!(
                        "measure {number}: attributes after notes are not supported"
                    )));
                }
                parse_attributes(child, attributes)?;
                attributes_seen = true;
                measure.attributes = *attributes;
                measure.explicit_attributes = true;
            }
            "note" => {
                let parsed = parse_note(child, attributes.divisions, &measure)?;
                if parsed.chord {
                    let index = last_event
                        .iter()
                        .find(|(voice, _)| *voice == parsed.voice)
                        .map(|(_, index)| *index)
                        .ok_or_else(|| {
                            Error::msg(format!("measure {number}: chord note without base note"))
                        })?;
                    let base = measure
                        .voice_mut(parsed.voice)
                        .events
                        .get_mut(index)
                        .ok_or_else(|| Error::msg("internal chord index"))?;
                    if base.duration.ticks(attributes.divisions) != parsed.ticks {
                        return Err(Error::msg(format!(
                            "measure {number}: chord note duration differs from base note"
                        )));
                    }
                    match &mut base.kind {
                        EventKind::Pitched(notes) => {
                            notes.push(parsed.note);
                            notes.sort_by_key(|note| {
                                (note.pitch.diatonic_index(), note.pitch.chromatic_index())
                            });
                        }
                        EventKind::Rest => {
                            return Err(Error::msg(format!(
                                "measure {number}: chord note follows a rest"
                            )))
                        }
                    }
                    continue;
                }
                let fill = voice_fill_value(&voice_fill, parsed.voice);
                if cursor < 0 {
                    return Err(Error::msg(format!(
                        "measure {number}: cursor became negative"
                    )));
                }
                if (cursor as u64) < fill {
                    return Err(Error::msg(format!(
                        "measure {number}: voice {} overlaps at tick {cursor}",
                        parsed.voice
                    )));
                }
                if cursor as u64 > fill {
                    let gap = cursor as u64 - fill;
                    let gap_duration = Duration::from_ticks(attributes.divisions, gap)
                        .ok_or_else(|| {
                            Error::msg(format!(
                                "measure {number}: voice {} starts after a gap of {gap} ticks that cannot be written with supported durations",
                                parsed.voice
                            ))
                        })?;
                    measure
                        .voices
                        .iter_mut()
                        .find(|voice| voice.number == parsed.voice)
                        .map(|voice| {
                            voice.events.push(Event::rest(gap_duration));
                        });
                    if measure.voice(parsed.voice).is_none() {
                        measure.voice_mut(parsed.voice).events.push(Event::rest(gap_duration));
                    }
                    set_voice_fill(&mut voice_fill, parsed.voice, cursor as u64);
                }
                let event = if parsed.is_rest {
                    let mut event = Event::rest(parsed.duration);
                    event.beams = parsed.beams;
                    event
                } else {
                    let mut event = Event::chord(vec![parsed.note], parsed.duration);
                    event.beams = parsed.beams;
                    event
                };
                last_note_voice = Some(parsed.voice);
                let voice = measure.voice_mut(parsed.voice);
                voice.events.push(event);
                let index = voice.events.len() - 1;
                last_event.retain(|(voice_number, _)| *voice_number != parsed.voice);
                last_event.push((parsed.voice, index));
                cursor += parsed.ticks as i64;
                set_voice_fill(&mut voice_fill, parsed.voice, cursor as u64);
            }
            "backup" => {
                let duration = parse_duration_ticks(child, attributes.divisions, "backup")?;
                cursor -= duration as i64;
                if cursor < 0 {
                    return Err(Error::msg(format!(
                        "measure {number}: backup moves before measure start"
                    )));
                }
            }
            "forward" => {
                let duration = parse_duration_ticks(child, attributes.divisions, "forward")?;
                cursor += duration as i64;
            }
            "direction" => {
                parse_direction(
                    child,
                    attributes.divisions,
                    cursor,
                    measure_index,
                    &mut measure,
                    &mut pending_dynamics,
                    last_note_voice.unwrap_or(1),
                )?;
            }
            "barline" => parse_barline(child, &mut measure)?,
            "print" => {
                if child.attr("new-system") == Some("yes") {
                    measure.system_break = true;
                }
                if child.attr("new-page") == Some("yes") {
                    measure.page_break = true;
                }
            }
            "sound" => {
                if let Some(text) = child.attr("tempo") {
                    let bpm = parse_f64_bpm(text)?;
                    measure.tempo = Some(Tempo::new(bpm, NoteValue::Quarter));
                }
            }
            "harmony" | "figured-bass" | "grouping" | "link" | "bookmark" => {
                // Presentational or out-of-scope; documented as ignored.
            }
            other => {
                return Err(Error::msg(format!(
                    "measure {number}: unsupported element <{other}>"
                )));
            }
        }
    }

    // Verify measure fill against the final time signature without rejecting
    // pickup measures or underfull voices (those are reported by inspection).
    let capacity = measure.capacity_ticks();
    for (voice, fill) in &voice_fill {
        if *fill > capacity {
            return Err(Error::msg(format!(
                "measure {number}: voice {voice} overfull ({fill} > {capacity} ticks)"
            )));
        }
    }

    apply_pending_dynamics(&mut measure, &pending_dynamics)?;
    measure.attributes = *attributes;
    Ok(measure)
}

fn parse_attributes(element: &Element, attributes: &mut Attributes) -> Result<()> {
    if let Some(text) = element.child_text("divisions") {
        let value = parse_u32(text, "divisions")?;
        if value == 0 || value > 10_000 {
            return Err(Error::out_of_range(format!("divisions {value}")));
        }
        attributes.divisions = value;
    }
    if let Some(key) = element.child("key") {
        let fifths = key
            .child_text("fifths")
            .ok_or_else(|| Error::msg("key without fifths"))?;
        let fifths = parse_i8(fifths, "fifths")?;
        if !(-7..=7).contains(&fifths) {
            return Err(Error::out_of_range(format!("key fifths {fifths}")));
        }
        let mode = key
            .child_text("mode")
            .map(KeyMode::from_musicxml_name)
            .unwrap_or(KeyMode::Major);
        attributes.key = Key::new(fifths, mode);
    }
    if let Some(time) = element.child("time") {
        let beats: Vec<&Element> = time.children_named("beats").collect();
        let types: Vec<&Element> = time.children_named("beat-type").collect();
        if beats.len() != 1 || types.len() != 1 {
            return Err(Error::msg(
                "composite or additive time signatures are not supported",
            ));
        }
        let beat_count = parse_u32(beats[0].text_trimmed(), "beats")?;
        let beat_type = parse_u32(types[0].text_trimmed(), "beat-type")?;
        let symbol = match time.attr("symbol") {
            Some("common") => TimeSymbol::Common,
            Some("cut") => TimeSymbol::Cut,
            _ => TimeSymbol::Normal,
        };
        let signature = TimeSignature {
            beats: beat_count,
            beat_type,
            symbol,
        };
        signature.validate()?;
        attributes.time = signature;
    }
    if let Some(staves) = element.child_text("staves") {
        let count = parse_u32(staves, "staves")?;
        if count > 1 {
            return Err(Error::msg(
                "multiple staves per part are not supported in this version",
            ));
        }
    }
    if let Some(clef) = element.child("clef") {
        if element.children_named("clef").count() > 1 {
            return Err(Error::msg(
                "multiple clefs per part are not supported in this version",
            ));
        }
        let sign_text = clef
            .child_text("sign")
            .ok_or_else(|| Error::msg("clef without sign"))?;
        let sign = ClefSign::from_musicxml_name(sign_text)
            .ok_or_else(|| Error::msg(format!("unsupported clef sign {sign_text:?}")))?;
        let line = clef
            .child_text("line")
            .map(|text| parse_u8(text, "clef line"))
            .transpose()?
            .unwrap_or(default_clef_line(sign));
        let octave_change = clef
            .child_text("clef-octave-change")
            .map(|text| parse_i8(text, "clef-octave-change"))
            .transpose()?
            .unwrap_or(0);
        let clef = Clef::new(sign, line, octave_change);
        clef.validate()?;
        attributes.clef = clef;
    }
    // Transposition is out of scope for v1; written pitch is preserved.
    Ok(())
}

fn default_clef_line(sign: ClefSign) -> u8 {
    match sign {
        ClefSign::G => 2,
        ClefSign::F => 4,
        ClefSign::C => 3,
        ClefSign::Percussion => 3,
    }
}

fn parse_note(element: &Element, divisions: u32, measure: &Measure) -> Result<ParsedNote> {
    if element.has_child("grace") {
        return Err(Error::msg("grace notes are not supported"));
    }
    if element.has_child("cue") {
        return Err(Error::msg("cue notes are not supported"));
    }
    if element.has_child("time-modification") {
        return Err(Error::msg("tuplets are not supported"));
    }
    if element
        .child("notations")
        .map(|notations| notations.has_child("tuplet"))
        .unwrap_or(false)
    {
        return Err(Error::msg("tuplets are not supported"));
    }
    if let Some(staff) = element.child_text("staff") {
        if parse_u32(staff, "staff")? > 1 {
            return Err(Error::msg("multiple staves per part are not supported"));
        }
    }
    let duration_text = element
        .child_text("duration")
        .ok_or_else(|| Error::msg("note without duration (grace notes are unsupported)"))?;
    let ticks = parse_u64(duration_text, "note duration")?;
    if ticks == 0 {
        return Err(Error::msg("zero-length note"));
    }
    let dots = element.children_named("dot").count();
    let written = if let Some(type_text) = element.child_text("type") {
        let value = NoteValue::from_musicxml_type(type_text).ok_or_else(|| {
            Error::msg(format!("unsupported note type {type_text:?}"))
        })?;
        let duration = Duration::new(value, dots as u8);
        let expected = duration.ticks_exact(divisions).ok_or_else(|| {
            Error::msg(format!(
                "duration {type_text} does not divide divisions {divisions}"
            ))
        })?;
        if expected != ticks {
            return Err(Error::msg(format!(
                "inconsistent duration: {type_text} is {expected} ticks but <duration> says {ticks}; tuplets are not supported"
            )));
        }
        duration
    } else {
        Duration::from_ticks(divisions, ticks).ok_or_else(|| {
            Error::msg(format!(
                "cannot represent {ticks} ticks as a written duration"
            ))
        })?
    };
    let is_rest = element.child("rest").is_some();
    if is_rest {
        if element.child("pitch").is_some() {
            return Err(Error::msg("note cannot be both rest and pitched"));
        }
    }
    let voice = element
        .child_text("voice")
        .map(|text| parse_u8(text, "voice"))
        .transpose()?
        .unwrap_or(1);
    if voice == 0 || voice > 16 {
        return Err(Error::out_of_range(format!("voice {voice}")));
    }
    let mut note = Note::new(Pitch::c4());
    if !is_rest {
        let pitch_element = element
            .child("pitch")
            .ok_or_else(|| Error::msg("note has neither pitch nor rest"))?;
        note.pitch = parse_pitch(pitch_element)?;
    }
    if let Some(text) = element.child_text("accidental") {
        note.accidental = Some(
            Accidental::from_musicxml_name(text)
                .ok_or_else(|| Error::msg(format!("unsupported accidental {text:?}")))?,
        );
    }
    for tie in element.children_named("tie") {
        match tie.attr("type") {
            Some("start") => note.tie.start = true,
            Some("stop") => note.tie.stop = true,
            Some(other) => {
                return Err(Error::msg(format!("unsupported tie type {other:?}")))
            }
            None => return Err(Error::msg("tie without type")),
        }
    }
    if let Some(notations) = element.child("notations") {
        for tied in notations.children_named("tied") {
            match tied.attr("type") {
                Some("start") => note.tie.start = true,
                Some("stop") => note.tie.stop = true,
                _ => {}
            }
        }
        for slur in notations.children_named("slur") {
            let kind = match slur.attr("type") {
                Some("start") => SlurKind::Start,
                Some("stop") => SlurKind::Stop,
                Some("continue") => SlurKind::Continue,
                Some(other) => {
                    return Err(Error::msg(format!("unsupported slur type {other:?}")))
                }
                None => return Err(Error::msg("slur without type")),
            };
            let number = slur
                .attr("number")
                .map(|text| parse_u8(text, "slur number"))
                .transpose()?
                .unwrap_or(1);
            note.slurs.push(SlurMark::new(number, kind));
        }
        if let Some(articulations) = notations.child("articulations") {
            for child in &articulations.children {
                let articulation =
                    Articulation::from_musicxml_name(child.local_name()).ok_or_else(|| {
                        Error::msg(format!(
                            "unsupported articulation <{}>",
                            child.local_name()
                        ))
                    })?;
                note.articulations.push(articulation);
            }
        }
        for fermata in notations.children_named("fermata") {
            let _ = fermata;
            if !note.articulations.contains(&Articulation::Fermata) {
                note.articulations.push(Articulation::Fermata);
            }
        }
        if notations.has_child("ornaments") {
            return Err(Error::msg("ornaments are not supported"));
        }
    }
    if let Some(lyrics) = element.child("lyric") {
        if let Some(lyric) = parse_lyric(lyrics)? {
            note.lyrics.push(lyric);
        }
        for extra in element.children_named("lyric").skip(1) {
            if let Some(lyric) = parse_lyric(extra)? {
                note.lyrics.push(lyric);
            }
        }
    }
    let mut beams = Vec::new();
    for beam in element.children_named("beam") {
        let kind = BeamKind::from_musicxml_name(beam.text_trimmed())
            .ok_or_else(|| Error::msg(format!("unsupported beam {:?}", beam.text_trimmed())))?;
        let number = beam
            .attr("number")
            .map(|text| parse_u8(text, "beam number"))
            .transpose()?
            .unwrap_or(1);
        if number == 0 || number > 8 {
            return Err(Error::out_of_range(format!("beam number {number}")));
        }
        beams.push(Beam::new(number, kind));
    }
    // Reject notes that cannot fit even an empty 64-beat measure.
    let max_ticks = divisions as u64 * 4 * 64;
    if ticks > max_ticks {
        return Err(Error::Limit("note duration"));
    }
    let _ = measure;
    Ok(ParsedNote {
        note,
        is_rest,
        chord: element.has_child("chord"),
        duration: written,
        ticks,
        voice,
        beams,
    })
}

fn parse_pitch(element: &Element) -> Result<Pitch> {
    let step_text = element
        .child_text("step")
        .ok_or_else(|| Error::msg("pitch without step"))?;
    let step = Step::parse(step_text)
        .ok_or_else(|| Error::msg(format!("unsupported pitch step {step_text:?}")))?;
    let octave = element
        .child_text("octave")
        .ok_or_else(|| Error::msg("pitch without octave"))?;
    let octave = parse_i8(octave, "octave")?;
    let alter = element
        .child_text("alter")
        .map(|text| parse_i8(text, "alter"))
        .transpose()?
        .unwrap_or(0);
    let pitch = Pitch::new(step, alter, octave);
    pitch.validate()?;
    Ok(pitch)
}

fn parse_lyric(element: &Element) -> Result<Option<Lyric>> {
    let number = element
        .attr("number")
        .map(|text| parse_u8(text, "lyric number"))
        .transpose()?
        .unwrap_or(1);
    if number == 0 || number > 8 {
        return Err(Error::out_of_range(format!("lyric number {number}")));
    }
    let text = element
        .child_text("text")
        .ok_or_else(|| Error::msg("lyric without text"))?;
    let syllabic = element
        .child_text("syllabic")
        .map(Syllabic::from_musicxml_name)
        .unwrap_or(Syllabic::Single);
    let extend = element.has_child("extend") || element.attr("extend") == Some("yes");
    if text.is_empty() && !extend {
        return Ok(None);
    }
    let lyric = Lyric {
        number,
        text: text.to_string(),
        syllabic,
        extend,
    };
    if !lyric.validate() {
        return Err(Error::Limit("lyric text"));
    }
    Ok(Some(lyric))
}

fn parse_duration_ticks(element: &Element, divisions: u32, name: &str) -> Result<u64> {
    let text = element
        .child_text("duration")
        .ok_or_else(|| Error::msg(format!("<{name}> without duration")))?;
    let ticks = parse_u64(text, "duration")?;
    if ticks == 0 {
        return Err(Error::msg(format!("<{name}> has zero duration")));
    }
    let max = divisions as u64 * 4 * 64;
    if ticks > max {
        return Err(Error::Limit("duration"));
    }
    Ok(ticks)
}

fn parse_direction(
    element: &Element,
    divisions: u32,
    cursor: i64,
    _measure_index: usize,
    measure: &mut Measure,
    pending: &mut Vec<PendingDynamic>,
    current_voice: u8,
) -> Result<()> {
    let offset = element
        .child_text("offset")
        .map(|text| {
            let value = parse_i64(text, "direction offset")?;
            Ok::<i64, Error>(value)
        })
        .transpose()?
        .unwrap_or(0);
    let cursor = cursor + offset;
    if cursor < 0 {
        return Err(Error::msg("direction offset moves before measure start"));
    }
    let cursor = cursor as u64;
    let Some(direction_type) = element.child("direction-type") else {
        // A direction with only <sound> is valid.
        if let Some(sound) = element.child("sound") {
            if let Some(tempo) = sound.attr("tempo") {
                measure.tempo = Some(Tempo::new(parse_f64_bpm(tempo)?, NoteValue::Quarter));
            }
        }
        return Ok(());
    };
    for child in &direction_type.children {
        match child.local_name() {
            "dynamics" => {
                for dynamic_child in &child.children {
                    let dynamic = Dynamic::from_musicxml_name(dynamic_child.local_name())
                        .ok_or_else(|| {
                            Error::msg(format!(
                                "unsupported dynamic <{}>",
                                dynamic_child.local_name()
                            ))
                        })?;
                    pending.push(PendingDynamic {
                        cursor,
                        dynamic,
                        voice: current_voice,
                    });
                }
            }
            "metronome" => {
                let unit = child
                    .child_text("beat-unit")
                    .ok_or_else(|| Error::msg("metronome without beat-unit"))?;
                let per_minute = child
                    .child_text("per-minute")
                    .ok_or_else(|| Error::msg("metronome without per-minute"))?;
                let beat = NoteValue::from_musicxml_type(unit).ok_or_else(|| {
                    Error::msg(format!("unsupported metronome beat-unit {unit:?}"))
                })?;
                let dots = child.children_named("beat-unit-dot").count() as u8;
                let beat = if dots == 0 {
                    beat
                } else {
                    return Err(Error::msg("dotted metronome units are not supported"));
                };
                measure.tempo = Some(Tempo::new(parse_f64_bpm(per_minute)?, beat));
            }
            "words" => {
                let text = child.text_trimmed();
                if !text.is_empty() && measure.rehearsal.is_none() {
                    let trimmed = if text.chars().count() > 128 {
                        text.chars().take(128).collect()
                    } else {
                        text.to_string()
                    };
                    measure.rehearsal = Some(trimmed);
                }
            }
            "rehearsal" => {
                let text = child.text_trimmed();
                if !text.is_empty() {
                    let trimmed = if text.chars().count() > 128 {
                        text.chars().take(128).collect()
                    } else {
                        text.to_string()
                    };
                    measure.rehearsal = Some(trimmed);
                }
            }
            "sound" => {
                if let Some(tempo) = child.attr("tempo") {
                    measure.tempo = Some(Tempo::new(parse_f64_bpm(tempo)?, NoteValue::Quarter));
                }
            }
            "wedge" | "pedal" | "dashes" | "bracket" | "octave-shift" | "harp-pedals"
            | "damp" | "eyeglasses" | "string-mute" | "scordatura" | "accordion-registration"
            | "percussion" | "principal-percussion" | "other-direction" | "staff-divide"
            | "symbol" | "segno" | "coda" => {
                // Expressive directions outside the v1 engraving subset.
            }
            other => {
                return Err(Error::msg(format!(
                    "unsupported direction-type <{other}>"
                )))
            }
        }
    }
    if let Some(sound) = element.child("sound") {
        if let Some(tempo) = sound.attr("tempo") {
            measure.tempo = Some(Tempo::new(parse_f64_bpm(tempo)?, NoteValue::Quarter));
        }
    }
    let _ = divisions;
    Ok(())
}

fn parse_barline(element: &Element, measure: &mut Measure) -> Result<()> {
    let location = element.attr("location").unwrap_or("right");
    let ending = element.child("ending");
    if let Some(ending_element) = ending {
        let number = ending_element
            .attr("number")
            .and_then(|text| text.split(',').next())
            .and_then(|text| text.split('-').next())
            .and_then(|text| text.trim().parse::<u8>().ok())
            .unwrap_or(1);
        let kind = match ending_element.attr("type") {
            Some("start") => EndingKind::Start,
            Some("stop") => EndingKind::Stop,
            Some("discontinue") => EndingKind::Discontinue,
            _ => EndingKind::Start,
        };
        measure.ending = Some(Ending { number, kind });
    }
    if let Some(repeat) = element.child("repeat") {
        let direction = match repeat.attr("direction") {
            Some("forward") => RepeatDirection::Forward,
            Some("backward") => RepeatDirection::Backward,
            Some(other) => {
                return Err(Error::msg(format!(
                    "unsupported repeat direction {other:?}"
                )))
            }
            None => return Err(Error::msg("repeat without direction")),
        };
        let times = repeat
            .attr("times")
            .map(|text| parse_u8(text, "repeat times"))
            .transpose()?
            .unwrap_or(2);
        let repeat = Repeat { direction, times };
        repeat.validate()?;
        measure.repeat = Some(repeat);
    }
    if location == "right" || location == "left" {
        if let Some(style) = element.child_text("bar-style") {
            measure.barline = BarlineStyle::from_musicxml_name(style).ok_or_else(|| {
                Error::msg(format!("unsupported bar-style {style:?}"))
            })?;
        }
    }
    Ok(())
}

fn apply_pending_dynamics(
    measure: &mut Measure,
    pending: &[PendingDynamic],
) -> Result<()> {
    let divisions = measure.attributes.divisions;
    for item in pending {
        let mut voices: Vec<u8> = measure.voices.iter().map(|voice| voice.number).collect();
        // Directions are stream-positioned. Try the voice that was current when
        // the direction appeared first, then other voices in score order.
        voices.sort_by_key(|number| if *number == item.voice { 0 } else { *number + 1 });
        let mut assigned = false;
        for voice_number in voices {
            let Some(voice) = measure.voices.iter_mut().find(|v| v.number == voice_number) else {
                continue;
            };
            let mut starts = Vec::with_capacity(voice.events.len());
            let mut start = 0u64;
            for event in &voice.events {
                starts.push((start, start + event.duration.ticks(divisions)));
                start += event.duration.ticks(divisions);
            }
            // Exact start, exact end, containing interval, then next event.
            let index = starts
                .iter()
                .position(|(s, _)| *s == item.cursor)
                .or_else(|| starts.iter().rposition(|(_, e)| *e == item.cursor))
                .or_else(|| {
                    starts
                        .iter()
                        .position(|(s, e)| item.cursor >= *s && item.cursor < *e)
                })
                .or_else(|| starts.iter().position(|(s, _)| *s > item.cursor));
            if let Some(index) = index {
                let event = &mut voice.events[index];
                if event.dynamics.is_none() {
                    event.dynamics = Some(item.dynamic);
                }
                assigned = true;
                break;
            }
        }
        if !assigned {
            if let Some(voice) = measure.voices.last_mut() {
                if let Some(event) = voice.events.last_mut() {
                    if event.dynamics.is_none() {
                        event.dynamics = Some(item.dynamic);
                    }
                }
            }
        }
    }
    Ok(())
}

fn voice_fill_value(fills: &[(u8, u64)], voice: u8) -> u64 {
    fills
        .iter()
        .find(|(number, _)| *number == voice)
        .map(|(_, fill)| *fill)
        .unwrap_or(0)
}

fn set_voice_fill(fills: &mut Vec<(u8, u64)>, voice: u8, value: u64) {
    if let Some(entry) = fills.iter_mut().find(|(number, _)| *number == voice) {
        entry.1 = value;
    } else {
        fills.push((voice, value));
    }
}

fn parse_f64_bpm(text: &str) -> Result<u32> {
    let value: f64 = text
        .trim()
        .parse()
        .map_err(|_| Error::msg(format!("invalid tempo {text:?}")))?;
    if !value.is_finite() || value < 10.0 || value > 400.0 {
        return Err(Error::out_of_range(format!("tempo {text}")));
    }
    Ok(value.round() as u32)
}

fn parse_u64(text: &str, what: &str) -> Result<u64> {
    text.trim()
        .parse::<u64>()
        .map_err(|_| Error::msg(format!("invalid {what} {text:?}")))
}

fn parse_i64(text: &str, what: &str) -> Result<i64> {
    text.trim()
        .parse::<i64>()
        .map_err(|_| Error::msg(format!("invalid {what} {text:?}")))
}

fn parse_u32(text: &str, what: &str) -> Result<u32> {
    text.trim()
        .parse::<u32>()
        .map_err(|_| Error::msg(format!("invalid {what} {text:?}")))
}

fn parse_u8(text: &str, what: &str) -> Result<u8> {
    text.trim()
        .parse::<u8>()
        .map_err(|_| Error::msg(format!("invalid {what} {text:?}")))
}

fn parse_i8(text: &str, what: &str) -> Result<i8> {
    text.trim()
        .parse::<i8>()
        .map_err(|_| Error::msg(format!("invalid {what} {text:?}")))
}

/// Export a score to deterministic MusicXML 4.0 `score-partwise`.
///
/// The writer emits only the supported subset, in a fixed element order, with
/// no timestamps or environment-dependent values. Importing the result yields
/// the same notes, durations, voices, lyrics, ties, slurs, dynamics, key/time
/// changes and part metadata.
pub fn export_score(score: &Score) -> Result<String> {
    score.validate()?;
    let report = score.inspect();
    if !report.is_ok() {
        let first = report
            .issues
            .iter()
            .find(|issue| issue.severity == cadence_core::Severity::Error)
            .map(|issue| issue.message.clone())
            .unwrap_or_else(|| "invalid score".to_string());
        return Err(Error::msg(first));
    }
    let mut writer = xml::XmlWriter::with_declaration();
    writer.start("score-partwise", &[("version", "4.0")])?;

    writer.start("work", &[])?;
    writer.text_element("work-title", &score.metadata.title)?;
    writer.end("work");
    if !score.metadata.subtitle.is_empty() {
        writer.text_element("movement-title", &score.metadata.subtitle)?;
    }

    let has_identification = !score.metadata.composer.is_empty()
        || !score.metadata.lyricist.is_empty()
        || !score.metadata.arranger.is_empty()
        || !score.metadata.copyright.is_empty()
        || !score.metadata.source.is_empty();
    if has_identification {
        writer.start("identification", &[])?;
        if !score.metadata.composer.is_empty() {
            writer.start("creator", &[("type", "composer")])?;
            writer.text(&score.metadata.composer);
            writer.end("creator");
        }
        if !score.metadata.lyricist.is_empty() {
            writer.start("creator", &[("type", "lyricist")])?;
            writer.text(&score.metadata.lyricist);
            writer.end("creator");
        }
        if !score.metadata.arranger.is_empty() {
            writer.start("creator", &[("type", "arranger")])?;
            writer.text(&score.metadata.arranger);
            writer.end("creator");
        }
        if !score.metadata.copyright.is_empty() {
            writer.text_element("rights", &score.metadata.copyright)?;
        }
        if !score.metadata.source.is_empty() {
            writer.text_element("source", &score.metadata.source)?;
        }
        writer.end("identification");
    }

    writer.start("part-list", &[])?;
    for part in &score.parts {
        writer.start("score-part", &[("id", &part.id)])?;
        writer.text_element("part-name", &part.name)?;
        if !part.abbreviation.is_empty() {
            writer.text_element("part-abbreviation", &part.abbreviation)?;
        }
        let program = part.midi_program as u32 + 1;
        let channel = part.midi_channel as u32 + 1;
        writer.start("midi-instrument", &[("id", &format!("{}-I1", part.id))])?;
        writer.text_element("midi-channel", &channel.to_string())?;
        writer.text_element("midi-program", &program.to_string())?;
        writer.end("midi-instrument");
        writer.end("score-part");
    }
    writer.end("part-list");

    for part in &score.parts {
        writer.start("part", &[("id", &part.id)])?;
        let mut previous_attributes: Option<Attributes> = None;
        for (index, measure) in part.measures.iter().enumerate() {
            writer.start("measure", &[("number", &measure.number.to_string())])?;
            if measure.system_break || measure.page_break {
                let mut attrs: Vec<(&str, &str)> = Vec::new();
                if measure.system_break {
                    attrs.push(("new-system", "yes"));
                }
                if measure.page_break {
                    attrs.push(("new-page", "yes"));
                }
                writer.empty("print", &attrs)?;
            }
            let emit_attributes = index == 0
                || measure.explicit_attributes
                || previous_attributes != Some(measure.attributes);
            if emit_attributes {
                write_attributes(&mut writer, measure.attributes)?;
            }
            previous_attributes = Some(measure.attributes);

            if let Some(rehearsal) = &measure.rehearsal {
                writer.start("direction", &[])?;
                writer.start("direction-type", &[])?;
                writer.text_element("rehearsal", rehearsal)?;
                writer.end("direction-type");
                writer.end("direction");
            }
            if let Some(tempo) = measure.tempo {
                writer.start("direction", &[])?;
                writer.start("direction-type", &[])?;
                writer.start("metronome", &[])?;
                writer.text_element("beat-unit", tempo.beat.musicxml_type())?;
                writer.text_element("per-minute", &tempo.bpm.to_string())?;
                writer.end("metronome");
                writer.end("direction-type");
                writer.end("direction");
            }

            let divisions = measure.attributes.divisions;
            let mut previous_voice_ticks = 0u64;
            for (voice_index, voice) in measure.voices.iter().enumerate() {
                if voice_index > 0 && previous_voice_ticks > 0 {
                    writer.start("backup", &[])?;
                    writer.text_element("duration", &previous_voice_ticks.to_string())?;
                    writer.end("backup");
                }
                let mut voice_ticks = 0u64;
                for event in &voice.events {
                    let ticks = event
                        .duration
                        .ticks_exact(divisions)
                        .ok_or_else(|| {
                            Error::msg(format!(
                                "cannot export {:?} at divisions {divisions}",
                                event.duration
                            ))
                        })?;
                    voice_ticks += ticks;
                    if let Some(dynamic) = event.dynamics {
                        writer.start("direction", &[])?;
                        writer.start("direction-type", &[])?;
                        writer.start("dynamics", &[])?;
                        writer.empty(dynamic.musicxml_name(), &[])?;
                        writer.end("dynamics");
                        writer.end("direction-type");
                        writer.end("direction");
                    }
                    let notes = event.notes();
                    for (note_index, note) in notes.iter().enumerate() {
                        write_note(
                            &mut writer,
                            event,
                            note,
                            note_index,
                            ticks,
                            voice.number,
                            divisions,
                        )?;
                    }
                    if notes.is_empty() {
                        write_rest(&mut writer, event, ticks, voice.number, divisions)?;
                    }
                }
                previous_voice_ticks = voice_ticks;
            }

            if measure.barline != BarlineStyle::Regular
                || measure.repeat.is_some()
                || measure.ending.is_some()
            {
                writer.start("barline", &[("location", "right")])?;
                if measure.barline != BarlineStyle::Regular {
                    writer.text_element("bar-style", measure.barline.musicxml_name())?;
                }
                if let Some(ending) = measure.ending {
                    let kind = match ending.kind {
                        EndingKind::Start => "start",
                        EndingKind::Stop => "stop",
                        EndingKind::Discontinue => "discontinue",
                    };
                    writer.empty(
                        "ending",
                        &[("number", &ending.number.to_string()), ("type", kind)],
                    )?;
                }
                if let Some(repeat) = measure.repeat {
                    let direction = match repeat.direction {
                        RepeatDirection::Forward => "forward",
                        RepeatDirection::Backward => "backward",
                    };
                    writer.empty(
                        "repeat",
                        &[
                            ("direction", direction),
                            ("times", &repeat.times.to_string()),
                        ],
                    )?;
                }
                writer.end("barline");
            }
            writer.end("measure");
        }
        writer.end("part");
    }
    writer.end("score-partwise");
    Ok(writer.finish())
}

fn write_attributes(writer: &mut xml::XmlWriter, attributes: Attributes) -> Result<()> {
    writer.start("attributes", &[])?;
    writer.text_element("divisions", &attributes.divisions.to_string())?;
    writer.start("key", &[])?;
    writer.text_element("fifths", &attributes.key.fifths.to_string())?;
    writer.text_element("mode", attributes.key.mode.musicxml_name())?;
    writer.end("key");
    let symbol = match attributes.time.symbol {
        TimeSymbol::Normal => None,
        TimeSymbol::Common => Some("common"),
        TimeSymbol::Cut => Some("cut"),
    };
    let mut time_attrs = Vec::new();
    if let Some(symbol) = symbol {
        time_attrs.push(("symbol", symbol));
    }
    writer.start("time", &time_attrs)?;
    writer.text_element("beats", &attributes.time.beats.to_string())?;
    writer.text_element("beat-type", &attributes.time.beat_type.to_string())?;
    writer.end("time");
    writer.start("clef", &[])?;
    writer.text_element("sign", attributes.clef.sign.musicxml_name())?;
    writer.text_element("line", &attributes.clef.line.to_string())?;
    if attributes.clef.octave_change != 0 {
        writer.text_element(
            "clef-octave-change",
            &attributes.clef.octave_change.to_string(),
        )?;
    }
    writer.end("clef");
    writer.end("attributes");
    Ok(())
}

fn write_note(
    writer: &mut xml::XmlWriter,
    event: &Event,
    note: &Note,
    note_index: usize,
    ticks: u64,
    voice: u8,
    divisions: u32,
) -> Result<()> {
    writer.start("note", &[])?;
    if note_index > 0 {
        writer.empty("chord", &[])?;
    }
    writer.start("pitch", &[])?;
    writer.text_element("step", note.pitch.step.musicxml_step())?;
    if note.pitch.alter != 0 {
        writer.text_element("alter", &note.pitch.alter.to_string())?;
    }
    writer.text_element("octave", &note.pitch.octave.to_string())?;
    writer.end("pitch");
    writer.text_element("duration", &ticks.to_string())?;
    writer.text_element("voice", &voice.to_string())?;
    writer.text_element("type", event.duration.value.musicxml_type())?;
    for _ in 0..event.duration.dots {
        writer.empty("dot", &[])?;
    }
    if let Some(accidental) = note.accidental {
        writer.text_element("accidental", accidental.musicxml_name())?;
    }
    if note.tie.start {
        writer.empty("tie", &[("type", "start")])?;
    }
    if note.tie.stop {
        writer.empty("tie", &[("type", "stop")])?;
    }
    if note_index == 0 {
        for beam in &event.beams {
            writer.start("beam", &[("number", &beam.number.to_string())])?;
            writer.text(beam.kind.musicxml_name());
            writer.end("beam");
        }
    }
    let has_notations = note.tie.is_tied() || !note.slurs.is_empty() || !note.articulations.is_empty();
    if has_notations {
        writer.start("notations", &[])?;
        if note.tie.start {
            writer.empty("tied", &[("type", "start")])?;
        }
        if note.tie.stop {
            writer.empty("tied", &[("type", "stop")])?;
        }
        for slur in &note.slurs {
            let kind = match slur.kind {
                SlurKind::Start => "start",
                SlurKind::Stop => "stop",
                SlurKind::Continue => "continue",
            };
            writer.empty(
                "slur",
                &[("type", kind), ("number", &slur.number.to_string())],
            )?;
        }
        if !note.articulations.is_empty() {
            writer.start("articulations", &[])?;
            for articulation in &note.articulations {
                writer.empty(articulation.musicxml_name(), &[])?;
            }
            writer.end("articulations");
        }
        writer.end("notations");
    }
    for lyric in &note.lyrics {
        let number = lyric.number.to_string();
        let mut attrs: Vec<(&str, &str)> = Vec::new();
        if lyric.number != 1 {
            attrs.push(("number", &number));
        }
        writer.start("lyric", &attrs)?;
        writer.text_element("syllabic", lyric.syllabic.musicxml_name())?;
        writer.text_element("text", &lyric.text)?;
        if lyric.extend {
            writer.empty("extend", &[])?;
        }
        writer.end("lyric");
    }
    writer.end("note");
    let _ = divisions;
    Ok(())
}

fn write_rest(
    writer: &mut xml::XmlWriter,
    event: &Event,
    ticks: u64,
    voice: u8,
    divisions: u32,
) -> Result<()> {
    writer.start("note", &[])?;
    let whole_measure = ticks == divisions as u64 * 4 && event.duration.value == NoteValue::Whole;
    if whole_measure {
        writer.empty("rest", &[("measure", "yes")])?;
    } else {
        writer.empty("rest", &[])?;
    }
    writer.text_element("duration", &ticks.to_string())?;
    writer.text_element("voice", &voice.to_string())?;
    writer.text_element("type", event.duration.value.musicxml_type())?;
    for _ in 0..event.duration.dots {
        writer.empty("dot", &[])?;
    }
    for beam in &event.beams {
        writer.start("beam", &[("number", &beam.number.to_string())])?;
        writer.text(beam.kind.musicxml_name());
        writer.end("beam");
    }
    writer.end("note");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIMPLE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<score-partwise version="4.0">
  <work><work-title>Import Test</work-title></work>
  <movement-title>Subtitle</movement-title>
  <identification>
    <creator type="composer">Composer</creator>
    <creator type="lyricist">Lyricist</creator>
    <rights>Public domain</rights>
  </identification>
  <part-list>
    <score-part id="P1">
      <part-name>Piano</part-name>
      <part-abbreviation>Pno.</part-abbreviation>
      <midi-instrument id="P1-I1"><midi-channel>1</midi-channel><midi-program>1</midi-program></midi-instrument>
    </score-part>
  </part-list>
  <part id="P1">
    <measure number="1">
      <attributes>
        <divisions>480</divisions>
        <key><fifths>1</fifths><mode>major</mode></key>
        <time><beats>4</beats><beat-type>4</beat-type></time>
        <clef><sign>G</sign><line>2</line></clef>
      </attributes>
      <direction><direction-type><metronome><beat-unit>quarter</beat-unit><per-minute>96</per-minute></metronome></direction-type></direction>
      <note>
        <pitch><step>G</step><octave>4</octave></pitch>
        <duration>480</duration><type>quarter</type><voice>1</voice>
        <lyric number="1"><syllabic>begin</syllabic><text>Ca</text></lyric>
        <notations><slur type="start" number="1"/><articulations><staccato/></articulations></notations>
      </note>
      <note>
        <pitch><step>A</step><octave>4</octave></pitch>
        <duration>480</duration><type>quarter</type><voice>1</voice>
        <lyric number="1"><syllabic>end</syllabic><text>dence</text></lyric>
      </note>
      <note>
        <pitch><step>B</step><alter>1</alter><octave>4</octave></pitch>
        <duration>480</duration><type>quarter</type><voice>1</voice>
        <accidental>sharp</accidental>
        <tie type="start"/>
        <notations><tied type="start"/><slur type="stop" number="1"/></notations>
      </note>
      <note>
        <pitch><step>B</step><alter>1</alter><octave>4</octave></pitch>
        <duration>480</duration><type>quarter</type><voice>1</voice>
        <tie type="stop"/>
        <notations><tied type="stop"/></notations>
      </note>
    </measure>
    <measure number="2">
      <note><pitch><step>C</step><octave>5</octave></pitch><duration>1920</duration><type>whole</type><voice>1</voice></note>
      <backup><duration>1920</duration></backup>
      <note><pitch><step>E</step><octave>4</octave></pitch><duration>960</duration><type>half</type><voice>2</voice></note>
      <note><pitch><step>G</step><octave>4</octave></pitch><duration>960</duration><type>half</type><voice>2</voice></note>
      <direction><direction-type><dynamics><mf/></dynamics></direction-type></direction>
      <barline location="right"><bar-style>light-heavy</bar-style></barline>
    </measure>
  </part>
</score-partwise>"#;

    #[test]
    fn imports_simple_score() {
        let score = import_score(SIMPLE).unwrap();
        assert_eq!(score.metadata.title, "Import Test");
        assert_eq!(score.metadata.subtitle, "Subtitle");
        assert_eq!(score.metadata.composer, "Composer");
        assert_eq!(score.parts.len(), 1);
        let part = &score.parts[0];
        assert_eq!(part.name, "Piano");
        assert_eq!(part.abbreviation, "Pno.");
        assert_eq!(part.measures.len(), 2);
        assert_eq!(part.note_count(), 7);
        assert_eq!(part.measures[0].tempo.unwrap().bpm, 96);
        assert_eq!(part.measures[0].voices.len(), 1);
        assert_eq!(part.measures[1].voices.len(), 2);
        let first = &part.measures[0].voice(1).unwrap().events[0];
        assert_eq!(first.primary_pitch().unwrap().name(), "G4");
        assert_eq!(first.notes()[0].lyrics[0].text, "Ca");
        assert!(first.notes()[0].tie.start == false);
        assert_eq!(part.measures[1].barline, BarlineStyle::Final);
        assert_eq!(part.midi_program, 0);
        assert_eq!(part.midi_channel, 0);
    }

    #[test]
    fn imports_ties_slurs_and_dynamics() {
        let score = import_score(SIMPLE).unwrap();
        let part = &score.parts[0];
        let first = &part.measures[0].voice(1).unwrap().events[0];
        assert!(first.notes()[0].articulations.contains(&Articulation::Staccato));
        let event = &part.measures[0].voice(1).unwrap().events[2];
        assert!(event.notes()[0].tie.start);
        let slurs = resolve_slurs(part);
        assert_eq!(slurs.spans.len(), 1);
        assert_eq!(slurs.spans[0].number, 1);
        let ties = resolve_ties(part);
        assert_eq!(ties.spans.len(), 1);
    }

    #[test]
    fn round_trip_preserves_notes_and_marks() {
        let score = import_score(SIMPLE).unwrap();
        let xml = export_score(&score).unwrap();
        let again = import_score(&xml).unwrap();
        assert_event_snapshot_eq(&score, &again);
        assert_eq!(again.metadata, score.metadata);
        assert_eq!(again.parts[0].measures[1].barline, BarlineStyle::Final);
        assert_eq!(again.parts[0].measures[0].tempo, score.parts[0].measures[0].tempo);
    }

    #[test]
    fn demo_round_trips() {
        let score = Score::demo();
        let xml = export_score(&score).unwrap();
        let again = import_score(&xml).unwrap();
        assert_event_snapshot_eq(&score, &again);
        assert_eq!(again.metadata, score.metadata);
        assert_eq!(again.parts.len(), score.parts.len());
        for (a, b) in score.parts.iter().zip(&again.parts) {
            assert_eq!(a.id, b.id);
            assert_eq!(a.name, b.name);
            assert_eq!(a.midi_program, b.midi_program);
            assert_eq!(a.midi_channel, b.midi_channel);
            assert_eq!(a.measures.len(), b.measures.len());
            for (ma, mb) in a.measures.iter().zip(&b.measures) {
                assert_eq!(ma.attributes, mb.attributes);
                assert_eq!(ma.rehearsal, mb.rehearsal);
                assert_eq!(ma.barline, mb.barline);
                assert_eq!(ma.repeat, mb.repeat);
                assert_eq!(ma.ending, mb.ending);
            }
        }
    }

    #[test]
    fn export_is_byte_stable() {
        let score = Score::demo();
        let a = export_score(&score).unwrap();
        let b = export_score(&score).unwrap();
        assert_eq!(a, b);
        assert!(!a.contains("2026"));
    }

    #[test]
    fn rejects_tuplets_and_grace_notes() {
        let tuplet = SIMPLE.replace(
            "<duration>480</duration><type>quarter</type>",
            "<duration>320</duration><type>eighth</type><time-modification><actual-notes>3</actual-notes><normal-notes>2</normal-notes></time-modification>",
        );
        assert!(import_score(&tuplet).is_err());
        let grace = SIMPLE.replace(
            "<pitch><step>G</step><octave>4</octave></pitch>",
            "<grace/><pitch><step>G</step><octave>4</octave></pitch>",
        );
        assert!(import_score(&grace).is_err());
    }

    #[test]
    fn rejects_unsupported_root_and_multi_staff() {
        let wrong = SIMPLE.replace("score-partwise", "score-timewise");
        assert!(import_score(&wrong).is_err());
        let multi = SIMPLE.replace(
            "<clef><sign>G</sign><line>2</line></clef>",
            "<staves>2</staves><clef><sign>G</sign><line>2</line></clef>",
        );
        assert!(import_score(&multi).is_err());
    }

    #[test]
    fn rejects_overfull_measure() {
        let overfull = SIMPLE.replace(
            "<note><pitch><step>C</step><octave>5</octave></pitch><duration>1920</duration>",
            "<note><pitch><step>C</step><octave>5</octave></pitch><duration>2400</duration>",
        );
        assert!(import_score(&overfull).is_err());
    }

    fn assert_event_snapshot_eq(a: &Score, b: &Score) {
        let left = snapshot(a);
        let right = snapshot(b);
        assert_eq!(left.len(), right.len());
        for (l, r) in left.iter().zip(&right) {
            assert_eq!(l, r);
        }
    }

    fn snapshot(score: &Score) -> Vec<String> {
        let mut out = Vec::new();
        for (part_index, part) in score.parts.iter().enumerate() {
            for (measure_index, measure) in part.measures.iter().enumerate() {
                for voice in &measure.voices {
                    for (event_index, event) in voice.events.iter().enumerate() {
                        let mut line = format!(
                            "p{part_index}/m{measure_index}/v{}/e{event_index} {} {:?} {:?}",
                            voice.number,
                            event.duration,
                            event.kind,
                            event.dynamics
                        );
                        for note in event.notes() {
                            line.push_str(&format!(
                                " | {} tie={:?} acc={:?} art={:?} lyr={:?} slurs={:?}",
                                note.pitch,
                                note.tie,
                                note.accidental,
                                note.articulations,
                                note.lyrics,
                                note.slurs
                            ));
                        }
                        out.push(line);
                    }
                }
            }
        }
        out
    }
}
