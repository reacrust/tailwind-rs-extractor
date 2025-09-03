use crate::config::{TailwindConfig, ObfuscationConfig};
use crate::errors::{ExtractorError, Result};
use crate::manifest::ManifestBuilder;
use indexmap::IndexMap;
use std::path::Path;
use tailwind_rs::TailwindBuilder;
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};
use chrono::Utc;

/// Comment type for CSS minification
#[derive(Debug, Clone, Copy, PartialEq)]
enum CommentType {
    None,
    Block,
}

/// Represents a Tailwind class extraction context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassInfo {
    /// Original class name
    pub original: String,
    
    /// Obfuscated class name (if obfuscation is enabled)
    pub obfuscated: Option<String>,
    
    /// Number of occurrences
    pub count: usize,
    
    /// Files where this class was found
    pub files: Vec<String>,
}

/// Main Tailwind extractor that processes files and generates CSS
pub struct TailwindExtractor {
    /// Configuration
    config: TailwindConfig,
    
    /// Tailwind CSS builder
    builder: TailwindBuilder,
    
    /// Tracked classes with their information
    classes: IndexMap<String, ClassInfo>,
    
    /// Obfuscation mapping (original -> obfuscated)
    obfuscation_map: IndexMap<String, String>
}

impl TailwindExtractor {
    /// Create a new extractor with default configuration
    pub fn new() -> Self {
        Self::with_config(TailwindConfig::default())
    }
    
    /// Create a new extractor with custom configuration
    pub fn with_config(config: TailwindConfig) -> Self {
        let builder = TailwindBuilder::default();
        
        Self {
            config,
            builder,
            classes: IndexMap::new(),
            obfuscation_map: IndexMap::new(),
        }
    }
    
    /// Create a new extractor with configuration and preflight setting
    pub fn with_config_and_preflight(config: TailwindConfig, no_preflight: bool) -> Self {
        let mut builder = TailwindBuilder::default();
        
        // Configure preflight
        builder.preflight.disable = no_preflight;
        
        Self {
            config,
            builder,
            classes: IndexMap::new(),
            obfuscation_map: IndexMap::new(),
        }
    }
    
    /// Load configuration from a file and create an extractor
    pub fn from_config_file(path: &Path) -> Result<Self> {
        let config = TailwindConfig::from_file(path)?;
        Ok(Self::with_config(config))
    }
    
    /// Add a potential Tailwind class from a file
    pub fn add_class(&mut self, class: &str, file_path: &str) -> Result<()> {
        // Skip empty or invalid classes
        if class.is_empty() || !Self::is_valid_class(class) {
            return Ok(());
        }
        
        // Try to trace the class with Tailwind to validate it
        // The second parameter (false) means we want CSS output, not inline styles
        // Since trace() is now idempotent and handles both Tailwind and custom classes,
        // we just check if it succeeds. The builder internally tracks what CSS to generate.
        if self.builder.trace(class, false).is_ok() {
            // Class was successfully traced, it's valid (either Tailwind or custom)
            // Continue to add it to our tracking
        } else {
            // Trace failed, not a valid class
            return Ok(());
        }
        
        // Update class info
        let info = self.classes.entry(class.to_string()).or_insert_with(|| {
            ClassInfo {
                original: class.to_string(),
                obfuscated: None,
                count: 0,
                files: Vec::new(),
            }
        });
        
        info.count += 1;
        if !info.files.contains(&file_path.to_string()) {
            info.files.push(file_path.to_string());
        }
        
        Ok(())
    }
    
    /// Add multiple classes at once
    pub fn add_classes(&mut self, classes: &[String], file_path: &str) -> Result<()> {
        for class in classes {
            self.add_class(class, file_path)?;
        }
        Ok(())
    }
    
    /// Generate CSS from the collected classes
    pub fn generate_css(&mut self, minify: bool) -> Result<String> {
        if self.classes.is_empty() {
            return Ok(Self::generate_css_header(true, minify));
        }
        
        // If obfuscation is enabled, generate obfuscated class names
        if self.config.obfuscation.enabled {
            self.generate_obfuscation_map();
        }
        
        // Create a new builder for CSS generation with same preflight setting
        let mut builder = TailwindBuilder::default();
        builder.preflight.disable = self.builder.preflight.disable;
        
        // Trace all valid classes
        for (class, _info) in &self.classes {
            // Use obfuscated class name if available, otherwise original
            let class_to_use = if self.config.obfuscation.enabled {
                self.obfuscation_map.get(class).unwrap_or(class)
            } else {
                class
            };
            
            // Trace the class for bundling
            // The result is handled internally by the builder
            // We don't need to use the returned Cow<str> here
            if let Err(e) = builder.trace(class_to_use, false) {
                eprintln!("Warning: Failed to trace class '{}': {:?}", class_to_use, e);
            }
        }
        
        // Generate the CSS bundle
        let mut css = builder.bundle()
            .map_err(|e| ExtractorError::TailwindError(format!("Failed to generate CSS: {:?}", e)))?;
        
        // Add header
        let header = Self::generate_css_header(false, minify);
        css = format!("{}{}", header, css);
        
        // Apply minification if requested
        if minify {
            Ok(Self::minify_css(&css))
        } else {
            Ok(css)
        }
    }
    
