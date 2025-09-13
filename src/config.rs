use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub node: NodeConfig,
    pub network: NetworkConfig,
    pub storage: StorageConfig,
    pub coordinator: CoordinatorConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    pub id: Option<String>,
    pub name: Option<String>,
    pub data_dir: PathBuf,
    pub listen_port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub bootstrap_peers: Vec<String>,
    pub max_connections: usize,
    pub connection_timeout: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub max_storage_gb: u64,
    pub chunk_size_mb: u32,
    pub replication_factor: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinatorConfig {
    pub endpoint: String,
    pub timeout: u64,
    pub retry_attempts: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            node: NodeConfig {
                id: None,
                name: None,
                data_dir: dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join(".zephyrfs"),
                listen_port: 8080,
            },
            network: NetworkConfig {
                bootstrap_peers: vec![
                    "127.0.0.1:8081".to_string(),
                    "127.0.0.1:8082".to_string(),
                ],
                max_connections: 50,
                connection_timeout: 30,
            },
            storage: StorageConfig {
                max_storage_gb: 10,
                chunk_size_mb: 1,
                replication_factor: 3,
            },
            coordinator: CoordinatorConfig {
                endpoint: "http://127.0.0.1:9000".to_string(),
                timeout: 30,
                retry_attempts: 3,
            },
        }
    }
}

impl Config {
    pub fn load(config_path: Option<&str>) -> Result<Self> {
        let path = match config_path {
            Some(p) => PathBuf::from(p),
            None => Self::default_config_path()?,
        };

        if !path.exists() {
            tracing::info!("Config file not found, using defaults: {:?}", path);
            return Ok(Self::default());
        }

        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config file: {:?}", path))?;

        let config: Config = if path.extension().and_then(|s| s.to_str()) == Some("toml") {
            toml::from_str(&content)
                .with_context(|| format!("Failed to parse TOML config: {:?}", path))?
        } else {
            serde_yaml::from_str(&content)
                .with_context(|| format!("Failed to parse YAML config: {:?}", path))?
        };

        tracing::info!("Loaded config from: {:?}", path);
        Ok(config)
    }

    pub fn save(&self, config_path: Option<&str>) -> Result<()> {
        let path = match config_path {
            Some(p) => PathBuf::from(p),
            None => Self::default_config_path()?,
        };

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config directory: {:?}", parent))?;
        }

        let content = if path.extension().and_then(|s| s.to_str()) == Some("toml") {
            toml::to_string_pretty(self)
                .context("Failed to serialize config to TOML")?
        } else {
            serde_yaml::to_string(self)
                .context("Failed to serialize config to YAML")?
        };

        fs::write(&path, content)
            .with_context(|| format!("Failed to write config file: {:?}", path))?;

        tracing::info!("Saved config to: {:?}", path);
        Ok(())
    }

    fn default_config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .or_else(|| dirs::home_dir().map(|h| h.join(".config")))
            .context("Could not determine config directory")?;
        
        Ok(config_dir.join("zephyrfs").join("config.yaml"))
    }

    pub fn ensure_data_dir(&self) -> Result<()> {
        fs::create_dir_all(&self.node.data_dir)
            .with_context(|| format!("Failed to create data directory: {:?}", self.node.data_dir))?;
        Ok(())
    }
}