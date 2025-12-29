mod add;
mod cli;
mod disable;
mod download;
mod init;
mod remove;
mod scripts;
mod server;
mod structs;
mod upgrade;

use add::display_successes_failures;
use anyhow::{Result, anyhow};
use clap::Parser;
use cli::{Ferrite, SubCommands};
use colored::Colorize;
use config::{Config, ConfigError, File};
use disable::disable;
use dotenvy::dotenv;
use libium::{
    config::structs::{Mod, ModIdentifier, ModLoader, Profile},
    iter_ext::IterExt,
};
use remove::remove;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    env, fs,
    io::Write,
    process::{Command, Stdio},
};
use upgrade::upgrade;

#[derive(Debug, Deserialize, Serialize, Clone)]
struct FerriteConfig {
    version: i64,
    autoupdate: bool,
    key_store: KeyStoreConfig,
    server: ServerConfig,
    ferium: FeriumConfig,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct ServerConfig {
    wrapper: String,
    executable: String,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
enum KeyStoreConfig {
    Pass,
    DotEnv,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct FeriumConfig {
    game_versions: Vec<String>,
    mod_loaders: Vec<ModLoader>,
    overrides: HashMap<String, ModIdentifier>,
    mods: Vec<Mod>,
    disabled: Vec<Mod>,
}

impl FerriteConfig {
    pub fn new(
        game_versions: Vec<String>,
        mod_loaders: Vec<ModLoader>,
        wrapper: String,
        executable: String,
    ) -> Self {
        Self {
            version: 1,
            autoupdate: true,
            key_store: KeyStoreConfig::DotEnv,
            server: ServerConfig {
                wrapper,
                executable,
            },
            ferium: FeriumConfig {
                mod_loaders,
                game_versions,
                overrides: HashMap::new(),
                mods: vec![],
                disabled: vec![],
            },
        }
    }

    pub fn write_config(&self) -> Result<()> {
        let serialized = serde_yml::to_string(self)?;

        let mut file = fs::File::create("ferrite.yaml")?;
        file.write_all(
            "# https://github.com/septechx/ferrite/blob/master/schema/ferrite.yaml\n".as_bytes(),
        )?;
        file.write_all(serialized.as_bytes())?;

        Ok(())
    }

    pub fn update(&mut self, profile: Profile) {
        self.ferium.mods = profile.mods;
        self.ferium.disabled = profile.disabled;
        if let Err(e) = self.write_config() {
            eprintln!("Error writing config: {}", e);
        }
    }
}

impl From<FerriteConfig> for Profile {
    fn from(config: FerriteConfig) -> Self {
        Self::new_complete(
            String::from("ferrite"),
            env::current_dir()
                .expect("Failed to get current directory")
                .join("mods"),
            config.ferium.game_versions,
            config.ferium.mod_loaders,
            config.ferium.mods,
            config.ferium.disabled,
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

            profile.disabled.retain(|m| {
                !profile
                    .mods
                    .iter()
                    .any(|mod_| mod_.identifier == m.identifier)
            });

            display_successes_failures(&successes, failures);

            if config.autoupdate {
                upgrade(&profile, false, &config.ferium.overrides).await?;
            }

            config.update(profile);
        }

        SubCommands::List => {
            let config = load_config()?;

            println!(
                "{} mods on {} {}\n",
                config.ferium.mods.len().to_string().yellow(),
                config.ferium.mod_loaders.iter().display(", ").purple(),
                config.ferium.game_versions.iter().display(", ").green(),
            );
            for mod_ in &config.ferium.mods {
                println!(
                    "{:20}  {}",
                    match &mod_.identifier {
                        ModIdentifier::CurseForgeProject(id)
                        | ModIdentifier::PinnedCurseForgeProject(id, _) =>
                            format!("{} {:8}", "CF".red(), id.to_string().dimmed()),
                        ModIdentifier::ModrinthProject(id)
                        | ModIdentifier::PinnedModrinthProject(id, _) =>
                            format!("{} {:8}", "MR".green(), id.dimmed()),
                        ModIdentifier::GitHubRepository(..)
                        | ModIdentifier::PinnedGitHubRepository(..) => "GH".purple().to_string(),
                    },
                    match &mod_.identifier {
                        ModIdentifier::ModrinthProject(_)
                        | ModIdentifier::CurseForgeProject(_)
                        | ModIdentifier::PinnedModrinthProject(_, _)
                        | ModIdentifier::PinnedCurseForgeProject(_, _) =>
                            mod_.name.bold().to_string(),
                        ModIdentifier::GitHubRepository(owner, repo)
                        | ModIdentifier::PinnedGitHubRepository((owner, repo), _) =>
                            format!("{}/{}", owner.dimmed(), repo.bold()),
                    },
                );
            }
        }

        SubCommands::Remove { mod_names } => {
            let mut config = load_config()?;
            let mut profile = Profile::from(config.clone());

            remove(&mut profile, mod_names)?;

            if config.autoupdate {
                upgrade(&profile, false, &config.ferium.overrides).await?;
            }

            config.update(profile);
        }

        SubCommands::Disable { mod_names } => {
            let mut config = load_config()?;
            let mut profile = Profile::from(config.clone());

            disable(&mut profile, mod_names)?;

            if config.autoupdate {
                upgrade(&profile, false, &config.ferium.overrides).await?;
            }

            config.update(profile);
        }

        SubCommands::Upgrade => {
            let config = load_config()?;
            let profile = Profile::from(config.clone());

            upgrade(&profile, true, &config.ferium.overrides).await?;
        }

        SubCommands::Override { mod_override } => {
            let mut config = load_config()?;

            anyhow::ensure!(mod_override.len() == 2, "Invalid amount of arguments");

            let identifier: ModIdentifier = if mod_override[1].contains('/') {
                let split = mod_override[1].split_once('/').unwrap();
                ModIdentifier::GitHubRepository(split.0.to_string(), split.1.to_string())
            } else if mod_override[1].chars().all(|c| c.is_ascii_digit()) {
                ModIdentifier::CurseForgeProject(mod_override[1].parse::<i32>()?)
            } else {
                ModIdentifier::ModrinthProject(mod_override[1].clone())
            };

            config
                .ferium
                .overrides
                .insert(mod_override[0].clone(), identifier);

            config.write_config()?;
        }

        SubCommands::Init {
            game_versions,
            mod_loaders,
        } => {
            let config = init::create(game_versions, mod_loaders).await?;
            config.write_config()?;
        }

        SubCommands::Start => {
            let config = load_config()?;

            let wrapper = config
                .server
                .wrapper
                .replace("{}", &config.server.executable);
            let parts = wrapper.split(' ').collect_vec();

            Command::new(parts[0])
                .args(&parts[1..])
                .stdin(Stdio::inherit())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .spawn()?
                .wait()?;
        }

        SubCommands::Script { script } => {
            let mut config = load_config()?;

            scripts::run(&mut config, &script)?;

            config.write_config()?;
        }
    }

    Ok(())
}

fn fix_config_v0() -> Result<Config, ConfigError> {
    Config::builder()
        .add_source(File::with_name("ferrite").required(true))
        .set_default("version", 1)?
        .build()
}

fn load_config() -> Result<FerriteConfig> {
    let mut serialized = Config::builder()
        .add_source(File::with_name("ferrite").required(true))
        .build()?;

    let version: i64 = serialized.get_int("version").unwrap_or_else(|_| {
        serialized = fix_config_v0().unwrap();
        1
    });

    let config: FerriteConfig = match version {
        1 => Ok(serialized.try_deserialize()?),
        _ => Err(anyhow!(format!("Invalid version: {}", version))),
    }?;

    match config.key_store {
        KeyStoreConfig::DotEnv => {
            if !fs::exists(".env")? {
                let mut file = fs::File::create(".env")?;
                file.write_all(
                    "# https://github.com/septechx/ferrite/blob/master/schema/.env".as_bytes(),
                )?;
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
