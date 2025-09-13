mod init;
mod join;
mod upload;
mod download;
mod list;
mod status;

pub use init::InitCommand;
pub use join::JoinCommand;
pub use upload::UploadCommand;
pub use download::DownloadCommand;
pub use list::ListCommand;
pub use status::StatusCommand;

use anyhow::Result;
use crate::config::Config;

#[async_trait::async_trait]
pub trait Command {
    async fn execute(&self, config: &Config) -> Result<()>;
}