pub mod args;
pub mod ast_visitor;
pub mod config;
pub mod errors;
pub mod extractor;
pub mod manifest;

pub use args::{Cli, Commands, ExtractArgs, PipeArgs};
pub use ast_visitor::{extract_strings_from_file, extract_strings_from_content, extract_strings_parallel, ExtractedString};
pub use config::{TailwindConfig, ObfuscationConfig};
pub use errors::{ExtractorError, Result};
pub use extractor::{TailwindExtractor, ClassInfo};
pub use manifest::{Manifest, ManifestBuilder};

use indicatif::{MultiProgress, ProgressBar, ProgressStyle, ProgressDrawTarget};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use std::fs;

/// Security configuration
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    /// Maximum file size in bytes (default: 10MB)
    pub max_file_size: u64,
    /// Allow symbolic links
    pub allow_symlinks: bool,
    /// Working directory for path traversal checks
    pub working_directory: PathBuf,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            max_file_size: 10 * 1024 * 1024, // 10MB
            allow_symlinks: false,
            working_directory: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }
}

/// Main extraction configuration
#[derive(Debug, Clone)]
pub struct ExtractorConfig {
    pub obfuscate: bool,
    pub minify: bool,
    pub verbose: bool,
    pub jobs: Option<usize>,
    pub security: SecurityConfig,
    pub no_preflight: bool,
}

impl From<&ExtractArgs> for ExtractorConfig {
    fn from(args: &ExtractArgs) -> Self {
        Self {
            obfuscate: args.obfuscate,
            minify: args.minify,
            verbose: args.verbose,
            jobs: args.jobs,
            security: SecurityConfig::default(),
            no_preflight: args.no_preflight,
        }
    }
}

/// Performance statistics
#[derive(Debug, Clone)]
pub struct PerformanceStats {
    pub total_duration: Duration,
    pub file_read_duration: Duration,
    pub parse_duration: Duration,
    pub extraction_duration: Duration,
    pub css_generation_duration: Duration,
    pub files_per_second: f64,
    pub bytes_processed: u64,
}

/// Result of the extraction process
#[derive(Debug)]
pub struct ExtractionResult {
    pub css_content: String,
    pub manifest: serde_json::Value,
    pub total_classes: usize,
    pub total_files_processed: usize,
    pub performance_stats: Option<PerformanceStats>,
}

