use anyhow::{Ok, Result, bail};
use colored::Colorize;
use ferinth::{Ferinth, structures::tag::GameVersion};
use inquire::MultiSelect;
use libium::{config::structs::ModLoader, iter_ext::IterExt};

use crate::FerriteConfig;
use crate::server::ServerInstallation;

/// Prompts the user to select mod loaders
pub fn pick_mod_loader() -> Result<Vec<ModLoader>> {
    let options = [
        ModLoader::Fabric,
        ModLoader::Quilt,
        ModLoader::NeoForge,
        ModLoader::Forge,
        ModLoader::Velocity,
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
        .map(|v| {
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

/// Sorts mod loaders in a consistent order
fn sort_mod_loaders(mod_loaders: &mut [ModLoader]) {
    mod_loaders.sort_by_key(|loader| match loader {
        ModLoader::NeoForge => 0,
        ModLoader::Forge => 1,
        ModLoader::Quilt => 2,
        ModLoader::Fabric => 3,
        ModLoader::Velocity => 4,
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
