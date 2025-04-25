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
use config::{Config, File};
use dotenvy::dotenv;
use libium::{
    config::{
        filters::ProfileParameters,
        structs::{Mod, ModIdentifier, ModLoader, Profile},
    },
    iter_ext::IterExt,
};
use remove::remove;
use serde::{Deserialize, Serialize};
use std::{env, fs, io::Write, process::Command};
use upgrade::upgrade;

#[derive(Debug, Deserialize, Serialize, Clone)]
struct FerriteConfig {
    autoupdate: bool,
    key_store: KeyStoreConfig,
    ferium: FeriumConfig,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
enum KeyStoreConfig {
    Pass,
    DotEnv,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct FeriumConfig {
    mods: Vec<Mod>,
    game_versions: Vec<String>,
    mod_loaders: Vec<ModLoader>,
}

impl FerriteConfig {
    pub fn new(game_versions: Vec<String>, mod_loaders: Vec<ModLoader>) -> Self {
        Self {
            autoupdate: true,
            key_store: KeyStoreConfig::DotEnv,
            ferium: FeriumConfig {
                mod_loaders,
                game_versions,
                mods: vec![],
            },
        }
    }

    pub fn write_config(&self) -> Result<()> {
        let serialized = serde_yaml::to_string(self)?;

        let mut file = fs::File::create("ferrite.yaml")?;
        file.write_all("# key_store: Pass / DotEnv".as_bytes())?;
        file.write_all(serialized.as_bytes())?;

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
            config.ferium.game_versions,
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
                upgrade(&profile, false).await?;
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

            if config.autoupdate {
                upgrade(&profile, false).await?;
            }

            config.update_mods(profile.mods);
        }
        SubCommands::Upgrade => {
            let config = load_config()?;
            let profile = Profile::from(config.clone());

            upgrade(&profile, true).await?;
        }

        SubCommands::Init {
            game_versions,
            mod_loaders,
        } => {
            let config = init::create(game_versions, mod_loaders).await?;
            config.write_config()?;
        }
    }

    Ok(())
}

fn load_config() -> Result<FerriteConfig> {
    let serialized = Config::builder()
        .add_source(File::with_name("ferrite").required(true))
        .build()?;

    let config: FerriteConfig = serialized.try_deserialize()?;

    match config.key_store {
        KeyStoreConfig::DotEnv => {
            if !fs::exists(".env")? {
                let mut file = fs::File::create(".env")?;
                file.write_all("# GITHUB_TOKEN / CURSEFORGE_API_KEY".as_bytes())?;
            };

            dotenv().ok();
        }
        KeyStoreConfig::Pass => {
            let gh_token = Command::new("pass")
                .arg("ferrite/github_token")
                .output()
                .expect("failed to run pass")
                .stdout;

            let token_str = String::from_utf8_lossy(&gh_token).trim().to_string();
            unsafe {
                env::set_var("GITHUB_TOKEN", token_str);
            }

            let cf_api_key = Command::new("pass")
                .arg("ferrite/curseforge_api_key")
                .output()
                .expect("failed to run pass")
                .stdout;

            let key_str = String::from_utf8_lossy(&cf_api_key).trim().to_string();
            unsafe {
                env::set_var("CURSEFORGE_API_KEY", key_str);
            }
        }
    }

    Ok(config)
}
