use tailwind_extractor::{ExtractArgs, extract};
use tempfile::tempdir;
use std::fs;

#[tokio::test]
async fn test_end_to_end_css_generation() {
    // Create a temporary directory for test files
    let temp_dir = tempdir().unwrap();
    
    // Create test JavaScript file with Tailwind classes
    let js_file = temp_dir.path().join("test.js");
    fs::write(&js_file, r##"
        import React from 'react';
        
        const Component = () => {
            return (
                <div className="bg-blue-500 p-4 text-white hover:bg-blue-600">
                    <h1 className="text-2xl font-bold">Hello World</h1>
                    <button className="px-4 py-2 rounded bg-red-500">Click me</button>
                </div>
            );
        };
        
        const styles = {
            container: "flex flex-col gap-4",
            card: "shadow-lg rounded-lg p-6",
        };
    "##).unwrap();
    
    // Create test ReScript output file
    let res_file = temp_dir.path().join("test.res.mjs");
    fs::write(&res_file, r##"
        let className = "border border-gray-300 m-2 p-2"
        let dynamicClass = `bg-${color}-500 text-${size}`
        let Component = () => {
            <div className="grid grid-cols-3 gap-4">
                <span className="text-sm text-gray-600" />
            </div>
        }
    "##).unwrap();
    
    // Create output paths
    let output_css = temp_dir.path().join("output.css");
    let output_manifest = temp_dir.path().join("manifest.json");
    
    // Create extraction arguments
    let args = ExtractArgs {
        input: vec![
            format!("{}/*.js", temp_dir.path().display()),
            format!("{}/*.mjs", temp_dir.path().display()),
        ],
        exclude: vec![],
        output_css: output_css.clone(),
        output_manifest: output_manifest.clone(),
        config: None,
        obfuscate: false,
        minify: false,
        watch: false,
        verbose: false,
        dry_run: false,
        jobs: None,
    };
    
    // Run extraction
    let result = extract(args).await.unwrap();
    
    // Verify results
    assert!(result.total_files_processed > 0);
    assert!(result.total_classes > 0);
    assert!(!result.css_content.is_empty());
    
    // Check that CSS was written
    assert!(output_css.exists());
    let css_content = fs::read_to_string(&output_css).unwrap();
    assert!(!css_content.is_empty());
    
    // Check manifest was written
    assert!(output_manifest.exists());
    let manifest_content = fs::read_to_string(&output_manifest).unwrap();
    let manifest: serde_json::Value = serde_json::from_str(&manifest_content).unwrap();
    
    assert!(manifest["metadata"]["classes_extracted"].as_u64().unwrap() > 0);
}

#[tokio::test]
async fn test_minification() {
    // Create a temporary directory for test files
    let temp_dir = tempdir().unwrap();
    
    // Create test file
    let js_file = temp_dir.path().join("test.js");
    fs::write(&js_file, r##"
        const className = "p-4 m-4 bg-white text-black";
    "##).unwrap();
    
    // Create output paths
    let output_css = temp_dir.path().join("output.css");
    let output_manifest = temp_dir.path().join("manifest.json");
    
    // Create extraction arguments with minification
    let args = ExtractArgs {
        input: vec![format!("{}/*.js", temp_dir.path().display())],
        exclude: vec![],
        output_css: output_css.clone(),
        output_manifest: output_manifest.clone(),
        config: None,
        obfuscate: false,
        minify: true,
        watch: false,
        verbose: false,
        dry_run: false,
        jobs: None,
    };
    
    // Run extraction
    let result = extract(args).await.unwrap();
    
    // Verify CSS is minified (no unnecessary whitespace)
    assert!(!result.css_content.contains("\n    "));
    assert!(output_css.exists());
}