/// Main extractor entry point
pub async fn extract(args: ExtractArgs) -> Result<ExtractionResult> {
    let start_time = Instant::now();
    let mut stats = PerformanceStats {
        total_duration: Duration::from_secs(0),
        file_read_duration: Duration::from_secs(0),
        parse_duration: Duration::from_secs(0),
        extraction_duration: Duration::from_secs(0),
        css_generation_duration: Duration::from_secs(0),
        files_per_second: 0.0,
        bytes_processed: 0,
    };

    // Validate arguments
    args.validate()
        .map_err(|e| ExtractorError::InvalidInput(e))?;

    // Create configuration
    let config = ExtractorConfig::from(&args);

    // Security: Validate output paths are safe
    validate_output_path(&args.output_css, &config.security)?;
    validate_output_path(&args.output_manifest, &config.security)?;

    if config.verbose {
        eprintln!("Starting Tailwind CSS extraction...");
        eprintln!("Input patterns: {:?}", args.input);
        eprintln!("Output CSS: {}", args.output_css.display());
        eprintln!("Output manifest: {}", args.output_manifest.display());
        eprintln!("Security: max file size = {} MB", config.security.max_file_size / (1024 * 1024));
    }

    // Collect files matching the patterns
    let files = collect_files_with_security(&args.input, &args.exclude, &config.security)?;
    
    if files.is_empty() {
        return Err(ExtractorError::NoFilesFound);
    }

    if config.verbose {
        eprintln!("Found {} files to process", files.len());
        let total_size: u64 = files.iter().map(|f| f.1).sum();
        eprintln!("Total size: {:.2} MB", total_size as f64 / (1024.0 * 1024.0));
    }

    // Create multi-progress container for better progress reporting
    let multi_progress = if !config.verbose {
        MultiProgress::new()
    } else {
        MultiProgress::with_draw_target(ProgressDrawTarget::hidden())
    };

    // Create main progress bar
    let progress_bar = if !config.verbose {
        let pb = multi_progress.add(ProgressBar::new(files.len() as u64));
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({msg})")
                .unwrap()
                .progress_chars("█▉▊▋▌▍▎▏  "),
        );
        pb.set_message("Starting extraction...");
        Some(pb)
    } else {
        None
    };

    // Extract strings from all files with progress tracking
    let extraction_start = Instant::now();
    
    let file_paths: Vec<PathBuf> = files.iter().map(|(path, _)| path.clone()).collect();
    stats.bytes_processed = files.iter().map(|f| f.1).sum();
    
    let extracted_strings = if let Some(jobs) = config.jobs {
        extract_strings_parallel_with_progress(&file_paths, Some(jobs), progress_bar.as_ref())?
    } else {
        extract_strings_parallel_with_progress(&file_paths, None, progress_bar.as_ref())?
    };
    
    stats.extraction_duration = extraction_start.elapsed();

    // Update progress bar
    if let Some(ref pb) = progress_bar {
        pb.set_message("Processing Tailwind classes...");
        pb.set_position(files.len() as u64);
    }

    // Collect unique class names
    let mut unique_classes = std::collections::HashSet::new();
    let mut class_locations = std::collections::HashMap::new();
    
    for extracted in &extracted_strings {
        unique_classes.insert(extracted.value.clone());
        class_locations.entry(extracted.value.clone())
            .or_insert_with(Vec::new)
            .push(format!("{}:{}:{}", extracted.file_path, extracted.line, extracted.column));
    }

    if config.verbose {
        eprintln!("Extracted {} unique classes from {} total occurrences", 
                  unique_classes.len(), extracted_strings.len());
    }

    // Load Tailwind configuration if provided
    let tailwind_config = if let Some(config_path) = &args.config {
        TailwindConfig::from_file(config_path)?
    } else {
        TailwindConfig::default()
    };
    
    // Apply obfuscation settings from command line
    let mut tailwind_config = tailwind_config;
    if config.obfuscate {
        tailwind_config.obfuscation.enabled = true;
    }
    
    // Create the Tailwind extractor with preflight configuration
    let mut extractor = TailwindExtractor::with_config_and_preflight(tailwind_config, config.no_preflight);
    
    // Add all extracted strings to the Tailwind extractor
    // Group by file for better tracking
    let mut file_classes: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    
    for extracted in &extracted_strings {
        file_classes.entry(extracted.file_path.clone())
            .or_insert_with(Vec::new)
            .push(extracted.value.clone());
    }
    
    // Add classes to the extractor
    for (file_path, classes) in file_classes {
        extractor.add_classes(&classes, &file_path)?;
    }
    
    if config.verbose {
        eprintln!("Processing {} valid Tailwind classes", extractor.class_count());
    }
    
    // Update progress for CSS generation
    if let Some(ref pb) = progress_bar {
        pb.set_message("Generating CSS...");
    }
    
    // Generate CSS and get sizes for statistics
    let css_start = Instant::now();
    let css_content = extractor.generate_css(config.minify)?;
    stats.css_generation_duration = css_start.elapsed();
    
    let css_size = css_content.len();
    let minified_size = if config.minify { Some(css_size) } else { None };
    
    // Generate manifest with full statistics
    let manifest = extractor.generate_manifest_with_stats(
        files.len(),
        css_size,
        minified_size
    );

    // Calculate final statistics
    stats.total_duration = start_time.elapsed();
    stats.files_per_second = files.len() as f64 / stats.total_duration.as_secs_f64();

    let result = ExtractionResult {
        css_content,
        manifest,
        total_classes: extractor.class_count(),
        total_files_processed: files.len(),
        performance_stats: Some(stats.clone()),
    };

    if let Some(pb) = progress_bar {
        pb.finish_with_message(format!("✓ Complete ({:.1} files/sec)", stats.files_per_second));
    }

    // Write output files if not in dry-run mode
    if !args.dry_run {
        write_output_files(&args, &result)?;
    }

    if config.verbose {
        eprintln!("\nExtraction complete:");
        eprintln!("  - Processed {} files", result.total_files_processed);
        eprintln!("  - Extracted {} unique classes", result.total_classes);
        eprintln!("\nPerformance:");
        eprintln!("  - Total time: {:.2}s", stats.total_duration.as_secs_f64());
        eprintln!("  - Extraction: {:.2}s", stats.extraction_duration.as_secs_f64());
        eprintln!("  - CSS generation: {:.2}s", stats.css_generation_duration.as_secs_f64());
        eprintln!("  - Processing rate: {:.1} files/sec", stats.files_per_second);
        eprintln!("  - Data processed: {:.2} MB", stats.bytes_processed as f64 / (1024.0 * 1024.0));
    }

    Ok(result)
}

