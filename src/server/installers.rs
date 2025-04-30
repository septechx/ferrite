use anyhow::Result;
use colored::Colorize;
use indicatif::ProgressBar;
use libium::iter_ext::IterExt;
use std::{fs, process::Command};

use super::{ServerInstallation, download_file_with_progress};
use crate::structs::*;

/// Installs a Fabric server
pub async fn install_fabric_server(
    game_version: &str,
    progress_bar: &ProgressBar,
) -> Result<ServerInstallation> {
    progress_bar.set_message(format!(
        "Fetching Fabric loader versions for {}",
        game_version.green()
    ));

    let fabric_version = fetch_fabric_loader_version(game_version).await?;

    progress_bar.set_message(format!(
        "Downloading Fabric server jar ({} / {})",
        game_version.green(),
        fabric_version.green()
    ));

    let url = format!(
        "https://meta.fabricmc.net/v2/versions/loader/{}/{}/1.0.3/server/jar",
        game_version, fabric_version
    );

    let filename = download_file_with_progress(&url, progress_bar).await?;

    progress_bar.finish_with_message(format!(
        "✓ Successfully downloaded server jar for {} ({})",
        game_version.green(),
        "Fabric".green()
    ));

    Ok(ServerInstallation {
        executable: filename,
        wrapper: String::from("java -Xmx2G -jar {} nogui"),
    })
}

/// Installs a Forge server
pub async fn install_forge_server(
    game_version: &str,
    progress_bar: &ProgressBar,
) -> Result<ServerInstallation> {
    progress_bar.set_message(format!(
        "Fetching Forge loader versions for {}",
        game_version.green()
    ));

    let forge_version = fetch_forge_loader_version(game_version).await?;

    progress_bar.set_message(format!(
        "Downloading Forge server installer jar ({} / {})",
        game_version.green(),
        forge_version.green()
    ));

    let url = format!(
        "https://maven.minecraftforge.net/net/minecraftforge/forge/{}/forge-{}-installer.jar",
        forge_version, forge_version
    );

    let installer_filename = download_file_with_progress(&url, progress_bar).await?;

    progress_bar.set_message(format!(
        "Installing Forge server ({} / {})",
        game_version.green(),
        forge_version.green()
    ));

    Command::new("java")
        .arg("-jar")
        .arg(&installer_filename)
        .arg("--installServer")
        .output()?;

    fs::remove_file(&installer_filename)?;
    fs::remove_file(format!("{}.log", &installer_filename))?;

    progress_bar.finish_with_message(format!(
        "✓ Successfully installed server for {} ({})",
        game_version.green(),
        "Forge".green()
    ));

    Ok(ServerInstallation {
        executable: if cfg!(windows) {
            "./run.bat".to_string()
        } else {
            "./run.sh".to_string()
        },
        wrapper: String::from("{} nogui"),
    })
}

/// Installs a Quilt server
pub async fn install_quilt_server(
    game_version: &str,
    progress_bar: &ProgressBar,
) -> Result<ServerInstallation> {
    progress_bar.set_message(format!(
        "Downloading Quilt server installer jar ({})",
        game_version.green()
    ));

    let url = "https://quiltmc.org/api/v1/download-latest-installer/java-universal";
    let installer_filename = download_file_with_progress(&url, progress_bar).await?;

    progress_bar.set_message(format!(
        "Installing Quilt server ({})",
        game_version.green()
    ));

    Command::new("java")
        .arg("-jar")
        .arg(&installer_filename)
        .arg("install")
        .arg("server")
        .arg(game_version)
        .arg("--download-server")
        .arg("--install-dir=./")
        .output()?;

    fs::remove_file(&installer_filename)?;

    progress_bar.finish_with_message(format!(
        "✓ Successfully installed server for {} ({})",
        game_version.green(),
        "Quilt".green()
    ));

    Ok(ServerInstallation {
        executable: "quilt-server-launch.jar".to_string(),
        wrapper: String::from("java -jar {} nogui"),
    })
}

