//! Collision detection over engraving symbols.

use crate::geom::{Symbol, SymbolKind};
use crate::layout::Layout;

#[derive(Debug, Clone, PartialEq)]
pub struct Collision {
    pub page: usize,
    pub system: usize,
    pub staff: usize,
    pub first: SymbolKind,
    pub second: SymbolKind,
    pub measure_a: usize,
    pub measure_b: usize,
}

impl Collision {
    pub fn describe(&self) -> String {
        format!(
            "page {} system {} staff {}: {:?} (measure {}) overlaps {:?} (measure {})",
            self.page + 1,
            self.system + 1,
            self.staff + 1,
            self.first,
            self.measure_a,
            self.second,
            self.measure_b
        )
    }
}

/// Find overlapping ink symbols. Barlines and ledger lines are connectors and
/// are excluded; everything a reader must identify separately is checked.
pub fn find_collisions(layout: &Layout) -> Vec<Collision> {
    let symbols: Vec<&Symbol> = layout.symbols().filter(|symbol| symbol.kind.is_ink()).collect();
    let mut collisions = Vec::new();
    for (index, a) in symbols.iter().enumerate() {
        for b in symbols.iter().skip(index + 1) {
            if a.page != b.page {
                continue;
            }
            if a.kind == SymbolKind::Mark || b.kind == SymbolKind::Mark {
                continue;
            }
            if a.bbox.intersects(b.bbox) {
                collisions.push(Collision {
                    page: a.page,
                    system: a.system,
                    staff: a.staff,
                    first: a.kind,
                    second: b.kind,
                    measure_a: a.measure,
                    measure_b: b.measure,
                });
            }
        }
    }
    collisions
}

/// True when no ink symbols overlap.
pub fn is_collision_free(layout: &Layout) -> bool {
    find_collisions(layout).is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::options::Options;

    #[test]
    fn demo_layout_has_no_collisions() {
        let layout = crate::engrave(&cadence_core::Score::demo(), &Options::default()).unwrap();
        let collisions = find_collisions(&layout);
        assert!(
            collisions.is_empty(),
            "{:?}",
            collisions.iter().map(Collision::describe).collect::<Vec<_>>()
        );
    }
}
