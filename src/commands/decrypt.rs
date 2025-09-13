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
pub struct DecryptCommand {
    /// Encrypted file to decrypt
    input: PathBuf,

    /// Output file path (defaults to original filename)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Password for decryption (will prompt if not provided)
    #[arg(short, long)]
    password: Option<String>,

    /// Show progress bar
    #[arg(long, default_value = "true")]
    progress: bool,

    /// Skip content verification
    #[arg(long)]
    skip_verify: bool,

    /// Force overwrite of existing output file
    #[arg(long)]
    force: bool,
}

#[async_trait::async_trait]
impl Command for DecryptCommand {
    async fn execute(&self, _config: &Config) -> Result<()> {
        info!("Decrypting file: {:?}", self.input);

        // Validate input file
        if !self.input.exists() {
            anyhow::bail!("File does not exist: {:?}", self.input);
        }

        if !self.input.is_file() {
            anyhow::bail!("Path is not a file: {:?}", self.input);
        }

        // Read and parse encrypted file
        let encrypted_data = tokio::fs::read(&self.input).await
            .with_context(|| format!("Failed to read encrypted file: {:?}", self.input))?;

        let encrypted_file: EncryptedFileFormat = serde_json::from_slice(&encrypted_data)
            .context("Failed to parse encrypted file format. This may not be a valid ZephyrFS encrypted file.")?;

        // Validate file format version
        if encrypted_file.version != 1 {
            anyhow::bail!("Unsupported encrypted file version: {}. This CLI supports version 1.", encrypted_file.version);
        }

        // Determine output path
        let output_path = self.output.clone().unwrap_or_else(|| {
            // Try to restore original filename
            if encrypted_file.filename != "unknown" {
                PathBuf::from(&encrypted_file.filename)
            } else {
                // Strip .zfs extension if present
                let mut output = self.input.clone();
                if let Some(stem) = output.file_stem().map(|s| s.to_os_string()) {
                    output.set_file_name(stem);
                } else {
                    output.set_extension("decrypted");
                }
                output
            }
        });

        // Check if output exists
        if output_path.exists() && !self.force {
            anyhow::bail!("Output file already exists: {:?}. Use --force to overwrite.", output_path);
        }

        println!("Decrypting: {}", encrypted_file.filename);
        println!("  Original size: {}", format_size(encrypted_file.original_size, BINARY));
        println!("  Segments: {}", encrypted_file.encrypted_segments.len());
        println!("  Created: {}", encrypted_file.created_at.format("%Y-%m-%d %H:%M:%S UTC"));
        println!("  Output: {:?}", output_path);

        // Get password
        let password = if let Some(ref pw) = self.password {
            pw.clone()
        } else {
            let theme = ColorfulTheme::default();
            Password::with_theme(&theme)
                .with_prompt("Enter password for decryption")
                .interact()
                .context("Failed to read password")?
        };

        // Create progress bar
        let progress = if self.progress {
            let pb = ProgressBar::new(encrypted_file.original_size);
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

        // Initialize crypto system
        let crypto = create_crypto_system(&password, encrypted_file.chunk_size_mb)?;

        // Convert segments back to internal format
        let encrypted_segments: Vec<zephyrfs_node::crypto::EncryptedData> = encrypted_file.encrypted_segments
            .into_iter()
            .map(|s| s.into())
            .collect();

        // Decrypt file
        let decrypted_data = crypto.decrypt_file(&encrypted_segments)
            .context("Failed to decrypt file. Check your password and try again.")?;

        // Verify content integrity if requested
        if !self.skip_verify {
            println!("Verifying content integrity...");
            let computed_content_id = crypto.content_id(&decrypted_data);
            let expected_content_id = zephyrfs_node::crypto::ContentId::from_hex(
                zephyrfs_node::crypto::HashAlgorithm::Blake3,
                &encrypted_file.content_id
            ).context("Invalid content ID in encrypted file")?;

            if !crypto.verify_content(&decrypted_data, &expected_content_id) {
                anyhow::bail!("Content verification failed! The decrypted file may be corrupted.");
            }
            println!("✅ Content integrity verified");
        }

        // Verify size matches
        if decrypted_data.len() as u64 != encrypted_file.original_size {
            anyhow::bail!("Size mismatch: expected {} bytes, got {} bytes", 
                         encrypted_file.original_size, 
                         decrypted_data.len());
        }

        // Write decrypted file
        tokio::fs::write(&output_path, &decrypted_data).await
            .with_context(|| format!("Failed to write decrypted file: {:?}", output_path))?;

        if let Some(pb) = progress {
            pb.finish_with_message("Decryption complete");
        }

        println!("✅ File decrypted successfully!");
        println!("  Restored file: {:?}", output_path);
        println!("  Size: {}", format_size(decrypted_data.len() as u64, BINARY));
        
        // Security cleanup note
        println!();
        println!("🔒 Security Notes:");
        println!("  • The original encrypted file is unchanged");
        println!("  • Consider securely deleting the encrypted file if no longer needed");

        Ok(())
    }
}

// Encrypted file format for storage (must match encrypt.rs)
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

// Helper function to create crypto system (must match encrypt.rs)
fn create_crypto_system(password: &str, chunk_size_mb: u32) -> Result<ZephyrCrypto> {
    use zephyrfs_node::crypto::{ZephyrCrypto, CryptoParams, ScryptParams, AesParams, HashParams, ContentHasher, VerificationHasher};
    
    // Create crypto parameters (must match encryption parameters)
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

// Convert between CLI types and zephyrfs_node types
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