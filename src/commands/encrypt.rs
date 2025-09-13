use clap::Args;
use anyhow::{Context, Result};
use dialoguer::{theme::ColorfulTheme, Password};
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;
use tracing::info;
use humansize::{format_size, BINARY};

use crate::config::Config;
use super::Command;

#[derive(Debug, Args)]
pub struct EncryptCommand {
    /// File to encrypt
    input: PathBuf,

    /// Output file path (defaults to input.zfs)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Password for encryption (will prompt if not provided)
    #[arg(short, long)]
    password: Option<String>,

    /// Show progress bar
    #[arg(long, default_value = "true")]
    progress: bool,

    /// Verification hash algorithm (blake3, sha256, both)
    #[arg(long, default_value = "both")]
    hash: String,

    /// Chunk size in MB for large files
    #[arg(long, default_value = "1")]
    chunk_size: u32,
}

#[async_trait::async_trait]
impl Command for EncryptCommand {
    async fn execute(&self, _config: &Config) -> Result<()> {
        info!("Encrypting file: {:?}", self.input);

        // Validate input file
        if !self.input.exists() {
            anyhow::bail!("File does not exist: {:?}", self.input);
        }

        if !self.input.is_file() {
            anyhow::bail!("Path is not a file: {:?}", self.input);
        }

        let metadata = tokio::fs::metadata(&self.input).await
            .with_context(|| format!("Failed to read file metadata: {:?}", self.input))?;

        let file_size = metadata.len();
        let display_name = self.input.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        // Determine output path
        let output_path = self.output.clone().unwrap_or_else(|| {
            let mut output = self.input.clone();
            let current_ext = output.extension()
                .and_then(|s| s.to_str())
                .unwrap_or("");
            if current_ext.is_empty() {
                output.set_extension("zfs");
            } else {
                output.set_extension(format!("{}.zfs", current_ext));
            }
            output
        });

        // Check if output exists
        if output_path.exists() {
            anyhow::bail!("Output file already exists: {:?}. Remove it first or specify a different output.", output_path);
        }

        println!("Encrypting: {}", display_name);
        println!("  Size: {}", format_size(file_size, BINARY));
        println!("  Output: {:?}", output_path);

        // Get password
        let password = if let Some(ref pw) = self.password {
            pw.clone()
        } else {
            let theme = ColorfulTheme::default();
            Password::with_theme(&theme)
                .with_prompt("Enter password for encryption")
                .interact()
                .context("Failed to read password")?
        };

        // Validate password strength
        if password.len() < 8 {
            anyhow::bail!("Password must be at least 8 characters long");
        }

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

        // Read file data
        let file_data = tokio::fs::read(&self.input).await
            .with_context(|| format!("Failed to read file: {:?}", self.input))?;

        // Initialize crypto system
        let crypto = create_crypto_system(&password, self.chunk_size)?;

        // Encrypt file
        let encrypted_data = crypto.encrypt_file(&file_data)
            .context("Failed to encrypt file")?;

        // Convert to CLI format
        let encrypted_segments: Vec<EncryptedSegment> = encrypted_data
            .into_iter()
            .map(|data| data.into())
            .collect();

        // Generate content verification
        let content_id = crypto.content_id(&file_data);
        
        // Create encrypted file format
        let encrypted_file = EncryptedFileFormat {
            version: 1,
            filename: display_name.to_string(),
            original_size: file_size,
            chunk_size_mb: self.chunk_size,
            content_id: content_id.to_hex(),
            hash_algorithm: self.hash.clone(),
            encrypted_segments,
            created_at: chrono::Utc::now(),
        };

        // Serialize and write encrypted file
        let serialized = serde_json::to_vec_pretty(&encrypted_file)
            .context("Failed to serialize encrypted file")?;

        tokio::fs::write(&output_path, &serialized).await
            .with_context(|| format!("Failed to write encrypted file: {:?}", output_path))?;

        if let Some(pb) = progress {
            pb.finish_with_message("Encryption complete");
        }

        println!("✅ File encrypted successfully!");
        println!("  Encrypted file: {:?}", output_path);
        println!("  Content ID: {}", encrypted_file.content_id);
        println!("  Segments: {}", encrypted_file.encrypted_segments.len());
        
        // Security reminder
        println!();
        println!("🔒 Security Notes:");
        println!("  • Keep your password safe - it cannot be recovered");
        println!("  • The encrypted file contains no plaintext metadata");
        println!("  • Use 'zephyrfs decrypt' to restore the original file");

        Ok(())
    }
}

// Encrypted file format for storage
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct EncryptedFileFormat {
    version: u32,
    filename: String,
    original_size: u64,
    chunk_size_mb: u32,
    content_id: String,
    hash_algorithm: String,
    encrypted_segments: Vec<EncryptedSegment>,
    created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct EncryptedSegment {
    segment_index: u64,
    ciphertext: Vec<u8>,
    nonce: [u8; 12],
    aad: Vec<u8>,
    key_path: Vec<u32>,
}

// Helper function to create crypto system
fn create_crypto_system(password: &str, chunk_size_mb: u32) -> Result<ZephyrCrypto> {
    use zephyrfs_node::crypto::{ZephyrCrypto, CryptoParams, ScryptParams, AesParams, HashParams, ContentHasher, VerificationHasher};
    
    // Create crypto parameters
    let params = CryptoParams {
        scrypt_params: ScryptParams {
            log_n: 17,          // Strong security
            r: 8,
            p: 1,
            output_len: 64,
        },
        aes_params: AesParams {
            key_len: 32,
            nonce_len: 12,
            tag_len: 16,
        },
        hash_params: HashParams {
            content_hasher: ContentHasher::Blake3,
            verification_hasher: VerificationHasher::Blake3,
        },
    };

    let mut crypto = ZephyrCrypto::with_params(params);
    crypto.init_from_password(password)
        .context("Failed to initialize crypto system with password")?;

    Ok(crypto)
}

// Convert between zephyrfs_node types and CLI types
impl From<zephyrfs_node::crypto::EncryptedData> for EncryptedSegment {
    fn from(data: zephyrfs_node::crypto::EncryptedData) -> Self {
        Self {
            segment_index: data.segment_index,
            ciphertext: data.ciphertext,
            nonce: data.nonce,
            aad: data.aad,
            key_path: data.key_path,
        }
    }
}

impl From<EncryptedSegment> for zephyrfs_node::crypto::EncryptedData {
    fn from(segment: EncryptedSegment) -> Self {
        Self {
            segment_index: segment.segment_index,
            ciphertext: segment.ciphertext,
            nonce: segment.nonce,
            aad: segment.aad,
            key_path: segment.key_path,
        }
    }
}

// Import necessary items from zephyrfs_node
use zephyrfs_node::crypto::ZephyrCrypto;