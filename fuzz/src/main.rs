//! Deterministic malformed-input and determinism harness.
//!
//! This is a development tool, not part of the product. It mutates MusicXML
//! fixtures with a fixed seed and asserts that parsing never panics.

fn main() {
    let iterations = std::env::args()
        .nth(1)
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(1000);
    let report = cadence_fuzz::run(iterations);
    println!("cadence-fuzz: {:?}", report);
    if report.panics > 0 {
        std::process::exit(1);
    }
}
