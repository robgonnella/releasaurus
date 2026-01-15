//! Shows information about prior and upcoming releases
use std::path::Path;
use tokio::fs;

use crate::{Orchestrator, Result, cli::GetCommand};

/// Get projected next release info as JSON, optionally filtered by package name
pub async fn execute(
    orchestrator: Orchestrator,
    cmd: GetCommand,
) -> Result<()> {
    match cmd {
        GetCommand::NextRelease {
            out_file, package, ..
        } => get_next_release(orchestrator, out_file, package).await,
        GetCommand::CurrentRelease { out_file, package } => {
            get_current_release(orchestrator, out_file, package).await
        }
        GetCommand::Release { out_file, tag } => {
            get_release(orchestrator, out_file, tag).await
        }
        GetCommand::RecompiledNotes { file, out_file } => {
            get_notes(orchestrator, file, out_file).await
        }
    }
}

/// Shows the most recent release for each package
async fn get_current_release(
    orchestrator: Orchestrator,
    out_file: Option<String>,
    target_package: Option<String>,
) -> Result<()> {
    let releases = orchestrator.get_current_releases(target_package).await?;
    let json = serde_json::json!(releases);
    print_json(json, out_file).await
}

async fn get_release(
    orchestrator: Orchestrator,
    out_file: Option<String>,
    tag: String,
) -> Result<()> {
    log::info!("retrieving release data for tag: {tag}");
    let data = orchestrator.get_release_by_tag(&tag).await?;
    let json = serde_json::json!(&data);
    print_json(json, out_file).await
}

async fn get_next_release(
    orchestrator: Orchestrator,
    out_file: Option<String>,
    package: Option<String>,
) -> Result<()> {
    let releasable_packages =
        orchestrator.get_next_releases(package.as_deref()).await?;
    let json = serde_json::json!(&releasable_packages);
    print_json(json, out_file).await
}

async fn get_notes(
    orchestrator: Orchestrator,
    file: String,
    out_file: Option<String>,
) -> Result<()> {
    let output = orchestrator
        .recompile_notes_from_release_file(&file)
        .await?;
    let json = serde_json::json!(output);
    print_json(json, out_file).await
}

async fn print_json(
    json: serde_json::Value,
    out_file: Option<String>,
) -> Result<()> {
    if let Some(out_file) = out_file {
        let file_path = Path::new(&out_file);

        if let Some(parent) = file_path.parent()
            && !parent.exists()
        {
            fs::create_dir_all(parent).await?;
        }

        let content = serde_json::to_string_pretty(&json)?;
        log::info!("writing json to: {}", file_path.display());
        fs::write(file_path, &content).await?;
    } else {
        println!("{json}");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::TempDir;

    #[tokio::test]
    async fn print_json_writes_to_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("output.json");

        let test_json = json!({
            "name": "test-package",
            "version": "1.0.0"
        });

        let result = print_json(
            test_json.clone(),
            Some(file_path.to_string_lossy().to_string()),
        )
        .await;

        assert!(result.is_ok());
        assert!(file_path.exists());

        let content = fs::read_to_string(&file_path).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();

        assert_eq!(parsed, test_json);
    }

    #[tokio::test]
    async fn print_json_creates_parent_directories() {
        let temp_dir = TempDir::new().unwrap();
        let nested_path = temp_dir
            .path()
            .join("nested")
            .join("dir")
            .join("output.json");

        let test_json = json!({"key": "value"});

        let result = print_json(
            test_json.clone(),
            Some(nested_path.to_string_lossy().to_string()),
        )
        .await;

        assert!(result.is_ok());
        assert!(nested_path.exists());
        assert!(nested_path.parent().unwrap().exists());
    }
}
