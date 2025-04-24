mod add;
mod cli;
mod download;
mod init;
mod remove;
mod upgrade;

use add::display_successes_failures;
use anyhow::Result;
use clap::Parser;
use cli::{Ferrite, SubCommands};
use colored::Colorize;
use config::{Config, ConfigError, File};
use libium::{
    config::{
        filters::ProfileParameters,
        structs::{Mod, ModIdentifier, ModLoader, Profile},
    },
    iter_ext::IterExt,
};
use remove::remove;
use serde::{Deserialize, Serialize};
use std::{env, fs};
use upgrade::upgrade;

#[derive(Debug, Deserialize, Serialize, Clone)]
struct FerriteConfig {
    autoupdate: bool,
    ferium: FeriumConfig,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct FeriumConfig {
    mods: Vec<Mod>,
    game_version: Vec<String>,
    mod_loaders: Vec<ModLoader>,
}

impl FerriteConfig {
    pub fn new(game_version: Vec<String>, mod_loaders: Vec<ModLoader>) -> Self {
        Self {
            autoupdate: true,
            ferium: FeriumConfig {
                game_version,
                mod_loaders,
                mods: vec![],
            },
        }
    }

    pub fn write_config(&self) -> Result<(), std::io::Error> {
        let toml_str =
            toml::to_string_pretty(self).expect("Failed to serialize FerriteConfig to TOML");
        fs::write("ferrite.toml", toml_str)?;

        Ok(())
    }

    pub fn update_mods(&mut self, mods: Vec<Mod>) {
        self.ferium.mods = mods;
        if let Err(e) = self.write_config() {
            eprintln!("Error writing config: {}", e);
        }
    }
}

impl From<FerriteConfig> for Profile {
    fn from(config: FerriteConfig) -> Self {
        Self::new_with_mods(
            String::from("ferrite"),
            env::current_dir()
                .expect("Failed to get current directory")
                .join("mods"),
            config.ferium.game_version,
            config.ferium.mod_loaders,
            config.ferium.mods,
        )
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Ferrite::parse();

    match cli.subcommand {
        SubCommands::Add { identifiers } => {
            let mut config = load_config()?;
            let mut profile = Profile::from(config.clone());

            let identifiers: Vec<_> = identifiers.into_iter().map(libium::add::parse_id).collect();
            let (successes, failures) =
                libium::add(&mut profile, identifiers, true, false, vec![]).await?;

            display_successes_failures(&successes, failures);

            if config.autoupdate {
                upgrade(&profile).await?;
            }

            config.update_mods(profile.mods);
        }

        SubCommands::List => {
            let config = load_config()?;
            let profile = Profile::from(config.clone());

            println!(
                "{} {} on {} {}\n",
                profile.name.bold(),
                format!("({} mods)", profile.mods.len()).yellow(),
                profile
                    .filters
                    .mod_loader()
                    .map(ToString::to_string)
                    .unwrap_or_default()
                    .purple(),
                profile
                    .filters
                    .game_versions()
                    .unwrap_or(&vec![])
                    .iter()
                    .display(", ")
                    .green(),
            );
            for mod_ in &profile.mods {
                println!(
                    "{:20}  {}",
                    match &mod_.identifier {
                        ModIdentifier::CurseForgeProject(id) =>
                            format!("{} {:8}", "CF".red(), id.to_string().dimmed()),
                        ModIdentifier::ModrinthProject(id) =>
                            format!("{} {:8}", "MR".green(), id.dimmed()),
                        ModIdentifier::GitHubRepository(..) => "GH".purple().to_string(),
                        _ => todo!(),
                    },
                    match &mod_.identifier {
                        ModIdentifier::ModrinthProject(_) | ModIdentifier::CurseForgeProject(_) =>
                            mod_.name.bold().to_string(),
                        ModIdentifier::GitHubRepository(owner, repo) =>
                            format!("{}/{}", owner.dimmed(), repo.bold()),
                        _ => todo!(),
                    },
                );
            }
        }

        SubCommands::Remove { mod_names } => {
            let mut config = load_config()?;
            let mut profile = Profile::from(config.clone());

            remove(&mut profile, mod_names)?;

            config.update_mods(profile.mods);
        }
        SubCommands::Upgrade => {
            let config = load_config()?;
            let profile = Profile::from(config.clone());

            upgrade(&profile).await?;
        }

        SubCommands::Init {
            game_version,
            mod_loaders,
        } => {
            let config = init::create(Some(game_version), Some(mod_loaders)).await?;
            let serialized = toml::to_string_pretty(&config)?;
        }
    }

    Ok(())
}

fn load_config() -> Result<FerriteConfig, ConfigError> {
    let config = Config::builder()
        .add_source(File::with_name("ferrite").required(true))
        .build()?;

    config.try_deserialize()
}