    /// Generate the class manifest with enhanced information
    pub fn generate_manifest(&self) -> serde_json::Value {
        self.generate_manifest_with_stats(0, 0, None)
    }
    
    /// Generate manifest with additional statistics
    pub fn generate_manifest_with_stats(&self, files_processed: usize, css_size: usize, minified_size: Option<usize>) -> serde_json::Value {
        let mut builder = ManifestBuilder::new();
        
        // Set basic metadata
        builder = builder
            .with_files_processed(files_processed)
            .with_classes_extracted(self.classes.len());
        
        // Build class information map
        let mut class_info_map = IndexMap::new();
        for (class_name, info) in &self.classes {
            class_info_map.insert(class_name.clone(), info.files.clone());
        }
        builder = builder.with_class_info(class_info_map);
        
        // Add obfuscation mappings if enabled
        if self.config.obfuscation.enabled && !self.obfuscation_map.is_empty() {
            builder = builder.with_mappings(self.obfuscation_map.clone());
        }
        
        // Build final manifest with statistics
        let manifest = builder.build(css_size, minified_size);
        manifest.to_json()
    }
    
    /// Get the total number of unique classes
    pub fn class_count(&self) -> usize {
        self.classes.len()
    }
    
    /// Reset the extractor state
    pub fn reset(&mut self) {
        self.classes.clear();
        self.obfuscation_map.clear();
        self.builder = TailwindBuilder::default();
    }
    
    /// Check if a string is a potentially valid Tailwind class
    fn is_valid_class(class: &str) -> bool {
        // Basic validation - avoid obvious non-classes
        if class.len() > 100 {
            return false;
        }
        
        // Check for dangerous characters
        if class.contains('<') || class.contains('>') || 
           class.contains('{') || class.contains('}') || 
           class.contains(';') {
            return false;
        }
        
        // Allow valid Tailwind class characters
        class.chars().all(|c| {
            c.is_alphanumeric() || 
            "-:/.[]!()#%&*_@".contains(c)
        })
    }
    
    /// Generate deterministic obfuscation mapping
    fn generate_obfuscation_map(&mut self) {
        let config = &self.config.obfuscation;
        
        for (class, info) in &mut self.classes {
            let obfuscated = Self::obfuscate_class(class, config);
            self.obfuscation_map.insert(class.clone(), obfuscated.clone());
            info.obfuscated = Some(obfuscated);
        }
    }
    
    /// Obfuscate a single class name deterministically
    fn obfuscate_class(class: &str, config: &ObfuscationConfig) -> String {
        use std::collections::hash_map::DefaultHasher;
        
        // Create a deterministic hash
        let mut hasher = DefaultHasher::new();
        config.seed.hash(&mut hasher);
        class.hash(&mut hasher);
        let hash = hasher.finish();
        
        // Convert to base62 for short, readable class names
        let base62 = Self::to_base62(hash);
        
        format!("{}{}", config.prefix, base62)
    }
    
    /// Convert a number to base62 string
    fn to_base62(mut num: u64) -> String {
        const CHARS: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
        
        if num == 0 {
            return "0".to_string();
        }
        
        let mut result = Vec::new();
        while num > 0 {
            result.push(CHARS[(num % 62) as usize]);
            num /= 62;
        }
        
        result.reverse();
        String::from_utf8(result).unwrap()
    }
    
    /// Generate CSS header comment
    fn generate_css_header(empty: bool, minified: bool) -> String {
        if minified {
            if empty {
                "/* tailwind-extractor-cli: No classes found */".to_string()
            } else {
                format!("/* Generated by tailwind-extractor-cli v{} at {} */",
                    env!("CARGO_PKG_VERSION"),
                    Utc::now().format("%Y-%m-%d %H:%M:%S UTC"))
            }
        } else {
            if empty {
                format!(r#"/**
 * Generated by tailwind-extractor-cli v{}
 * Generation time: {}
 * 
 * No Tailwind classes found
 */
"#,
                    env!("CARGO_PKG_VERSION"),
                    Utc::now().format("%Y-%m-%d %H:%M:%S UTC"))
            } else {
                format!(r#"/**
 * Generated by tailwind-extractor-cli v{}
 * Generation time: {}
 * 
 * This file contains extracted Tailwind CSS utilities.
 * DO NOT EDIT - This file is auto-generated.
 */

"#,
                    env!("CARGO_PKG_VERSION"),
                    Utc::now().format("%Y-%m-%d %H:%M:%S UTC"))
            }
        }
    }
    
