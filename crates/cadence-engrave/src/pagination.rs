//! System and page breaking.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MeasurePlan {
    pub index: usize,
    pub width: f64,
    pub system_break: bool,
    pub page_break: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemPlan {
    pub measures: Vec<usize>,
    /// The first measure of this system asked for a page break.
    pub new_page: bool,
}

pub fn plan_systems(
    plans: &[MeasurePlan],
    prefix_width: f64,
    content_width: f64,
    max_measures: usize,
) -> Vec<SystemPlan> {
    let max_measures = max_measures.max(1);
    let mut systems: Vec<SystemPlan> = Vec::new();
    let mut current: Vec<usize> = Vec::new();
    let mut current_width = 0.0;
    let mut new_page = false;
    for plan in plans {
        let first = current.is_empty();
        let extra = if first { prefix_width } else { 0.0 };
        let must_break = !current.is_empty()
            && (plan.system_break
                || plan.page_break
                || current.len() >= max_measures
                || current_width + extra + plan.width > content_width + 0.01);
        if must_break {
            systems.push(SystemPlan {
                measures: current,
                new_page,
            });
            current = Vec::new();
            current_width = 0.0;
            new_page = plan.page_break;
        }
        if current.is_empty() {
            current_width += prefix_width;
            new_page = new_page || plan.page_break;
        }
        current.push(plan.index);
        current_width += plan.width;
    }
    if !current.is_empty() {
        systems.push(SystemPlan {
            measures: current,
            new_page,
        });
    }
    systems
}

/// Assign systems to pages using their measured heights.
pub fn paginate(
    heights: &[f64],
    system_gap: f64,
    first_page_capacity: f64,
    next_page_capacity: f64,
    force_page: &[bool],
) -> Vec<Vec<usize>> {
    let mut pages: Vec<Vec<usize>> = Vec::new();
    let mut current: Vec<usize> = Vec::new();
    let mut used = 0.0;
    let mut capacity = first_page_capacity.max(1.0);
    for (index, height) in heights.iter().enumerate() {
        let forced = force_page.get(index).copied().unwrap_or(false);
        let gap = if current.is_empty() { 0.0 } else { system_gap };
        if !current.is_empty() && (forced || used + gap + height > capacity + 0.01) {
            pages.push(current);
            current = Vec::new();
            used = 0.0;
            capacity = next_page_capacity.max(1.0);
        }
        if current.is_empty() {
            used = *height;
        } else {
            used += system_gap + *height;
        }
        current.push(index);
    }
    if !current.is_empty() {
        pages.push(current);
    }
    pages
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plans(widths: &[f64]) -> Vec<MeasurePlan> {
        widths
            .iter()
            .enumerate()
            .map(|(index, width)| MeasurePlan {
                index,
                width: *width,
                system_break: false,
                page_break: false,
            })
            .collect()
    }

    #[test]
    fn systems_break_at_content_width() {
        let systems = plan_systems(&plans(&[200.0; 5]), 40.0, 480.0, 12);
        assert_eq!(systems.len(), 3);
        assert_eq!(systems[0].measures, vec![0, 1]);
        assert_eq!(systems[1].measures, vec![2, 3]);
        assert_eq!(systems[2].measures, vec![4]);
    }

    #[test]
    fn explicit_breaks_are_respected() {
        let mut plans = plans(&[100.0; 4]);
        plans[2].system_break = true;
        plans[3].page_break = true;
        let systems = plan_systems(&plans, 20.0, 1000.0, 12);
        assert_eq!(systems.len(), 3);
        assert_eq!(systems[1].measures, vec![2]);
        assert!(systems[2].new_page);
    }

    #[test]
    fn pages_paginate_by_height() {
        let pages = paginate(&[100.0, 100.0, 100.0, 100.0], 10.0, 250.0, 250.0, &[false; 4]);
        assert_eq!(pages.len(), 2);
        assert_eq!(pages[0], vec![0, 1]);
        assert_eq!(pages[1], vec![2, 3]);
    }

    #[test]
    fn force_page_starts_a_new_page() {
        let pages = paginate(&[50.0, 50.0, 50.0], 10.0, 500.0, 500.0, &[false, true, false]);
        assert_eq!(pages.len(), 2);
        assert_eq!(pages[0], vec![0]);
        assert_eq!(pages[1], vec![1, 2]);
    }
}
