use clap::{Parser, Subcommand};
use anyhow::Result;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

mod commands;
mod config;
mod client;

use commands::{InitCommand, JoinCommand, UploadCommand, DownloadCommand, ListCommand, StatusCommand, Command};

#[derive(Parser)]
#[command(name = "zephyrfs")]
#[command(about = "A distributed P2P storage system")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(short, long, global = true)]
    verbose: bool,

    #[arg(short, long, global = true)]
    config: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new ZephyrFS node
    Init(InitCommand),
    
    /// Join an existing ZephyrFS network
    Join(JoinCommand),
    
    /// Upload a file to the network
    Upload(UploadCommand),
    
    /// Download a file from the network
    Download(DownloadCommand),
    
    /// List files in the network
    #[command(alias = "ls")]
    List(ListCommand),
    
    /// Show node status and network information
    Status(StatusCommand),
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Initialize logging
    let level = if cli.verbose { Level::DEBUG } else { Level::INFO };
    let subscriber = FmtSubscriber::builder()
        .with_max_level(level)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default subscriber failed");

    info!("Starting ZephyrFS CLI");

    // Load configuration
    let config_path = cli.config.as_deref();
    let config = config::Config::load(config_path)?;

    // Execute command
    match cli.command {
        Commands::Init(cmd) => cmd.execute(&config).await,
        Commands::Join(cmd) => cmd.execute(&config).await,
        Commands::Upload(cmd) => cmd.execute(&config).await,
        Commands::Download(cmd) => cmd.execute(&config).await,
        Commands::List(cmd) => cmd.execute(&config).await,
        Commands::Status(cmd) => cmd.execute(&config).await,
    }
}