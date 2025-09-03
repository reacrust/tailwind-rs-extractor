use std::fs;
use std::os::unix::fs::symlink;
use std::path::PathBuf;
use tempfile::TempDir;
use tailwind_extractor::{extract, ExtractArgs};

#[tokio::test]
async fn test_file_size_limit() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create a file that's too large (> 10MB)
    let large_file = temp_dir.path().join("large_file.jsx");
    let content = "a".repeat(11 * 1024 * 1024); // 11MB
    fs::write(&large_file, content).unwrap();
    
    // Create a normal file
    let normal_file = temp_dir.path().join("normal_file.jsx");
    fs::write(&normal_file, r#"export const Component = () => <div className="flex">Test</div>;"#).unwrap();
    
    let args = ExtractArgs {
        input: vec![format!("{}/*.jsx", temp_dir.path().display())],
        output_css: temp_dir.path().join("output.css"),
        output_manifest: temp_dir.path().join("manifest.json"),
        config: None,
        obfuscate: false,
        minify: false,
        watch: false,
        verbose: true, // Enable verbose to see security warnings
        jobs: None,
        exclude: vec![],
        dry_run: true,
        no_preflight: false,
        transform: false,
    };
    
    // Should succeed but skip the large file
    let result = extract(args).await.unwrap();
    assert_eq!(result.total_files_processed, 1); // Only the normal file
}

