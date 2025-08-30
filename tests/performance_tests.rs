use std::fs;
use std::time::{Duration, Instant};
use tempfile::TempDir;
use tailwind_extractor::{extract, ExtractArgs};

#[tokio::test]
async fn test_performance_1000_files_under_10_seconds() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create 1000 test files
    let content = r#"
        import React from 'react';
        
        export const Component = () => (
            <div className="flex flex-col items-center justify-center p-4 bg-blue-500 text-white">
                <h1 className="text-2xl font-bold mb-4">Title</h1>
                <p className="text-base leading-relaxed">Content</p>
                <button className="px-4 py-2 bg-green-500 hover:bg-green-600 rounded-lg transition-colors">
                    Click Me
                </button>
            </div>
        );
    "#;
    
    for i in 0..1000 {
        let file = temp_dir.path().join(format!("component_{}.jsx", i));
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
        jobs: Some(8), // Use parallel processing
        exclude: vec![],
        dry_run: false,
        no_preflight: false,
        transform: false,
    };
    
    let start = Instant::now();
    let result = extract(args).await.unwrap();
    let duration = start.elapsed();
    
    // Assert performance requirements
    assert_eq!(result.total_files_processed, 1000);
    assert!(duration < Duration::from_secs(10), 
            "Processing 1000 files took {:?}, expected < 10s", duration);
    
    // Check performance stats are available
    assert!(result.performance_stats.is_some());
    if let Some(stats) = result.performance_stats {
        assert!(stats.files_per_second > 100.0, 
                "Expected > 100 files/sec, got {:.1}", stats.files_per_second);
    }
}

#[tokio::test]
async fn test_performance_stats_accuracy() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create test files
    for i in 0..10 {
        let file = temp_dir.path().join(format!("file_{}.jsx", i));
        let content = format!(
            r#"export const Component{} = () => <div className="class-{}">Test</div>;"#, 
            i, i
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
        verbose: true, // Enable verbose to get stats
        jobs: Some(4),
        exclude: vec![],
        dry_run: false,
        no_preflight: false,
        transform: false,
    };
    
    let result = extract(args).await.unwrap();
    
    assert!(result.performance_stats.is_some());
    let stats = result.performance_stats.unwrap();
    
    // Verify stats are reasonable
    assert!(stats.total_duration > Duration::from_millis(1));
    assert!(stats.extraction_duration > Duration::from_millis(0));
    assert!(stats.css_generation_duration > Duration::from_millis(0));
    assert!(stats.files_per_second > 0.0);
    assert!(stats.bytes_processed > 0);
}

#[tokio::test]
async fn test_early_termination_empty_files() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create many empty files
    for i in 0..100 {
        let file = temp_dir.path().join(format!("empty_{}.jsx", i));
        fs::write(&file, "").unwrap();
    }
    
    // Create a few files with content
    for i in 0..5 {
        let file = temp_dir.path().join(format!("content_{}.jsx", i));
        fs::write(&file, format!(
            r#"export const C{} = () => <div className="flex bg-blue-500">Test</div>;"#, i
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
        verbose: false,
        jobs: Some(4),
        exclude: vec![],
        dry_run: false,
        no_preflight: false,
        transform: false,
    };
    
    let start = Instant::now();
    let result = extract(args).await.unwrap();
    let duration = start.elapsed();
    
    // Should process all files but terminate early for empty ones
    assert_eq!(result.total_files_processed, 105);
    assert!(duration < Duration::from_secs(2), 
            "Early termination should make processing fast, took {:?}", duration);
}

#[tokio::test]
async fn test_memory_efficient_deduplication() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create files with many duplicate classes
    let content = r#"
        export const Component = () => (
            <div className="flex flex flex flex flex">
                <span className="text-lg text-lg text-lg">
                    <button className="bg-blue-500 bg-blue-500 bg-blue-500">Test</button>
                </span>
            </div>
        );
    "#;
    
    for i in 0..100 {
        let file = temp_dir.path().join(format!("duplicate_{}.jsx", i));
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
        verbose: true,
        jobs: Some(4),
        exclude: vec![],
        dry_run: false,
        no_preflight: false,
        transform: false,
    };
    
    let result = extract(args).await.unwrap();
    
    // Should have deduplicated classes efficiently
    assert_eq!(result.total_files_processed, 100);
    assert!(result.total_classes <= 10, 
            "Expected deduplicated classes, got {}", result.total_classes);
}

#[tokio::test]
async fn test_parallel_vs_sequential_performance() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create test files
    for i in 0..50 {
        let file = temp_dir.path().join(format!("file_{}.jsx", i));
        fs::write(&file, format!(
            r#"
            export const Component{} = () => (
                <div className="flex-{} items-center-{} justify-{} p-{} m-{}">
                    <span className="text-{} font-{} leading-{}">Content</span>
                </div>
            );
            "#,
            i, i % 10, i % 5, i % 3, i % 4, i % 2,
            i % 6, i % 7, i % 8
        )).unwrap();
    }
    
    // Test with single thread
    let args_single = ExtractArgs {
        input: vec![format!("{}/*.jsx", temp_dir.path().display())],
        output_css: temp_dir.path().join("output_single.css"),
        output_manifest: temp_dir.path().join("manifest_single.json"),
        config: None,
        obfuscate: false,
        minify: false,
        watch: false,
        verbose: false,
        jobs: Some(1),
        exclude: vec![],
        dry_run: true,
        no_preflight: false,
        transform: false,
    };
    
    let start_single = Instant::now();
    let result_single = extract(args_single).await.unwrap();
    let duration_single = start_single.elapsed();
    
    // Test with multiple threads
    let args_multi = ExtractArgs {
        input: vec![format!("{}/*.jsx", temp_dir.path().display())],
        output_css: temp_dir.path().join("output_multi.css"),
        output_manifest: temp_dir.path().join("manifest_multi.json"),
        config: None,
        obfuscate: false,
        minify: false,
        watch: false,
        verbose: false,
        jobs: Some(4),
        exclude: vec![],
        dry_run: true,
        no_preflight: false,
        transform: false,
    };
    
    let start_multi = Instant::now();
    let result_multi = extract(args_multi).await.unwrap();
    let duration_multi = start_multi.elapsed();
    
    // Multi-threaded should be faster (allowing some margin for small file counts)
    assert!(duration_multi <= duration_single || 
            duration_multi < duration_single + Duration::from_millis(100),
            "Multi-threaded ({:?}) should be faster than single-threaded ({:?})",
            duration_multi, duration_single);
    
    // Both should produce the same results
    assert_eq!(result_single.total_files_processed, result_multi.total_files_processed);
    assert_eq!(result_single.total_classes, result_multi.total_classes);
}