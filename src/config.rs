use serde::{Deserialize, Serialize};
use std::path::Path;
use crate::errors::{ExtractorError, Result};

/// Tailwind configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TailwindConfig {
    /// Content paths to scan
    pub content: Vec<String>,
    
    /// Theme configuration
    pub theme: TailwindTheme,
    
    /// Configuration for obfuscation
    pub obfuscation: ObfuscationConfig,
}

impl Default for TailwindConfig {
    fn default() -> Self {
        Self {
            content: vec![
                "./src/**/*.res.mjs".to_string(),
                "./src/**/*.js".to_string(),
                "./src/**/*.jsx".to_string(),
            ],
            theme: TailwindTheme::default(),
            obfuscation: ObfuscationConfig::default(),
        }
    }
}

/// Theme configuration for Tailwind
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TailwindTheme {
    /// Theme extensions
    pub extend: TailwindThemeExtend,
}

impl Default for TailwindTheme {
    fn default() -> Self {
        Self {
            extend: TailwindThemeExtend::default(),
        }
    }
}

/// Theme extensions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TailwindThemeExtend {
    /// Custom colors
    pub colors: std::collections::HashMap<String, String>,
    
    /// Custom font families
    pub font_family: std::collections::HashMap<String, Vec<String>>,
    
    /// Custom spacing values
    pub spacing: std::collections::HashMap<String, String>,
}

impl Default for TailwindThemeExtend {
    fn default() -> Self {
        Self {
            colors: std::collections::HashMap::new(),
            font_family: std::collections::HashMap::new(),
            spacing: std::collections::HashMap::new(),
        }
    }
}

/// Configuration for class obfuscation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ObfuscationConfig {
    /// Enable obfuscation
    pub enabled: bool,
    
    /// Prefix for obfuscated classes
    pub prefix: String,
    
    /// Seed for deterministic hashing
    pub seed: u64,
}

impl Default for ObfuscationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            prefix: "tw".to_string(),
            seed: 0x1337_BEEF_CAFE_BABE,
        }
    }
}

impl TailwindConfig {
    /// Load configuration from a YAML file
    pub fn from_yaml_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| ExtractorError::ConfigError {
                message: format!("Failed to read config file {}: {}", path.display(), e),
            })?;
            
        serde_yaml::from_str(&content)
            .map_err(|e| ExtractorError::ConfigError {
                message: format!("Failed to parse YAML config: {}", e),
            })
    }
    
    /// Load configuration from a JSON file
    pub fn from_json_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| ExtractorError::ConfigError {
                message: format!("Failed to read config file {}: {}", path.display(), e),
            })?;
            
        serde_json::from_str(&content)
            .map_err(|e| ExtractorError::ConfigError {
                message: format!("Failed to parse JSON config: {}", e),
            })
    }
    
    /// Load configuration from a file (auto-detect format)
    pub fn from_file(path: &Path) -> Result<Self> {
        match path.extension().and_then(|s| s.to_str()) {
            Some("yaml") | Some("yml") => Self::from_yaml_file(path),
            Some("json") => Self::from_json_file(path),
            _ => Err(ExtractorError::ConfigError {
                message: format!(
                    "Unsupported config file format: {}. Use .yaml, .yml, or .json",
                    path.display()
                ),
            }),
        }
    }
    
    /// Merge with another configuration
    pub fn merge(mut self, other: Self) -> Self {
        // Merge content paths
        for path in other.content {
            if !self.content.contains(&path) {
                self.content.push(path);
            }
        }
        
        // Merge theme extensions
        self.theme.extend.colors.extend(other.theme.extend.colors);
        self.theme.extend.font_family.extend(other.theme.extend.font_family);
        self.theme.extend.spacing.extend(other.theme.extend.spacing);
        
        // Override obfuscation settings if enabled in other
        if other.obfuscation.enabled {
            self.obfuscation = other.obfuscation;
        }
        
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;
    
    #[test]
    fn test_default_config() {
        let config = TailwindConfig::default();
        assert!(!config.content.is_empty());
        assert!(!config.obfuscation.enabled);
    }
    
    #[test]
    fn test_yaml_config_loading() {
        let yaml_content = r##"
content:
  - "./src/**/*.js"
  - "./components/**/*.jsx"
theme:
  extend:
    colors:
      primary: "#1a73e8"
      secondary: "#ff6b6b"
obfuscation:
  enabled: true
  prefix: "c"
"##;
        
        let mut file = NamedTempFile::with_suffix(".yaml").unwrap();
        file.write_all(yaml_content.as_bytes()).unwrap();
        
        let config = TailwindConfig::from_yaml_file(file.path()).unwrap();
        assert_eq!(config.content.len(), 2);
        assert_eq!(config.theme.extend.colors.get("primary"), Some(&"#1a73e8".to_string()));
        assert!(config.obfuscation.enabled);
        assert_eq!(config.obfuscation.prefix, "c");
    }
    
    #[test]
    fn test_json_config_loading() {
        let json_content = r##"{
  "content": ["./dist/**/*.js"],
  "theme": {
    "extend": {
      "colors": {
        "brand": "#0066cc"
      }
    }
  }
}"##;
        
        let mut file = NamedTempFile::with_suffix(".json").unwrap();
        file.write_all(json_content.as_bytes()).unwrap();
        
        let config = TailwindConfig::from_json_file(file.path()).unwrap();
        assert_eq!(config.content.len(), 1);
        assert_eq!(config.theme.extend.colors.get("brand"), Some(&"#0066cc".to_string()));
    }
    
    #[test]
    fn test_config_merge() {
        let mut base = TailwindConfig::default();
        base.theme.extend.colors.insert("primary".to_string(), "#111".to_string());
        
        let mut other = TailwindConfig::default();
        other.content = vec!["./custom/**/*.js".to_string()];
        other.theme.extend.colors.insert("primary".to_string(), "#222".to_string());
        other.theme.extend.colors.insert("secondary".to_string(), "#333".to_string());
        
        let merged = base.merge(other);
        assert!(merged.content.contains(&"./custom/**/*.js".to_string()));
        assert_eq!(merged.theme.extend.colors.get("primary"), Some(&"#222".to_string()));
        assert_eq!(merged.theme.extend.colors.get("secondary"), Some(&"#333".to_string()));
    }
}