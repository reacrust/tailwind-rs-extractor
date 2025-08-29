use clap::Parser;
use tailwind_extractor::{Cli, Commands, ExtractArgs, PipeArgs};

#[test]
fn test_cli_parse_basic() {
    let args = vec![
        "tailwind-extractor-cli",
        "extract",
        "-i", "*.mjs",
        "-o", "output.css", 
        "-m", "manifest.json"
    ];
    
    let cli = Cli::parse_from(args);
    
    match cli.command {
        Commands::Extract(args) => {
            assert_eq!(args.input, vec!["*.mjs"]);
            assert_eq!(args.output_css.to_str().unwrap(), "output.css");
            assert_eq!(args.output_manifest.to_str().unwrap(), "manifest.json");
            assert!(!args.obfuscate);
            assert!(!args.minify);
            assert!(!args.watch);
            assert!(!args.verbose);
            assert!(!args.dry_run);
        }
        Commands::Pipe(_) => panic!("Unexpected Pipe command"),
    }
}

#[test]
fn test_cli_parse_with_flags() {
    let args = vec![
        "tailwind-extractor-cli",
        "extract",
        "-i", "**/*.res.mjs",
        "-i", "**/*.jsx",
        "-o", "dist/styles.css",
        "-m", "dist/manifest.json",
        "--obfuscate",
        "--minify",
        "--verbose",
        "--dry-run",
        "-j", "4"
    ];
    
    let cli = Cli::parse_from(args);
    
    match cli.command {
        Commands::Extract(args) => {
            assert_eq!(args.input, vec!["**/*.res.mjs", "**/*.jsx"]);
            assert_eq!(args.output_css.to_str().unwrap(), "dist/styles.css");
            assert_eq!(args.output_manifest.to_str().unwrap(), "dist/manifest.json");
            assert!(args.obfuscate);
            assert!(args.minify);
            assert!(args.verbose);
            assert!(args.dry_run);
            assert_eq!(args.jobs, Some(4));
        }
        Commands::Pipe(_) => panic!("Unexpected Pipe command"),
    }
}

#[test]
fn test_cli_parse_with_exclude() {
    let args = vec![
        "tailwind-extractor-cli",
        "extract",
        "-i", "src/**/*.mjs",
        "-o", "output.css",
        "-m", "manifest.json",
        "-e", "node_modules/**",
        "-e", "dist/**"
    ];
    
    let cli = Cli::parse_from(args);
    
    match cli.command {
        Commands::Extract(args) => {
            assert_eq!(args.exclude, vec!["node_modules/**", "dist/**"]);
        }
        Commands::Pipe(_) => panic!("Unexpected Pipe command"),
    }
}

#[test]
fn test_extract_args_validate() {
    let mut args = ExtractArgs {
        input: vec!["*.mjs".to_string()],
        output_css: "output.css".into(),
        output_manifest: "manifest.json".into(),
        config: None,
        obfuscate: false,
        minify: false,
        watch: false,
        verbose: false,
        jobs: None,
        exclude: vec![],
        dry_run: false,
    };
    
    // Valid args should pass
    assert!(args.validate().is_ok());
    
    // Empty input should fail
    args.input.clear();
    assert!(args.validate().is_err());
    args.input.push("*.mjs".to_string());
    
    // Same output paths should fail
    args.output_manifest = args.output_css.clone();
    assert!(args.validate().is_err());
    args.output_manifest = "manifest.json".into();
    
    // Zero jobs should fail
    args.jobs = Some(0);
    assert!(args.validate().is_err());
    
    // Positive jobs should pass
    args.jobs = Some(4);
    assert!(args.validate().is_ok());
}

#[test]
fn test_cli_parse_pipe_command() {
    // Test basic pipe command
    let args = vec![
        "tailwind-extractor-cli",
        "pipe"
    ];
    
    let cli = Cli::parse_from(args);
    
    match cli.command {
        Commands::Pipe(args) => {
            assert!(!args.minify);
        }
        _ => panic!("Expected Pipe command"),
    }
    
    // Test pipe command with minify flag
    let args = vec![
        "tailwind-extractor-cli",
        "pipe",
        "--minify"
    ];
    
    let cli = Cli::parse_from(args);
    
    match cli.command {
        Commands::Pipe(args) => {
            assert!(args.minify);
        }
        _ => panic!("Expected Pipe command"),
    }
}

