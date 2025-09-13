use clap::Args;
use anyhow::{Context, Result};
use tracing::info;

use crate::config::Config;
use crate::client::ZephyrClient;
use super::Command;

#[derive(Debug, Args)]
pub struct JoinCommand {
    /// Bootstrap peer address (host:port)
    bootstrap_peer: String,

    /// Timeout in seconds
    #[arg(short, long, default_value = "30")]
    timeout: u64,

    /// Skip connectivity test
    #[arg(long)]
    skip_test: bool,
}

#[async_trait::async_trait]
impl Command for JoinCommand {
    async fn execute(&self, config: &Config) -> Result<()> {
        info!("Joining ZephyrFS network via bootstrap peer: {}", self.bootstrap_peer);

        let client = ZephyrClient::new(config);

        // Test connectivity to bootstrap peer if not skipped
        if !self.skip_test {
            println!("Testing connectivity to bootstrap peer...");
            // TODO: Implement ping/connectivity test
        }

        // Join the network
        println!("Joining network...");
        client.join_network(&self.bootstrap_peer).await
            .context("Failed to join network")?;

        // Verify connection
        println!("Verifying connection...");
        let node_info = client.get_node_status().await
            .context("Failed to get node status after joining")?;

        println!("✓ Successfully joined ZephyrFS network!");
        println!("  Bootstrap peer: {}", self.bootstrap_peer);
        println!("  Connected peers: {}", node_info.peers_connected);
        println!("  Node status: {}", node_info.status);

        if node_info.peers_connected == 0 {
            println!("\n⚠  Warning: No peers connected yet. This might be expected for a new network.");
        }

        Ok(())
    }
}