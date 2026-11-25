//! Cadence launcher.
//!
//! Starts the local Studio on a loopback port and opens the default browser.
//! There is no terminal interaction: all editing happens in the browser.

fn main() {
    if let Err(err) = cadence_studio::run() {
        eprintln!("cadence: {err}");
        std::process::exit(1);
    }
}
