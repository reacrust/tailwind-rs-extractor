use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use tailwind_extractor::{extract, ExtractArgs};

/// Create test files for benchmarking
fn create_test_files(dir: &Path, count: usize, size: &str) -> Vec<PathBuf> {
    let mut files = Vec::new();
    
    let content = match size {
        "small" => {
            // Small file with 10 Tailwind classes
            r#"
            import React from 'react';
            
            export const Component = () => (
                <div className="flex flex-col items-center justify-center p-4 bg-blue-500 text-white rounded-lg shadow-md hover:bg-blue-600">
                    Hello World
                </div>
            );
            "#
        }
        "medium" => {
            // Medium file with 50 Tailwind classes
            let base = r#"<div className="flex flex-col items-center justify-center p-4 bg-blue-500 text-white rounded-lg shadow-md hover:bg-blue-600">"#;
            let mut content = String::from("import React from 'react';\n\n");
            for i in 0..5 {
                content.push_str(&format!("export const Component{} = () => (\n", i));
                content.push_str("  <div>\n");
                for _ in 0..5 {
                    content.push_str(&format!("    {}\n", base));
                }
                content.push_str("  </div>\n);\n\n");
            }
            content
        }
        "large" => {
            // Large file with 500 Tailwind classes
            let classes = vec![
                "flex", "flex-col", "flex-row", "items-center", "justify-center",
                "p-4", "m-2", "bg-blue-500", "text-white", "rounded-lg",
                "shadow-md", "hover:bg-blue-600", "focus:outline-none", "transition-all",
                "duration-300", "ease-in-out", "transform", "hover:scale-105",
                "grid", "grid-cols-3", "gap-4", "space-x-2", "space-y-4"
            ];
            
            let mut content = String::from("import React from 'react';\n\n");
            for i in 0..20 {
                content.push_str(&format!("export const Component{} = () => (\n", i));
                content.push_str("  <div>\n");
                for j in 0..25 {
                    let class_list = classes.iter()
                        .cycle()
                        .skip(j % classes.len())
                        .take(10)
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(" ");
                    content.push_str(&format!("    <div className=\"{}\">\n", class_list));
                    content.push_str(&format!("      Content {}-{}\n", i, j));
                    content.push_str("    </div>\n");
                }
                content.push_str("  </div>\n);\n\n");
            }
            content
        }
        _ => panic!("Unknown size: {}", size),
    };
    
    for i in 0..count {
        let file_path = dir.join(format!("test_file_{}.jsx", i));
        fs::write(&file_path, &content).unwrap();
        files.push(file_path);
    }
    
    files
}

fn benchmark_extraction(c: &mut Criterion) {
    let mut group = c.benchmark_group("extraction");
    group.sample_size(10); // Reduce sample size for faster benchmarking
    
    // Benchmark different file counts with medium-sized files
    for count in [10, 50, 100, 500, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::new("file_count", count),
            count,
            |b, &count| {
                b.iter_with_setup(
                    || {
                        let temp_dir = TempDir::new().unwrap();
                        let files = create_test_files(temp_dir.path(), count, "medium");
                        
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
                            dry_run: true, // Don't write files in benchmarks
                        };
                        (temp_dir, args)
                    },
                    |(temp_dir, args)| {
                        let rt = tokio::runtime::Runtime::new().unwrap();
                        rt.block_on(async {
                            extract(args).await.unwrap()
                        });
                        black_box(temp_dir); // Keep temp_dir alive
                    }
                );
            },
        );
    }
    
    // Benchmark different file sizes with fixed count
    for size in ["small", "medium", "large"].iter() {
        group.bench_with_input(
            BenchmarkId::new("file_size", size),
            size,
            |b, &size| {
                b.iter_with_setup(
                    || {
                        let temp_dir = TempDir::new().unwrap();
                        let files = create_test_files(temp_dir.path(), 100, size);
                        
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
                            dry_run: true,
                        };
                        (temp_dir, args)
                    },
                    |(temp_dir, args)| {
                        let rt = tokio::runtime::Runtime::new().unwrap();
                        rt.block_on(async {
                            extract(args).await.unwrap()
                        });
                        black_box(temp_dir);
                    }
                );
            },
        );
    }
    
    group.finish();
}

fn benchmark_parallel_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("parallel_processing");
    group.sample_size(10);
    
    // Benchmark different thread counts
    for threads in [1, 2, 4, 8, 16].iter() {
        group.bench_with_input(
            BenchmarkId::new("threads", threads),
            threads,
            |b, &threads| {
                b.iter_with_setup(
                    || {
                        let temp_dir = TempDir::new().unwrap();
                        let files = create_test_files(temp_dir.path(), 200, "medium");
                        
                        let args = ExtractArgs {
                            input: vec![format!("{}/*.jsx", temp_dir.path().display())],
                            output_css: temp_dir.path().join("output.css"),
                            output_manifest: temp_dir.path().join("manifest.json"),
                            config: None,
                            obfuscate: false,
                            minify: false,
                            watch: false,
                            verbose: false,
                            jobs: Some(threads),
                            exclude: vec![],
                            dry_run: true,
                        };
                        (temp_dir, args)
                    },
                    |(temp_dir, args)| {
                        let rt = tokio::runtime::Runtime::new().unwrap();
                        rt.block_on(async {
                            extract(args).await.unwrap()
                        });
                        black_box(temp_dir);
                    }
                );
            },
        );
    }
    
    group.finish();
}

criterion_group!(benches, benchmark_extraction, benchmark_parallel_processing);
criterion_main!(benches);