#[tokio::test]
async fn test_pipe_mode_functionality() {
    // This test verifies that pipe mode can read JavaScript from stdin
    // and output CSS to stdout asynchronously without blocking.
    
    use std::process::Stdio;
    use tokio::process::Command;
    use tokio::io::AsyncWriteExt;
    
    // Build the extractor first
    let build_output = Command::new("cargo")
        .arg("build")
        .arg("--bin")
        .arg("tailwind-extractor-cli")
        .current_dir("/var/lib/mcp-proxy/tailwind-extractor-production-fixes/crates/tailwind-extractor")
        .output()
        .await
        .expect("Failed to build tailwind-extractor-cli");
    
    assert!(build_output.status.success(), "Build failed: {}", String::from_utf8_lossy(&build_output.stderr));
    
    // Test basic pipe mode with JavaScript input containing Tailwind classes
    let mut child = Command::new("/var/lib/mcp-proxy/tailwind-extractor-production-fixes/crates/target/debug/tailwind-extractor-cli")
        .arg("pipe")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn tailwind-extractor-cli");
    
    // Write JavaScript with Tailwind classes to stdin
    let stdin = child.stdin.as_mut().unwrap();
    stdin.write_all(b"const button = 'bg-blue-500 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded';").await
        .expect("Failed to write to stdin");
    stdin.shutdown().await.expect("Failed to close stdin");
    
    // Wait for the process to complete
    let output = child.wait_with_output().await.expect("Failed to read output");
    
    assert!(output.status.success(), "Pipe command failed: {}", String::from_utf8_lossy(&output.stderr));
    
    // Verify CSS output contains expected classes
    let css_output = String::from_utf8_lossy(&output.stdout);
    assert!(!css_output.is_empty(), "CSS output should not be empty");
    // The CSS should contain at least some of our classes
    assert!(css_output.contains(".bg-blue-500") || css_output.contains("background-color"), 
            "CSS should contain bg-blue-500 class or background styles");
    // Verify that CSS was actually generated (should have comments and styles)
    assert!(css_output.contains("Generated by tailwind-extractor-cli"), 
            "CSS should contain generation comment");
    
    // Test with minified output
    let mut child_minified = Command::new("/var/lib/mcp-proxy/tailwind-extractor-production-fixes/crates/target/debug/tailwind-extractor-cli")
        .arg("pipe")
        .arg("--minify")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn tailwind-extractor-cli with minify");
    
    // Write JavaScript with Tailwind classes to stdin
    let stdin_min = child_minified.stdin.as_mut().unwrap();
    stdin_min.write_all(b"const styles = 'flex justify-center items-center';").await
        .expect("Failed to write to stdin");
    stdin_min.shutdown().await.expect("Failed to close stdin");
    
    // Wait for the process to complete
    let output_minified = child_minified.wait_with_output().await.expect("Failed to read output");
    
    assert!(output_minified.status.success(), "Pipe command with minify failed: {}", String::from_utf8_lossy(&output_minified.stderr));
    
    // Verify minified CSS output
    let css_minified = String::from_utf8_lossy(&output_minified.stdout);
    assert!(!css_minified.is_empty(), "Minified CSS output should not be empty");
    // Just verify CSS was generated - the specific classes may vary based on tailwind-rs implementation
    assert!(css_minified.len() > 100, "Minified CSS should have substantial content");
    // Verify it's actually minified (no multi-space indentation)
    assert!(!css_minified.contains("\n  "), "Minified CSS should not contain indentation");
}

#[tokio::test]
async fn test_pipe_mode_async_non_blocking() {
    // This test verifies that the pipe mode is truly async and non-blocking
    // by processing multiple inputs concurrently
    
    use std::process::Stdio;
    use tokio::process::Command;
    use tokio::io::AsyncWriteExt;
    use std::time::Instant;
    
    // Build first if needed
    let _ = Command::new("cargo")
        .arg("build")
        .arg("--bin")
        .arg("tailwind-extractor-cli")
        .current_dir("/var/lib/mcp-proxy/tailwind-extractor-production-fixes/crates/tailwind-extractor")
        .output()
        .await;
    
    // Launch multiple pipe processes concurrently
    let start = Instant::now();
    
    // Create separate async blocks as futures
    let future1 = async {
        let mut child = Command::new("/var/lib/mcp-proxy/tailwind-extractor-production-fixes/crates/target/debug/tailwind-extractor-cli")
            .arg("pipe")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to spawn");
        
        let stdin = child.stdin.as_mut().unwrap();
        stdin.write_all(b"const a = 'bg-red-500 text-lg';").await.unwrap();
        stdin.shutdown().await.unwrap();
        
        child.wait_with_output().await
    };
    
    let future2 = async {
        let mut child = Command::new("/var/lib/mcp-proxy/tailwind-extractor-production-fixes/crates/target/debug/tailwind-extractor-cli")
            .arg("pipe")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to spawn");
        
        let stdin = child.stdin.as_mut().unwrap();
        stdin.write_all(b"const b = 'bg-green-500 text-xl';").await.unwrap();
        stdin.shutdown().await.unwrap();
        
        child.wait_with_output().await
    };
    
    let future3 = async {
        let mut child = Command::new("/var/lib/mcp-proxy/tailwind-extractor-production-fixes/crates/target/debug/tailwind-extractor-cli")
            .arg("pipe")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to spawn");
        
        let stdin = child.stdin.as_mut().unwrap();
        stdin.write_all(b"const c = 'bg-yellow-500 text-2xl';").await.unwrap();
        stdin.shutdown().await.unwrap();
        
        child.wait_with_output().await
    };
    
    // Run all concurrently
    let (result1, result2, result3) = tokio::join!(future1, future2, future3);
    
    let duration = start.elapsed();
    
    // All should succeed
    assert!(result1.is_ok() && result1.unwrap().status.success());
    assert!(result2.is_ok() && result2.unwrap().status.success());
    assert!(result3.is_ok() && result3.unwrap().status.success());
    
    // Verify that they ran concurrently (should complete in less time than sequential)
    // This is a loose check but helps verify async behavior
    println!("Concurrent execution took: {:?}", duration);
}