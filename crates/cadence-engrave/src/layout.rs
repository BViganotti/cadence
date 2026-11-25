//! Deterministic score layout: spacing, vertical placement, systems, pages,
//! glyphs, hit regions and collision-visible symbols.

use crate::beaming;
use crate::geom::{Align, Bbox, Element, HitRegion, PathSeg, Point, Symbol, SymbolKind};
use crate::glyphs;
use crate::options::Options;
use crate::pagination::{self, MeasurePlan};
use crate::spacing::{self, MeasureSpacing};
use crate::text;
use cadence_core::notation::{resolve_slurs, resolve_ties};
use cadence_core::{
    Accidental, Attributes, Clef, Measure, Metadata, NoteValue, Score,
};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct Layout {
    pub metadata: Metadata,
    pub options: Options,
    pub pages: Vec<PageLayout>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PageLayout {
    pub number: usize,
    pub width: f64,
    pub height: f64,
    pub elements: Vec<Element>,
    pub symbols: Vec<Symbol>,
    pub hits: Vec<HitRegion>,
    pub systems: Vec<SystemLayout>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SystemLayout {
    pub index: usize,
    pub top: f64,
    pub bottom: f64,
    pub x_start: f64,
    pub x_end: f64,
    pub staff_tops: Vec<f64>,
    pub measures: Vec<usize>,
}

impl Layout {
    pub fn symbols(&self) -> impl Iterator<Item = &Symbol> {
        self.pages.iter().flat_map(|page| page.symbols.iter())
    }

    pub fn hits(&self) -> impl Iterator<Item = &HitRegion> {
        self.pages.iter().flat_map(|page| page.hits.iter())
    }

    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    pub fn system_count(&self) -> usize {
        self.pages.iter().map(|page| page.systems.len()).sum()
    }
}

struct StaffExtent {
    above: f64,
    below: f64,
}

struct SystemGeom {
    plan: pagination::SystemPlan,
    tops: Vec<f64>,
    height: f64,
}

struct NoteGeom {
    x: f64,
    y: f64,
}

struct EventGeom {
    event_index: usize,
    voice_index: usize,
    x: f64,
    notes: Vec<NoteGeom>,
    dir_up: bool,
    stem_x: f64,
    base_y: f64,
    tip_y: f64,
    has_stem: bool,
    beamed: bool,
    is_rest: bool,
    rest_y: f64,
    duration: cadence_core::Duration,
}

struct SystemContext<'a> {
    options: &'a Options,
    spacings: &'a [Vec<MeasureSpacing>],
    measure_index: usize,
    page: usize,
    system: usize,
    staff_top: f64,
    staff_bottom: f64,
    part: usize,
    clef: Clef,
    divisions: u32,
    measure_x: f64,
}

pub fn engrave(score: &Score, options: &Options) -> cadence_core::Result<Layout> {
    options.validate()?;
    score.validate()?;
    let mut layout = Layout {
        metadata: score.metadata.clone(),
        options: *options,
        pages: Vec::new(),
    };
    if score.parts.is_empty() || score.measure_count() == 0 {
        let mut page = PageLayout::new(1, options);
        if !score.metadata.title.is_empty() {
            draw_header(&mut page, &score.metadata, options);
        }
        layout.pages.push(page);
        return Ok(layout);
    }

    let measure_count = score.measure_count();
    let mut spacings: Vec<Vec<MeasureSpacing>> = Vec::new();
    for part in &score.parts {
        let mut part_spacings = Vec::with_capacity(part.measures.len());
        for measure in &part.measures {
            part_spacings.push(spacing::space_measure(measure, options));
        }
        spacings.push(part_spacings);
    }

    let mut global_prefix: f64 = 0.0;
    for part in &score.parts {
        for measure in &part.measures {
            global_prefix = global_prefix
                .max(spacing::prefix_width(measure.attributes, options));
        }
    }
    if global_prefix == 0.0 {
        global_prefix = spacing::prefix_width(Attributes::default(), options);
    }

    let mut plans = Vec::with_capacity(measure_count);
    for measure_index in 0..measure_count {
        let mut width = 0.0f64;
        for (part_index, _part) in score.parts.iter().enumerate() {
            if let Some(measure_spacing) = spacings
                .get(part_index)
                .and_then(|part_spacings| part_spacings.get(measure_index))
            {
                width = width.max(measure_spacing.width);
            }
        }
        let mut system_break = false;
        let mut page_break = false;
        for part in &score.parts {
            if let Some(measure) = part.measures.get(measure_index) {
                page_break |= measure.page_break;
                system_break |= measure.system_break;
                if measure_index > 0 {
                    if let Some(previous) = part.measures.get(measure_index - 1) {
                        if previous.attributes != measure.attributes {
                            system_break = true;
                        }
                    }
                }
            }
        }
        plans.push(MeasurePlan {
            index: measure_index,
            width: width.max(60.0),
            system_break,
            page_break,
        });
    }

    let system_plans = pagination::plan_systems(
        &plans,
        global_prefix,
        options.content_width(),
        options.max_measures_per_system,
    );

    let mut geoms: Vec<SystemGeom> = Vec::new();
    for plan in system_plans {
        let extents = staff_extents(score, &plan.measures, options);
        let s = options.staff_space;
        let staff_height = options.staff_height();
        let mut tops = Vec::with_capacity(extents.len());
        for (index, extent) in extents.iter().enumerate() {
            let top = if index == 0 {
                extent.above + 0.6 * s
            } else {
                let previous_top = tops[index - 1];
                previous_top
                    + staff_height
                    + extent.above.max(extents[index - 1].below)
                    + 1.2 * s
            };
            tops.push(top);
        }
        let last_top = tops.last().copied().unwrap_or(0.0);
        let last_below = extents.last().map(|extent| extent.below).unwrap_or(0.0);
        let height = last_top + staff_height + last_below + 0.5 * s;
        geoms.push(SystemGeom { plan, tops, height });
    }

    let header_height = header_height(&score.metadata, options);
    let first_capacity = (options.content_height() - header_height - options.header_gap).max(80.0);
    let heights: Vec<f64> = geoms.iter().map(|geom| geom.height).collect();
    let force_page: Vec<bool> = geoms.iter().map(|geom| geom.plan.new_page).collect();
    let page_systems = pagination::paginate(
        &heights,
        options.system_gap,
        first_capacity,
        options.content_height(),
        &force_page,
    );

    let mut anchors: HashMap<(usize, usize, u8, usize, usize), NotePosGeom> = HashMap::new();
    for (page_index, systems) in page_systems.iter().enumerate() {
        let mut page = PageLayout::new(page_index + 1, options);
        let is_first = page_index == 0;
        if is_first {
            draw_header(&mut page, &score.metadata, options);
        }
        draw_footer(&mut page, page_index + 1, options);
        let mut y = options.margins.top
            + if is_first {
                header_height + options.header_gap
            } else {
                0.0
            };
        let system_x_start = options.margins.left;
        for system_index in systems {
            let geom = &geoms[*system_index];
            let system_top = y;
            let mut staff_tops_abs = Vec::with_capacity(geom.tops.len());
            for top in &geom.tops {
                staff_tops_abs.push(system_top + *top);
            }
            let system_width = global_prefix
                + geom
                    .plan
                    .measures
                    .iter()
                    .map(|index| plans[*index].width)
                    .sum::<f64>();
            let system_x_end = system_x_start + system_width;
            let system_layout = SystemLayout {
                index: *system_index,
                top: system_top,
                bottom: system_top + geom.height,
                x_start: system_x_start,
                x_end: system_x_end,
                staff_tops: staff_tops_abs.clone(),
                measures: geom.plan.measures.clone(),
            };
            draw_system(
                &mut page,
                &mut anchors,
                score,
                options,
                &spacings,
                &plans,
                geom,
                *system_index,
                page_index,
                system_x_start,
                system_x_end,
                &staff_tops_abs,
                global_prefix,
            );
            page.systems.push(system_layout);
            y += geom.height + options.system_gap;
        }
        layout.pages.push(page);
    }

    draw_slurs_and_ties(&mut layout, score, &anchors, options);
    Ok(layout)
}

#[derive(Clone, Copy)]
struct NotePosGeom {
    x: f64,
    y: f64,
    staff_top: f64,
    staff_bottom: f64,
    page: usize,
    system: usize,
}

impl PageLayout {
    fn new(number: usize, options: &Options) -> PageLayout {
        PageLayout {
            number,
            width: options.page_width,
            height: options.page_height,
            elements: Vec::new(),
            symbols: Vec::new(),
            hits: Vec::new(),
            systems: Vec::new(),
        }
    }
}

fn header_height(metadata: &Metadata, options: &Options) -> f64 {
    if metadata.title.is_empty() {
        return 0.0;
    }
    let mut height = options.title_size * 1.2;
    if !metadata.subtitle.is_empty() {
        height += options.subtitle_size * 1.5;
    }
    if !metadata.composer.is_empty() || !metadata.lyricist.is_empty() {
        height += options.text_size * 1.6;
    }
    height + 10.0
}

fn draw_header(page: &mut PageLayout, metadata: &Metadata, options: &Options) {
    let center = options.page_width / 2.0;
    let mut y = options.margins.top;
    if !metadata.title.is_empty() {
        page.elements.push(Element::text(
            center,
            y,
            options.title_size,
            &metadata.title,
            Align::Center,
        ));
        page.symbols.push(Symbol::new(
            SymbolKind::Title,
            0,
            0,
            0,
            0,
            0,
            0,
            Bbox::new(
                center - text::width(&metadata.title, options.title_size) / 2.0,
                y,
                text::width(&metadata.title, options.title_size),
                options.title_size,
            ),
        ));
        y += options.title_size * 1.2;
    }
    if !metadata.subtitle.is_empty() {
        page.elements.push(Element::text(
            center,
            y,
            options.subtitle_size,
            &metadata.subtitle,
            Align::Center,
        ));
        page.symbols.push(Symbol::new(
            SymbolKind::Subtitle,
            0,
            0,
            0,
            0,
            0,
            0,
            Bbox::new(
                center - text::width(&metadata.subtitle, options.subtitle_size) / 2.0,
                y,
                text::width(&metadata.subtitle, options.subtitle_size),
                options.subtitle_size,
            ),
        ));
        y += options.subtitle_size * 1.5;
    }
    let mut left = !metadata.lyricist.is_empty();
    if !metadata.lyricist.is_empty() {
        page.elements.push(Element::text(
            options.margins.left,
            y,
            options.text_size,
            &metadata.lyricist,
            Align::Start,
        ));
        left = true;
    }
    if !metadata.composer.is_empty() {
        page.elements.push(Element::text(
            options.page_width - options.margins.right,
            y,
            options.text_size,
            &metadata.composer,
            Align::End,
        ));
        page.symbols.push(Symbol::new(
            SymbolKind::Composer,
            0,
            0,
            0,
            0,
            0,
            0,
            Bbox::new(
                options.page_width
                    - options.margins.right
                    - text::width(&metadata.composer, options.text_size),
                y,
                text::width(&metadata.composer, options.text_size),
                options.text_size,
            ),
        ));
    }
    let _ = left;
}

fn draw_footer(page: &mut PageLayout, number: usize, options: &Options) {
    let y = options.page_height - options.margins.bottom * 0.55;
    let label = number.to_string();
    page.elements.push(Element::text(
        options.page_width / 2.0,
        y,
        options.small_size,
        &label,
        Align::Center,
    ));
    page.symbols.push(Symbol::new(
        SymbolKind::Footer,
        number.saturating_sub(1),
        0,
        0,
        0,
        0,
        0,
        Bbox::new(
            options.page_width / 2.0 - text::width(&label, options.small_size) / 2.0,
            y,
            text::width(&label, options.small_size),
            options.small_size,
        ),
    ));
}

fn staff_extents(score: &Score, measures: &[usize], options: &Options) -> Vec<StaffExtent> {
    let s = options.staff_space;
    let staff_height = options.staff_height();
    let line_height = options.lyric_size * 1.55;
    let mut extents = Vec::with_capacity(score.parts.len());
    for (part_index, part) in score.parts.iter().enumerate() {
        let mut above = 1.6 * s;
        let mut below = 2.2 * s;
        if options.show_measure_numbers {
            above += 1.2 * s;
        }
        let mut max_lyric_line = 0.0f64;
        let mut max_note_bottom = staff_height;
        let mut has_dynamic = false;
        let mut max_voice_index = 0usize;
        let mut has_tempo = false;
        let mut has_rehearsal = false;
        let slur_resolution = resolve_slurs(part);
        let tie_resolution = resolve_ties(part);
        let has_slur = slur_resolution.spans.iter().any(|span| {
            measures.contains(&span.start.measure) && measures.contains(&span.end.measure)
        }) || tie_resolution.spans.iter().any(|span| {
            measures.contains(&span.start.measure) && measures.contains(&span.end.measure)
        });
        for measure_index in measures {
            let Some(measure) = part.measures.get(*measure_index) else {
                continue;
            };
            if part_index == 0 && measure.tempo.is_some() {
                has_tempo = true;
            }
            if part_index == 0 && measure.rehearsal.is_some() {
                has_rehearsal = true;
            }
            for (voice_index, voice) in measure.voices.iter().enumerate() {
                for event in &voice.events {
                    for note in event.notes() {
                        let step = measure
                            .attributes
                            .clef
                            .staff_step(note.pitch.diatonic_index());
                        let rel = staff_height - step as f64 * s * 0.5;
                        above = above.max(-rel + 3.5 * s);
                        below = below.max(rel - staff_height + 3.5 * s);
                        max_note_bottom = max_note_bottom.max(rel + 3.3 * s);
                    }
                    for note in event.notes() {
                        for lyric in &note.lyrics {
                            let line = voice_index as f64 * 3.0
                                + (lyric.number.saturating_sub(1)) as f64;
                            max_lyric_line = max_lyric_line.max(line);
                        }
                    }
                    if event.dynamics.is_some() {
                        has_dynamic = true;
                        max_voice_index = max_voice_index.max(voice_index);
                    }
                }
            }
        }
        let lyric_base = max_note_bottom.max(staff_height) + 1.2 * s;
        if max_lyric_line > 0.0 {
            below = below.max(
                lyric_base
                    + (max_lyric_line + 1.0) * line_height
                    + options.lyric_size
                    + 0.6 * s,
            );
        }
        if has_dynamic {
            below = below.max(
                lyric_base
                    + (max_lyric_line + 1.0) * line_height
                    + max_voice_index as f64 * options.text_size * 1.4
                    + options.text_size
                    + 0.6 * s,
            );
        }
        if has_slur {
            above += 1.0 * s;
            below += 0.8 * s;
        }
        if part_index == 0 {
            if has_rehearsal {
                above += options.text_size * 1.7;
            }
            if has_tempo {
                above += options.text_size * 1.7;
            }
        }
        extents.push(StaffExtent {
            above: above + 0.4 * s,
            below: below + 0.4 * s,
        });
    }
    extents
}

fn plan_width(plans: &[MeasurePlan], index: usize) -> f64 {
    // plans is still passed through for clarity in the caller; the width is
    // recovered from the measure plan list captured by the system plan.
    plans.get(index).map(|plan| plan.width).unwrap_or(60.0)
}

fn draw_system(
    page: &mut PageLayout,
    anchors: &mut HashMap<(usize, usize, u8, usize, usize), NotePosGeom>,
    score: &Score,
    options: &Options,
    spacings: &[Vec<MeasureSpacing>],
    plans: &[MeasurePlan],
    geom: &SystemGeom,
    system_index: usize,
    page_index: usize,
    x_start: f64,
    x_end: f64,
    staff_tops: &[f64],
    global_prefix: f64,
) {
    let s = options.staff_space;
    let staff_height = options.staff_height();
    for part_index in 0..score.parts.len() {
        let top = staff_tops[part_index];
        for line in 0..5 {
            let y = top + line as f64 * s;
            page.elements.push(Element::line(
                x_start,
                y,
                x_end,
                y,
                0.55,
            ));
        }
        if options.show_part_names {
            let part = &score.parts[part_index];
            let label = if part.abbreviation.is_empty() {
                truncate(&part.name, 10)
            } else {
                truncate(&part.abbreviation, 10)
            };
            if !label.is_empty() {
                page.elements.push(Element::text(
                    x_start - 5.0,
                    top + staff_height / 2.0 - options.small_size * 0.35,
                    options.small_size,
                    &label,
                    Align::End,
                ));
                page.symbols.push(Symbol::new(
                    SymbolKind::PartName,
                    page_index,
                    system_index,
                    part_index,
                    geom.plan.measures.first().copied().unwrap_or(0),
                    0,
                    0,
                    Bbox::new(
                        x_start - 5.0 - text::width(&label, options.small_size),
                        top + staff_height / 2.0 - options.small_size * 0.35,
                        text::width(&label, options.small_size),
                        options.small_size,
                    ),
                ));
            }
        }
    }

    // Prefix: clef, key signature and time signature on each staff.
    for part_index in 0..score.parts.len() {
        let part = &score.parts[part_index];
        let measure_index = geom.plan.measures.first().copied().unwrap_or(0);
        let attributes = part
            .measures
            .get(measure_index)
            .or_else(|| part.measures.last())
            .map(|measure| measure.attributes)
            .unwrap_or_default();
        let top = staff_tops[part_index];
        let clef_width = glyphs::clef_width(attributes.clef, s);
        page.elements
            .extend(glyphs::clef(attributes.clef, x_start + 0.3 * s, top, s));
        page.symbols.push(Symbol::new(
            SymbolKind::Clef,
            page_index,
            system_index,
            part_index,
            measure_index,
            0,
            0,
            Bbox::new(x_start, top - s, clef_width, staff_height + 2.0 * s),
        ));
        let key_x = x_start + 0.3 * s + clef_width + 0.45 * s;
        let (key_elements, key_width) =
            glyphs::key_signature(attributes.key, attributes.clef, key_x, top, s);
        page.elements.extend(key_elements);
        if key_width > 0.0 {
            page.symbols.push(Symbol::new(
                SymbolKind::KeySignature,
                page_index,
                system_index,
                part_index,
                measure_index,
                0,
                0,
                Bbox::new(key_x, top, key_width, staff_height),
            ));
        }
        let time_x = key_x + key_width + 0.45 * s;
        page.elements
            .extend(glyphs::time_signature(attributes.time, time_x, top, s));
        page.symbols.push(Symbol::new(
            SymbolKind::TimeSignature,
            page_index,
            system_index,
            part_index,
            measure_index,
            0,
            0,
            Bbox::new(
                time_x,
                top,
                glyphs::time_signature_width(attributes.time, s),
                staff_height,
            ),
        ));
    }

    let mut measure_x = x_start + global_prefix;
    for (position, measure_index) in geom.plan.measures.iter().enumerate() {
        let measure_width = plan_width(plans, *measure_index);
        // Rehearsal, tempo and the system-start measure number only on the
        // first measure of a system.
        if position == 0 {
            for part in &score.parts {
                if let Some(measure) = part.measures.get(*measure_index) {
                    draw_measure_heading(
                        page,
                        options,
                        measure,
                        measure_x,
                        staff_tops.first().copied().unwrap_or(0.0),
                        page_index,
                        system_index,
                        *measure_index,
                    );
                    break;
                }
            }
        }
        for part_index in 0..score.parts.len() {
            let part = &score.parts[part_index];
            let Some(measure) = part.measures.get(*measure_index) else {
                continue;
            };
            let context = SystemContext {
                options,
                spacings,
                measure_index: *measure_index,
                page: page_index,
                system: system_index,
                staff_top: staff_tops[part_index],
                staff_bottom: staff_tops[part_index] + staff_height,
                part: part_index,
                clef: measure.attributes.clef,
                divisions: measure.attributes.divisions,
                measure_x,
            };
            draw_measure(page, anchors, &context, measure);
            draw_barline(
                page,
                &context,
                measure,
                measure_x + measure_width,
                page_index,
                system_index,
            );
        }
        measure_x += measure_width;
    }
}

fn draw_measure_heading(
    page: &mut PageLayout,
    options: &Options,
    measure: &Measure,
    x: f64,
    top: f64,
    page_index: usize,
    system_index: usize,
    measure_index: usize,
) {
    let s = options.staff_space;
    let mut y = top - 1.2 * s;
    if options.show_measure_numbers {
        let label = measure.number.to_string();
        let width = text::width(&label, options.small_size);
        page.elements.push(Element::text(
            x + 2.0,
            y - options.small_size,
            options.small_size,
            &label,
            Align::Start,
        ));
        page.symbols.push(Symbol::new(
            SymbolKind::MeasureNumber,
            page_index,
            system_index,
            0,
            measure_index,
            0,
            0,
            Bbox::new(x + 2.0, y - options.small_size, width, options.small_size),
        ));
        y -= options.small_size * 1.5;
    }
    if let Some(rehearsal) = &measure.rehearsal {
        let width = text::width(rehearsal, options.text_size);
        page.elements.push(Element::rect(
            x + 1.0,
            y - options.text_size - 2.0,
            width + 6.0,
            options.text_size + 4.0,
            false,
        ));
        page.elements.push(Element::text(
            x + 4.0,
            y - options.text_size,
            options.text_size,
            rehearsal,
            Align::Start,
        ));
        page.symbols.push(Symbol::new(
            SymbolKind::Rehearsal,
            page_index,
            system_index,
            0,
            measure_index,
            0,
            0,
            Bbox::new(
                x + 1.0,
                y - options.text_size - 2.0,
                width + 6.0,
                options.text_size + 4.0,
            ),
        ));
        y -= options.text_size * 1.7;
    }
    if let Some(tempo) = measure.tempo {
        let label = format!("{} = {}", tempo.beat.label(), tempo.bpm);
        let width = text::width(&label, options.small_size);
        page.elements.push(Element::text(
            x + 2.0,
            y - options.small_size,
            options.small_size,
            &label,
            Align::Start,
        ));
        page.symbols.push(Symbol::new(
            SymbolKind::Tempo,
            page_index,
            system_index,
            0,
            measure_index,
            0,
            0,
            Bbox::new(x + 2.0, y - options.small_size, width, options.small_size),
        ));
    }
}

fn draw_barline(
    page: &mut PageLayout,
    context: &SystemContext<'_>,
    measure: &Measure,
    x: f64,
    page_index: usize,
    system_index: usize,
) {
    let top = context.staff_top;
    let bottom = context.staff_bottom;
    let width = 0.8;
    let mut lines = 0;
    match measure.barline {
        cadence_core::BarlineStyle::Regular => {
            page.elements.push(Element::line(x, top, x, bottom, width));
            lines = 1;
        }
        cadence_core::BarlineStyle::Double => {
            page.elements.push(Element::line(x - 4.0, top, x - 4.0, bottom, 0.7));
            page.elements.push(Element::line(x, top, x, bottom, 0.7));
            lines = 2;
        }
        cadence_core::BarlineStyle::Final => {
            page.elements.push(Element::line(x - 4.0, top, x - 4.0, bottom, 0.7));
            page.elements.push(Element::rect(x - 1.5, top, 3.0, bottom - top, true));
            lines = 2;
        }
        cadence_core::BarlineStyle::Dashed => {
            page.elements.push(Element::line(x, top, x, bottom, width));
            lines = 1;
        }
        cadence_core::BarlineStyle::Dotted => {
            page.elements.push(Element::line(x, top, x, bottom, width));
            lines = 1;
        }
        cadence_core::BarlineStyle::Invisible => {}
    }
    let _ = lines;
    if let Some(ending) = measure.ending {
        let label = ending.label();
        let y = top - 1.9 * context.options.staff_space;
        let width = text::width(&label, context.options.small_size) + 12.0;
        page.elements.push(Element::line(
            x - width,
            y - 3.0,
            x,
            y - 3.0,
            0.6,
        ));
        page.elements.push(Element::text(
            x - width + 3.0,
            y - context.options.small_size,
            context.options.small_size,
            &label,
            Align::Start,
        ));
        page.symbols.push(Symbol::new(
            SymbolKind::MeasureNumber,
            page_index,
            system_index,
            context.part,
            context.measure_index,
            0,
            0,
            Bbox::new(
                x - width,
                y - context.options.small_size - 3.0,
                width,
                context.options.small_size + 3.0,
            ),
        ));
    }
    if let Some(repeat) = measure.repeat {
        let s = context.options.staff_space;
        let dot_y = top + 2.0 * s;
        match repeat.direction {
            cadence_core::RepeatDirection::Forward => {
                page.elements.push(Element::line(x - 7.0, top, x - 7.0, bottom, 0.8));
                page.elements.push(Element::ellipse(x - 4.5, dot_y, 0.7, 0.7, true));
                page.elements.push(Element::ellipse(x - 4.5, dot_y + s, 0.7, 0.7, true));
            }
            cadence_core::RepeatDirection::Backward => {
                page.elements.push(Element::line(x + 4.0, top, x + 4.0, bottom, 0.8));
                page.elements.push(Element::ellipse(x + 1.5, dot_y, 0.7, 0.7, true));
                page.elements.push(Element::ellipse(x + 1.5, dot_y + s, 0.7, 0.7, true));
            }
        }
    }
}

fn draw_measure(
    page: &mut PageLayout,
    anchors: &mut HashMap<(usize, usize, u8, usize, usize), NotePosGeom>,
    context: &SystemContext<'_>,
    measure: &Measure,
) {
    let s = context.options.staff_space;
    let staff_height = context.options.staff_height();
    let divisions = context.divisions;
    let spacing = context
        .spacings
        .get(context.part)
        .and_then(|part| part.get(context.measure_index));
    let Some(spacing) = spacing else {
        return;
    };
    let accidental_map = accidental_map(measure);
    let line_height = context.options.lyric_size * 1.55;
    let mut content_bottom = context.staff_bottom;
    for voice in &measure.voices {
        for event in &voice.events {
            for note in event.notes() {
                let step = context.clef.staff_step(note.pitch.diatonic_index());
                let y = context.staff_top + staff_height - step as f64 * s * 0.5;
                content_bottom = content_bottom.max(y + 3.3 * s);
            }
        }
    }
    let lyric_base = content_bottom + 1.2 * s;
    let max_lyric_line: f64 = measure
        .voices
        .iter()
        .enumerate()
        .flat_map(|(voice_index, voice)| {
            voice.events.iter().flat_map(move |event| {
                event.notes().iter().flat_map(move |note| {
                    note.lyrics.iter().map(move |lyric| {
                        voice_index as f64 * 3.0 + (lyric.number.saturating_sub(1)) as f64
                    })
                })
            })
        })
        .fold(0.0, f64::max);

    for (voice_index, voice) in measure.voices.iter().enumerate() {
        let mut tick = 0u64;
        let mut event_geoms: Vec<EventGeom> = Vec::with_capacity(voice.events.len());
        for (event_index, event) in voice.events.iter().enumerate() {
            let column = spacing.column_for_tick(tick);
            let chord_pad = if event.is_chord() {
                event.notes().len().saturating_sub(1) as f64 * 0.95 * s
            } else {
                0.0
            };
            let anchor_x = context.measure_x
                + spacing
                    .columns
                    .get(column)
                    .map(|column| column.x)
                    .unwrap_or(0.0)
                + 2.0 * s
                + chord_pad
                + if voice_index > 0 { 4.0 * s } else { 0.0 };
            let mut geom = EventGeom {
                event_index,
                voice_index,
                x: anchor_x,
                notes: Vec::new(),
                dir_up: true,
                stem_x: anchor_x,
                base_y: 0.0,
                tip_y: 0.0,
                has_stem: event.duration.value.has_stem(),
                beamed: false,
                is_rest: event.is_rest(),
                rest_y: context.staff_top + staff_height / 2.0,
                duration: event.duration,
            };
            if event.is_rest() {
                let rest_x = anchor_x + if voice_index > 0 { 2.0 * s } else { 0.0 };
                let mut rest_top = context.staff_top;
                if voice_index > 0 {
                    rest_top += 1.2 * s;
                }
                page.elements
                    .extend(glyphs::rest(event.duration.value, rest_x, rest_top, s));
                let glyph_height = match event.duration.value {
                    NoteValue::Whole | NoteValue::Half => 0.6 * s,
                    NoteValue::Quarter => 2.2 * s,
                    _ => 1.8 * s,
                };
                let rest_y = rest_top + 1.7 * s;
                page.symbols.push(Symbol::new(
                    SymbolKind::Rest,
                    context.page,
                    context.system,
                    context.part,
                    context.measure_index,
                    voice.number,
                    event_index,
                    Bbox::new(
                        rest_x - 0.8 * s,
                        rest_y - glyph_height / 2.0,
                        1.6 * s,
                        glyph_height,
                    ),
                ));
                geom.rest_y = rest_y;
                for dot in 0..event.duration.dots {
                    let dot_x = rest_x + 1.0 * s + dot as f64 * 0.55 * s;
                    page.elements
                        .push(Element::ellipse(dot_x, rest_y, 0.16 * s, 0.16 * s, true));
                }
            } else {
                let notes = event.notes();
                let average_step = if notes.is_empty() {
                    4
                } else {
                    notes
                        .iter()
                        .map(|note| context.clef.staff_step(note.pitch.diatonic_index()))
                        .sum::<i32>()
                        / notes.len() as i32
                };
                let dir_up = if measure.voices.len() > 1 {
                    voice_index == 0
                } else {
                    average_step < 4
                };
                geom.dir_up = dir_up;
                let mut previous_y: Option<f64> = None;
                let mut previous_offset = 0.0f64;
                let mut accidental_index = 0usize;
                let ry = 0.46 * s;
                for (note_index, note) in notes.iter().enumerate() {
                    let staff_step = context.clef.staff_step(note.pitch.diatonic_index());
                    let y = context.staff_top + staff_height - staff_step as f64 * s * 0.5;
                    let offset = if let Some(previous) = previous_y {
                        if (previous - y).abs() < 2.0 * ry + 0.02 {
                            if previous_offset == 0.0 {
                                1.6 * s
                            } else {
                                0.0
                            }
                        } else {
                            0.0
                        }
                    } else {
                        0.0
                    };
                    previous_y = Some(y);
                    previous_offset = offset;
                    let x = anchor_x + offset;
                    let accidental = accidental_map
                        .get(&(voice.number, event_index, note_index))
                        .copied()
                        .flatten();
                    let rx = 0.68 * s;
                    page.elements
                        .push(Element::ellipse(x, y, rx, ry, true));
                    page.symbols.push(Symbol::new(
                        SymbolKind::Notehead,
                        context.page,
                        context.system,
                        context.part,
                        context.measure_index,
                        voice.number,
                        event_index,
                        Bbox::new(x - rx, y - ry, rx * 2.0, ry * 2.0),
                    ));
                    if let Some(accidental) = accidental {
                        let accidental_x = anchor_x - 1.8 * s
                            - accidental_index as f64 * 0.95 * s;
                        accidental_index += 1;
                        page.elements
                            .extend(glyphs::accidental(accidental, accidental_x, y, s));
                        page.symbols.push(Symbol::new(
                            SymbolKind::Accidental,
                            context.page,
                            context.system,
                            context.part,
                            context.measure_index,
                            voice.number,
                            event_index,
                            Bbox::new(
                                accidental_x,
                                y - 0.9 * s,
                                0.9 * s,
                                1.8 * s,
                            ),
                        ));
                    }
                    for dot in 0..event.duration.dots {
                        let chord_width = if event.is_chord() { 1.6 * s } else { 0.0 };
                        let dot_x = anchor_x
                            + chord_width
                            + 0.95 * s
                            + note_index as f64 * 0.55 * s
                            + dot as f64 * 0.55 * s;
                        let dot_y = if staff_step % 2 == 0 { y - 0.3 * s } else { y };
                        page.elements
                            .push(Element::ellipse(dot_x, dot_y, 0.16 * s, 0.16 * s, true));
                        page.symbols.push(Symbol::new(
                            SymbolKind::Dot,
                            context.page,
                            context.system,
                            context.part,
                            context.measure_index,
                            voice.number,
                            event_index,
                            Bbox::new(dot_x - 0.2 * s, dot_y - 0.2 * s, 0.4 * s, 0.4 * s),
                        ));
                    }
                    if staff_step > 8 {
                        let mut line_step = 10;
                        while line_step <= staff_step {
                            let ly =
                                context.staff_top + staff_height - line_step as f64 * s * 0.5;
                            page.elements.push(Element::line(
                                x - 0.9 * s,
                                ly,
                                x + 0.9 * s,
                                ly,
                                0.55,
                            ));
                            line_step += 2;
                        }
                    } else if staff_step < 0 {
                        let mut line_step = -2;
                        while line_step >= staff_step {
                            let ly =
                                context.staff_top + staff_height - line_step as f64 * s * 0.5;
                            page.elements.push(Element::line(
                                x - 0.9 * s,
                                ly,
                                x + 0.9 * s,
                                ly,
                                0.55,
                            ));
                            line_step -= 2;
                        }
                    }
                    geom.notes.push(NoteGeom { x, y });
                    anchors.insert(
                        (
                            context.part,
                            context.measure_index,
                            voice.number,
                            event_index,
                            note_index,
                        ),
                        NotePosGeom {
                            x,
                            y,
                            staff_top: context.staff_top,
                            staff_bottom: context.staff_bottom,
                            page: context.page,
                            system: context.system,
                        },
                    );
                }
                // Stem geometry; final tip is adjusted by beaming below.
                let xs: Vec<f64> = geom.notes.iter().map(|note| note.x).collect();
                let ys: Vec<f64> = geom.notes.iter().map(|note| note.y).collect();
                if !xs.is_empty() {
                    geom.stem_x = if dir_up {
                        xs.iter().copied().fold(f64::NEG_INFINITY, f64::max) + 0.65 * s
                    } else {
                        xs.iter().copied().fold(f64::INFINITY, f64::min) - 0.65 * s
                    };
                    geom.base_y = if dir_up {
                        ys.iter().copied().fold(f64::NEG_INFINITY, f64::max)
                    } else {
                        ys.iter().copied().fold(f64::INFINITY, f64::min)
                    };
                    geom.tip_y = geom.base_y + if dir_up { -3.3 * s } else { 3.3 * s };
                }
                // Lyrics.
                if let Some(note) = notes.first() {
                    for lyric in &note.lyrics {
                        let line = voice_index as f64 * 3.0
                            + (lyric.number.saturating_sub(1)) as f64;
                        let ly = lyric_base + line * line_height;
                        let mut label = lyric.text.clone();
                        if lyric.syllabic.needs_hyphen() {
                            label.push('-');
                        }
                        page.elements.push(Element::text(
                            anchor_x,
                            ly,
                            context.options.lyric_size,
                            &label,
                            Align::Start,
                        ));
                        page.symbols.push(Symbol::new(
                            SymbolKind::Lyric,
                            context.page,
                            context.system,
                            context.part,
                            context.measure_index,
                            voice.number,
                            event_index,
                            Bbox::new(
                                anchor_x,
                                ly,
                                text::width(&label, context.options.lyric_size),
                                context.options.lyric_size,
                            ),
                        ));
                        if lyric.extend {
                            page.elements.push(Element::line(
                                anchor_x + text::width(&label, context.options.lyric_size) + 2.0,
                                ly + context.options.lyric_size * 0.75,
                                anchor_x + text::width(&label, context.options.lyric_size) + 14.0,
                                ly + context.options.lyric_size * 0.75,
                                0.5,
                            ));
                        }
                    }
                }
                // Articulations.
                let mut above_index = 0.0;
                let mut below_index = 0.0;
                for note in notes {
                    for articulation in &note.articulations {
                        let y = if dir_up {
                            above_index += 1.0;
                            geom.notes
                                .iter()
                                .map(|note| note.y)
                                .fold(f64::INFINITY, f64::min)
                                - 1.25 * s
                                - (above_index - 1.0) * 0.7 * s
                        } else {
                            below_index += 1.0;
                            geom.notes
                                .iter()
                                .map(|note| note.y)
                                .fold(f64::NEG_INFINITY, f64::max)
                                + 1.25 * s
                                + (below_index - 1.0) * 0.7 * s
                        };
                        page.elements.extend(glyphs::articulation(
                            *articulation,
                            anchor_x,
                            y,
                            s,
                            dir_up,
                        ));
                        page.symbols.push(Symbol::new(
                            SymbolKind::Articulation,
                            context.page,
                            context.system,
                            context.part,
                            context.measure_index,
                            voice.number,
                            event_index,
                            Bbox::new(anchor_x - 0.55 * s, y - 0.5 * s, 1.1 * s, 1.0 * s),
                        ));
                    }
                }
                if let Some(dynamic) = event.dynamics {
                    let dy = lyric_base
                        + (max_lyric_line + 1.0) * line_height
                        + voice_index as f64 * context.options.text_size * 1.4;
                    let label = glyphs::dynamic_text(dynamic);
                    page.elements.push(Element::text(
                        anchor_x,
                        dy,
                        context.options.text_size,
                        label,
                        Align::Start,
                    ));
                    page.symbols.push(Symbol::new(
                        SymbolKind::Dynamic,
                        context.page,
                        context.system,
                        context.part,
                        context.measure_index,
                        voice.number,
                        event_index,
                        Bbox::new(
                            anchor_x,
                            dy,
                            text::width(label, context.options.text_size),
                            context.options.text_size,
                        ),
                    ));
                }
                // Hit region for the event.
                page.hits.push(HitRegion {
                    part: context.part,
                    measure: context.measure_index,
                    voice: voice.number,
                    event: event_index,
                    x: anchor_x - 1.1 * s,
                    y: context.staff_top,
                    w: 2.6 * s,
                    h: staff_height,
                });
            }
            tick += event.duration.ticks(divisions);
            event_geoms.push(geom);
        }

        // Beams and stems.
        let groups = beaming::beam_groups(measure, voice.number);
        let mut tip_overrides: HashMap<usize, f64> = HashMap::new();
        for group in &groups {
            if group.end_event >= event_geoms.len() {
                continue;
            }
            let first = &event_geoms[group.start_event];
            let last = &event_geoms[group.end_event];
            if first.notes.is_empty() || last.notes.is_empty() {
                continue;
            }
            let dir_up = if measure.voices.len() > 1 {
                first.voice_index == 0
            } else {
                first.dir_up
            };
            let mut tips = Vec::new();
            for index in group.start_event..=group.end_event {
                let geom = &event_geoms[index];
                if geom.notes.is_empty() {
                    continue;
                }
                let base = if dir_up {
                    geom.notes
                        .iter()
                        .map(|note| note.y)
                        .fold(f64::NEG_INFINITY, f64::max)
                } else {
                    geom.notes
                        .iter()
                        .map(|note| note.y)
                        .fold(f64::INFINITY, f64::min)
                };
                tips.push((
                    index,
                    base + if dir_up { -3.3 * s } else { 3.3 * s },
                ));
            }
            if tips.len() < 2 {
                continue;
            }
            let first_tip = tips[0].1;
            let last_tip = tips[tips.len() - 1].1;
            let first_x = event_geoms[group.start_event].x;
            let last_x = event_geoms[group.end_event].x;
            let mut level_map: HashMap<usize, f64> = HashMap::new();
            for (index, initial_tip) in &tips {
                let geom = &mut event_geoms[*index];
                geom.dir_up = dir_up;
                geom.beamed = true;
                let fraction = if (last_x - first_x).abs() < 0.001 {
                    0.0
                } else {
                    ((geom.x - first_x) / (last_x - first_x)).clamp(0.0, 1.0)
                };
                let tip = first_tip + (last_tip - first_tip) * fraction;
                geom.tip_y = tip;
                level_map.insert(*index, tip);
                let _ = initial_tip;
            }
            let max_level = group
                .start_event
                .max(0)
                .min(event_geoms.len().saturating_sub(1));
            let _ = max_level;
            let levels = (group.start_event..=group.end_event)
                .map(|index| beaming::beam_levels(event_geoms[index].duration))
                .max()
                .unwrap_or(1)
                .max(1);
            for level in 1..=levels {
                let mut run_start: Option<usize> = None;
                for index in group.start_event..=group.end_event {
                    let has_level = beaming::beam_levels(event_geoms[index].duration) >= level;
                    if has_level {
                        if run_start.is_none() {
                            run_start = Some(index);
                        }
                    } else if let Some(start) = run_start.take() {
                        if start < index - 1 {
                            draw_beam_segment(
                                page,
                                s,
                                dir_up,
                                &event_geoms,
                                &level_map,
                                start,
                                index - 1,
                                level,
                            );
                        }
                    }
                }
                if let Some(start) = run_start {
                    if start < group.end_event {
                        draw_beam_segment(
                            page,
                            s,
                            dir_up,
                            &event_geoms,
                            &level_map,
                            start,
                            group.end_event,
                            level,
                        );
                    }
                }
            }
            for (index, _) in &tips {
                tip_overrides.insert(*index, level_map[index]);
            }
        }

        for geom in &mut event_geoms {
            if geom.is_rest || geom.notes.is_empty() {
                continue;
            }
            if let Some(tip) = tip_overrides.get(&geom.event_index) {
                geom.tip_y = *tip;
            }
            let stem_x = if geom.dir_up {
                geom.notes
                    .iter()
                    .map(|note| note.x)
                    .fold(f64::NEG_INFINITY, f64::max)
                    + 0.65 * s
            } else {
                geom.notes
                    .iter()
                    .map(|note| note.x)
                    .fold(f64::INFINITY, f64::min)
                    - 0.65 * s
            };
            geom.stem_x = stem_x;
            if geom.has_stem {
                page.elements.push(Element::line(
                    stem_x,
                    geom.base_y,
                    stem_x,
                    geom.tip_y,
                    0.7,
                ));
                if !geom.beamed {
                    let flags = beaming::beam_levels(geom.duration);
                    if flags > 0 {
                        page.elements.extend(glyphs::flag(
                            stem_x,
                            geom.tip_y,
                            s,
                            geom.dir_up,
                            flags,
                        ));
                    }
                }
            }
        }
    }
}

fn draw_beam_segment(
    page: &mut PageLayout,
    s: f64,
    dir_up: bool,
    events: &[EventGeom],
    tips: &HashMap<usize, f64>,
    start: usize,
    end: usize,
    level: u8,
) {
    if start >= end || end >= events.len() {
        return;
    }
    let x1 = events[start].stem_x;
    let x2 = events[end].stem_x;
    let y1 = tips.get(&start).copied().unwrap_or(events[start].tip_y)
        - if dir_up {
            (level as f64 - 1.0) * 0.55 * s
        } else {
            -(level as f64 - 1.0) * 0.55 * s
        };
    let y2 = tips.get(&end).copied().unwrap_or(events[end].tip_y)
        - if dir_up {
            (level as f64 - 1.0) * 0.55 * s
        } else {
            -(level as f64 - 1.0) * 0.55 * s
        };
    page.elements
        .push(glyphs::beam(x1, y1, x2, y2, 0.5 * s));
}

fn accidental_map(measure: &Measure) -> HashMap<(u8, usize, usize), Option<Accidental>> {
    let mut map = HashMap::new();
    let key = measure.attributes.key;
    let divisions = measure.attributes.divisions;
    let mut entries: Vec<(u64, u8, usize)> = Vec::new();
    for voice in &measure.voices {
        let mut tick = 0u64;
        for (event_index, event) in voice.events.iter().enumerate() {
            entries.push((tick, voice.number, event_index));
            tick += event.duration.ticks(divisions);
        }
    }
    entries.sort_unstable();
    let mut state: HashMap<(cadence_core::Step, i8), i8> = HashMap::new();
    for (_, voice_number, event_index) in entries {
        let Some(voice) = measure.voice(voice_number) else {
            continue;
        };
        let Some(event) = voice.events.get(event_index) else {
            continue;
        };
        for (note_index, note) in event.notes().iter().enumerate() {
            let current = state
                .get(&(note.pitch.step, note.pitch.octave))
                .copied()
                .unwrap_or_else(|| key.alter_for(note.pitch.step));
            let display = if let Some(explicit) = note.accidental {
                state.insert((note.pitch.step, note.pitch.octave), note.pitch.alter);
                Some(explicit)
            } else if note.pitch.alter != current {
                state.insert((note.pitch.step, note.pitch.octave), note.pitch.alter);
                Accidental::from_alter(note.pitch.alter)
            } else {
                None
            };
            map.insert((voice_number, event_index, note_index), display);
        }
    }
    map
}

fn draw_slurs_and_ties(
    layout: &mut Layout,
    score: &Score,
    anchors: &HashMap<(usize, usize, u8, usize, usize), NotePosGeom>,
    options: &Options,
) {
    let s = options.staff_space;
    for (part_index, part) in score.parts.iter().enumerate() {
        for span in resolve_slurs(part).spans {
            let start = anchors.get(&(
                part_index,
                span.start.measure,
                span.start.voice,
                span.start.event,
                span.start.note,
            ));
            let end = anchors.get(&(
                part_index,
                span.end.measure,
                span.end.voice,
                span.end.event,
                span.end.note,
            ));
            if let (Some(start), Some(end)) = (start, end) {
                if start.page == end.page && start.system == end.system {
                    let above = (start.y + end.y) / 2.0
                        < (start.staff_top + start.staff_bottom) / 2.0;
                    draw_curve(
                        &mut layout.pages[start.page],
                        start,
                        end,
                        above,
                        s * 1.2,
                    );
                }
            }
        }
        for span in resolve_ties(part).spans {
            let start = anchors.get(&(
                part_index,
                span.start.measure,
                span.start.voice,
                span.start.event,
                span.start.note,
            ));
            let end = anchors.get(&(
                part_index,
                span.end.measure,
                span.end.voice,
                span.end.event,
                span.end.note,
            ));
            if let (Some(start), Some(end)) = (start, end) {
                if start.page == end.page && start.system == end.system {
                    let above = start.y < end.staff_top + (end.staff_bottom - end.staff_top) / 2.0;
                    draw_curve(
                        &mut layout.pages[start.page],
                        start,
                        end,
                        above,
                        s * 0.7,
                    );
                }
            }
        }
    }
}

fn draw_curve(
    page: &mut PageLayout,
    start: &NotePosGeom,
    end: &NotePosGeom,
    above: bool,
    offset: f64,
) {
    let (y1, y2) = if above {
        (start.y - offset, end.y - offset)
    } else {
        (start.y + offset, end.y + offset)
    };
    let mid = (y1 + y2) / 2.0;
    let control_y = if above {
        mid - offset * 1.6
    } else {
        mid + offset * 1.6
    };
    let mid_x = (start.x + end.x) / 2.0;
    page.elements.push(Element::Path {
        segments: vec![
            PathSeg::Move(Point::new(start.x, y1)),
            PathSeg::Cubic {
                c1: Point::new(start.x + (mid_x - start.x) * 0.4, control_y),
                c2: Point::new(end.x - (end.x - mid_x) * 0.4, control_y),
                to: Point::new(end.x, y2),
            },
        ],
        stroke: 0.8,
        fill: false,
    });
}

fn truncate(text_value: &str, max: usize) -> String {
    let mut out = String::new();
    for (index, c) in text_value.chars().enumerate() {
        if index >= max {
            break;
        }
        out.push(c);
    }
    out
}

