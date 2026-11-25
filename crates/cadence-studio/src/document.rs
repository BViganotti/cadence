//! In-memory Studio document: one score, bounded undo, editing commands and
//! export helpers. The HTTP layer is a thin wrapper around this type.

use crate::json::{self, Value};
use cadence_core::{
    Duration, Dynamic, Error, Event, KeyMode, Lyric, Note, NoteValue, Pitch, Result, Score,
    Syllabic,
};
use cadence_engrave::{engrave, Layout, Options};
use cadence_export::{render_midi, render_pdf, render_svg, render_svg_with_hits};
use cadence_import::{export_score, import_score};

pub const UNDO_LIMIT: usize = 32;

#[derive(Debug, Clone)]
pub struct Document {
    pub score: Score,
    pub version: u64,
    pub selected_part: usize,
    pub source: String,
    undo: Vec<Score>,
    redo: Vec<Score>,
}

impl Default for Document {
    fn default() -> Self {
        Document::demo()
    }
}

impl Document {
    pub fn demo() -> Document {
        Document {
            score: Score::demo(),
            version: 1,
            selected_part: 0,
            source: "Built-in demo".to_string(),
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    pub fn from_score(score: Score, source: impl Into<String>) -> Document {
        Document {
            score,
            version: 1,
            selected_part: 0,
            source: source.into(),
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    pub fn import_xml(&mut self, xml: &str) -> Result<()> {
        let score = import_score(xml)?;
        if score.parts.is_empty() {
            return Err(Error::msg("imported score has no parts"));
        }
        self.push_undo();
        self.score = score;
        self.selected_part = 0;
        self.source = "Imported MusicXML".to_string();
        self.version += 1;
        Ok(())
    }

    pub fn undo(&mut self) -> Result<()> {
        let Some(previous) = self.undo.pop() else {
            return Err(Error::msg("nothing to undo"));
        };
        self.redo.push(self.score.clone());
        if self.redo.len() > UNDO_LIMIT {
            self.redo.remove(0);
        }
        self.score = previous;
        self.selected_part = self.selected_part.min(self.score.parts.len().saturating_sub(1));
        self.version += 1;
        Ok(())
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn redo(&mut self) -> Result<()> {
        let Some(next) = self.redo.pop() else {
            return Err(Error::msg("nothing to redo"));
        };
        self.undo.push(self.score.clone());
        if self.undo.len() > UNDO_LIMIT {
            self.undo.remove(0);
        }
        self.score = next;
        self.selected_part = self.selected_part.min(self.score.parts.len().saturating_sub(1));
        self.version += 1;
        Ok(())
    }

    pub fn select_part(&mut self, index: usize) -> Result<()> {
        if index >= self.score.parts.len() {
            return Err(Error::out_of_range(format!("part {index}")));
        }
        self.selected_part = index;
        Ok(())
    }

    pub fn layout(&self, part: Option<usize>) -> Result<Layout> {
        let score = self.score_owned(part)?;
        let mut layout = engrave(&score, &Options::default())?;
        if let Some(index) = part {
            remap_hits(&mut layout, index);
        }
        Ok(layout)
    }

    pub fn layout_with_options(&self, part: Option<usize>, options: &Options) -> Result<Layout> {
        let score = self.score_owned(part)?;
        let mut layout = engrave(&score, options)?;
        if let Some(index) = part {
            remap_hits(&mut layout, index);
        }
        Ok(layout)
    }

    pub fn svg(&self, part: Option<usize>, hits: bool) -> Result<String> {
        let layout = self.layout(part)?;
        Ok(if hits {
            render_svg_with_hits(&layout, true)
        } else {
            render_svg(&layout)
        })
    }

    pub fn pdf(&self, part: Option<usize>) -> Result<Vec<u8>> {
        let layout = self.layout(part)?;
        render_pdf(&layout)
    }

    pub fn midi(&self, part: Option<usize>) -> Result<Vec<u8>> {
        let score = self.score_owned(part)?;
        render_midi(&score)
    }

    pub fn musicxml(&self, part: Option<usize>) -> Result<String> {
        let score = self.score_owned(part)?;
        export_score(&score)
    }

    fn score_owned(&self, part: Option<usize>) -> Result<Score> {
        match part {
            Some(index) => self
                .score
                .extract_part(index)
                .ok_or_else(|| Error::out_of_range(format!("part {index}"))),
            None => Ok(self.score.clone()),
        }
    }

    pub fn apply(&mut self, command: EditCommand) -> Result<()> {
        let snapshot = self.score.clone();
        self.apply_inner(&command)?;
        self.score.validate()?;
        let report = self.score.inspect();
        if !report.is_ok() {
            self.score = snapshot;
            return Err(Error::msg(
                report
                    .issues
                    .iter()
                    .find(|issue| issue.severity == cadence_core::Severity::Error)
                    .map(|issue| issue.message.clone())
                    .unwrap_or_else(|| "edit would make the score invalid".to_string()),
            ));
        }
        self.undo.push(snapshot);
        if self.undo.len() > UNDO_LIMIT {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.version += 1;
        self.selected_part = self.selected_part.min(self.score.parts.len().saturating_sub(1));
        Ok(())
    }

    fn apply_inner(&mut self, command: &EditCommand) -> Result<()> {
        match command {
            EditCommand::AddNote {
                part,
                measure,
                voice,
                pitch,
                duration,
                dots,
                accidental,
            } => {
                let pitch = Pitch::parse(pitch)?;
                let duration = parse_duration(duration, *dots)?;
                let measure = self.measure_mut(*part, *measure)?;
                let ticks = duration.ticks(measure.attributes.divisions);
                let fill = measure
                    .voice(*voice)
                    .map(|voice| voice.duration_ticks(measure.attributes.divisions))
                    .unwrap_or(0);
                if fill + ticks > measure.capacity_ticks() {
                    return Err(Error::msg("measure is full for that duration"));
                }
                let mut event = Event::note(pitch, duration);
                if let Some(text) = accidental {
                    if !text.is_empty() {
                        event.notes_mut()[0].accidental =
                            Some(cadence_core::Accidental::from_musicxml_name(text).ok_or_else(
                                || Error::msg(format!("unsupported accidental {text:?}")),
                            )?);
                    }
                }
                measure.push_event(*voice, event);
            }
            EditCommand::AddRest {
                part,
                measure,
                voice,
                duration,
                dots,
            } => {
                let duration = parse_duration(duration, *dots)?;
                let measure = self.measure_mut(*part, *measure)?;
                let ticks = duration.ticks(measure.attributes.divisions);
                let fill = measure
                    .voice(*voice)
                    .map(|voice| voice.duration_ticks(measure.attributes.divisions))
                    .unwrap_or(0);
                if fill + ticks > measure.capacity_ticks() {
                    return Err(Error::msg("measure is full for that duration"));
                }
                measure.push_rest(*voice, duration);
            }
            EditCommand::AddChordNote {
                part,
                measure,
                voice,
                event,
                pitch,
            } => {
                let pitch = Pitch::parse(pitch)?;
                let measure = self.measure_mut(*part, *measure)?;
                let event = measure
                    .voice_mut(*voice)
                    .events
                    .get_mut(*event)
                    .ok_or_else(|| Error::out_of_range("event index"))?;
                if event.is_rest() {
                    return Err(Error::msg("cannot add a chord note to a rest"));
                }
                event.add_note(Note::new(pitch));
            }
            EditCommand::DeleteEvent {
                part,
                measure,
                voice,
                event,
            } => {
                let measure = self.measure_mut(*part, *measure)?;
                if measure.remove_event(*voice, *event).is_none() {
                    return Err(Error::out_of_range("event index"));
                }
            }
            EditCommand::SetPitch {
                part,
                measure,
                voice,
                event,
                note,
                pitch,
            } => {
                let pitch = Pitch::parse(pitch)?;
                let measure = self.measure_mut(*part, *measure)?;
                let event = measure
                    .voice_mut(*voice)
                    .events
                    .get_mut(*event)
                    .ok_or_else(|| Error::out_of_range("event index"))?;
                let note = event
                    .notes_mut()
                    .get_mut(*note)
                    .ok_or_else(|| Error::out_of_range("note index"))?;
                note.pitch = pitch;
                note.accidental = None;
            }
            EditCommand::SetDuration {
                part,
                measure,
                voice,
                event,
                duration,
                dots,
            } => {
                let new_duration = parse_duration(duration, *dots)?;
                let measure = self.measure_mut(*part, *measure)?;
                let divisions = measure.attributes.divisions;
                let capacity = measure.capacity_ticks();
                let voice_ref = measure.voice(*voice).ok_or_else(|| Error::out_of_range("voice"))?;
                let old_ticks = voice_ref
                    .events
                    .get(*event)
                    .ok_or_else(|| Error::out_of_range("event index"))?
                    .duration
                    .ticks(divisions);
                let fill = voice_ref.duration_ticks(divisions);
                let new_ticks = new_duration.ticks(divisions);
                if fill - old_ticks + new_ticks > capacity {
                    return Err(Error::msg("new duration would overfill the measure"));
                }
                let voice_ref = measure.voice_mut(*voice);
                voice_ref.events[*event].duration = new_duration;
            }
            EditCommand::SetDynamic {
                part,
                measure,
                voice,
                event,
                dynamic,
            } => {
                let dynamic = match dynamic {
                    None => None,
                    Some(text) if text.is_empty() => None,
                    Some(text) => Some(
                        Dynamic::from_musicxml_name(text)
                            .ok_or_else(|| Error::msg(format!("unsupported dynamic {text:?}")))?,
                    ),
                };
                let measure = self.measure_mut(*part, *measure)?;
                let event = measure
                    .voice_mut(*voice)
                    .events
                    .get_mut(*event)
                    .ok_or_else(|| Error::out_of_range("event index"))?;
                event.dynamics = dynamic;
            }
            EditCommand::SetLyric {
                part,
                measure,
                voice,
                event,
                note,
                text,
                syllabic,
            } => {
                if text.chars().count() > 512 {
                    return Err(Error::Limit("lyric text"));
                }
                let measure = self.measure_mut(*part, *measure)?;
                let event = measure
                    .voice_mut(*voice)
                    .events
                    .get_mut(*event)
                    .ok_or_else(|| Error::out_of_range("event index"))?;
                let note = event
                    .notes_mut()
                    .get_mut(*note)
                    .ok_or_else(|| Error::out_of_range("note index"))?;
                note.lyrics.clear();
                if !text.is_empty() {
                    note.lyrics.push(Lyric {
                        number: 1,
                        text: text.clone(),
                        syllabic: syllabic
                            .as_deref()
                            .map(Syllabic::from_musicxml_name)
                            .unwrap_or_default(),
                        extend: false,
                    });
                }
            }
            EditCommand::SetTempo { part, measure, bpm } => {
                if !(10..=400).contains(bpm) {
                    return Err(Error::out_of_range(format!("tempo {bpm}")));
                }
                let measure = self.measure_mut(*part, *measure)?;
                measure.tempo = Some(cadence_core::Tempo::new(*bpm, NoteValue::Quarter));
            }
            EditCommand::SetTitle { title } => {
                if title.chars().count() > 512 {
                    return Err(Error::Limit("title"));
                }
                self.score.metadata.title = title.clone();
            }
            EditCommand::Transpose { part, semitones } => {
                if *part >= self.score.parts.len() {
                    return Err(Error::out_of_range(format!("part {part}")));
                }
                self.score.transpose_part(*part, *semitones)?;
            }
            EditCommand::AddMeasure { part } => {
                let part = self
                    .score
                    .parts
                    .get_mut(*part)
                    .ok_or_else(|| Error::out_of_range(format!("part {part}")))?;
                if part.measures.len() >= 10_000 {
                    return Err(Error::Limit("measures per part"));
                }
                let attributes = part
                    .measures
                    .last()
                    .map(|measure| measure.attributes)
                    .unwrap_or_default();
                part.push_measure(attributes);
            }
            EditCommand::DeleteMeasure { part, measure } => {
                let part = self
                    .score
                    .parts
                    .get_mut(*part)
                    .ok_or_else(|| Error::out_of_range(format!("part {part}")))?;
                if part.measures.len() <= 1 {
                    return Err(Error::msg("cannot delete the last measure"));
                }
                if *measure >= part.measures.len() {
                    return Err(Error::out_of_range("measure index"));
                }
                part.measures.remove(*measure);
                for (index, measure) in part.measures.iter_mut().enumerate() {
                    measure.number = index + 1;
                }
            }
            EditCommand::SetKey { part, measure, fifths } => {
                if !(-7..=7).contains(fifths) {
                    return Err(Error::out_of_range(format!("key fifths {fifths}")));
                }
                let measure = self.measure_mut(*part, *measure)?;
                measure.attributes.key = cadence_core::Key::new(*fifths, KeyMode::Major);
                measure.explicit_attributes = true;
            }
            EditCommand::SelectPart { part } => {
                self.select_part(*part)?;
            }
            EditCommand::AddPart { name } => {
                if self.score.parts.len() >= Score::MAX_PARTS {
                    return Err(Error::Limit("parts per score"));
                }
                if name.chars().count() > 256 {
                    return Err(Error::Limit("part name"));
                }
                let id = {
                    let mut suffix = self.score.parts.len() + 1;
                    loop {
                        let candidate = format!("P{suffix}");
                        if self.score.part_index_by_id(&candidate).is_none() {
                            break candidate;
                        }
                        suffix += 1;
                    }
                };
                let measure_count = self.score.measure_count().max(1);
                let attributes = self
                    .score
                    .parts
                    .first()
                    .and_then(|part| part.measures.first())
                    .map(|measure| measure.attributes)
                    .unwrap_or_default();
                let mut part = cadence_core::Part::new(id, name.clone());
                part.midi_channel = (self.score.parts.len() % 16) as u8;
                for number in 1..=measure_count {
                    part.measures.push(cadence_core::Measure::new(number, attributes));
                }
                self.score.parts.push(part);
                self.selected_part = self.score.parts.len() - 1;
            }
            EditCommand::DeletePart { part } => {
                if self.score.parts.len() <= 1 {
                    return Err(Error::msg("cannot delete the last part"));
                }
                if *part >= self.score.parts.len() {
                    return Err(Error::out_of_range(format!("part {part}")));
                }
                self.score.parts.remove(*part);
                self.selected_part = self
                    .selected_part
                    .min(self.score.parts.len().saturating_sub(1));
            }
        }
        Ok(())
    }

    fn measure_mut(&mut self, part: usize, measure: usize) -> Result<&mut cadence_core::Measure> {
        self.score
            .parts
            .get_mut(part)
            .ok_or_else(|| Error::out_of_range(format!("part {part}")))?
            .measures
            .get_mut(measure)
            .ok_or_else(|| Error::out_of_range(format!("measure {measure}")))
    }

    fn push_undo(&mut self) {
        self.undo.push(self.score.clone());
        if self.undo.len() > UNDO_LIMIT {
            self.undo.remove(0);
        }
        self.redo.clear();
    }


    pub fn event_json(
        &self,
        part: usize,
        measure: usize,
        voice: u8,
        event: usize,
    ) -> Result<Value> {
        let part_ref = self
            .score
            .parts
            .get(part)
            .ok_or_else(|| Error::out_of_range(format!("part {part}")))?;
        let measure_ref = part_ref
            .measures
            .get(measure)
            .ok_or_else(|| Error::out_of_range(format!("measure {measure}")))?;
        let voice_ref = measure_ref
            .voice(voice)
            .ok_or_else(|| Error::out_of_range(format!("voice {voice}")))?;
        let event_ref = voice_ref
            .events
            .get(event)
            .ok_or_else(|| Error::out_of_range(format!("event {event}")))?;
        let notes: Vec<Value> = event_ref
            .notes()
            .iter()
            .enumerate()
            .map(|(index, note)| {
                json::object(vec![
                    ("index", json::count(index)),
                    ("pitch", json::string(note.pitch.name())),
                    ("tieStart", json::bool(note.tie.start)),
                    ("tieStop", json::bool(note.tie.stop)),
                    (
                        "lyric",
                        note.lyrics
                            .first()
                            .map(|lyric| json::string(lyric.text.clone()))
                            .unwrap_or_else(json::null),
                    ),
                ])
            })
            .collect();
        Ok(json::object(vec![
            ("part", json::count(part)),
            ("measure", json::count(measure)),
            ("voice", json::count(voice as usize)),
            ("event", json::count(event)),
            ("isRest", json::bool(event_ref.is_rest())),
            (
                "duration",
                json::string(event_ref.duration.value.musicxml_type()),
            ),
            ("dots", json::count(event_ref.duration.dots as usize)),
            (
                "dynamic",
                event_ref
                    .dynamics
                    .map(|dynamic| json::string(dynamic.musicxml_name().to_string()))
                    .unwrap_or_else(json::null),
            ),
            ("notes", json::array(notes)),
            (
                "measureNumber",
                json::count(measure_ref.number),
            ),
            (
                "fifths",
                json::number(measure_ref.attributes.key.fifths as f64),
            ),
            (
                "tempo",
                measure_ref
                    .tempo
                    .map(|tempo| json::count(tempo.bpm as usize))
                    .unwrap_or_else(json::null),
            ),
            (
                "partName",
                json::string(part_ref.name.clone()),
            ),
        ]))
    }

    pub fn summary(&self) -> Value {
        let report = self.score.inspect();
        let parts: Vec<Value> = self
            .score
            .parts
            .iter()
            .enumerate()
            .map(|(index, part)| {
                json::object(vec![
                    ("index", json::count(index)),
                    ("id", json::string(part.id.clone())),
                    ("name", json::string(part.name.clone())),
                    ("abbreviation", json::string(part.abbreviation.clone())),
                    ("measures", json::count(part.measures.len())),
                    ("notes", json::count(part.note_count())),
                ])
            })
            .collect();
        let diagnostics: Vec<Value> = report
            .issues
            .iter()
            .take(64)
            .map(|issue| {
                json::object(vec![
                    (
                        "severity",
                        json::string(if issue.severity == cadence_core::Severity::Error {
                            "error"
                        } else {
                            "warning"
                        }),
                    ),
                    ("message", json::string(issue.message.clone())),
                ])
            })
            .collect();
        let mut chord_count = 0usize;
        let mut rest_count = 0usize;
        for part in &self.score.parts {
            for measure in &part.measures {
                for voice in &measure.voices {
                    for event in &voice.events {
                        if event.is_chord() {
                            chord_count += 1;
                        }
                        if event.is_rest() {
                            rest_count += 1;
                        }
                    }
                }
            }
        }
        json::object(vec![
            ("version", json::count_u64(self.version)),
            ("source", json::string(self.source.clone())),
            ("selectedPart", json::count(self.selected_part)),
            ("canUndo", json::bool(self.can_undo())),
            ("canRedo", json::bool(self.can_redo())),
            (
                "metadata",
                json::object(vec![
                    ("title", json::string(self.score.metadata.title.clone())),
                    ("subtitle", json::string(self.score.metadata.subtitle.clone())),
                    ("composer", json::string(self.score.metadata.composer.clone())),
                    ("lyricist", json::string(self.score.metadata.lyricist.clone())),
                ]),
            ),
            ("parts", json::array(parts)),
            ("measureCount", json::count(self.score.measure_count())),
            ("noteCount", json::count(self.score.note_count())),
            ("eventCount", json::count(self.score.event_count())),
            ("chordCount", json::count(chord_count)),
            ("restCount", json::count(rest_count)),
            (
                "diagnostics",
                json::object(vec![
                    ("errors", json::count(report.error_count())),
                    ("warnings", json::count(report.warning_count())),
                    ("items", json::array(diagnostics)),
                ]),
            ),
        ])
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditCommand {
    AddNote {
        part: usize,
        measure: usize,
        voice: u8,
        pitch: String,
        duration: String,
        dots: u8,
        accidental: Option<String>,
    },
    AddRest {
        part: usize,
        measure: usize,
        voice: u8,
        duration: String,
        dots: u8,
    },
    AddChordNote {
        part: usize,
        measure: usize,
        voice: u8,
        event: usize,
        pitch: String,
    },
    DeleteEvent {
        part: usize,
        measure: usize,
        voice: u8,
        event: usize,
    },
    SetPitch {
        part: usize,
        measure: usize,
        voice: u8,
        event: usize,
        note: usize,
        pitch: String,
    },
    SetDuration {
        part: usize,
        measure: usize,
        voice: u8,
        event: usize,
        duration: String,
        dots: u8,
    },
    SetDynamic {
        part: usize,
        measure: usize,
        voice: u8,
        event: usize,
        dynamic: Option<String>,
    },
    SetLyric {
        part: usize,
        measure: usize,
        voice: u8,
        event: usize,
        note: usize,
        text: String,
        syllabic: Option<String>,
    },
    SetTempo {
        part: usize,
        measure: usize,
        bpm: u32,
    },
    SetTitle {
        title: String,
    },
    Transpose {
        part: usize,
        semitones: i8,
    },
    AddMeasure {
        part: usize,
    },
    DeleteMeasure {
        part: usize,
        measure: usize,
    },
    SetKey {
        part: usize,
        measure: usize,
        fifths: i8,
    },
    SelectPart {
        part: usize,
    },
    AddPart {
        name: String,
    },
    DeletePart {
        part: usize,
    },
}

impl EditCommand {
    pub fn from_json(value: &Value) -> Result<EditCommand> {
        let op = value
            .get("op")
            .and_then(Value::as_str)
            .ok_or_else(|| Error::msg("edit command needs an op string"))?;
        let part = get_usize(value, "part", 0)?;
        match op {
            "add_note" => Ok(EditCommand::AddNote {
                part,
                measure: get_usize(value, "measure", 0)?,
                voice: get_u8(value, "voice", 1)?,
                pitch: get_string(value, "pitch", "C4")?,
                duration: get_string(value, "duration", "quarter")?,
                dots: get_u8(value, "dots", 0)?,
                accidental: value
                    .get("accidental")
                    .and_then(Value::as_str)
                    .map(|text| text.to_string()),
            }),
            "add_rest" => Ok(EditCommand::AddRest {
                part,
                measure: get_usize(value, "measure", 0)?,
                voice: get_u8(value, "voice", 1)?,
                duration: get_string(value, "duration", "quarter")?,
                dots: get_u8(value, "dots", 0)?,
            }),
            "add_chord_note" => Ok(EditCommand::AddChordNote {
                part,
                measure: get_usize(value, "measure", 0)?,
                voice: get_u8(value, "voice", 1)?,
                event: get_usize(value, "event", 0)?,
                pitch: get_string(value, "pitch", "E4")?,
            }),
            "delete_event" => Ok(EditCommand::DeleteEvent {
                part,
                measure: get_usize(value, "measure", 0)?,
                voice: get_u8(value, "voice", 1)?,
                event: get_usize(value, "event", 0)?,
            }),
            "set_pitch" => Ok(EditCommand::SetPitch {
                part,
                measure: get_usize(value, "measure", 0)?,
                voice: get_u8(value, "voice", 1)?,
                event: get_usize(value, "event", 0)?,
                note: get_usize(value, "note", 0)?,
                pitch: get_string(value, "pitch", "C4")?,
            }),
            "set_duration" => Ok(EditCommand::SetDuration {
                part,
                measure: get_usize(value, "measure", 0)?,
                voice: get_u8(value, "voice", 1)?,
                event: get_usize(value, "event", 0)?,
                duration: get_string(value, "duration", "quarter")?,
                dots: get_u8(value, "dots", 0)?,
            }),
            "set_dynamic" => Ok(EditCommand::SetDynamic {
                part,
                measure: get_usize(value, "measure", 0)?,
                voice: get_u8(value, "voice", 1)?,
                event: get_usize(value, "event", 0)?,
                dynamic: value
                    .get("dynamic")
                    .and_then(Value::as_str)
                    .map(|text| text.to_string()),
            }),
            "set_lyric" => Ok(EditCommand::SetLyric {
                part,
                measure: get_usize(value, "measure", 0)?,
                voice: get_u8(value, "voice", 1)?,
                event: get_usize(value, "event", 0)?,
                note: get_usize(value, "note", 0)?,
                text: get_string(value, "text", "")?,
                syllabic: value
                    .get("syllabic")
                    .and_then(Value::as_str)
                    .map(|text| text.to_string()),
            }),
            "set_tempo" => Ok(EditCommand::SetTempo {
                part,
                measure: get_usize(value, "measure", 0)?,
                bpm: get_usize(value, "bpm", 96)? as u32,
            }),
            "set_title" => Ok(EditCommand::SetTitle {
                title: get_string(value, "title", "Untitled")?,
            }),
            "transpose" => Ok(EditCommand::Transpose {
                part,
                semitones: get_i8(value, "semitones", 0)?,
            }),
            "add_measure" => Ok(EditCommand::AddMeasure { part }),
            "delete_measure" => Ok(EditCommand::DeleteMeasure {
                part,
                measure: get_usize(value, "measure", 0)?,
            }),
            "set_key" => Ok(EditCommand::SetKey {
                part,
                measure: get_usize(value, "measure", 0)?,
                fifths: get_i8(value, "fifths", 0)?,
            }),
            "select_part" => Ok(EditCommand::SelectPart { part }),
            "add_part" => Ok(EditCommand::AddPart {
                name: get_string(value, "name", "New part")?,
            }),
            "delete_part" => Ok(EditCommand::DeletePart { part }),
            other => Err(Error::msg(format!("unknown edit op {other:?}"))),
        }
    }
}

fn remap_hits(layout: &mut Layout, part: usize) {
    for page in &mut layout.pages {
        for hit in &mut page.hits {
            hit.part = part;
        }
    }
}

fn parse_duration(value: &str, dots: u8) -> Result<Duration> {
    let value = NoteValue::from_musicxml_type(value)
        .ok_or_else(|| Error::msg(format!("unsupported duration {value:?}")))?;
    if dots > Duration::MAX_DOTS {
        return Err(Error::out_of_range(format!("dots {dots}")));
    }
    Ok(Duration::new(value, dots))
}

fn get_usize(value: &Value, key: &str, default: usize) -> Result<usize> {
    match value.get(key) {
        None => Ok(default),
        Some(value) => value
            .as_u64()
            .map(|number| number as usize)
            .ok_or_else(|| Error::msg(format!("{key} must be a non-negative integer"))),
    }
}

fn get_u8(value: &Value, key: &str, default: u8) -> Result<u8> {
    match value.get(key) {
        None => Ok(default),
        Some(value) => value
            .as_u64()
            .filter(|number| *number <= u8::MAX as u64)
            .map(|number| number as u8)
            .ok_or_else(|| Error::msg(format!("{key} must fit in a byte"))),
    }
}

fn get_i8(value: &Value, key: &str, default: i8) -> Result<i8> {
    match value.get(key) {
        None => Ok(default),
        Some(value) => value
            .as_i64()
            .filter(|number| *number >= i8::MIN as i64 && *number <= i8::MAX as i64)
            .map(|number| number as i8)
            .ok_or_else(|| Error::msg(format!("{key} must fit in a signed byte"))),
    }
}

fn get_string(value: &Value, key: &str, default: &str) -> Result<String> {
    match value.get(key) {
        None => Ok(default.to_string()),
        Some(value) => value
            .as_str()
            .map(|text| text.to_string())
            .ok_or_else(|| Error::msg(format!("{key} must be a string"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_document_has_summary_and_layout() {
        let document = Document::demo();
        let summary = document.summary().stringify();
        assert!(summary.contains("\"parts\""));
        assert!(summary.contains("Melody"));
        let layout = document.layout(Some(0)).unwrap();
        assert!(layout.page_count() >= 1);
        let svg = document.svg(Some(0), true).unwrap();
        assert!(svg.contains("data-part="));
    }

    #[test]
    fn add_note_and_undo_round_trip() {
        let mut document = Document::demo();
        let before = document.score.note_count();
        let command = EditCommand::AddNote {
            part: 0,
            measure: 0,
            voice: 1,
            pitch: "C5".to_string(),
            duration: "quarter".to_string(),
            dots: 0,
            accidental: None,
        };
        // The first demo measure is full, so this must fail without changes.
        assert!(document.apply(command.clone()).is_err());
        assert_eq!(document.score.note_count(), before);
        let command = EditCommand::AddRest {
            part: 0,
            measure: 0,
            voice: 2,
            duration: "quarter".to_string(),
            dots: 0,
        };
        document.apply(command).unwrap();
        assert!(document.can_undo());
        document.undo().unwrap();
        assert_eq!(document.score.note_count(), before);
    }

    #[test]
    fn delete_last_event_and_measure() {
        let mut document = Document::demo();
        document
            .apply(EditCommand::DeleteEvent {
                part: 0,
                measure: 0,
                voice: 1,
                event: 0,
            })
            .unwrap();
        document
            .apply(EditCommand::DeleteMeasure {
                part: 0,
                measure: 7,
            })
            .unwrap();
        assert_eq!(document.score.parts[0].measures.len(), 7);
        assert!(document
            .apply(EditCommand::DeleteMeasure {
                part: 0,
                measure: 0,
            })
            .is_ok());
    }

    #[test]
    fn edit_command_parsing_validates_shape() {
        let value = crate::json::parse(
            r#"{"op":"add_note","part":0,"measure":1,"voice":1,"pitch":"D4","duration":"eighth","dots":0}"#,
        )
        .unwrap();
        let command = EditCommand::from_json(&value).unwrap();
        assert!(matches!(command, EditCommand::AddNote { .. }));
        let bad = crate::json::parse(r#"{"op":"nope"}"#).unwrap();
        assert!(EditCommand::from_json(&bad).is_err());
    }

    #[test]
    fn import_replaces_document_and_exports_round_trip() {
        let mut document = Document::demo();
        let xml = document.musicxml(None).unwrap();
        document.import_xml(&xml).unwrap();
        assert!(document.score.note_count() > 0);
        assert!(document.pdf(None).unwrap().starts_with(b"%PDF"));
        assert!(document.midi(None).unwrap().starts_with(b"MThd"));
        assert!(document.svg(None, false).unwrap().contains("<svg"));
    }

    #[test]
    fn set_pitch_dynamic_lyric_and_tempo() {
        let mut document = Document::demo();
        document
            .apply(EditCommand::SetPitch {
                part: 0,
                measure: 0,
                voice: 1,
                event: 0,
                note: 0,
                pitch: "C5".to_string(),
            })
            .unwrap();
        document
            .apply(EditCommand::SetDynamic {
                part: 0,
                measure: 0,
                voice: 1,
                event: 0,
                dynamic: Some("pp".to_string()),
            })
            .unwrap();
        document
            .apply(EditCommand::SetLyric {
                part: 0,
                measure: 0,
                voice: 1,
                event: 0,
                text: "la".to_string(),
                syllabic: Some("single".to_string()),
            })
            .unwrap();
        document
            .apply(EditCommand::SetTempo {
                part: 0,
                measure: 0,
                bpm: 120,
            })
            .unwrap();
        assert_eq!(
            document.score.parts[0].measures[0].voice(1).unwrap().events[0]
                .primary_pitch()
                .unwrap()
                .name(),
            "C5"
        );
    }

    #[test]
    fn transpose_and_key_change_are_sound() {
        let mut document = Document::demo();
        document
            .apply(EditCommand::Transpose {
                part: 0,
                semitones: 2,
            })
            .unwrap();
        document
            .apply(EditCommand::SetKey {
                part: 0,
                measure: 0,
                fifths: -2,
            })
            .unwrap();
        assert_eq!(document.score.parts[0].measures[0].attributes.key.fifths, -2);
    }
}
