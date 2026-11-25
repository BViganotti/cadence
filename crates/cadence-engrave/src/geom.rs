//! Geometry primitives shared by engraver and exporters.

pub use crate::text::Align;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub const fn new(x: f64, y: f64) -> Point {
        Point { x, y }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bbox {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl Bbox {
    pub const fn new(x: f64, y: f64, w: f64, h: f64) -> Bbox {
        Bbox { x, y, w, h }
    }

    pub fn right(self) -> f64 {
        self.x + self.w
    }

    pub fn bottom(self) -> f64 {
        self.y + self.h
    }

    pub fn intersects(self, other: Bbox) -> bool {
        let epsilon = 0.01;
        self.x + epsilon < other.right()
            && other.x + epsilon < self.right()
            && self.y + epsilon < other.bottom()
            && other.y + epsilon < self.bottom()
    }

    pub fn expand(self, amount: f64) -> Bbox {
        Bbox::new(
            self.x - amount,
            self.y - amount,
            self.w + amount * 2.0,
            self.h + amount * 2.0,
        )
    }

    pub fn union(self, other: Bbox) -> Bbox {
        let x = self.x.min(other.x);
        let y = self.y.min(other.y);
        let right = self.right().max(other.right());
        let bottom = self.bottom().max(other.bottom());
        Bbox::new(x, y, right - x, bottom - y)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PathSeg {
    Move(Point),
    Line(Point),
    Cubic { c1: Point, c2: Point, to: Point },
    Close,
}

/// A deterministic drawing primitive. Coordinates are in points with y down.
#[derive(Debug, Clone, PartialEq)]
pub enum Element {
    Line {
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        width: f64,
    },
    Rect {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        filled: bool,
    },
    Ellipse {
        cx: f64,
        cy: f64,
        rx: f64,
        ry: f64,
        filled: bool,
    },
    Path {
        segments: Vec<PathSeg>,
        stroke: f64,
        fill: bool,
    },
    Text {
        x: f64,
        y: f64,
        size: f64,
        text: String,
        align: Align,
    },
}

impl Element {
    pub fn line(x1: f64, y1: f64, x2: f64, y2: f64, width: f64) -> Element {
        Element::Line {
            x1,
            y1,
            x2,
            y2,
            width,
        }
    }

    pub fn rect(x: f64, y: f64, w: f64, h: f64, filled: bool) -> Element {
        Element::Rect {
            x,
            y,
            w,
            h,
            filled,
        }
    }

    pub fn ellipse(cx: f64, cy: f64, rx: f64, ry: f64, filled: bool) -> Element {
        Element::Ellipse {
            cx,
            cy,
            rx,
            ry,
            filled,
        }
    }

    pub fn text(x: f64, y: f64, size: f64, text: impl Into<String>, align: Align) -> Element {
        Element::Text {
            x,
            y,
            size,
            text: text.into(),
            align,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SymbolKind {
    Notehead,
    Accidental,
    Dot,
    Rest,
    Lyric,
    Dynamic,
    Articulation,
    Clef,
    KeySignature,
    TimeSignature,
    Barline,
    MeasureNumber,
    PartName,
    Rehearsal,
    Tempo,
    Title,
    Subtitle,
    Composer,
    Footer,
    Mark,
}

impl SymbolKind {
    /// Connectors are allowed to cross other ink; collision checks consider
    /// only glyphs that must remain legible on their own.
    pub fn is_ink(self) -> bool {
        !matches!(self, SymbolKind::Barline | SymbolKind::Mark)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Symbol {
    pub kind: SymbolKind,
    pub page: usize,
    pub system: usize,
    pub staff: usize,
    pub measure: usize,
    pub voice: u8,
    pub event: usize,
    pub bbox: Bbox,
}

impl Symbol {
    pub fn new(
        kind: SymbolKind,
        page: usize,
        system: usize,
        staff: usize,
        measure: usize,
        voice: u8,
        event: usize,
        bbox: Bbox,
    ) -> Symbol {
        Symbol {
            kind,
            page,
            system,
            staff,
            measure,
            voice,
            event,
            bbox,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HitRegion {
    pub part: usize,
    pub measure: usize,
    pub voice: u8,
    pub event: usize,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}
