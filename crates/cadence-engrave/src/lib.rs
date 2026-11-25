//! Cadence engraver: deterministic spacing, beaming, collisions and pages.

pub mod beaming;
pub mod collision;
pub mod geom;
pub mod glyphs;
pub mod layout;
pub mod options;
pub mod pagination;
pub mod spacing;
pub mod text;

pub use collision::{find_collisions, is_collision_free, Collision};
pub use geom::{Align, Bbox, Element, HitRegion, PathSeg, Point, Symbol, SymbolKind};
pub use layout::{engrave, Layout, PageLayout, SystemLayout};
pub use options::{Margins, Options};
