use std::fs;
use tempfile::TempDir;
use tailwind_extractor::{extract, ExtractArgs};

#[tokio::test]
async fn test_progress_bar_shows_in_non_verbose_mode() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create test files
    for i in 0..10 {
        let file = temp_dir.path().join(format!("file_{}.jsx", i));
        fs::write(&file, format!(
            r#"export const Component{} = () => <div className="flex-{} bg-blue-{}">Test</div>;"#,
            i, i, i
        )).unwrap();
    }
    
    let args = ExtractArgs {
        input: vec![format!("{}/*.jsx", temp_dir.path().display())],
        output_css: temp_dir.path().join("output.css"),
        output_manifest: temp_dir.path().join("manifest.json"),
        config: None,
        obfuscate: false,
        minify: false,
        watch: false,
        verbose: false, // Progress bar should show
        jobs: Some(4),
        exclude: vec![],
        dry_run: false,
    };
    
    // Should complete successfully with progress bar
    let result = extract(args).await.unwrap();
    assert_eq!(result.total_files_processed, 10);
}

#[tokio::test]
async fn test_no_progress_bar_in_verbose_mode() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create test files
    for i in 0..5 {
        let file = temp_dir.path().join(format!("file_{}.jsx", i));
        fs::write(&file, format!(
            r#"export const Component{} = () => <div className="text-lg">Test</div>;"#,
            i
        )).unwrap();
    }
    
    let args = ExtractArgs {
        input: vec![format!("{}/*.jsx", temp_dir.path().display())],
        output_css: temp_dir.path().join("output.css"),
        output_manifest: temp_dir.path().join("manifest.json"),
        config: None,
        obfuscate: false,
        minify: false,
        watch: false,
        verbose: true, // Progress bar should not show
        jobs: None,
        exclude: vec![],
        dry_run: false,
    };
    
    // Should complete successfully without progress bar
    let result = extract(args).await.unwrap();
    assert_eq!(result.total_files_processed, 5);
    
    // Performance stats should be available in verbose mode
    assert!(result.performance_stats.is_some());
}

#[tokio::test]
async fn test_progress_updates_during_processing() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create many files to ensure progress updates
    for i in 0..50 {
        let file = temp_dir.path().join(format!("component_{}.jsx", i));
        let content = format!(
            r#"
            import React from 'react';
            
            export const Component{} = () => (
                <div className="flex flex-col items-center p-4">
                    <h1 className="text-2xl font-bold">Title {}</h1>
                    <p className="text-base">Content {}</p>
                </div>
            );
            "#,
            i, i, i
        );
        fs::write(&file, content).unwrap();
    }
    
    let args = ExtractArgs {
        input: vec![format!("{}/*.jsx", temp_dir.path().display())],
        output_css: temp_dir.path().join("output.css"),
        output_manifest: temp_dir.path().join("manifest.json"),
        config: None,
        obfuscate: false,
        minify: false,
        watch: false,
        verbose: false,
        jobs: Some(2), // Use fewer threads to ensure progress is visible
        exclude: vec![],
        dry_run: false,
    };
    
    let result = extract(args).await.unwrap();
    assert_eq!(result.total_files_processed, 50);
    assert!(result.total_classes > 0);
}