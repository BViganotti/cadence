//! Engraving options. All values are in PostScript points and affect only
//! layout, never parsing or musical semantics.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Margins {
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
}

impl Default for Margins {
    fn default() -> Self {
        Margins {
            top: 54.0,
            right: 52.0,
            bottom: 52.0,
            left: 64.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Options {
    pub page_width: f64,
    pub page_height: f64,
    pub margins: Margins,
    /// Distance between adjacent staff lines.
    pub staff_space: f64,
    /// Vertical gap between systems on the same page.
    pub system_gap: f64,
    /// Gap between the header and the first system.
    pub header_gap: f64,
    pub min_event_width: f64,
    pub duration_scale: f64,
    pub barline_width: f64,
    pub lyric_size: f64,
    pub text_size: f64,
    pub small_size: f64,
    pub title_size: f64,
    pub subtitle_size: f64,
    pub max_measures_per_system: usize,
    pub show_measure_numbers: bool,
    pub show_part_names: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            page_width: 595.276,
            page_height: 841.89,
            margins: Margins::default(),
            staff_space: 8.0,
            system_gap: 30.0,
            header_gap: 26.0,
            min_event_width: 9.0,
            duration_scale: 20.0,
            barline_width: 10.0,
            lyric_size: 8.5,
            text_size: 10.0,
            small_size: 7.0,
            title_size: 22.0,
            subtitle_size: 12.0,
            max_measures_per_system: 12,
            show_measure_numbers: true,
            show_part_names: true,
        }
    }
}

impl Options {
    pub fn staff_height(&self) -> f64 {
        self.staff_space * 4.0
    }

    pub fn content_width(&self) -> f64 {
        (self.page_width - self.margins.left - self.margins.right).max(80.0)
    }

    pub fn content_height(&self) -> f64 {
        (self.page_height - self.margins.top - self.margins.bottom).max(80.0)
    }

    pub fn validate(&self) -> cadence_core::Result<()> {
        let values = [
            self.page_width,
            self.page_height,
            self.margins.top,
            self.margins.right,
            self.margins.bottom,
            self.margins.left,
            self.staff_space,
            self.system_gap,
            self.header_gap,
            self.min_event_width,
            self.duration_scale,
            self.barline_width,
            self.lyric_size,
            self.text_size,
            self.small_size,
            self.title_size,
            self.subtitle_size,
        ];
        if values.iter().any(|value| !value.is_finite()) {
            return Err(cadence_core::Error::msg(
                "engraving options contain a non-finite value",
            ));
        }
        if self.page_width < 200.0
            || self.page_height < 200.0
            || self.staff_space < 3.0
            || self.staff_space > 30.0
        {
            return Err(cadence_core::Error::out_of_range("engraving page size"));
        }
        if self.content_width() < 120.0 || self.content_height() < 120.0 {
            return Err(cadence_core::Error::out_of_range("engraving margins"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_valid_a4() {
        let options = Options::default();
        options.validate().unwrap();
        assert!((options.content_width() - 479.276).abs() < 0.01);
        assert_eq!(options.staff_height(), 32.0);
    }

    #[test]
    fn invalid_options_are_rejected() {
        let mut options = Options::default();
        options.staff_space = 0.0;
        assert!(options.validate().is_err());
        options = Options::default();
        options.page_width = f64::NAN;
        assert!(options.validate().is_err());
        options = Options::default();
        options.margins.left = 500.0;
        assert!(options.validate().is_err());
    }
}
