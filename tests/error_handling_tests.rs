use std::fs;
use tempfile::TempDir;
use tailwind_extractor::{extract, ExtractArgs};

#[tokio::test]
async fn test_helpful_error_messages_for_parse_errors() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create a file with syntax error
    let bad_file = temp_dir.path().join("broken.jsx");
    fs::write(&bad_file, r#"
        export const Component = () => {
            return <div className="flex {{ broken syntax
    "#).unwrap();
    
    let args = ExtractArgs {
        input: vec![format!("{}/*.jsx", temp_dir.path().display())],
        output_css: temp_dir.path().join("output.css"),
        output_manifest: temp_dir.path().join("manifest.json"),
        config: None,
        obfuscate: false,
        minify: false,
        watch: false,
        verbose: false,
        jobs: None,
        exclude: vec![],
        dry_run: false,
    };
    
    let result = extract(args).await;
    assert!(result.is_err());
    
    if let Err(e) = result {
        let error_msg = format!("{}", e);
        // Should include the file path in the error
        assert!(error_msg.contains("broken.jsx"), 
                "Error message should contain file path: {}", error_msg);
        // Should mention it's a parse error
        assert!(error_msg.contains("parse") || error_msg.contains("Parse"),
                "Error message should indicate parse failure: {}", error_msg);
    }
}

#[tokio::test]
async fn test_error_message_for_no_files_found() {
    let temp_dir = TempDir::new().unwrap();
    
    // No files created - directory is empty
    
    let args = ExtractArgs {
        input: vec![format!("{}/*.jsx", temp_dir.path().display())],
        output_css: temp_dir.path().join("output.css"),
        output_manifest: temp_dir.path().join("manifest.json"),
        config: None,
        obfuscate: false,
        minify: false,
        watch: false,
        verbose: false,
        jobs: None,
        exclude: vec![],
        dry_run: false,
    };
    
    let result = extract(args).await;
    assert!(result.is_err());
    
    if let Err(e) = result {
        let error_msg = format!("{}", e);
        assert!(error_msg.contains("No files found"),
                "Error should clearly state no files were found: {}", error_msg);
    }
}

#[tokio::test]
async fn test_error_message_for_invalid_glob_pattern() {
    let temp_dir = TempDir::new().unwrap();
    
    let args = ExtractArgs {
        input: vec!["[invalid glob".to_string()], // Invalid glob pattern
        output_css: temp_dir.path().join("output.css"),
        output_manifest: temp_dir.path().join("manifest.json"),
        config: None,
        obfuscate: false,
        minify: false,
        watch: false,
        verbose: false,
        jobs: None,
        exclude: vec![],
        dry_run: false,
    };
    
    let result = extract(args).await;
    assert!(result.is_err());
    
    if let Err(e) = result {
        let error_msg = format!("{}", e);
        assert!(error_msg.contains("Pattern") || error_msg.contains("glob"),
                "Error should mention pattern/glob issue: {}", error_msg);
    }
}

#[tokio::test]
async fn test_error_message_for_write_permission_denied() {
    use std::os::unix::fs::PermissionsExt;
    
    let temp_dir = TempDir::new().unwrap();
    
    // Create a test file
    let test_file = temp_dir.path().join("test.jsx");
    fs::write(&test_file, r#"export const C = () => <div className="flex">Test</div>;"#).unwrap();
    
    // Create output directory with no write permissions
    let output_dir = temp_dir.path().join("no_write");
    fs::create_dir(&output_dir).unwrap();
    let mut perms = fs::metadata(&output_dir).unwrap().permissions();
    perms.set_mode(0o555); // Read and execute only
    fs::set_permissions(&output_dir, perms.clone()).unwrap();
    
    let args = ExtractArgs {
        input: vec![format!("{}/*.jsx", temp_dir.path().display())],
        output_css: output_dir.join("output.css"),
        output_manifest: output_dir.join("manifest.json"),
        config: None,
        obfuscate: false,
        minify: false,
        watch: false,
        verbose: false,
        jobs: None,
        exclude: vec![],
        dry_run: false,
    };
    
    let result = extract(args).await;
    
    // Restore permissions for cleanup
    perms.set_mode(0o755);
    fs::set_permissions(&output_dir, perms).unwrap();
    
    assert!(result.is_err());
    
    if let Err(e) = result {
        let error_msg = format!("{}", e);
        assert!(error_msg.contains("output") || error_msg.contains("Output") || 
                error_msg.contains("write") || error_msg.contains("Write") ||
                error_msg.contains("permission") || error_msg.contains("Permission"),
                "Error should indicate output/write issue: {}", error_msg);
    }
}

#[tokio::test]
async fn test_security_error_messages_are_clear() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create a file that's too large
    let large_file = temp_dir.path().join("huge.jsx");
    let huge_content = "a".repeat(11 * 1024 * 1024); // 11MB
    fs::write(&large_file, huge_content).unwrap();
    
    // Create a normal file too
    let normal_file = temp_dir.path().join("normal.jsx");
    fs::write(&normal_file, r#"export const C = () => <div className="p-4">Normal</div>;"#).unwrap();
    
    let args = ExtractArgs {
        input: vec![format!("{}/*.jsx", temp_dir.path().display())],
        output_css: temp_dir.path().join("output.css"),
        output_manifest: temp_dir.path().join("manifest.json"),
        config: None,
        obfuscate: false,
        minify: false,
        watch: false,
        verbose: true, // Enable to see security warnings
        jobs: None,
        exclude: vec![],
        dry_run: false,
    };
    
    // Should succeed but skip the large file with a warning
    let result = extract(args).await.unwrap();
    assert_eq!(result.total_files_processed, 1); // Only normal file
}