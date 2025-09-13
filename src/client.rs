use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Duration;
use tokio::fs;

use crate::config::Config;

#[derive(Debug, Clone)]
pub struct ZephyrClient {
    client: Client,
    coordinator_url: String,
    node_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeInfo {
    pub id: String,
    pub name: String,
    pub status: String,
    pub peers_connected: usize,
    pub storage_used_gb: f64,
    pub storage_available_gb: f64,
    pub uptime_seconds: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileInfo {
    pub name: String,
    pub size: u64,
    pub hash: String,
    pub uploaded_at: chrono::DateTime<chrono::Utc>,
    pub chunks: usize,
    pub replicas: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UploadRequest {
    pub file_path: String,
    pub file_name: String,
    pub file_size: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UploadResponse {
    pub success: bool,
    pub file_hash: String,
    pub chunks_uploaded: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DownloadRequest {
    pub file_hash: String,
    pub output_path: String,
}

impl ZephyrClient {
    pub fn new(config: &Config) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.coordinator.timeout))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            coordinator_url: config.coordinator.endpoint.clone(),
            node_url: format!("http://127.0.0.1:{}", config.node.listen_port),
        }
    }

    pub async fn get_node_status(&self) -> Result<NodeInfo> {
        let response = self
            .client
            .get(&format!("{}/api/status", self.node_url))
            .send()
            .await
            .context("Failed to connect to node")?;

        if !response.status().is_success() {
            anyhow::bail!("Node returned error: {}", response.status());
        }

        let node_info: NodeInfo = response
            .json()
            .await
            .context("Failed to parse node status response")?;

        Ok(node_info)
    }

    pub async fn list_files(&self) -> Result<Vec<FileInfo>> {
        let response = self
            .client
            .get(&format!("{}/api/files", self.node_url))
            .send()
            .await
            .context("Failed to connect to node")?;

        if !response.status().is_success() {
            anyhow::bail!("Node returned error: {}", response.status());
        }

        let files: Vec<FileInfo> = response
            .json()
            .await
            .context("Failed to parse files list response")?;

        Ok(files)
    }

    pub async fn upload_file(&self, file_path: &Path) -> Result<UploadResponse> {
        let metadata = fs::metadata(file_path).await
            .with_context(|| format!("Failed to read file metadata: {:?}", file_path))?;

        let file_name = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .context("Invalid file name")?;

        let request = UploadRequest {
            file_path: file_path.to_string_lossy().to_string(),
            file_name: file_name.to_string(),
            file_size: metadata.len(),
        };

        // Read file content
        let file_content = fs::read(file_path).await
            .with_context(|| format!("Failed to read file: {:?}", file_path))?;

        let response = self
            .client
            .post(&format!("{}/api/upload", self.node_url))
            .json(&request)
            .body(file_content)
            .send()
            .await
            .context("Failed to upload file to node")?;

        if !response.status().is_success() {
            anyhow::bail!("Upload failed: {}", response.status());
        }

        let upload_response: UploadResponse = response
            .json()
            .await
            .context("Failed to parse upload response")?;

        Ok(upload_response)
    }

    pub async fn download_file(&self, file_hash: &str, output_path: &Path) -> Result<()> {
        let request = DownloadRequest {
            file_hash: file_hash.to_string(),
            output_path: output_path.to_string_lossy().to_string(),
        };

        let response = self
            .client
            .post(&format!("{}/api/download", self.node_url))
            .json(&request)
            .send()
            .await
            .context("Failed to download file from node")?;

        if !response.status().is_success() {
            anyhow::bail!("Download failed: {}", response.status());
        }

        let file_content = response
            .bytes()
            .await
            .context("Failed to read download response")?;

        fs::write(output_path, file_content).await
            .with_context(|| format!("Failed to write file: {:?}", output_path))?;

        Ok(())
    }

    pub async fn join_network(&self, bootstrap_peer: &str) -> Result<()> {
        let response = self
            .client
            .post(&format!("{}/api/network/join", self.node_url))
            .json(&serde_json::json!({ "bootstrap_peer": bootstrap_peer }))
            .send()
            .await
            .context("Failed to join network")?;

        if !response.status().is_success() {
            anyhow::bail!("Failed to join network: {}", response.status());
        }

        Ok(())
    }

    pub async fn initialize_node(&self) -> Result<()> {
        let response = self
            .client
            .post(&format!("{}/api/node/init", self.node_url))
            .send()
            .await
            .context("Failed to initialize node")?;

        if !response.status().is_success() {
            anyhow::bail!("Failed to initialize node: {}", response.status());
        }

        Ok(())
    }
}