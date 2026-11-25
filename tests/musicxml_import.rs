//! End-to-end MusicXML import acceptance tests for structural features.

use cadence_core::{BarlineStyle, EndingKind, KeyMode, NoteValue, RepeatDirection};
use cadence_import::import_score;

fn rich_score() -> &'static str {
    r#"<?xml version="1.0" encoding="UTF-8"?>
<score-partwise version="4.0">
  <work><work-title>Rich Import</work-title></work>
  <movement-title>Movement</movement-title>
  <identification>
    <creator type="composer">A. Composer</creator>
    <creator type="poet">P. Poet</creator>
    <creator type="arranger">R. Arranger</creator>
    <rights>CC0</rights>
    <source>unit test</source>
  </identification>
  <part-list>
    <score-part id="P1"><part-name>Flute</part-name><part-abbreviation>Fl.</part-abbreviation>
      <midi-instrument id="P1-I1"><midi-channel>3</midi-channel><midi-program>74</midi-program></midi-instrument>
    </score-part>
  </part-list>
  <part id="P1">
    <measure number="1">
      <attributes><divisions>480</divisions><key><fifths>2</fifths><mode>major</mode></key>
        <time symbol="common"><beats>4</beats><beat-type>4</beat-type></time>
        <clef><sign>G</sign><line>2</line></clef></attributes>
      <print new-system="yes"/>
      <direction><direction-type><rehearsal>Intro</rehearsal></direction-type></direction>
      <direction><direction-type><metronome><beat-unit>half</beat-unit><per-minute>72</per-minute></metronome></direction-type></direction>
      <direction><direction-type><dynamics><p/></dynamics></direction-type></direction>
      <note><pitch><step>D</step><octave>5</octave></pitch><duration>960</duration><type>half</type>
        <lyric number="1"><syllabic>begin</syllabic><text>Hel</text></lyric>
        <notations><slur type="start" number="1"/><articulations><tenuto/></articulations></notations>
      </note>
      <note><pitch><step>E</step><octave>5</octave></pitch><duration>480</duration><type>quarter</type>
        <lyric number="1"><syllabic>end</syllabic><text>lo</text></lyric>
        <notations><slur type="stop" number="1"/></notations>
      </note>
      <note><pitch><step>F</step><alter>1</alter><octave>5</octave></pitch><duration>480</duration><type>quarter</type>
        <accidental>sharp</accidental></note>
      <barline location="right"><ending number="1" type="start"/><repeat direction="backward" times="2"/></barline>
    </measure>
    <measure number="2">
      <attributes><key><fifths>-1</fifths><mode>minor</mode></key><time><beats>3</beats><beat-type>4</beat-type></time></attributes>
      <note><pitch><step>A</step><octave>4</octave></pitch><duration>1440</duration><type>half</type><dot/></note>
      <barline location="right"><bar-style>light-heavy</bar-style><ending number="1" type="stop"/></barline>
    </measure>
    <measure number="3">
      <note><pitch><step>C</step><octave>5</octave></pitch><duration>1440</duration><type>half</type><dot/></note>
    </measure>
  </part>
</score-partwise>"#
}

#[test]
fn metadata_parts_and_midi_metadata_survive_import() {
    let score = import_score(rich_score()).unwrap();
    assert_eq!(score.metadata.title, "Rich Import");
    assert_eq!(score.metadata.subtitle, "Movement");
    assert_eq!(score.metadata.composer, "A. Composer");
    assert_eq!(score.metadata.lyricist, "P. Poet");
    assert_eq!(score.metadata.arranger, "R. Arranger");
    assert_eq!(score.metadata.copyright, "CC0");
    assert_eq!(score.metadata.source, "unit test");
    let part = &score.parts[0];
    assert_eq!(part.name, "Flute");
    assert_eq!(part.abbreviation, "Fl.");
    assert_eq!(part.midi_channel, 2);
    assert_eq!(part.midi_program, 73);
    assert_eq!(part.measures.len(), 3);
}

