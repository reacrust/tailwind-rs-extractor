use serde::{Deserialize, Serialize};
use serde_json::Value;
use indexmap::IndexMap;
use chrono::{DateTime, Utc};

/// Metadata for the generated manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestMetadata {
    /// Version of the manifest format
    pub version: String,
    
    /// Timestamp when the manifest was generated
    pub generated_at: DateTime<Utc>,
    
    /// Number of files processed
    pub files_processed: usize,
    
    /// Number of unique classes extracted
    pub classes_extracted: usize,
    
    /// Whether obfuscation was enabled
    pub obfuscation_enabled: bool,
    
    /// Build mode (development or production)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build_mode: Option<String>,
    
    /// Extractor version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extractor_version: Option<String>,
}

/// Detailed class information in the manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestClassInfo {
    /// Number of occurrences of this class
    pub count: usize,
    
    /// Files where this class was found (with line:column)
    pub files: Vec<String>,
    
    /// Size contribution in bytes (approximate)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<usize>,
}

/// Complete manifest structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    /// Metadata about the extraction
    pub metadata: ManifestMetadata,
    
    /// Map of class names to their usage information
    pub classes: IndexMap<String, ManifestClassInfo>,
    
    /// Obfuscation mappings (original -> obfuscated)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mappings: Option<IndexMap<String, String>>,
    
    /// Statistics about the extraction
    #[serde(skip_serializing_if = "Option::is_none")]
    pub statistics: Option<ManifestStatistics>,
}

/// Statistics about the extraction process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestStatistics {
    /// Total CSS size in bytes
    pub css_size_bytes: usize,
    
    /// CSS size after minification (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minified_size_bytes: Option<usize>,
    
    /// Number of files that matched patterns
    pub files_matched: usize,
    
    /// Number of files actually containing classes
    pub files_with_classes: usize,
    
    /// Processing time in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub processing_time_ms: Option<u64>,
    
    /// Top used classes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_classes: Option<Vec<TopClass>>,
}

/// Information about frequently used classes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopClass {
    pub name: String,
    pub count: usize,
    pub file_count: usize,
}

impl Manifest {
    /// Create a new manifest with default metadata
    pub fn new() -> Self {
        Self {
            metadata: ManifestMetadata {
                version: "1.0.0".to_string(),
                generated_at: Utc::now(),
                files_processed: 0,
                classes_extracted: 0,
                obfuscation_enabled: false,
                build_mode: None,
                extractor_version: Some(env!("CARGO_PKG_VERSION").to_string()),
            },
            classes: IndexMap::new(),
            mappings: None,
            statistics: None,
        }
    }
    
    /// Add or update class information
    pub fn add_class(&mut self, class_name: String, file_location: String) {
        let entry = self.classes.entry(class_name).or_insert_with(|| ManifestClassInfo {
            count: 0,
            files: Vec::new(),
            size_bytes: None,
        });
        
        entry.count += 1;
        if !entry.files.contains(&file_location) {
            entry.files.push(file_location);
        }
    }
    
    /// Set obfuscation mappings
    pub fn set_mappings(&mut self, mappings: IndexMap<String, String>) {
        self.metadata.obfuscation_enabled = true;
        self.mappings = Some(mappings);
    }
    
    /// Calculate and set statistics
    pub fn calculate_statistics(&mut self, css_size: usize, minified_size: Option<usize>, processing_time_ms: Option<u64>) {
        // Count files with classes
        let mut files_with_classes = std::collections::HashSet::new();
        for class_info in self.classes.values() {
            for file in &class_info.files {
                // Extract just the file path (before line:column)
                if let Some(path) = file.split(':').next() {
                    files_with_classes.insert(path.to_string());
                }
            }
        }
        
        // Find top classes
        let mut class_list: Vec<_> = self.classes.iter()
            .map(|(name, info)| TopClass {
                name: name.clone(),
                count: info.count,
                file_count: info.files.len(),
            })
            .collect();
        
        class_list.sort_by(|a, b| b.count.cmp(&a.count));
        let top_classes = class_list.into_iter().take(10).collect();
        
        self.statistics = Some(ManifestStatistics {
            css_size_bytes: css_size,
            minified_size_bytes: minified_size,
            files_matched: self.metadata.files_processed,
            files_with_classes: files_with_classes.len(),
            processing_time_ms,
            top_classes: Some(top_classes),
        });
    }
    
    /// Convert manifest to JSON value
    pub fn to_json(&self) -> Value {
        serde_json::to_value(self).unwrap_or_else(|_| serde_json::json!({}))
    }
    
    /// Convert manifest to pretty JSON string
    pub fn to_pretty_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
    
