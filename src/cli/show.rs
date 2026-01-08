//! Shows information about prior and upcoming releases
use std::path::Path;
use tokio::fs;

use crate::{Orchestrator, Result, cli::ShowCommand};

/// Get projected next release info as JSON, optionally filtered by package name
pub async fn execute(
    orchestrator: Orchestrator,
    cmd: ShowCommand,
) -> Result<()> {
    match cmd {
        ShowCommand::NextRelease {
            out_file, package, ..
        } => show_next_release(orchestrator, out_file, package).await,
        ShowCommand::CurrentRelease { out_file, package } => {
            show_current_release(orchestrator, out_file, package).await
        }
        ShowCommand::Release { out_file, tag } => {
            show_release(orchestrator, out_file, tag).await
        }
    }
}

/// Shows the most recent release for each package
async fn show_current_release(
    orchestrator: Orchestrator,
    out_file: Option<String>,
    target_package: Option<String>,
) -> Result<()> {
    let releases = orchestrator.get_current_releases(target_package).await?;
    let json = serde_json::json!(releases);
    print_json(json, out_file).await
}

async fn show_release(
    orchestrator: Orchestrator,
    out_file: Option<String>,
    tag: String,
) -> Result<()> {
    log::info!("retrieving release data for tag: {tag}");
    let data = orchestrator.get_release_by_tag(&tag).await?;
    let json = serde_json::json!(&data);
    print_json(json, out_file).await
}

async fn show_next_release(
    orchestrator: Orchestrator,
    out_file: Option<String>,
    package: Option<String>,
) -> Result<()> {
    let releasable_packages = orchestrator.get_next_releases(package).await?;
    let json = serde_json::json!(&releasable_packages);
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