/// Validate that a path is safe (no path traversal)
fn validate_output_path(path: &Path, security: &SecurityConfig) -> Result<()> {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let working_dir = security.working_directory.canonicalize()
        .unwrap_or_else(|_| security.working_directory.clone());
    
    // Check if path is within working directory
    if !canonical.starts_with(&working_dir) && path.is_relative() {
        return Err(ExtractorError::SecurityError(
            format!("Output path '{}' appears to use path traversal", path.display())
        ));
    }
    
    Ok(())
}

/// Check if a file is safe to read
fn validate_input_file(path: &Path, security: &SecurityConfig) -> Result<()> {
    // Check for symlinks if not allowed
    if !security.allow_symlinks && path.is_symlink() {
        return Err(ExtractorError::SecurityError(
            format!("Symbolic link not allowed: {}", path.display())
        ));
    }
    
    // If it's a symlink and we allow them, validate the target
    if security.allow_symlinks && path.is_symlink() {
        let target = fs::read_link(path).map_err(|e| ExtractorError::SecurityError(
            format!("Cannot read symlink target for '{}': {}", path.display(), e)
        ))?;
        
        // Ensure target is within working directory
        let canonical_target = target.canonicalize().unwrap_or_else(|_| target.clone());
        let working_dir = security.working_directory.canonicalize()
            .unwrap_or_else(|_| security.working_directory.clone());
        
        if !canonical_target.starts_with(&working_dir) {
            return Err(ExtractorError::SecurityError(
                format!("Symlink target '{}' is outside working directory", target.display())
            ));
        }
    }
    
    // Check file size
    let metadata = fs::metadata(path).map_err(|e| ExtractorError::SecurityError(
        format!("Cannot read file metadata for '{}': {}", path.display(), e)
    ))?;
    
    if metadata.len() > security.max_file_size {
        return Err(ExtractorError::SecurityError(
            format!("File '{}' exceeds maximum size limit ({} MB > {} MB)",
                    path.display(),
                    metadata.len() / (1024 * 1024),
                    security.max_file_size / (1024 * 1024))
        ));
    }
    
    Ok(())
}

/// Collect files matching the given patterns with security checks
fn collect_files_with_security(
    patterns: &[String], 
    exclude_patterns: &[String],
    security: &SecurityConfig
) -> Result<Vec<(PathBuf, u64)>> {
    let mut files = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut skipped_count = 0;

    for pattern in patterns {
        for entry in glob::glob(pattern)? {
            let path = entry?;
            
            // Skip if excluded
            if should_exclude(&path, exclude_patterns)? {
                continue;
            }

            // Skip directories
            if path.is_dir() {
                continue;
            }
            
            // Security validation
            match validate_input_file(&path, security) {
                Ok(_) => {},
                Err(e) => {
                    eprintln!("Warning: Skipping file - {}", e);
                    skipped_count += 1;
                    continue;
                }
            }
            
            // Get file size for statistics
            let size = fs::metadata(&path)
                .map(|m| m.len())
                .unwrap_or(0);

            // Add only if not already seen
            if seen.insert(path.clone()) {
                files.push((path, size));
            }
        }
    }
    
    if skipped_count > 0 {
        eprintln!("Skipped {} files due to security constraints", skipped_count);
    }

    Ok(files)
}

