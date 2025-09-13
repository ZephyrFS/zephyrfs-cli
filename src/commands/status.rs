use clap::Args;
use anyhow::{Context, Result};
use humansize::{format_size, BINARY};
use std::time::Duration;
use tracing::info;

use crate::config::Config;
use crate::client::ZephyrClient;
use super::Command;

#[derive(Debug, Args)]
pub struct StatusCommand {
    /// Show detailed status information
    #[arg(short, long)]
    detailed: bool,

    /// Output format (table, json)
    #[arg(long, default_value = "table")]
    format: String,

    /// Refresh interval in seconds (0 for single check)
    #[arg(short, long, default_value = "0")]
    watch: u64,
}

#[async_trait::async_trait]
impl Command for StatusCommand {
    async fn execute(&self, config: &Config) -> Result<()> {
        info!("Getting ZephyrFS node status");

        if self.watch > 0 {
            self.watch_status(config).await
        } else {
            self.show_status(config).await
        }
    }
}

impl StatusCommand {
    async fn show_status(&self, config: &Config) -> Result<()> {
        let client = ZephyrClient::new(config);
        
        // Get node status
        let node_info = client.get_node_status().await
            .context("Failed to get node status. Is the node running?")?;

        match self.format.as_str() {
            "table" => self.print_table_status(&node_info, config),
            "json" => self.print_json_status(&node_info)?,
            _ => anyhow::bail!("Invalid format: {}. Valid options: table, json", self.format),
        }

        Ok(())
    }

    async fn watch_status(&self, config: &Config) -> Result<()> {
        println!("Watching node status (refresh every {} seconds)...", self.watch);
        println!("Press Ctrl+C to stop\n");

        loop {
            // Clear screen
            print!("\x1B[2J\x1B[1;1H");
            
            match self.show_status(config).await {
                Ok(()) => {},
                Err(e) => println!("Error getting status: {}", e),
            }

            tokio::time::sleep(Duration::from_secs(self.watch)).await;
        }
    }

    fn print_table_status(&self, node_info: &crate::client::NodeInfo, config: &Config) {
        println!("ZephyrFS Node Status");
        println!("{}", "=".repeat(50));
        println!();

        // Basic info
        println!("Node Information:");
        println!("  ID: {}", node_info.id);
        println!("  Name: {}", node_info.name);
        println!("  Status: {}", self.format_status(&node_info.status));
        println!();

        // Network info
        println!("Network:");
        println!("  Connected peers: {}", node_info.peers_connected);
        println!("  Listen port: {}", config.node.listen_port);
        println!("  Coordinator: {}", config.coordinator.endpoint);
        println!();

        // Storage info
        let used_gb = node_info.storage_used_gb;
        let available_gb = node_info.storage_available_gb;
        let total_gb = used_gb + available_gb;
        let usage_percent = if total_gb > 0.0 { (used_gb / total_gb) * 100.0 } else { 0.0 };

        println!("Storage:");
        println!("  Used: {} ({:.1}%)", format_size((used_gb * 1e9) as u64, BINARY), usage_percent);
        println!("  Available: {}", format_size((available_gb * 1e9) as u64, BINARY));
        println!("  Total allocated: {}", format_size((total_gb * 1e9) as u64, BINARY));
        println!("  Max allocation: {} GB", config.storage.max_storage_gb);
        println!();

        // Runtime info
        let uptime = Duration::from_secs(node_info.uptime_seconds);
        println!("Runtime:");
        println!("  Uptime: {}", self.format_duration(uptime));
        println!("  Data directory: {:?}", config.node.data_dir);
        
        if self.detailed {
            println!();
            println!("Configuration:");
            println!("  Chunk size: {} MB", config.storage.chunk_size_mb);
            println!("  Replication factor: {}", config.storage.replication_factor);
            println!("  Max connections: {}", config.network.max_connections);
            println!("  Connection timeout: {}s", config.network.connection_timeout);
        }
    }

    fn print_json_status(&self, node_info: &crate::client::NodeInfo) -> Result<()> {
        let json = serde_json::to_string_pretty(node_info)
            .context("Failed to serialize node info to JSON")?;
        println!("{}", json);
        Ok(())
    }

    fn format_status(&self, status: &str) -> String {
        match status.to_lowercase().as_str() {
            "running" => "🟢 Running".to_string(),
            "starting" => "🟡 Starting".to_string(),
            "stopping" => "🟡 Stopping".to_string(),
            "stopped" => "🔴 Stopped".to_string(),
            "error" => "❌ Error".to_string(),
            _ => format!("❓ {}", status),
        }
    }

    fn format_duration(&self, duration: Duration) -> String {
        let secs = duration.as_secs();
        
        if secs < 60 {
            format!("{}s", secs)
        } else if secs < 3600 {
            format!("{}m {}s", secs / 60, secs % 60)
        } else if secs < 86400 {
            let hours = secs / 3600;
            let mins = (secs % 3600) / 60;
            format!("{}h {}m", hours, mins)
        } else {
            let days = secs / 86400;
            let hours = (secs % 86400) / 3600;
            format!("{}d {}h", days, hours)
        }
    }
}