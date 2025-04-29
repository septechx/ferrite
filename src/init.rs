use std::{collections::HashMap, fs::File, io::Write, time::Duration};

use anyhow::{Ok, Result, bail};
use colored::Colorize;
use ferinth::{Ferinth, structures::tag::GameVersion};
use indicatif::{ProgressBar, ProgressStyle};
use inquire::MultiSelect;
use libium::{config::structs::ModLoader, iter_ext::IterExt};
use reqwest::header::CONTENT_DISPOSITION;

use crate::server::ServerInstallation;
use crate::{FerriteConfig, structs::*};

/// Creates a progress bar with a spinner and message
fn create_progress_bar(message: &str) -> ProgressBar {
    let style = ProgressStyle::default_spinner()
        .template("{spinner} {msg}")
        .expect("Progress bar template parse failure");
    let progress_bar = ProgressBar::new_spinner().with_style(style);
    progress_bar.set_message(message.to_string());
    progress_bar.enable_steady_tick(Duration::from_millis(100));
    progress_bar
}

/// Prompts the user to select mod loaders
pub fn pick_mod_loader() -> Result<Vec<ModLoader>> {
    let options = [
        ModLoader::Fabric,
        ModLoader::Quilt,
        ModLoader::NeoForge,
        ModLoader::Forge,
    ];
    let picker = MultiSelect::new("Which mod loader do you use?", options.into());
    Ok(picker.prompt()?)
}

/// Fetches and sorts Minecraft versions
async fn fetch_minecraft_versions() -> Result<Vec<GameVersion>> {
    let mut versions = Ferinth::default().tag_list_game_versions().await?;
    versions.sort_by(|a, b| {
        // Sort by release type (release > snapshot > beta > alpha) then in reverse chronological order
        a.version_type
            .cmp(&b.version_type)
            .then(b.date.cmp(&a.date))
    });
    Ok(versions)
}

/// Prompts the user to select Minecraft versions
pub async fn pick_minecraft_versions() -> Result<Vec<String>> {
    let versions = fetch_minecraft_versions().await?;
    let display_versions = versions
        .iter()
        .enumerate()
        .map(|(_, v)| {
            if v.major {
                v.version.bold()
            } else {
                v.version.clone().into()
            }
        })
        .collect_vec();

    let selected_versions =
        MultiSelect::new("Which version of Minecraft do you play?", display_versions)
            .raw_prompt()?
            .into_iter()
            .map(|s| s.index)
            .collect_vec();

    Ok(versions
        .into_iter()
        .enumerate()
        .filter_map(|(i, v)| {
            if selected_versions.contains(&i) {
                Some(v.version)
            } else {
                None
            }
        })
        .collect_vec())
}

/// Fetches the latest stable Fabric loader version for a given Minecraft version
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
    .json::<HashMap<String, Vec<String>>>()
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

/// Downloads a server jar file
async fn download_server_jar(url: &str) -> Result<String> {
    let jar = reqwest::get(url).await?;
    let content_disposition = jar
        .headers()
        .get(CONTENT_DISPOSITION)
        .and_then(|h| h.to_str().ok())
        .map(String::from);

    let bytes = jar.bytes().await?;

    let filename = if let Some(content_disposition) = content_disposition {
        content_disposition
            .split(';')
            .find_map(|part| part.trim().strip_prefix("filename="))
            .ok_or_else(|| anyhow::anyhow!("No filename in Content-Disposition"))?
            .trim_matches('"')
            .to_string()
    } else {
        format!("server-{}.jar", blake3::hash(&bytes))
    };

    let mut file = File::create(&filename)?;
    file.write_all(&bytes)?;
    Ok(filename)
}

/// Sorts mod loaders in a consistent order
fn sort_mod_loaders(mod_loaders: &mut Vec<ModLoader>) {
    mod_loaders.sort_by_key(|loader| match loader {
        ModLoader::NeoForge => 0,
        ModLoader::Forge => 1,
        ModLoader::Quilt => 2,
        ModLoader::Fabric => 3,
    });
}

/// Creates a new Ferrite configuration
pub async fn create(
    game_versions: Option<Vec<String>>,
    mod_loaders: Option<Vec<ModLoader>>,
) -> Result<FerriteConfig> {
    match (game_versions, mod_loaders) {
        (Some(game_versions), Some(mut mod_loaders)) => {
            sort_mod_loaders(&mut mod_loaders);
            let ServerInstallation {
                executable,
                wrapper,
            } = crate::server::get_server_jar(&game_versions[0], &mod_loaders[0]).await?;
            Ok(FerriteConfig::new(
                game_versions,
                mod_loaders,
                wrapper,
                executable,
            ))
        }
        (None, None) => {
            let game_versions = pick_minecraft_versions().await?;
            let mut mod_loaders = pick_mod_loader()?;
            sort_mod_loaders(&mut mod_loaders);
            let ServerInstallation {
                executable,
                wrapper,
            } = crate::server::get_server_jar(&game_versions[0], &mod_loaders[0]).await?;
            Ok(FerriteConfig::new(
                game_versions,
                mod_loaders,
                wrapper,
                executable,
            ))
        }
        _ => bail!("Provide both game versions and mod loaders to create a profile"),
    }
}
