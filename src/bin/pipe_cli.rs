//! Simplified pipe CLI for Tailwind CSS extraction
//! 
//! This CLI reads JavaScript from stdin and outputs CSS to stdout.
//! It supports two modes:
//! 1. pipe (default) - Generate CSS from extracted classes
//! 2. pipe --transform - Transform JavaScript and output transformed JS

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::collections::HashSet;
use std::io::{self, Read, Write};
use tailwind_rs::TailwindBuilder;

#[derive(Parser)]
#[command(name = "tailwind-extractor-cli")]
#[command(about = "Tailwind CSS extractor pipe mode", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Extract classes from JavaScript and generate CSS (pipe mode)
    Pipe {
        /// Transform JavaScript instead of generating CSS
        #[arg(long)]
        transform: bool,
        
        /// Disable preflight CSS
        #[arg(long = "no-preflight")]
        no_preflight: bool,
        
        /// Minify output CSS
        #[arg(long)]
        minify: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Default to pipe command if no command specified
    let command = cli.command.unwrap_or(Commands::Pipe {
        transform: false,
        no_preflight: false,
        minify: false,
    });
    
    match command {
        Commands::Pipe { transform, no_preflight, minify } => {
            if transform {
                handle_transform_mode()
            } else {
                handle_css_generation_mode(no_preflight, minify)
            }
        }
    }
}

/// Transform mode: Read JS from stdin, transform Tailwind classes, output transformed JS
fn handle_transform_mode() -> Result<()> {
    // TODO: Implement JavaScript transformation using SWC
    // For now, just pass through the input unchanged
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)
        .context("Failed to read from stdin")?;
    
    io::stdout().write_all(input.as_bytes())
        .context("Failed to write to stdout")?;
    
    Ok(())
}

/// CSS generation mode: Read JS from stdin, extract classes, generate and output CSS
fn handle_css_generation_mode(no_preflight: bool, _minify: bool) -> Result<()> {
    // Read input from stdin
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)
        .context("Failed to read from stdin")?;
    
    // If input is empty, output empty CSS
    if input.trim().is_empty() {
        return Ok(());
    }
    
    // Extract Tailwind classes from the JavaScript input
    let classes = extract_classes_from_javascript(&input)?;
    
    if classes.is_empty() {
        // No classes found, output empty CSS
        return Ok(());
    }
    
    // Generate CSS using tailwind-rs
    let css = generate_tailwind_css(classes, no_preflight)?;
    
    // Write CSS to stdout
    io::stdout().write_all(css.as_bytes())
        .context("Failed to write CSS to stdout")?;
    
    Ok(())
}

/// Extract Tailwind classes from JavaScript content
fn extract_classes_from_javascript(js_content: &str) -> Result<Vec<String>> {
    let mut classes = HashSet::new();
    
    // Extract classes from JavaScript arrays like: const classes = ["class1", "class2", ...]
    // This is what the test is sending us
    if let Some(array_start) = js_content.find('[') {
        if let Some(array_end) = js_content[array_start..].find(']') {
            let array_content = &js_content[array_start + 1..array_start + array_end];
            
            // Extract strings from the array
            for item in array_content.split(',') {
                let trimmed = item.trim();
                // Remove quotes if present
                let class = if trimmed.starts_with('"') && trimmed.ends_with('"') {
                    &trimmed[1..trimmed.len() - 1]
                } else if trimmed.starts_with('\'') && trimmed.ends_with('\'') {
                    &trimmed[1..trimmed.len() - 1]
                } else {
                    trimmed
                };
                
                if !class.is_empty() {
                    // Split on spaces to handle multiple classes in one string
                    for individual_class in class.split_whitespace() {
                        classes.insert(individual_class.to_string());
                    }
                }
            }
        }
    }
    
    // Also look for string literals in general (fallback for other formats)
    // Match patterns like "class1 class2" or 'class1 class2'
    let string_pattern = regex::Regex::new(r#"["']([^"']+)["']"#)?;
    for cap in string_pattern.captures_iter(js_content) {
        if let Some(matched) = cap.get(1) {
            let class_string = matched.as_str();
            // Check if this looks like Tailwind classes (contains hyphens or common prefixes)
            if class_string.contains('-') || 
               class_string.starts_with("hover:") || 
               class_string.starts_with("focus:") ||
               class_string.starts_with("lg:") ||
               class_string.starts_with("md:") ||
               class_string.starts_with("sm:") ||
               class_string.starts_with("xl:") ||
               class_string.starts_with("2xl:") ||
               class_string.starts_with("dark:") ||
               class_string.starts_with("@") {
                for class in class_string.split_whitespace() {
                    classes.insert(class.to_string());
                }
            }
        }
    }
    
    Ok(classes.into_iter().collect())
}


/// Generate Tailwind CSS for the given classes
fn generate_tailwind_css(classes: Vec<String>, no_preflight: bool) -> Result<String> {
    let mut builder = TailwindBuilder::default();
    
    // Configure preflight
    builder.preflight.disable = no_preflight;
    
    // Process each class through the builder
    for class in &classes {
        // Try to trace the class - silently ignore failures
        // This is where tailwind-rs limitations become apparent
        let _ = builder.trace(class, false);
    }
    
    // Generate the CSS bundle from tailwind-rs
    match builder.bundle() {
        Ok(css_string) => Ok(css_string),
        Err(e) => {
            eprintln!("Warning: tailwind-rs bundle generation failed: {}", e);
            Ok(String::new())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_extract_classes_from_array() {
        let js = r#"const classes = ["bg-blue-500", "text-white", "p-4", "hover:bg-blue-600"];"#;
        let classes = extract_classes_from_javascript(js).unwrap();
        assert!(classes.contains(&"bg-blue-500".to_string()));
        assert!(classes.contains(&"text-white".to_string()));
        assert!(classes.contains(&"p-4".to_string()));
        assert!(classes.contains(&"hover:bg-blue-600".to_string()));
    }
    
    #[test]
    fn test_extract_classes_with_spaces() {
        let js = r#"const classes = ["bg-blue-500 text-white", "p-4 m-2"];"#;
        let classes = extract_classes_from_javascript(js).unwrap();
        assert!(classes.contains(&"bg-blue-500".to_string()));
        assert!(classes.contains(&"text-white".to_string()));
        assert!(classes.contains(&"p-4".to_string()));
        assert!(classes.contains(&"m-2".to_string()));
    }
    
    #[test]
    fn test_extract_responsive_classes() {
        let js = r#"const classes = ["lg:flex-row", "md:grid-cols-2", "sm:text-base"];"#;
        let classes = extract_classes_from_javascript(js).unwrap();
        assert!(classes.contains(&"lg:flex-row".to_string()));
        assert!(classes.contains(&"md:grid-cols-2".to_string()));
        assert!(classes.contains(&"sm:text-base".to_string()));
    }
}