    /// Convert manifest to compact JSON string
    pub fn to_compact_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

impl Default for Manifest {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder pattern for creating manifests
pub struct ManifestBuilder {
    manifest: Manifest,
    start_time: Option<std::time::Instant>,
}

impl ManifestBuilder {
    /// Create a new manifest builder
    pub fn new() -> Self {
        Self {
            manifest: Manifest::new(),
            start_time: Some(std::time::Instant::now()),
        }
    }
    
    /// Set the build mode
    pub fn with_build_mode(mut self, mode: String) -> Self {
        self.manifest.metadata.build_mode = Some(mode);
        self
    }
    
    /// Set the number of files processed
    pub fn with_files_processed(mut self, count: usize) -> Self {
        self.manifest.metadata.files_processed = count;
        self
    }
    
    /// Set the number of classes extracted
    pub fn with_classes_extracted(mut self, count: usize) -> Self {
        self.manifest.metadata.classes_extracted = count;
        self
    }
    
    /// Add class information from a HashMap
    pub fn with_class_info(mut self, classes: IndexMap<String, Vec<String>>) -> Self {
        for (class_name, locations) in classes {
            let info = ManifestClassInfo {
                count: locations.len(),
                files: locations,
                size_bytes: None,
            };
            self.manifest.classes.insert(class_name, info);
        }
        self
    }
    
    /// Set obfuscation mappings
    pub fn with_mappings(mut self, mappings: IndexMap<String, String>) -> Self {
        self.manifest.set_mappings(mappings);
        self
    }
    
    /// Build the final manifest with statistics
    pub fn build(mut self, css_size: usize, minified_size: Option<usize>) -> Manifest {
        let processing_time = self.start_time.map(|t| t.elapsed().as_millis() as u64);
        self.manifest.calculate_statistics(css_size, minified_size, processing_time);
        self.manifest
    }
}

impl Default for ManifestBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_manifest_creation() {
        let manifest = Manifest::new();
        assert_eq!(manifest.metadata.version, "1.0.0");
        assert_eq!(manifest.classes.len(), 0);
        assert!(!manifest.metadata.obfuscation_enabled);
    }
    
    #[test]
    fn test_add_class() {
        let mut manifest = Manifest::new();
        manifest.add_class("bg-blue-500".to_string(), "src/app.js:10:15".to_string());
        manifest.add_class("bg-blue-500".to_string(), "src/app.js:20:10".to_string());
        manifest.add_class("text-white".to_string(), "src/app.js:10:30".to_string());
        
        assert_eq!(manifest.classes.len(), 2);
        assert_eq!(manifest.classes["bg-blue-500"].count, 2);
        assert_eq!(manifest.classes["text-white"].count, 1);
    }
    
    #[test]
    fn test_manifest_builder() {
        let mut classes = IndexMap::new();
        classes.insert("p-4".to_string(), vec![
            "src/app.js:1:1".to_string(),
            "src/app.js:2:1".to_string(),
        ]);
        classes.insert("m-2".to_string(), vec!["src/other.js:5:10".to_string()]);
        
        let manifest = ManifestBuilder::new()
            .with_build_mode("production".to_string())
            .with_files_processed(10)
            .with_classes_extracted(50)
            .with_class_info(classes)
            .build(1024, Some(512));
        
        assert_eq!(manifest.metadata.files_processed, 10);
        assert_eq!(manifest.metadata.classes_extracted, 50);
        assert_eq!(manifest.metadata.build_mode, Some("production".to_string()));
        assert!(manifest.statistics.is_some());
        
        let stats = manifest.statistics.unwrap();
        assert_eq!(stats.css_size_bytes, 1024);
        assert_eq!(stats.minified_size_bytes, Some(512));
    }
    
    #[test]
    fn test_json_serialization() {
        let manifest = Manifest::new();
        let json = manifest.to_json();
        
        assert!(json["metadata"].is_object());
        assert_eq!(json["metadata"]["version"], "1.0.0");
        assert!(json["classes"].is_object());
    }
    
    #[test]
    fn test_top_classes() {
        let mut manifest = Manifest::new();
        
        // Add classes with different frequencies
        for i in 0..5 {
            manifest.add_class("frequent".to_string(), format!("file{}:1:1", i));
        }
        for i in 0..3 {
            manifest.add_class("moderate".to_string(), format!("file{}:1:1", i));
        }
        manifest.add_class("rare".to_string(), "file1:1:1".to_string());
        
        manifest.calculate_statistics(1000, None, None);
        
        let stats = manifest.statistics.unwrap();
        let top_classes = stats.top_classes.unwrap();
        
        assert_eq!(top_classes[0].name, "frequent");
        assert_eq!(top_classes[0].count, 5);
        assert_eq!(top_classes[1].name, "moderate");
        assert_eq!(top_classes[1].count, 3);
    }
}