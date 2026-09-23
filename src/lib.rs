//! mango's build pipeline.
//!
//! The crate is a binary first; this library is an **internal seam** shared by
//! `src/main.rs` and the in-process tests. It may change freely, is not
//! documented in `README.md`, and carries no semver promise.
//!
//! A build is two phases: [`plan`] loads, checks and renders everything while
//! writing nothing, and [`commit`] takes the resulting [`BuildPlan`] and is
//! the only thing that empties and writes the output folder.

mod build;
mod config;
mod content;
mod error;
mod render;

pub use build::clean::clean;
pub use build::pipeline::{BuildOptions, BuildPlan, PlannedOutput, commit, plan};
pub use error::MangoError;
