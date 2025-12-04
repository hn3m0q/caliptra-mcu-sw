//! `defmt` global logger for Tock/RISC-V
//!
//! This crate provides a global logger implementation for defmt that outputs
//! encoded log frames to the Tock console. The frames are not human-readable
//! but can be decoded by defmt tooling.
//!
//! # Critical section implementation
//!
//! This crate uses `critical-section` to ensure only one thread is writing
//! to the buffer at a time. The critical-section implementation must be
//! provided by the application.
//!
//! # Logger Selection
//!
//! Two logger implementations are available:
//! - `logger_a` (default): Outputs with [DEFMT-A: prefix
//! - `logger_b`: Outputs with [DEFMT-B: prefix
//!
//! Enable via features in your Cargo.toml:
//! ```toml
//! defmt-logger = { path = "../defmt-logger", features = ["logger_b"] }
//! ```
//!
//! # Usage
//!
//! Simply add `defmt-logger` as a dependency and import it:
//!
//! ```rust,ignore
//! use defmt_logger::{debug, info, warn, error, trace};
//!
//! error!("This is an error message");
//! ```

#![no_std]
#![cfg_attr(target_arch = "riscv32", feature(impl_trait_in_assoc_type))]

// Re-export defmt for users
pub use defmt;

// Re-export defmt logging macros directly
pub use defmt::{debug, error, info, trace, warn};

// Conditionally include logger implementation based on features
#[cfg(feature = "logger_b")]
mod logger_b;

#[cfg(not(feature = "logger_b"))]
mod logger_a;

// Embassy task module (only available on riscv32)
#[cfg(target_arch = "riscv32")]
pub mod task;

/// Timestamp function required by defmt
///
/// Returns a simple counter-based timestamp. In a real implementation,
/// this would return a hardware timer value.
defmt::timestamp!("{=u64}", {
    // For this minimal implementation, we just return 0
    // In a real system, this would return a monotonic timestamp
    0
});
