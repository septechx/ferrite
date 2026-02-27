mod cli;
mod config;
mod init;
mod mods;
mod scripts;
mod server;
mod upgrade;

use std::process::ExitCode;

use clap::Parser;
use cli::{Ferrite, SubCommands};
use colored::Colorize;
use config::load_config;
use mods::disable;
use mods::display_successes_failures;

use libium::{config::structs::ModIdentifier, iter_ext::IterExt};
use mods::remove;
use upgrade::upgrade;

fn main() -> ExitCode {
    #[tokio::main]
    async fn run_async() -> Result<(), FerriteError> {
        run().await
    }

    match run_async() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{} {}", "Error:".red(), e);
            ExitCode::FAILURE
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum FerriteError {
    #[error("Configuration error: {0}")]
    Config(#[from] config::ConfigError),

    #[error("Mod error: {0}")]
    Mod(#[from] mods::ModError),

    #[error("Server error: {0}")]
    Server(#[from] server::ServerError),

    #[error("Upgrade error: {0}")]
    Upgrade(#[from] upgrade::UpgradeError),

    #[error("Initialization error: {0}")]
    Init(#[from] init::InitError),

    #[error("Script error: {0}")]
    Script(#[from] scripts::ScriptError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Parse error: {0}")]
    Parse(#[from] std::num::ParseIntError),

    #[error("Libium error: {0}")]
    Libium(#[from] libium::add::Error),

    #[error("Invalid identifier format: '{0}'. Expected 'owner/repo' for GitHub")]
    InvalidIdentifierFormat(String),
}

async fn run() -> Result<(), FerriteError> {
    let cli = Ferrite::parse();

    match cli.subcommand {
        SubCommands::Add { identifiers } => {
            let mut config = load_config()?;
            let mut profile = config.clone().into();

            let identifiers: Vec<_> = identifiers
                .into_iter()
                .map(libium::add::parse_id)
                .collect::<Result<Vec<_>, _>>()?;

            let (successes, failures) =
                libium::add(&mut profile, identifiers, true, false, vec![]).await?;

            profile.disabled.retain(|m| {
                !profile
                    .mods
                    .iter()
                    .any(|mod_| mod_.identifier == m.identifier)
            });

            display_successes_failures(
                &successes.iter().map(|(s, _)| s.clone()).collect::<Vec<_>>(),
                failures,
            );

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
                        ModIdentifier::CurseForgeProject(id, _) => {
                            format!("{} {:8}", "CF".red(), id.to_string().dimmed())
                        }
                        ModIdentifier::ModrinthProject(id, version) => {
                            if version.is_some() {
                                format!("{} {:8}", "CF".red(), id.dimmed())
                            } else {
                                format!("{} {:8}", "MR".green(), id.dimmed())
                            }
                        }
                        ModIdentifier::GitHubRepository(..) => "GH".purple().to_string(),
                    },
                    match &mod_.identifier {
                        ModIdentifier::ModrinthProject(_, _)
                        | ModIdentifier::CurseForgeProject(_, _) => mod_.name.bold().to_string(),
                        ModIdentifier::GitHubRepository((owner, repo), _) => {
                            format!("{}/{}", owner.dimmed(), repo.bold())
                        }
                    },
                );
            }
        }

        SubCommands::Remove { mod_names } => {
            let mut config = load_config()?;
            let mut profile = config.clone().into();

            remove(&mut profile, mod_names)?;

            if config.autoupdate {
                upgrade(&profile, false, &config.ferium.overrides).await?;
            }

            config.update(profile);
        }

        SubCommands::Disable { mod_names } => {
            let mut config = load_config()?;
            let mut profile = config.clone().into();

            disable(&mut profile, mod_names)?;

            if config.autoupdate {
                upgrade(&profile, false, &config.ferium.overrides).await?;
            }

            config.update(profile);
        }

        SubCommands::Upgrade => {
            let config = load_config()?;
            let profile = config.clone().into();

            upgrade(&profile, true, &config.ferium.overrides).await?;
        }

        SubCommands::Override {
            mod_name,
            identifier,
        } => {
            let mut config = load_config()?;

            let parsed_identifier: ModIdentifier = if identifier.contains('/') {
                let split = identifier
                    .split_once('/')
                    .ok_or_else(|| FerriteError::InvalidIdentifierFormat(identifier.clone()))?;
                ModIdentifier::GitHubRepository((split.0.to_string(), split.1.to_string()), None)
            } else if identifier.chars().all(|c| c.is_ascii_digit()) {
                ModIdentifier::CurseForgeProject(identifier.parse::<i32>()?, None)
            } else {
                ModIdentifier::ModrinthProject(identifier.clone(), None)
            };

            config
                .ferium
                .overrides
                .insert(mod_name.clone(), parsed_identifier);

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

            std::process::Command::new(parts[0])
                .args(&parts[1..])
                .stdin(std::process::Stdio::inherit())
                .stdout(std::process::Stdio::inherit())
                .stderr(std::process::Stdio::inherit())
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
