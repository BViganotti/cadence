//! Development example: engrave the built-in demo score and write SVG/PDF.
use cadence_core::Score;
use cadence_engrave::{engrave, Options};
use cadence_export::{render_midi, render_pdf, render_svg};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let score = Score::demo();
    let options = Options::default();
    let layout = engrave(&score, &options)?;
    let svg = render_svg(&layout);
    std::fs::write("examples/demo.svg", svg)?;
    let pdf = render_pdf(&layout)?;
    std::fs::write("examples/demo.pdf", pdf)?;
    let midi = render_midi(&score)?;
    std::fs::write("examples/demo.mid", midi)?;
    println!(
        "wrote examples/demo.svg, examples/demo.pdf and examples/demo.mid ({} page(s))",
        layout.page_count()
    );
    Ok(())
}
