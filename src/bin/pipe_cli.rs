//! Tailwind CSS extractor CLI with transform and generate modes
//!
//! This CLI provides two distinct modes:
//! 1. transform - Read JS from stdin, transform it using AST transformer, output to stdout, write metadata to file
//! 2. generate - Read metadata JSON from stdin, generate CSS using tailwind-rs, output to stdout

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Read, Write};
use std::path::PathBuf;
use tailwind_extractor::{transform_source, TransformConfig};
use tailwind_rs::TailwindBuilder;

#[derive(Parser)]
#[command(name = "tailwind-extractor-cli")]
#[command(about = "Tailwind CSS extractor and transformer CLI", long_about = None)]
#[command(version = env!("CARGO_PKG_VERSION"))]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Transform JavaScript/TypeScript, extracting Tailwind classes
    Transform {
        /// Path to write metadata JSON file
        #[arg(value_name = "METADATA_PATH")]
        metadata_output: PathBuf,
        
        /// Obfuscate Tailwind classes for production
        #[arg(long)]
        obfuscate: bool,
        
        /// Source file name (optional, for metadata)
        #[arg(long)]
        source_file: Option<String>,
    },
    
    /// Generate CSS from metadata JSON
    Generate {
        /// Disable preflight CSS
        #[arg(long = "no-preflight")]
        no_preflight: bool,
        
        /// Minify output CSS
        #[arg(long)]
        minify: bool,

        /// Obfuscate Tailwind classes for production
        #[arg(long)]
        obfuscate: bool,
    },
}

/// Metadata format for class extraction
#[derive(Debug, Serialize, Deserialize)]
struct Metadata {
    /// Deduplicated list of all classes found
    classes: Vec<String>,
    /// Original source file name (if provided)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "sourceFile")]
    source_file: Option<String>,
    /// ISO timestamp of processing
    #[serde(rename = "processedAt")]
    processed_at: String,
    /// Crate version
    version: String,
    /// Statistics about extraction
    stats: Stats,
}

#[derive(Debug, Serialize, Deserialize)]
struct Stats {
    /// Count of classes before deduplication
    #[serde(rename = "originalCount")]
    original_count: usize,
    /// Count of unique classes
    #[serde(rename = "uniqueCount")]
    unique_count: usize,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Transform { metadata_output, obfuscate, source_file } => {
            handle_transform_mode(metadata_output, obfuscate, source_file)
        }
        Commands::Generate { no_preflight, obfuscate, minify } => {
            handle_generate_mode(no_preflight, obfuscate, minify)
        }
    }
}

/// Transform mode: Read JS from stdin, transform it, output transformed JS and metadata
fn handle_transform_mode(
    metadata_output: PathBuf,
    obfuscate: bool,
    source_file: Option<String>,
) -> Result<()> {
    // Read JavaScript from stdin
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .context("Failed to read JavaScript from stdin")?;
    
    // Configure transformation
    let config = TransformConfig {
        obfuscate,
        source_maps: false,
    };
    
    // Transform the source code using AST transformer
    let (transformed_js, transform_metadata) = transform_source(&input, config)
        .context("Failed to transform JavaScript")?;
    
    // Write transformed JavaScript to stdout
    io::stdout()
        .write_all(transformed_js.as_bytes())
        .context("Failed to write transformed JavaScript to stdout")?;
    
    // Prepare metadata
    let unique_count = transform_metadata.classes.len();
    let metadata = Metadata {
        classes: transform_metadata.classes,
        source_file,
        processed_at: chrono::Utc::now().to_rfc3339(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        stats: Stats {
            original_count: transform_metadata.original_count,
            unique_count,
        },
    };
    
    // Write metadata to file
    let metadata_json = serde_json::to_string_pretty(&metadata)
        .context("Failed to serialize metadata")?;
    
    fs::write(&metadata_output, metadata_json)
        .with_context(|| format!("Failed to write metadata to {:?}", metadata_output))?;
    
    Ok(())
}

/// Generate mode: Read metadata JSON from stdin, generate CSS and output to stdout
fn handle_generate_mode(no_preflight: bool, obfuscate: bool, minify: bool) -> Result<()> {
    // Read metadata JSON from stdin
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .context("Failed to read metadata JSON from stdin")?;
    
    // If input is empty, output empty CSS
    if input.trim().is_empty() {
        return Ok(());
    }
    
    // Parse metadata
    let metadata: Metadata = serde_json::from_str(&input)
        .context("Failed to parse metadata JSON")?;
    
    // If no classes, output empty CSS
    if metadata.classes.is_empty() {
        return Ok(());
    }
    
    // Generate CSS using tailwind-rs
    let css = generate_tailwind_css(metadata.classes, no_preflight, minify, obfuscate)?;
    
    // Write CSS to stdout
    io::stdout()
        .write_all(css.as_bytes())
        .context("Failed to write CSS to stdout")?;
    
    Ok(())
}

/// Generate Tailwind CSS for the given classes
fn generate_tailwind_css(
    classes: Vec<String>,
    no_preflight: bool,
    _minify: bool, // Note: minify isn't directly supported by tailwind-rs yet
    obfuscate: bool, // Note: minify isn't directly supported by tailwind-rs yet
) -> Result<String> {
    let mut builder = TailwindBuilder::default();
    
    // Configure preflight
    builder.preflight.disable = no_preflight;
    
    // Process each class through the builder
    for class in &classes {
        // Try to trace the class - silently ignore failures for unknown classes
        let _ = builder.trace(class, obfuscate);
    }
    
    // Generate the CSS bundle
    match builder.bundle() {
        Ok(css_string) => {
            // TODO: If minify is true, we could post-process the CSS here
            // For now, return as-is since tailwind-rs doesn't have built-in minification
            Ok(css_string)
        }
        Err(e) => {
            // Log warning to stderr and return empty CSS
            eprintln!("Warning: CSS generation failed: {}", e);
            Ok(String::new())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_metadata_serialization() {
        let metadata = Metadata {
            classes: vec!["bg-blue-500".to_string(), "text-white".to_string()],
            source_file: Some("test.js".to_string()),
            processed_at: "2024-01-01T00:00:00Z".to_string(),
            version: "0.1.0".to_string(),
            stats: Stats {
                original_count: 3,
                unique_count: 2,
            },
        };
        
        let json = serde_json::to_string(&metadata).unwrap();
        let parsed: Metadata = serde_json::from_str(&json).unwrap();
        
        assert_eq!(parsed.classes.len(), 2);
        assert_eq!(parsed.stats.original_count, 3);
        assert_eq!(parsed.stats.unique_count, 2);
    }
    
    #[test]
    fn test_generate_css_from_metadata() {
        let metadata = Metadata {
            classes: vec![
                "bg-blue-500".to_string(),
                "text-white".to_string(),
                "p-4".to_string(),
            ],
            source_file: None,
            processed_at: chrono::Utc::now().to_rfc3339(),
            version: "0.1.0".to_string(),
            stats: Stats {
                original_count: 3,
                unique_count: 3,
            },
        };
        
        let css = generate_tailwind_css(metadata.classes, true, false).unwrap();
        
        // Should contain CSS for the classes
        assert!(!css.is_empty());
        // With no-preflight, shouldn't contain reset styles
        assert!(!css.contains("html"));
    }
}
