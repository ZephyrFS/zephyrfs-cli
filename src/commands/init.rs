use clap::Args;
use anyhow::{Context, Result};
use dialoguer::{theme::ColorfulTheme, Input, Confirm};
use std::path::PathBuf;
use tracing::{info, warn};

use crate::config::Config;
use crate::client::ZephyrClient;
use super::Command;

#[derive(Debug, Args)]
pub struct InitCommand {
    /// Node name (optional)
    #[arg(short, long)]
    name: Option<String>,

    /// Data directory path
    #[arg(short, long)]
    data_dir: Option<PathBuf>,

    /// Listen port for the node
    #[arg(short, long)]
    port: Option<u16>,

    /// Maximum storage allocation in GB
    #[arg(short, long)]
    storage: Option<u64>,

    /// Skip interactive configuration
    #[arg(long)]
    no_interactive: bool,
}

#[async_trait::async_trait]
impl Command for InitCommand {
    async fn execute(&self, config: &Config) -> Result<()> {
        info!("Initializing ZephyrFS node...");

        let mut node_config = config.clone();

        // Interactive configuration if not skipped
        if !self.no_interactive {
            self.interactive_config(&mut node_config)?;
        } else {
            self.apply_args(&mut node_config)?;
        }

        // Ensure data directory exists
        node_config.ensure_data_dir()
            .context("Failed to create data directory")?;

        // Save configuration
        node_config.save(None)
            .context("Failed to save configuration")?;

        // Initialize the node
        let client = ZephyrClient::new(&node_config);
        client.initialize_node().await
            .context("Failed to initialize node")?;

        println!("✓ ZephyrFS node initialized successfully!");
        println!("  Node ID: {}", node_config.node.id.unwrap_or_else(|| "auto-generated".to_string()));
        println!("  Data directory: {:?}", node_config.node.data_dir);
        println!("  Listen port: {}", node_config.node.listen_port);
        println!("  Max storage: {} GB", node_config.storage.max_storage_gb);
        
        println!("\nNext steps:");
        println!("  1. Start the node: zephyrfs status");
        println!("  2. Join a network: zephyrfs join <bootstrap-peer>");

        Ok(())
    }
}

impl InitCommand {
    fn interactive_config(&self, config: &mut Config) -> Result<()> {
        let theme = ColorfulTheme::default();

        // Node name
        if let Some(name) = &self.name {
            config.node.name = Some(name.clone());
        } else {
            let name: String = Input::with_theme(&theme)
                .with_prompt("Node name (optional)")
                .default(config.node.name.clone().unwrap_or_else(|| "zephyr-node".to_string()))
                .allow_empty(true)
                .interact_text()?;
            
            config.node.name = if name.is_empty() { None } else { Some(name) };
        }

        // Data directory
        if let Some(data_dir) = &self.data_dir {
            config.node.data_dir = data_dir.clone();
        } else {
            let data_dir_str = Input::with_theme(&theme)
                .with_prompt("Data directory")
                .default(config.node.data_dir.to_string_lossy().to_string())
                .interact_text()?;
            
            config.node.data_dir = PathBuf::from(data_dir_str);
        }

        // Listen port
        if let Some(port) = self.port {
            config.node.listen_port = port;
        } else {
            config.node.listen_port = Input::with_theme(&theme)
                .with_prompt("Listen port")
                .default(config.node.listen_port)
                .interact()?;
        }

        // Storage allocation
        if let Some(storage) = self.storage {
            config.storage.max_storage_gb = storage;
        } else {
            config.storage.max_storage_gb = Input::with_theme(&theme)
                .with_prompt("Maximum storage allocation (GB)")
                .default(config.storage.max_storage_gb)
                .interact()?;
        }

        // Confirm settings
        println!("\nConfiguration summary:");
        println!("  Name: {}", config.node.name.as_deref().unwrap_or("auto-generated"));
        println!("  Data directory: {:?}", config.node.data_dir);
        println!("  Listen port: {}", config.node.listen_port);
        println!("  Max storage: {} GB", config.storage.max_storage_gb);

        let confirm = Confirm::with_theme(&theme)
            .with_prompt("Proceed with initialization?")
            .default(true)
            .interact()?;

        if !confirm {
            warn!("Initialization cancelled by user");
            std::process::exit(0);
        }

        Ok(())
    }

    fn apply_args(&self, config: &mut Config) -> Result<()> {
        if let Some(name) = &self.name {
            config.node.name = Some(name.clone());
        }

        if let Some(data_dir) = &self.data_dir {
            config.node.data_dir = data_dir.clone();
        }

        if let Some(port) = self.port {
            config.node.listen_port = port;
        }

        if let Some(storage) = self.storage {
            config.storage.max_storage_gb = storage;
        }

        Ok(())
    }
}