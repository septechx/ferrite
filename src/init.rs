use std::{fs::File, io::Write, time::Duration};

use anyhow::{Result, bail};
use colored::Colorize;
use ferinth::{Ferinth, structures::tag::GameVersion};
use indicatif::{ProgressBar, ProgressStyle};
use inquire::MultiSelect;
use libium::{config::structs::ModLoader, iter_ext::IterExt};
use reqwest::header::CONTENT_DISPOSITION;

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

/// Downloads a server jar file
async fn download_server_jar(url: &str, progress_bar: &ProgressBar) -> Result<String> {
    let jar = reqwest::get(url).await?;

    let filename = jar
        .headers()
        .get(CONTENT_DISPOSITION)
        .ok_or_else(|| anyhow::anyhow!("No Content-Disposition header found"))?
        .to_str()?
        .split(';')
        .find_map(|part| part.trim().strip_prefix("filename="))
        .ok_or_else(|| anyhow::anyhow!("No filename in Content-Disposition"))?
        .trim_matches('"')
        .to_string();

    progress_bar.set_message(format!("Saving server jar to {}", filename));
    let mut file = File::create(&filename)?;
    let content = jar.bytes().await?;
    file.write_all(&content)?;

    Ok(filename)
}

/// Gets the server jar for a specific game version and mod loader
pub async fn get_server_jar(
    game_version: &str,
    mod_loader: &ModLoader,
) -> Result<(String, String)> {
    let progress_bar = create_progress_bar(&format!(
        "Downloading server jar for {} ({})",
        game_version, mod_loader
    ));

    let result = match mod_loader {
        ModLoader::Fabric => {
            progress_bar.set_message(format!(
                "Fetching Fabric loader versions for {}",
                game_version
            ));

            let fabric_version = fetch_fabric_loader_version(game_version).await?;

            progress_bar.set_message(format!(
                "Downloading Fabric server jar ({} / {})",
                game_version, fabric_version
            ));

            let url = format!(
                "https://meta.fabricmc.net/v2/versions/loader/{}/{}/1.0.3/server/jar",
                game_version, fabric_version
            );

            let filename = download_server_jar(&url, &progress_bar).await?;

            progress_bar.finish_with_message(format!(
                "✓ {}: Downloaded {} successfully",
                mod_loader.to_string().green(),
                filename
            ));

            (filename, String::from("java -Xmx2G -jar {} nogui"))
        }
        _ => {
            progress_bar.finish_with_message(format!(
                "Mod loader {} is not implemented yet",
                mod_loader.to_string().red()
            ));
            todo!()
        }
    };

    Ok(result)
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
            let (executable, wrapper) = get_server_jar(&game_versions[0], &mod_loaders[0]).await?;
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
            let (executable, wrapper) = get_server_jar(&game_versions[0], &mod_loaders[0]).await?;
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