/// Installs a NeoForge server
pub async fn install_neoforge_server(
    game_version: &str,
    progress_bar: &ProgressBar,
) -> Result<ServerInstallation> {
    progress_bar.set_message(format!(
        "Fetching NeoForge loader versions for {}",
        game_version.green()
    ));

    let neoforge_version = fetch_neoforge_loader_version(game_version).await?;

    progress_bar.set_message(format!(
        "Downloading NeoForge server installer jar ({} / {})",
        game_version.green(),
        neoforge_version.green()
    ));

    let url = format!(
        "https://maven.neoforged.net/releases/net/neoforged/neoforge/{}/neoforge-{}-installer.jar",
        neoforge_version, neoforge_version
    );

    let installer_filename = download_file_with_progress(&url, progress_bar).await?;

    progress_bar.set_message(format!(
        "Installing NeoForge server ({} / {})",
        game_version.green(),
        neoforge_version.green()
    ));

    Command::new("java")
        .arg("-jar")
        .arg(&installer_filename)
        .arg("--installServer")
        .output()?;

    fs::remove_file(&installer_filename)?;
    fs::remove_file(format!("{}.log", &installer_filename))?;

    progress_bar.finish_with_message(format!(
        "✓ Successfully installed server for {} ({})",
        game_version.green(),
        "NeoForge".green()
    ));

    Ok(ServerInstallation {
        executable: if cfg!(windows) {
            "./run.bat".to_string()
        } else {
            "./run.sh".to_string()
        },
        wrapper: String::from("{} nogui"),
    })
}

/// Fetches the latest Fabric loader version for a given Minecraft version
async fn fetch_fabric_loader_version(game_version: &str) -> Result<String> {
    let versions = reqwest::get(format!(
        "https://meta.fabricmc.net/v2/versions/loader/{}",
        game_version,
    ))
    .await?
    .json::<Vec<FabricLoaderEntry>>()
    .await?;

    // Try to find a stable version first
    if let Some(loader) = versions.iter().find(|l| l.loader.stable) {
        return Ok(loader.loader.version.clone());
    }

    // Fall back to any version if no stable version is found
    versions
        .first()
        .map(|l| l.loader.version.clone())
        .ok_or_else(|| anyhow::anyhow!("No Fabric loader version found for {}", game_version))
}

/// Fetches the latest Forge loader version for a given Minecraft version
async fn fetch_forge_loader_version(game_version: &str) -> Result<String> {
    let versions = reqwest::get(
        "https://files.minecraftforge.net/net/minecraftforge/forge/maven-metadata.json",
    )
    .await?
    .json::<std::collections::HashMap<String, Vec<String>>>()
    .await?;

    let versions = versions
        .get(game_version)
        .ok_or_else(|| anyhow::anyhow!("No Forge loader version found for {}", game_version))?;

    versions
        .last()
        .map(|v| v.clone())
        .ok_or_else(|| anyhow::anyhow!("No Forge loader version found for {}", game_version))
}

/// Fetches the latest NeoForge loader version for a given Minecraft version
async fn fetch_neoforge_loader_version(game_version: &str) -> Result<String> {
    let versions = reqwest::get(
        "https://maven.neoforged.net/releases/net/neoforged/neoforge/maven-metadata.xml",
    )
    .await?
    .text()
    .await?;

    let versions: NeoForgeLoaderMetadata = serde_xml_rs::from_str(&versions)?;

    let versions = versions
        .versioning
        .versions
        .version
        .iter()
        .filter(|v| v.starts_with(game_version.strip_prefix("1.").unwrap()))
        .collect_vec();

    versions
        .last()
        .ok_or_else(|| anyhow::anyhow!("No NeoForge loader version found for {}", game_version))
        .cloned()
        .cloned()
}
