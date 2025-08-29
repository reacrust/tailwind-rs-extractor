use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Tailwind CSS Extraction CLI - Extracts Tailwind classes from compiled ReScript files
#[derive(Parser, Debug)]
#[command(name = "tailwind-extractor-cli")]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Extract Tailwind CSS classes from source files
    Extract(ExtractArgs),
    /// Process JavaScript content from stdin and output CSS to stdout
    Pipe(PipeArgs),
}

/// Arguments for the extract command
#[derive(Parser, Debug, Clone)]
pub struct ExtractArgs {
    /// Input file patterns (glob patterns supported)
    #[arg(
        short = 'i',
        long = "input",
        value_name = "PATTERN",
        required = true,
        num_args = 1..,
        help = "Input file patterns to scan for Tailwind classes"
    )]
    pub input: Vec<String>,

    /// Output CSS file path
    #[arg(
        short = 'o',
        long = "output-css",
        value_name = "PATH",
        required = true,
        help = "Path where the generated CSS file will be written"
    )]
    pub output_css: PathBuf,

    /// Output manifest file path (JSON)
    #[arg(
        short = 'm',
        long = "output-manifest",
        value_name = "PATH",
        required = true,
        help = "Path where the JSON manifest will be written"
    )]
    pub output_manifest: PathBuf,

    /// Configuration file path (YAML)
    #[arg(
        short = 'c',
        long = "config",
        value_name = "PATH",
        help = "Path to configuration file (YAML format)"
    )]
    pub config: Option<PathBuf>,

    /// Enable class name obfuscation
    #[arg(
        long = "obfuscate",
        default_value_t = false,
        help = "Enable obfuscation of Tailwind class names"
    )]
    pub obfuscate: bool,

    /// Enable CSS minification
    #[arg(
        long = "minify",
        default_value_t = false,
        help = "Enable minification of the output CSS"
    )]
    pub minify: bool,

    /// Watch mode (continuously watch for changes)
    #[arg(
        short = 'w',
        long = "watch",
        default_value_t = false,
        help = "Watch for file changes and re-extract automatically"
    )]
    pub watch: bool,

    /// Verbose output
    #[arg(
        short = 'v',
        long = "verbose",
        default_value_t = false,
        help = "Enable verbose output"
    )]
    pub verbose: bool,

    /// Number of parallel threads to use
    #[arg(
        short = 'j',
        long = "jobs",
        value_name = "NUM",
        help = "Number of parallel threads to use (defaults to number of CPU cores)"
    )]
    pub jobs: Option<usize>,

    /// Exclude patterns (glob patterns to exclude)
    #[arg(
        short = 'e',
        long = "exclude",
        value_name = "PATTERN",
        num_args = 0..,
        help = "Patterns to exclude from scanning"
    )]
    pub exclude: Vec<String>,

    /// Dry run (don't write output files)
    #[arg(
        long = "dry-run",
        default_value_t = false,
        help = "Perform extraction but don't write output files"
    )]
    pub dry_run: bool,

    /// Disable preflight CSS generation
    #[arg(
        long = "no-preflight",
        default_value_t = false,
        help = "Disable generation of Tailwind preflight/reset CSS"
    )]
    pub no_preflight: bool,
}

/// Arguments for the pipe command
#[derive(Parser, Debug, Clone)]
pub struct PipeArgs {
    /// Enable CSS minification
    #[arg(
        long = "minify",
        default_value_t = false,
        help = "Enable minification of the output CSS"
    )]
    pub minify: bool,

    /// Disable preflight CSS generation
    #[arg(
        long = "no-preflight",
        default_value_t = false,
        help = "Disable generation of Tailwind preflight/reset CSS"
    )]
    pub no_preflight: bool,
}

impl ExtractArgs {
    /// Validate that the arguments are consistent
    pub fn validate(&self) -> Result<(), String> {
        // Check that input patterns are not empty
        if self.input.is_empty() {
            return Err("At least one input pattern must be provided".to_string());
        }

        // Check that output paths are not the same
        if self.output_css == self.output_manifest {
            return Err("Output CSS and manifest paths must be different".to_string());
        }

        // Validate number of jobs if specified
        if let Some(jobs) = self.jobs {
            if jobs == 0 {
                return Err("Number of jobs must be at least 1".to_string());
            }
        }

        Ok(())
    }
}