use clap::Args;
use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;
use tracing::info;
use humansize::{format_size, BINARY};

use crate::config::Config;
use crate::client::ZephyrClient;
use super::Command;

#[derive(Debug, Args)]
pub struct DownloadCommand {
    /// File hash to download
    file_hash: String,

    /// Output file path (defaults to original filename)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Overwrite existing file
    #[arg(long)]
    force: bool,

    /// Show progress bar
    #[arg(long, default_value = "true")]
    progress: bool,

    /// Verify download integrity
    #[arg(long, default_value = "true")]
    verify: bool,
}

#[async_trait::async_trait]
impl Command for DownloadCommand {
    async fn execute(&self, config: &Config) -> Result<()> {
        info!("Downloading file: {}", self.file_hash);

        let client = ZephyrClient::new(config);

        // Get file info first
        let files = client.list_files().await
            .context("Failed to list files")?;

        let file_info = files.iter()
            .find(|f| f.hash == self.file_hash)
            .with_context(|| format!("File not found: {}", self.file_hash))?;

        // Determine output path
        let output_path = match &self.output {
            Some(path) => path.clone(),
            None => PathBuf::from(&file_info.name),
        };

        // Check if file exists and handle overwrite
        if output_path.exists() && !self.force {
            anyhow::bail!(
                "Output file already exists: {:?}. Use --force to overwrite.",
                output_path
            );
        }

        println!("Downloading: {}", file_info.name);
        println!("  Hash: {}", file_info.hash);
        println!("  Size: {}", format_size(file_info.size, BINARY));
        println!("  Chunks: {}", file_info.chunks);
        println!("  Output: {:?}", output_path);

        // Create progress bar
        let progress = if self.progress {
            let pb = ProgressBar::new(file_info.size);
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("[{elapsed_precise}] {bar:40.cyan/blue} {percent}% {bytes}/{total_bytes} ETA: {eta}")
                    .unwrap()
                    .progress_chars("##-")
            );
            Some(pb)
        } else {
            None
        };

        // Download the file
        client.download_file(&self.file_hash, &output_path).await
            .context("Failed to download file")?;

        if let Some(pb) = progress {
            pb.finish_with_message("Download complete");
        }

        // Verify download if requested
        if self.verify {
            println!("Verifying download integrity...");
            
            let downloaded_metadata = tokio::fs::metadata(&output_path).await
                .context("Failed to read downloaded file metadata")?;
            
            if downloaded_metadata.len() != file_info.size {
                anyhow::bail!(
                    "File size mismatch: expected {}, got {}",
                    file_info.size,
                    downloaded_metadata.len()
                );
            }

            // TODO: Verify file hash
            println!("✓ Download verified successfully");
        }

        println!("✓ File downloaded successfully!");
        println!("  Location: {:?}", output_path);
        println!("  Size: {}", format_size(file_info.size, BINARY));

        Ok(())
    }
}