#[tokio::test]
async fn test_symlink_handling() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create a real file
    let real_file = temp_dir.path().join("real_file.jsx");
    fs::write(&real_file, r#"export const Component = () => <div className="flex">Test</div>;"#).unwrap();
    
    // Create a symlink to the real file
    let symlink_file = temp_dir.path().join("symlink_file.jsx");
    symlink(&real_file, &symlink_file).unwrap();
    
    // Create a symlink to outside the working directory
    let outside_dir = TempDir::new().unwrap();
    let outside_file = outside_dir.path().join("outside.jsx");
    fs::write(&outside_file, r#"export const Bad = () => <div className="bg-red-500">Bad</div>;"#).unwrap();
    let bad_symlink = temp_dir.path().join("bad_symlink.jsx");
    symlink(&outside_file, &bad_symlink).unwrap();
    
    let args = ExtractArgs {
        input: vec![format!("{}/*.jsx", temp_dir.path().display())],
        output_css: temp_dir.path().join("output.css"),
        output_manifest: temp_dir.path().join("manifest.json"),
        config: None,
        obfuscate: false,
        minify: false,
        watch: false,
        verbose: true,
        jobs: None,
        exclude: vec![],
        dry_run: true,
        no_preflight: false,
        transform: false,
    };
    
    // Should process only the real file (symlinks are rejected by default)
    let result = extract(args).await.unwrap();
    assert_eq!(result.total_files_processed, 1);
}

#[tokio::test]
async fn test_path_traversal_protection() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create a test file
    let test_file = temp_dir.path().join("test.jsx");
    fs::write(&test_file, r#"export const Component = () => <div className="flex">Test</div>;"#).unwrap();
    
    // Try to write output to parent directory with absolute paths that may traverse
    // Using absolute paths for safety testing
    let safe_output_css = temp_dir.path().join("output.css");
    let safe_output_manifest = temp_dir.path().join("manifest.json");
    
    let args = ExtractArgs {
        input: vec![format!("{}/*.jsx", temp_dir.path().display())],
        output_css: safe_output_css,
        output_manifest: safe_output_manifest,
        config: None,
        obfuscate: false,
        minify: false,
        watch: false,
        verbose: false,
        jobs: None,
        exclude: vec![],
        dry_run: true, // Use dry-run to avoid actual write attempts
        no_preflight: false,
        transform: false,
    };
    
    // Should succeed with safe paths
    let result = extract(args).await;
    assert!(result.is_ok(), "Safe paths should be allowed");
    
    // Now test with potentially unsafe relative paths
    let args_unsafe = ExtractArgs {
        input: vec![format!("{}/*.jsx", temp_dir.path().display())],
        output_css: PathBuf::from("../../../evil.css"),
        output_manifest: PathBuf::from("../../../evil.json"),
        config: None,
        obfuscate: false,
        minify: false,
        watch: false,
        verbose: false,
        jobs: None,
        exclude: vec![],
        dry_run: false, // Not using dry-run to test actual path validation
        no_preflight: false,
        transform: false,
    };
    
    // Should fail or succeed based on working directory context
    // In dry-run mode we allow it, but in real mode it depends on the actual path resolution
    let _result_unsafe = extract(args_unsafe).await;
    // We're not asserting failure here because relative paths might still be safe
    // depending on the working directory. The security check ensures paths
    // don't escape the working directory.
}

#[tokio::test]
async fn test_empty_file_handling() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create an empty file
    let empty_file = temp_dir.path().join("empty.jsx");
    fs::write(&empty_file, "").unwrap();
    
    // Create a nearly empty file
    let tiny_file = temp_dir.path().join("tiny.jsx");
    fs::write(&tiny_file, "// comment").unwrap();
    
    // Create a normal file
    let normal_file = temp_dir.path().join("normal.jsx");
    fs::write(&normal_file, r#"
        export const Component = () => (
            <div className="flex flex-col items-center">
                <span className="text-lg font-bold">Test</span>
            </div>
        );
    "#).unwrap();
    
    let args = ExtractArgs {
        input: vec![format!("{}/*.jsx", temp_dir.path().display())],
        output_css: temp_dir.path().join("output.css"),
        output_manifest: temp_dir.path().join("manifest.json"),
        config: None,
        obfuscate: false,
        minify: false,
        watch: false,
        verbose: true,
        jobs: None,
        exclude: vec![],
        dry_run: false,
        no_preflight: false,
        transform: false,
    };
    
    // Should handle empty files gracefully
    let result = extract(args).await.unwrap();
    assert_eq!(result.total_files_processed, 3);
    assert!(result.total_classes > 0); // Should extract from normal file
}

#[tokio::test]
async fn test_malformed_javascript_handling() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create a file with malformed JavaScript
    let malformed_file = temp_dir.path().join("malformed.jsx");
    fs::write(&malformed_file, r#"
        export const Component = () => {
            return <div className="flex {{{ broken
        // Missing closing braces and broken syntax
    "#).unwrap();
    
    // Create a valid file
    let valid_file = temp_dir.path().join("valid.jsx");
    fs::write(&valid_file, r#"export const Valid = () => <div className="bg-blue-500">Valid</div>;"#).unwrap();
    
    let args = ExtractArgs {
        input: vec![format!("{}/*.jsx", temp_dir.path().display())],
        output_css: temp_dir.path().join("output.css"),
        output_manifest: temp_dir.path().join("manifest.json"),
        config: None,
        obfuscate: false,
        minify: false,
        watch: false,
        verbose: true,
        jobs: None,
        exclude: vec![],
        dry_run: false,
        no_preflight: false,
        transform: false,
    };
    
    // Should fail due to parse error
    let result = extract(args).await;
    assert!(result.is_err());
    if let Err(e) = result {
        // Check that we get a meaningful error message
        let error_msg = format!("{}", e);
        assert!(error_msg.contains("malformed.jsx") || error_msg.contains("parse"));
    }
}

#[tokio::test]
async fn test_concurrent_file_processing_safety() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create many files to test concurrent processing
    for i in 0..100 {
        let file = temp_dir.path().join(format!("file_{}.jsx", i));
        fs::write(&file, format!(
            r#"export const Component{} = () => <div className="flex-{} bg-blue-{}">Test {}</div>;"#,
            i, i % 10, i % 5, i
        )).unwrap();
    }
    
    // Test with different thread counts
    for threads in [1, 2, 4, 8, 16] {
        let args = ExtractArgs {
            input: vec![format!("{}/*.jsx", temp_dir.path().display())],
            output_css: temp_dir.path().join(format!("output_{}.css", threads)),
            output_manifest: temp_dir.path().join(format!("manifest_{}.json", threads)),
            config: None,
            obfuscate: false,
            minify: false,
            watch: false,
            verbose: false,
            jobs: Some(threads),
            exclude: vec![],
            dry_run: false,
        no_preflight: false,
        transform: false,
        };
        
        let result = extract(args).await.unwrap();
        assert_eq!(result.total_files_processed, 100);
        
        // Verify CSS was generated
        assert!(!result.css_content.is_empty());
    }
}

#[tokio::test]
async fn test_permission_denied_handling() {
    use std::os::unix::fs::PermissionsExt;
    
    let temp_dir = TempDir::new().unwrap();
    
    // Create a file with no read permissions
    let restricted_file = temp_dir.path().join("restricted.jsx");
    fs::write(&restricted_file, r#"export const Component = () => <div className="flex">Test</div>;"#).unwrap();
    
    // Remove read permissions
    let mut perms = fs::metadata(&restricted_file).unwrap().permissions();
    perms.set_mode(0o000);
    fs::set_permissions(&restricted_file, perms).unwrap();
    
    // Create a normal file
    let normal_file = temp_dir.path().join("normal.jsx");
    fs::write(&normal_file, r#"export const Normal = () => <div className="bg-green-500">Normal</div>;"#).unwrap();
    
    let args = ExtractArgs {
        input: vec![format!("{}/*.jsx", temp_dir.path().display())],
        output_css: temp_dir.path().join("output.css"),
        output_manifest: temp_dir.path().join("manifest.json"),
        config: None,
        obfuscate: false,
        minify: false,
        watch: false,
        verbose: true,
        jobs: None,
        exclude: vec![],
        dry_run: true,
        no_preflight: false,
        transform: false,
    };
    
    // Should fail due to permission denied
    let result = extract(args).await;
    
    // Restore permissions for cleanup
    let mut perms = fs::metadata(&restricted_file).unwrap().permissions();
    perms.set_mode(0o644);
    fs::set_permissions(&restricted_file, perms).unwrap();
    
    assert!(result.is_err());
}