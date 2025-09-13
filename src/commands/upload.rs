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
pub struct UploadCommand {
    /// File path to upload
    file: PathBuf,

    /// Custom file name (defaults to original filename)
    #[arg(short, long)]
    name: Option<String>,

    /// Show progress bar
    #[arg(long, default_value = "true")]
    progress: bool,

    /// Verify upload after completion
    #[arg(long)]
    verify: bool,
}

#[async_trait::async_trait]
impl Command for UploadCommand {
    async fn execute(&self, config: &Config) -> Result<()> {
        info!("Uploading file: {:?}", self.file);

        // Validate file exists and is readable
        if !self.file.exists() {
            anyhow::bail!("File does not exist: {:?}", self.file);
        }

        if !self.file.is_file() {
            anyhow::bail!("Path is not a file: {:?}", self.file);
        }

        let metadata = tokio::fs::metadata(&self.file).await
            .with_context(|| format!("Failed to read file metadata: {:?}", self.file))?;

        let file_size = metadata.len();
        let display_name = self.name.as_deref()
            .or_else(|| self.file.file_name().and_then(|n| n.to_str()))
            .unwrap_or("unknown");

        println!("Uploading: {}", display_name);
        println!("  Size: {}", format_size(file_size, BINARY));
        println!("  Path: {:?}", self.file);

        // Create progress bar
        let progress = if self.progress {
            let pb = ProgressBar::new(file_size);
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

        let client = ZephyrClient::new(config);

        // Upload the file
        let upload_result = client.upload_file(&self.file).await
            .context("Failed to upload file")?;

        if let Some(pb) = progress {
            pb.finish_with_message("Upload complete");
        }

        println!("✓ File uploaded successfully!");
        println!("  File hash: {}", upload_result.file_hash);
        println!("  Chunks: {}", upload_result.chunks_uploaded);
        
        // Verify upload if requested
        if self.verify {
            println!("\nVerifying upload...");
            let files = client.list_files().await
                .context("Failed to list files for verification")?;
            
            if files.iter().any(|f| f.hash == upload_result.file_hash) {
                println!("✓ Upload verified successfully");
            } else {
                println!("⚠  Warning: Could not verify upload in file list");
            }
        }

        println!("\nTo download this file later, use:");
        println!("  zephyrfs download {}", upload_result.file_hash);

        Ok(())
    }
}