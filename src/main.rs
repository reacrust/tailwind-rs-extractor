use clap::Parser;
use tailwind_extractor::{Cli, Commands, extract, handle_pipe_command};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Parse command line arguments
    let cli = Cli::parse();

    // Handle commands
    match cli.command {
        Commands::Extract(args) => {
            // Run the extraction
            match extract(args).await {
                Ok(result) => {
                    println!("Extraction successful!");
                    println!("  - Processed {} files", result.total_files_processed);
                    println!("  - Extracted {} unique classes", result.total_classes);
                    Ok(())
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Pipe(args) => {
            // Handle pipe mode
            handle_pipe_command(args).await?;
            Ok(())
        }
    }
}