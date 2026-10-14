//! Independent engine for local PDF optimization.
//!
//! Supports lossless stream optimization and optional lossy image compression.

mod image_usage;
mod images;
mod optimizer;
mod streams;
mod validation;

pub use optimizer::{optimize, OptimizeError, OptimizeOptions, OptimizeReport, Outcome};

/// Requested optimization policy.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CompressionLevel {
    /// Structural optimizations without intentional visual loss.
    Light,
    /// Structural optimizations and eligible image optimization.
    #[default]
    Normal,
    /// More aggressive optimization of eligible images.
    Ultra,
    /// Maximum reduction of eligible images; visual quality may degrade substantially.
    Maximum,
}
