use async_trait::async_trait;
use color_eyre::eyre::Result;
use std::{collections::HashMap, path::Path};
use tokio::fs;

pub mod cli;
pub mod logging;
pub mod slack;

#[async_trait]
pub trait PlatformClient {
    async fn get_user_name_tag_hash(&self) -> Result<HashMap<String, String>>;
}

pub async fn write_json(
    json: &serde_json::Value,
    out_file: &str,
) -> Result<()> {
    let file_path = Path::new(&out_file);

    if let Some(parent) = file_path.parent()
        && !parent.exists()
    {
        fs::create_dir_all(parent).await?;
    }

    let content = serde_json::to_string_pretty(json)?;
    log::info!("writing json to: {}", file_path.display());
    fs::write(file_path, &content).await?;

    Ok(())
}
