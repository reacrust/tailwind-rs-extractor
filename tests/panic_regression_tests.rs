//! Regression tests to ensure tailwind-rs vendor code doesn't panic on invalid input
//! These tests verify that invalid/unknown Tailwind classes are gracefully handled
//! instead of causing panics.

use std::process::{Command, Stdio};
use std::io::Write;
use tempfile::tempdir;
use std::fs;

#[test]
fn test_invalid_flex_values_no_panic() {
    // These used to cause panics in flexbox parsing
    let test_cases = vec![
        "flex-invalid-value",
        "flex-unknown",
        "flex-bad-123",
        "content-invalid",
        "content-unknown-value",
    ];
    
    for test_class in test_cases {
        let input = format!("const x = \"{}\"", test_class);
        let result = run_extractor_pipe(&input);
        
        assert!(result.success || result.stderr.contains("Warning"), 
            "Class '{}' should either succeed or produce a warning, not panic. stderr: {}", 
            test_class, result.stderr);
        assert!(!result.stderr.contains("panicked at"), 
            "Class '{}' caused a panic: {}", test_class, result.stderr);
    }
}

#[test]
fn test_invalid_typography_values_no_panic() {
    // These used to cause panics in typography parsing
    let test_cases = vec![
        "text-overflow-bad",
        "text-overflow-invalid",
        "break-invalid",
        "break-unknown-pattern",
    ];
    
    for test_class in test_cases {
        let input = format!("const x = \"{}\"", test_class);
        let result = run_extractor_pipe(&input);
        
        assert!(result.success || result.stderr.contains("Warning"), 
            "Class '{}' should either succeed or produce a warning, not panic. stderr: {}", 
            test_class, result.stderr);
        assert!(!result.stderr.contains("panicked at"), 
            "Class '{}' caused a panic: {}", test_class, result.stderr);
    }
}

#[test]
fn test_mixed_valid_and_invalid_classes() {
    // Ensure that invalid classes don't prevent valid ones from being processed
    let input = r#"
        const Component = () => {
            return <div className="
                bg-blue-500 
                flex-invalid 
                text-white 
                content-unknown 
                p-4 
                break-bad 
                hover:bg-blue-600
            "></div>
        }
    "#;
    
    let result = run_extractor_pipe(input);
    
    // Should not panic
    assert!(!result.stderr.contains("panicked at"), 
        "Mixed classes caused a panic: {}", result.stderr);
    
    // Should still process valid classes
    assert!(result.stdout.contains("bg-blue-500"),
        "Valid class bg-blue-500 was not processed");
    assert!(result.stdout.contains("text-white") || result.stdout.contains("FFFFFF"),
        "Valid class text-white was not processed");
}

#[test]
fn test_complex_invalid_patterns() {
    // Test more complex invalid patterns that might trigger edge cases
    let test_cases = vec![
        "flex-[invalid-arbitrary-value]",
        "content-['invalid quoted']",
        "grid-cols-invalid-12",
        "break-after-invalid-page",
    ];
    
    for test_class in test_cases {
        let input = format!("const x = \"{}\"", test_class);
        let result = run_extractor_pipe(&input);
        
        assert!(!result.stderr.contains("panicked at"), 
            "Class '{}' caused a panic: {}", test_class, result.stderr);
    }
}

#[test]  
fn test_file_mode_with_invalid_classes() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test.js");
    
    let content = r#"
        export const Button = () => (
            <button className="
                px-4 py-2 
                flex-invalid-value 
                bg-primary 
                content-unknown
                hover:opacity-90
            ">
                Click me
            </button>
        );
    "#;
    
    fs::write(&file_path, content).unwrap();
    
    let output = Command::new(env!("CARGO_BIN_EXE_tailwind-extractor-cli"))
        .arg("file")
        .arg(&file_path)
        .output()
        .expect("Failed to execute tailwind-extractor-cli");
    
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Should not panic
    assert!(!stderr.contains("panicked at"), 
        "File mode caused a panic: {}", stderr);
    
    // Should process valid classes
    assert!(stdout.contains("px-4") || stdout.contains(".px-4"),
        "Valid classes were not processed in file mode");
}

// Helper function to run the extractor in pipe mode
fn run_extractor_pipe(input: &str) -> ExtractorResult {
    let mut child = Command::new(env!("CARGO_BIN_EXE_tailwind-extractor-cli"))
        .arg("pipe")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn tailwind-extractor-cli");
    
    // Write input to stdin
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(input.as_bytes()).unwrap();
    }
    
    let output = child.wait_with_output().expect("Failed to wait for output");
    
    ExtractorResult {
        success: output.status.success(),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    }
}

struct ExtractorResult {
    success: bool,
    stdout: String,
    stderr: String,
}