    /// Enhanced CSS minification with better compression
    fn minify_css(css: &str) -> String {
        let mut result = String::with_capacity(css.len());
        let mut prev_char = ' ';
        let mut in_comment = false;
        let mut comment_type = CommentType::None;
        
        let chars: Vec<char> = css.chars().collect();
        let mut i = 0;
        
        while i < chars.len() {
            let ch = chars[i];
            
            // Handle comments
            if !in_comment && i + 1 < chars.len() {
                if ch == '/' && chars[i + 1] == '*' {
                    // Start of block comment - preserve if it's a header comment
                    if i == 0 || (i < 100 && css[..i].trim().is_empty()) {
                        // This looks like a header comment, preserve it
                        result.push(ch);
                        result.push(chars[i + 1]);
                        i += 2;
                        
                        // Copy until end of comment
                        while i + 1 < chars.len() {
                            result.push(chars[i]);
                            if chars[i] == '*' && chars[i + 1] == '/' {
                                result.push(chars[i + 1]);
                                i += 2;
                                break;
                            }
                            i += 1;
                        }
                        continue;
                    } else {
                        in_comment = true;
                        comment_type = CommentType::Block;
                        i += 2;
                        continue;
                    }
                }
            }
            
            if in_comment {
                if comment_type == CommentType::Block && ch == '*' && i + 1 < chars.len() && chars[i + 1] == '/' {
                    in_comment = false;
                    comment_type = CommentType::None;
                    i += 2;
                    continue;
                }
                i += 1;
                continue;
            }
            
            // Skip unnecessary whitespace
            if ch.is_whitespace() {
                if !prev_char.is_whitespace() && 
                   prev_char != '{' && prev_char != '}' && 
                   prev_char != ';' && prev_char != ':' && 
                   prev_char != ',' {
                    // Check if next char needs space
                    if i + 1 < chars.len() {
                        let next = chars[i + 1];
                        if next != '{' && next != '}' && 
                           next != ';' && next != ':' && 
                           next != ',' && !next.is_whitespace() {
                            // Keep as single space
                            result.push(' ');
                            prev_char = ' ';
                        }
                    }
                }
            } else {
                result.push(ch);
                prev_char = ch;
            }
            
            i += 1;
        }
        
        result
    }
}

impl Default for TailwindExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_extractor_creation() {
        let extractor = TailwindExtractor::new();
        assert_eq!(extractor.class_count(), 0);
    }
    
    #[test]
    fn test_class_validation() {
        assert!(TailwindExtractor::is_valid_class("bg-blue-500"));
        assert!(TailwindExtractor::is_valid_class("hover:text-white"));
        assert!(TailwindExtractor::is_valid_class("md:p-4"));
        assert!(TailwindExtractor::is_valid_class("w-1/2"));
        assert!(TailwindExtractor::is_valid_class("text-[#1a73e8]"));
        
        assert!(!TailwindExtractor::is_valid_class("<script>"));
        assert!(!TailwindExtractor::is_valid_class("class{bad}"));
        assert!(!TailwindExtractor::is_valid_class("semi;colon"));
    }
    
    #[test]
    fn test_add_class() {
        let mut extractor = TailwindExtractor::new();
        
        // Add valid Tailwind classes
        extractor.add_class("p-4", "test.js").unwrap();
        extractor.add_class("bg-blue-500", "test.js").unwrap();
        extractor.add_class("p-4", "other.js").unwrap(); // Duplicate class
        
        // Check that we have 2 unique classes
        assert_eq!(extractor.class_count(), 2);
        
        // Check class info
        let p4_info = extractor.classes.get("p-4").unwrap();
        assert_eq!(p4_info.count, 2);
        assert_eq!(p4_info.files.len(), 2);
    }
    
    #[test]
    fn test_obfuscation() {
        let mut config = TailwindConfig::default();
        config.obfuscation.enabled = true;
        config.obfuscation.prefix = "c".to_string();
        
        let obfuscated1 = TailwindExtractor::obfuscate_class("bg-blue-500", &config.obfuscation);
        let obfuscated2 = TailwindExtractor::obfuscate_class("bg-blue-500", &config.obfuscation);
        
        // Should be deterministic
        assert_eq!(obfuscated1, obfuscated2);
        
        // Should start with prefix
        assert!(obfuscated1.starts_with("c"));
        
        // Should be different for different classes
        let obfuscated3 = TailwindExtractor::obfuscate_class("text-white", &config.obfuscation);
        assert_ne!(obfuscated1, obfuscated3);
    }
    
    #[test]
    fn test_base62_conversion() {
        assert_eq!(TailwindExtractor::to_base62(0), "0");
        assert_eq!(TailwindExtractor::to_base62(61), "z");
        assert_eq!(TailwindExtractor::to_base62(62), "10");
        assert_eq!(TailwindExtractor::to_base62(3843), "zz");
    }
    
    #[test]
    fn test_css_minification() {
        let css = r#"
            .class1 {
                color: blue;
                padding: 10px;
            }
            
            .class2 {
                background: red;
            }
        "#;
        
        let minified = TailwindExtractor::minify_css(css);
        assert!(!minified.contains('\n'));
        assert!(!minified.contains("  "));
        assert!(minified.contains(".class1{"));
        assert!(minified.contains("color:blue;"));
    }
}