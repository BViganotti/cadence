//! Cadence Studio: a loopback HTTP/SSE server with embedded UI assets.

pub mod assets;
pub mod document;
pub mod json;
pub mod server;

pub use server::run;