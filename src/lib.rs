//! Tailwind CSS class extraction and processing library
//!
//! This library provides a trait and utilities for processing Tailwind CSS classes
//! in server-side rendering contexts. It's designed to work with the V8DirectRenderer
//! and other systems that need to extract and process Tailwind classes from JavaScript/TypeScript.

pub mod processor;

// Re-export the main trait at the crate root for convenience
pub use processor::TailwindClassProcessor;

// Re-export TailwindBuilder for consumers who need it
pub use tailwind_rs::TailwindBuilder;