use clap::Args;
use anyhow::{Context, Result};
use chrono::{DateTime, Local};
use humansize::{format_size, BINARY};
use tracing::info;

use crate::config::Config;
use crate::client::ZephyrClient;
use super::Command;

#[derive(Debug, Args)]
pub struct ListCommand {
    /// Show detailed information
    #[arg(short, long)]
    long: bool,

    /// Sort by column (name, size, date, chunks)
    #[arg(short, long, default_value = "name")]
    sort: String,

    /// Reverse sort order
    #[arg(short, long)]
    reverse: bool,

    /// Filter by name pattern
    #[arg(short, long)]
    filter: Option<String>,

    /// Output format (table, json, csv)
    #[arg(long, default_value = "table")]
    format: String,
}

#[async_trait::async_trait]
impl Command for ListCommand {
    async fn execute(&self, config: &Config) -> Result<()> {
        info!("Listing files in ZephyrFS network");

        let client = ZephyrClient::new(config);
        let mut files = client.list_files().await
            .context("Failed to list files")?;

        // Apply filter if specified
        if let Some(filter) = &self.filter {
            files.retain(|f| f.name.contains(filter));
        }

        // Sort files
        match self.sort.as_str() {
            "name" => files.sort_by(|a, b| a.name.cmp(&b.name)),
            "size" => files.sort_by(|a, b| a.size.cmp(&b.size)),
            "date" => files.sort_by(|a, b| a.uploaded_at.cmp(&b.uploaded_at)),
            "chunks" => files.sort_by(|a, b| a.chunks.cmp(&b.chunks)),
            _ => anyhow::bail!("Invalid sort column: {}. Valid options: name, size, date, chunks", self.sort),
        }

        if self.reverse {
            files.reverse();
        }

        if files.is_empty() {
            println!("No files found in the network.");
            return Ok(());
        }

        match self.format.as_str() {
            "table" => self.print_table(&files),
            "json" => self.print_json(&files)?,
            "csv" => self.print_csv(&files),
            _ => anyhow::bail!("Invalid format: {}. Valid options: table, json, csv", self.format),
        }

        println!("\nTotal files: {}", files.len());
        let total_size: u64 = files.iter().map(|f| f.size).sum();
        println!("Total size: {}", format_size(total_size, BINARY));

        Ok(())
    }
}

impl ListCommand {
    fn print_table(&self, files: &[crate::client::FileInfo]) {
        if self.long {
            // Detailed view
            println!("{:<12} {:>10} {:>8} {:>8} {:<20} {}", 
                     "HASH", "SIZE", "CHUNKS", "REPLICAS", "UPLOADED", "NAME");
            println!("{}", "-".repeat(80));
            
            for file in files {
                let local_time: DateTime<Local> = file.uploaded_at.into();
                let hash_short = if file.hash.len() > 12 {
                    &file.hash[..12]
                } else {
                    &file.hash
                };
                
                println!("{:<12} {:>10} {:>8} {:>8} {:<20} {}", 
                         hash_short,
                         format_size(file.size, BINARY),
                         file.chunks,
                         file.replicas,
                         local_time.format("%Y-%m-%d %H:%M"),
                         file.name);
            }
        } else {
            // Simple view
            println!("{:<40} {:>10} {}", "NAME", "SIZE", "HASH");
            println!("{}", "-".repeat(60));
            
            for file in files {
                let hash_short = if file.hash.len() > 8 {
                    &file.hash[..8]
                } else {
                    &file.hash
                };
                
                println!("{:<40} {:>10} {}", 
                         if file.name.len() > 40 {
                             format!("{}...", &file.name[..37])
                         } else {
                             file.name.clone()
                         },
                         format_size(file.size, BINARY),
                         hash_short);
            }
        }
    }

    fn print_json(&self, files: &[crate::client::FileInfo]) -> Result<()> {
        let json = serde_json::to_string_pretty(files)
            .context("Failed to serialize files to JSON")?;
        println!("{}", json);
        Ok(())
    }

    fn print_csv(&self, files: &[crate::client::FileInfo]) {
        println!("name,size,hash,chunks,replicas,uploaded_at");
        for file in files {
            println!("{},{},{},{},{},{}",
                     file.name,
                     file.size,
                     file.hash,
                     file.chunks,
                     file.replicas,
                     file.uploaded_at.to_rfc3339());
        }
    }
}