#[test]
fn directions_marks_and_chords_are_mapped() {
    let score = import_score(rich_score()).unwrap();
    let part = &score.parts[0];
    let measure = &part.measures[0];
    assert_eq!(measure.rehearsal.as_deref(), Some("Intro"));
    assert_eq!(measure.tempo.unwrap().bpm, 72);
    assert_eq!(measure.tempo.unwrap().beat, NoteValue::Half);
    assert!(measure.system_break);
    let first = &measure.voice(1).unwrap().events[0];
    assert_eq!(first.dynamics, Some(cadence_core::Dynamic::P));
    assert!(first.notes()[0].tie.start == false);
    assert_eq!(first.notes()[0].lyrics[0].text, "Hel");
    assert!(first.notes()[0]
        .articulations
        .contains(&cadence_core::Articulation::Tenuto));
    let second = &measure.voice(1).unwrap().events[1];
    assert_eq!(second.notes()[0].lyrics[0].text, "lo");
    let third = &measure.voice(1).unwrap().events[2];
    assert_eq!(third.notes()[0].accidental, Some(cadence_core::Accidental::Sharp));
}

#[test]
fn repeats_endings_barlines_and_changes_are_mapped() {
    let score = import_score(rich_score()).unwrap();
    let part = &score.parts[0];
    let first = &part.measures[0];
    assert_eq!(first.barline, BarlineStyle::Regular);
    assert_eq!(first.repeat.unwrap().direction, RepeatDirection::Backward);
    assert_eq!(first.repeat.unwrap().times, 2);
    assert_eq!(first.ending.unwrap().number, 1);
    assert_eq!(first.ending.unwrap().kind, EndingKind::Start);
    let second = &part.measures[1];
    assert_eq!(second.barline, BarlineStyle::Final);
    assert_eq!(second.ending.unwrap().kind, EndingKind::Stop);
    assert_eq!(second.attributes.key.fifths, -1);
    assert_eq!(second.attributes.key.mode, KeyMode::Minor);
    assert_eq!(second.attributes.time.beats, 3);
    assert_eq!(second.attributes.time.beat_type, 4);
    assert!(second.explicit_attributes);
}

#[test]
fn pickup_and_forward_gaps_are_materialized_as_rests() {
    let xml = r#"<score-partwise><part-list><score-part id="P1"><part-name>Pickup</part-name></score-part></part-list>
    <part id="P1">
      <measure number="1"><attributes><divisions>480</divisions></attributes>
        <note><pitch><step>G</step><octave>4</octave></pitch><duration>480</duration><type>quarter</type></note>
      </measure>
      <measure number="2"><attributes><divisions>480</divisions></attributes>
        <note><pitch><step>C</step><octave>5</octave></pitch><duration>1920</duration><type>whole</type><voice>1</voice></note>
        <backup><duration>1920</duration></backup>
        <forward><duration>480</duration></forward>
        <note><pitch><step>E</step><octave>4</octave></pitch><duration>960</duration><type>half</type><voice>2</voice></note>
      </measure>
    </part></score-partwise>"#;
    let score = import_score(xml).unwrap();
    let part = &score.parts[0];
    assert_eq!(part.measure_start_ticks(1), 480);
    let voice2 = part.measures[1].voice(2).unwrap();
    assert_eq!(voice2.events.len(), 2);
    assert!(voice2.events[0].is_rest());
    assert_eq!(voice2.events[0].duration, cadence_core::Duration::quarter());
    assert_eq!(voice2.events[1].duration, cadence_core::Duration::half());
}

#[test]
fn import_rejects_structurally_invalid_scores_cleanly() {
    let cases = [
        "<score-partwise><part-list/></score-partwise>",
        "<score-partwise><part-list><score-part id=\"P1\"><part-name>X</part-name></score-part></part-list><part id=\"P1\"><measure number=\"1\"><note><pitch><step>C</step><octave>4</octave></pitch></note></measure></part></score-partwise>",
        "<score-partwise><part-list><score-part id=\"P1\"><part-name>X</part-name></score-part></part-list><part id=\"P1\"><measure number=\"1\"><attributes><divisions>1</divisions></attributes><note><pitch><step>C</step><octave>4</octave></pitch><duration>3</duration><type>whole</type></note></measure></part></score-partwise>",
        "<score-partwise><part-list><score-part id=\"P1\"><part-name>X</part-name></score-part></part-list><part id=\"P1\"><measure number=\"1\"><attributes><divisions>480</divisions><staves>2</staves></attributes><note><rest/><duration>1920</duration><type>whole</type></note></measure></part></score-partwise>",
    ];
    for case in cases {
        assert!(import_score(case).is_err(), "accepted {case}");
    }
}
