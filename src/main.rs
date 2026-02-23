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
use config::{Config, File};
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
            version: 2,
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
        let serialized = serde_norway::to_string(self)?;

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

        SubCommands::Override {
            mod_name,
            identifier,
        } => {
            let mut config = load_config()?;

            let parsed_identifier: ModIdentifier = if identifier.contains('/') {
                let split = identifier.split_once('/').unwrap();
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

fn upgrade_config_to_v3(content: &str) -> String {
    let yaml: serde_norway::Value = serde_norway::from_str(content).unwrap();

    fn convert_identifier_to_tagged(value: &serde_norway::Value) -> serde_norway::Value {
        match value {
            serde_norway::Value::String(s) => serde_norway::Value::Mapping(
                [(
                    serde_norway::Value::String("ModrinthProject".to_string()),
                    serde_norway::Value::String(s.clone()),
                )]
                .into_iter()
                .collect(),
            ),
            serde_norway::Value::Number(n) => serde_norway::Value::Mapping(
                [(
                    serde_norway::Value::String("CurseForgeProject".to_string()),
                    serde_norway::Value::Number(n.clone()),
                )]
                .into_iter()
                .collect(),
            ),
            serde_norway::Value::Sequence(seq) if seq.len() == 2 => serde_norway::Value::Mapping(
                [(
                    serde_norway::Value::String("GitHubRepository".to_string()),
                    serde_norway::Value::Sequence(seq.clone()),
                )]
                .into_iter()
                .collect(),
            ),
            serde_norway::Value::Mapping(map)
                if map.contains_key(&serde_norway::Value::String("github".to_string())) =>
            {
                if let Some(serde_norway::Value::Mapping(github_map)) =
                    map.get(&serde_norway::Value::String("github".to_string()))
                {
                    let owner = github_map
                        .get(&serde_norway::Value::String("owner".to_string()))
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let repo = github_map
                        .get(&serde_norway::Value::String("repo".to_string()))
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    serde_norway::Value::Mapping(
                        [(
                            serde_norway::Value::String("GitHubRepository".to_string()),
                            serde_norway::Value::Sequence(vec![
                                serde_norway::Value::String(owner.to_string()),
                                serde_norway::Value::String(repo.to_string()),
                            ]),
                        )]
                        .into_iter()
                        .collect(),
                    )
                } else {
                    value.clone()
                }
            }
            _ => value.clone(),
        }
    }

    fn convert_mod_list(mods: &[serde_norway::Value]) -> Vec<serde_norway::Value> {
        mods.iter()
            .map(|m| {
                if let serde_norway::Value::Mapping(map) = m {
                    let mut new_map = map.clone();
                    if let Some(identifier) =
                        map.get(serde_norway::Value::String("identifier".to_string()))
                    {
                        new_map.insert(
                            serde_norway::Value::String("identifier".to_string()),
                            convert_identifier_to_tagged(identifier),
                        );
                    }
                    serde_norway::Value::Mapping(new_map)
                } else {
                    m.clone()
                }
            })
            .collect()
    }

    if let serde_norway::Value::Mapping(root) = &yaml {
        let mut new_root = root.clone();

        if let Some(ferium) = root.get(serde_norway::Value::String("ferium".to_string()))
            && let serde_norway::Value::Mapping(ferium_map) = ferium
        {
            let mut new_ferium = ferium_map.clone();

            if let Some(overrides) =
                ferium_map.get(serde_norway::Value::String("overrides".to_string()))
                && let serde_norway::Value::Mapping(overrides_map) = overrides
            {
                let mut new_overrides = serde_norway::Mapping::new();
                for (key, value) in overrides_map {
                    new_overrides.insert(key.clone(), convert_identifier_to_tagged(value));
                }
                new_ferium.insert(
                    serde_norway::Value::String("overrides".to_string()),
                    serde_norway::Value::Mapping(new_overrides),
                );
            }

            if let Some(mods) = ferium_map.get(serde_norway::Value::String("mods".to_string()))
                && let serde_norway::Value::Sequence(mods_seq) = mods
            {
                new_ferium.insert(
                    serde_norway::Value::String("mods".to_string()),
                    serde_norway::Value::Sequence(convert_mod_list(mods_seq)),
                );
            }

            if let Some(disabled) =
                ferium_map.get(serde_norway::Value::String("disabled".to_string()))
                && let serde_norway::Value::Sequence(disabled_seq) = disabled
            {
                new_ferium.insert(
                    serde_norway::Value::String("disabled".to_string()),
                    serde_norway::Value::Sequence(convert_mod_list(disabled_seq)),
                );
            }

            new_root.insert(
                serde_norway::Value::String("ferium".to_string()),
                serde_norway::Value::Mapping(new_ferium),
            );
        }

        new_root.insert(
            serde_norway::Value::String("version".to_string()),
            serde_norway::Value::Number(serde_norway::Number::from(3)),
        );

        serde_norway::to_string(&serde_norway::Value::Mapping(new_root))
            .unwrap_or_else(|_| content.to_string())
    } else {
        content.to_string()
    }
}

fn load_config() -> Result<FerriteConfig> {
    let config_content = fs::read_to_string("ferrite.yaml")?;

    let version: i64 = serde_norway::from_str::<serde_norway::Value>(&config_content)
        .ok()
        .and_then(|v| v.get("version").and_then(|vv| vv.as_i64()))
        .unwrap_or(0);

    if version < 3 {
        println!(
            "{} Detected config version {}. Auto-upgrading to version 3...",
            "⚠".yellow(),
            version
        );
        let upgraded = upgrade_config_to_v3(&config_content);
        fs::write("ferrite.yaml", &upgraded)?;
        println!(
            "{} Config upgraded to version 3. Please review the changes.",
            "✓".green()
        );

        let serialized = Config::builder()
            .add_source(File::with_name("ferrite").required(true))
            .build()?;

        let config: FerriteConfig = serialized.try_deserialize()?;
        return Ok(config);
    }

    let serialized = Config::builder()
        .add_source(File::with_name("ferrite").required(true))
        .build()?;

    let config: FerriteConfig = match version {
        3 => Ok(serialized.try_deserialize()?),
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