/// Check if a path should be excluded
fn should_exclude(path: &Path, exclude_patterns: &[String]) -> Result<bool> {
    for pattern in exclude_patterns {
        let pattern = glob::Pattern::new(pattern)?;
        if pattern.matches_path(path) {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Write the extraction results to output files with atomic writes
fn write_output_files(args: &ExtractArgs, result: &ExtractionResult) -> Result<()> {
    use std::fs;
    
    // Create parent directories if they don't exist
    if let Some(parent) = args.output_css.parent() {
        fs::create_dir_all(parent)?;
    }
    if let Some(parent) = args.output_manifest.parent() {
        fs::create_dir_all(parent)?;
    }

    // Write CSS file atomically
    write_atomic(&args.output_css, &result.css_content)
        .map_err(|e| ExtractorError::OutputError {
            path: args.output_css.display().to_string(),
            message: e.to_string(),
        })?;

    // Write manifest file atomically
    let manifest_content = if args.minify {
        serde_json::to_string(&result.manifest)?
    } else {
        serde_json::to_string_pretty(&result.manifest)?
    };
    
    write_atomic(&args.output_manifest, &manifest_content)
        .map_err(|e| ExtractorError::OutputError {
            path: args.output_manifest.display().to_string(),
            message: e.to_string(),
        })?;

    Ok(())
}

/// Write file atomically by writing to temp file then renaming
fn write_atomic<P: AsRef<std::path::Path>>(path: P, content: &str) -> std::io::Result<()> {
    use std::fs;
    use std::io::Write;
    
    let path = path.as_ref();
    let temp_path = path.with_extension(".tmp");
    
    // Write to temporary file
    let mut file = fs::File::create(&temp_path)?;
    file.write_all(content.as_bytes())?;
    file.sync_all()?; // Ensure data is flushed to disk
    
    // Atomically rename temp file to final name
    fs::rename(&temp_path, path)?;
    
    Ok(())
}

/// Extract strings from files with progress reporting
fn extract_strings_parallel_with_progress(
    files: &[PathBuf],
    jobs: Option<usize>,
    progress_bar: Option<&ProgressBar>,
) -> Result<Vec<ExtractedString>> {
    use rayon::prelude::*;
    use std::sync::{Arc, Mutex};
    
    // Configure thread pool if specified
    if let Some(num_jobs) = jobs {
        let _ = rayon::ThreadPoolBuilder::new()
            .num_threads(num_jobs)
            .build_global();
    }
    
    // Create a shared counter for progress
    let processed = Arc::new(Mutex::new(0usize));
    
    // Process files in parallel with progress updates
    let results: std::result::Result<Vec<_>, _> = files
        .par_iter()
        .map(|file_path| {
            // Early termination for empty files
            if let Ok(metadata) = fs::metadata(file_path) {
                if metadata.len() == 0 {
                    // Update progress
                    if let Some(pb) = progress_bar {
                        let mut count = processed.lock().unwrap();
                        *count += 1;
                        pb.set_position(*count as u64);
                        pb.set_message(format!("Skipped empty: {}", 
                            file_path.file_name().unwrap_or_default().to_string_lossy()));
                    }
                    return Ok(Vec::new());
                }
            }
            
            let result = extract_strings_from_file(file_path);
            
            // Update progress
            if let Some(pb) = progress_bar {
                let mut count = processed.lock().unwrap();
                *count += 1;
                pb.set_position(*count as u64);
                pb.set_message(format!("Processing: {}", 
                    file_path.file_name().unwrap_or_default().to_string_lossy()));
            }
            
            result
        })
        .collect();
    
    // Flatten results and deduplicate efficiently
    let mut all_strings = Vec::new();
    let mut seen = std::collections::HashSet::new();
    
    for file_results in results? {
        for extracted in file_results {
            // Deduplicate at the source to save memory
            let key = (extracted.value.clone(), extracted.file_path.clone(), 
                       extracted.line, extracted.column);
            if seen.insert(key) {
                all_strings.push(extracted);
            }
        }
    }
    
    Ok(all_strings)
}

// Re-export chrono for timestamp generation
extern crate chrono;

/// Handle pipe command - read JavaScript from stdin, output CSS to stdout
pub async fn handle_pipe_command(args: PipeArgs) -> Result<()> {
    use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
    
    // Read JavaScript content from stdin asynchronously
    let mut input = String::new();
    let mut stdin = io::stdin();
    stdin.read_to_string(&mut input).await
        .map_err(|e| ExtractorError::InputError(format!("Failed to read from stdin: {}", e)))?;
    
    // If input is empty, output empty CSS
    if input.trim().is_empty() {
        return Ok(());
    }
    
    // Extract strings from the JavaScript content
    let extracted_strings = extract_strings_from_content(&input, "stdin")?;
    
    // Collect unique class names
    let mut unique_classes = std::collections::HashSet::new();
    for extracted in &extracted_strings {
        unique_classes.insert(extracted.value.clone());
    }
    
    // If no classes found, output empty CSS
    if unique_classes.is_empty() {
        return Ok(());
    }
    
    // Create a default Tailwind configuration
    let tailwind_config = TailwindConfig::default();
    
    // Create the Tailwind extractor with preflight configuration
    let mut extractor = TailwindExtractor::with_config_and_preflight(tailwind_config, args.no_preflight);
    
    // Add all extracted classes
    let classes_vec: Vec<String> = unique_classes.into_iter().collect();
    extractor.add_classes(&classes_vec, "stdin")?;
    
    // Generate CSS with optional minification
    let css_content = extractor.generate_css(args.minify)?;
    
    // Write CSS to stdout asynchronously
    let mut stdout = io::stdout();
    stdout.write_all(css_content.as_bytes()).await
        .map_err(|e| ExtractorError::OutputError {
            path: "stdout".to_string(),
            message: e.to_string(),
        })?;
    
    // Ensure output is flushed
    stdout.flush().await
        .map_err(|e| ExtractorError::OutputError {
            path: "stdout".to_string(),
            message: e.to_string(),
        })?;
    
    Ok(())
}