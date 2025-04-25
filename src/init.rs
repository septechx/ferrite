use anyhow::{Result, bail};
use colored::Colorize;
use ferinth::Ferinth;
use inquire::MultiSelect;
use libium::{config::structs::ModLoader, iter_ext::IterExt};

use crate::FerriteConfig;

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

pub async fn pick_minecraft_versions() -> Result<Vec<String>> {
    let mut versions = Ferinth::default().list_game_versions().await?;
    versions.sort_by(|a, b| {
        // Sort by release type (release > snapshot > beta > alpha) then in reverse chronological order
        a.version_type
            .cmp(&b.version_type)
            .then(b.date.cmp(&a.date))
    });
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

pub async fn create(
    game_versions: Option<Vec<String>>,
    mod_loaders: Option<Vec<ModLoader>>,
) -> Result<FerriteConfig> {
    Ok(match (game_versions, mod_loaders) {
        (Some(game_versions), Some(mod_loaders)) => FerriteConfig::new(game_versions, mod_loaders),
        (None, None) => FerriteConfig::new(pick_minecraft_versions().await?, pick_mod_loader()?),
        _ => {
            bail!("Provide the game versions and mod loaders to create a profile")
        }
    })
}
