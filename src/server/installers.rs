use anyhow::{anyhow, Result};
use colored::Colorize;
use indicatif::ProgressBar;
use libium::iter_ext::IterExt;
use std::{collections::HashMap, fs, process::Command};

use super::{download_file_with_progress, ServerInstallation};
use crate::structs::*;

pub trait Installer {
    async fn install(game_version: &str, progress_bar: &ProgressBar) -> Result<ServerInstallation>;
}

pub struct FabricInstaller;

impl Installer for FabricInstaller {
    async fn install(game_version: &str, progress_bar: &ProgressBar) -> Result<ServerInstallation> {
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

        let launcher_version = "1.1.0";
        let url = format!(
            "https://meta.fabricmc.net/v2/versions/loader/{game_version}/{fabric_version}/{launcher_version}/server/jar",
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
}

pub struct ForgeInstaller;

impl Installer for ForgeInstaller {
    async fn install(game_version: &str, progress_bar: &ProgressBar) -> Result<ServerInstallation> {
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
            "https://maven.minecraftforge.net/net/minecraftforge/forge/{forge_version}/forge-{forge_version}-installer.jar",
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
}

pub struct QuiltInstaller;

impl Installer for QuiltInstaller {
    async fn install(game_version: &str, progress_bar: &ProgressBar) -> Result<ServerInstallation> {
        progress_bar.set_message(format!(
            "Downloading Quilt server installer jar ({})",
            game_version.green()
        ));

        let url = "https://quiltmc.org/api/v1/download-latest-installer/java-universal";
        let installer_filename = download_file_with_progress(url, progress_bar).await?;

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
}

pub struct NeoForgeInstaller;

impl Installer for NeoForgeInstaller {
    async fn install(game_version: &str, progress_bar: &ProgressBar) -> Result<ServerInstallation> {
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
            "https://maven.neoforged.net/releases/net/neoforged/neoforge/{neoforge_version}/neoforge-{neoforge_version}-installer.jar",
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
}

pub struct VelocityInstaller;

impl Installer for VelocityInstaller {
    async fn install(
        _game_version: &str,
        progress_bar: &ProgressBar,
    ) -> Result<ServerInstallation> {
        progress_bar.set_message("Fetching Velocity proxy versions");

        let user_agent = format!(
            "ferrite/{} (github.com/septechx/ferrite)",
            env!("CARGO_PKG_VERSION")
        );

        let velocity_version = fetch_velocity_proxy_version(&user_agent).await?;

        progress_bar.set_message(format!(
            "Downloading Velocity proxy jar ({})",
            velocity_version.green()
        ));

        let client = reqwest::Client::new();
        let url = format!(
            "https://fill.papermc.io/v3/projects/velocity/versions/{velocity_version}/builds",
        );

        let download_url = client
            .get(url)
            .header(reqwest::header::USER_AGENT, user_agent)
            .send()
            .await?
            .json::<Vec<VelocityVersionBuild>>()
            .await?;

        let download_url = download_url
            .first()
            .ok_or_else(|| {
                anyhow!(
                    "No Velocity proxy download URL found for {}",
                    velocity_version
                )
            })?
            .downloads
            .server_default
            .url
            .clone();

        let filename = download_file_with_progress(&download_url, progress_bar).await?;

        progress_bar.finish_with_message(format!(
            "✓ Successfully downloaded proxy jar for {} ({})",
            velocity_version.green(),
            "Velocity".green()
        ));

        Ok(ServerInstallation {
            executable: filename,
            wrapper: String::from("java -jar {}"),
        })
    }
}

async fn fetch_fabric_loader_version(game_version: &str) -> Result<String> {
    let versions = reqwest::get(format!(
        "https://meta.fabricmc.net/v2/versions/loader/{game_version}",
    ))
    .await?
    .json::<Vec<FabricLoaderEntry>>()
    .await?;

    if let Some(loader) = versions.iter().find(|l| l.loader.stable) {
        return Ok(loader.loader.version.clone());
    }

    versions
        .first()
        .map(|l| l.loader.version.clone())
        .ok_or_else(|| anyhow!("No Fabric loader version found for {}", game_version))
}

async fn fetch_forge_loader_version(game_version: &str) -> Result<String> {
    let versions = reqwest::get(
        "https://files.minecraftforge.net/net/minecraftforge/forge/maven-metadata.json",
    )
    .await?
    .json::<HashMap<String, Vec<String>>>()
    .await?;

    let versions = versions
        .get(game_version)
        .ok_or_else(|| anyhow!("No Forge loader version found for {}", game_version))?;

    versions
        .last()
        .cloned()
        .ok_or_else(|| anyhow!("No Forge loader version found for {}", game_version))
}

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
        .ok_or_else(|| anyhow!("No NeoForge loader version found for {}", game_version))
        .cloned()
        .cloned()
}

pub async fn fetch_velocity_proxy_version(user_agent: &str) -> Result<String> {
    let client = reqwest::Client::new();

    let latest_version = client
        .get("https://fill.papermc.io/v3/projects/velocity/versions")
        .header(reqwest::header::USER_AGENT, user_agent)
        .send()
        .await?
        .json::<VelocityVersions>()
        .await?;

    let latest_version = latest_version
        .versions
        .first()
        .ok_or_else(|| anyhow!("No Velocity version found"))?
        .version
        .id
        .to_owned();

    Ok(latest_version)
}
