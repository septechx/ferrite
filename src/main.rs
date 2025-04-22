mod add;
mod cli;

use add::display_successes_failures;
use anyhow::Result;
use clap::Parser;
use cli::{Ferrite, SubCommands};
use config::{Config, ConfigError, File};
use libium::config::structs::{Mod, ModLoader, Profile};
use serde::Deserialize;
use std::env;

#[derive(Debug, Deserialize, Clone)]
struct FerriteConfig {
    mods: Vec<Mod>,
    game_versions: Vec<String>,
    mod_loaders: Vec<ModLoader>,
}

impl FerriteConfig {
    fn update_mods(&mut self, mods: Vec<Mod>) {
        self.mods = mods;
    }
}

impl From<FerriteConfig> for Profile {
    fn from(config: FerriteConfig) -> Self {
        Self::new_with_mods(
            String::from("ferrite"),
            env::current_dir()
                .expect("Failed to get current directory")
                .join("mods"),
            config.game_versions,
            config.mod_loaders,
            config.mods,
        )
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut config = load_config()?;
    let mut profile = Profile::from(config.clone());
    let cli = Ferrite::parse();

    match cli.subcommand {
        SubCommands::Add { identifiers } => {
            let identifiers: Vec<_> = identifiers.into_iter().map(libium::add::parse_id).collect();
            let (successes, failures) =
                libium::add(&mut profile, identifiers, true, false, vec![]).await?;

            display_successes_failures(&successes, failures);

            config.update_mods(profile.mods);
        }
        _ => todo!(),
    }

    Ok(())
}

fn load_config() -> Result<FerriteConfig, ConfigError> {
    let config = Config::builder()
        .add_source(File::with_name("ferrite"))
        .build()?;

    config.try_deserialize()
}
