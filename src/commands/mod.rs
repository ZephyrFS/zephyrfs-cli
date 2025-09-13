mod init;
mod join;
mod upload;
mod download;
mod list;
mod status;
mod encrypt;
mod decrypt;

pub use init::InitCommand;
pub use join::JoinCommand;
pub use upload::UploadCommand;
pub use download::DownloadCommand;
pub use list::ListCommand;
pub use status::StatusCommand;
pub use encrypt::EncryptCommand;
pub use decrypt::DecryptCommand;

use anyhow::Result;
use crate::config::Config;

#[async_trait::async_trait]
pub trait Command {
    async fn execute(&self, config: &Config) -> Result<()>;
}