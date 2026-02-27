use colored::Colorize;
use ferinth::Ferinth;
use inquire::Select;
use libium::{
    CURSEFORGE_API, MODRINTH_API,
    config::structs::{Mod, ModIdentifier, ModLoader, Profile},
};
use std::fmt;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModCompatibility {
    Compatible,
    Uncompatible,
    Unknown,
}

impl fmt::Display for ModCompatibility {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModCompatibility::Compatible => write!(f, "compatible"),
            ModCompatibility::Uncompatible => write!(f, "uncompatible"),
            ModCompatibility::Unknown => write!(f, "unknown"),
        }
    }
}

use crate::{
    config::{ConfigError, FerriteConfig},
    server::{ServerError, get_server_jar},
    upgrade,
};

#[derive(Debug, Error)]
pub enum UpdateVersionError {
    #[error("Modrinth API error: {0}")]
    ModrinthApi(#[from] ferinth::Error),

    #[error("CurseForge API error: {0}")]
    CurseApi(#[from] furse::Error),

    #[error("User input error: {0}")]
    Input(String),

    #[error("User cancelled input")]
    Cancelled,

    #[error("Server error: {0}")]
    Server(#[from] ServerError),

    #[error("Mod loader {0} does not support Minecraft version {1}")]
    LoaderNotSupported(String, String),

    #[error("No compatible version found for mod {0}")]
    NoCompatibleVersion(String),

    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    #[error("Upgrade error: {0}")]
    Upgrade(#[from] upgrade::UpgradeError),
}

impl From<inquire::InquireError> for UpdateVersionError {
    fn from(e: inquire::InquireError) -> Self {
        match e {
            inquire::InquireError::OperationCanceled
            | inquire::InquireError::OperationInterrupted => UpdateVersionError::Cancelled,
            _ => UpdateVersionError::Input(e.to_string()),
        }
    }
}

pub type Result<T> = std::result::Result<T, UpdateVersionError>;

/// Fetches and sorts Minecraft versions from Modrinth
async fn fetch_minecraft_versions() -> Result<Vec<ferinth::structures::tag::GameVersion>> {
    let mut versions = Ferinth::default().tag_list_game_versions().await?;
    versions.sort_by(|a, b| {
        a.version_type
            .cmp(&b.version_type)
            .then(b.date.cmp(&a.date))
    });
    Ok(versions)
}

/// Prompts the user to select a Minecraft version
pub async fn pick_minecraft_version() -> Result<String> {
    let versions = fetch_minecraft_versions().await?;
    let display_versions: Vec<_> = versions
        .iter()
        .map(|v| {
            if v.major {
                v.version.bold().to_string()
            } else {
                v.version.clone()
            }
        })
        .collect();

    let selected = Select::new(
        "Which Minecraft version do you want to upgrade to?",
        display_versions,
    )
    .raw_prompt()?;

    Ok(versions[selected.index].version.clone())
}

/// Check if a mod loader supports a specific Minecraft version
async fn check_loader_support(loader: &ModLoader, version: &str) -> Result<bool> {
    match loader {
        ModLoader::Fabric => {
            let result = reqwest::get(format!(
                "https://meta.fabricmc.net/v2/versions/loader/{version}",
            ))
            .await;
            match result {
                Ok(response) => {
                    if response.status().is_success() {
                        let text = response.text().await.unwrap_or_default();
                        Ok(!text.is_empty() && text != "[]")
                    } else {
                        Ok(false)
                    }
                }
                Err(_) => Ok(false),
            }
        }
        ModLoader::Quilt => {
            todo!()
        }
        ModLoader::Forge => {
            let result = reqwest::get(
                "https://files.minecraftforge.net/net/minecraftforge/forge/maven-metadata.json",
            )
            .await;
            match result {
                Ok(response) => {
                    if response.status().is_success() {
                        let text = response.text().await.unwrap_or_default();
                        // Simple check if version is in the JSON response
                        Ok(text.contains(&format!("\"{version}\"")))
                    } else {
                        Ok(false)
                    }
                }
                Err(_) => Ok(false),
            }
        }
        ModLoader::NeoForge => {
            // NeoForge uses shortened version format (e.g., "1.20" instead of "1.20.1")
            let short_version = version.split('.').take(2).collect::<Vec<_>>().join(".");
            let result = reqwest::get(
                "https://maven.neoforged.net/releases/net/neoforged/neoforge/maven-metadata.xml",
            )
            .await;
            match result {
                Ok(response) => {
                    if response.status().is_success() {
                        let xml_content = response.text().await.unwrap_or_default();
                        // Check if any version starts with the short version
                        Ok(xml_content.contains(&format!("<version>{}.", short_version)))
                    } else {
                        Ok(false)
                    }
                }
                Err(_) => Ok(false),
            }
        }
        ModLoader::Velocity => {
            // Velocity is version-agnostic for the most part
            Ok(true)
        }
    }
}

/// Check if a mod is compatible with the target version
async fn check_mod_compatibility(
    mod_: &Mod,
    target_version: &str,
    _mod_loader: &ModLoader,
) -> Result<ModCompatibility> {
    match &mod_.identifier {
        ModIdentifier::ModrinthProject(id, _) => {
            let project = MODRINTH_API.project_get(id).await?;
            if project.game_versions.contains(&target_version.to_string()) {
                Ok(ModCompatibility::Compatible)
            } else {
                Ok(ModCompatibility::Uncompatible)
            }
        }
        ModIdentifier::CurseForgeProject(id, _) => {
            let project = CURSEFORGE_API.get_mod(*id).await?;
            if project
                .latest_files
                .iter()
                .any(|f| f.game_versions.contains(&target_version.to_string()))
            {
                Ok(ModCompatibility::Compatible)
            } else {
                Ok(ModCompatibility::Uncompatible)
            }
        }
        ModIdentifier::GitHubRepository(_, _) => {
            // GitHub mods are harder to check, we'll rely on the upgrade process
            Ok(ModCompatibility::Unknown)
        }
    }
}

/// Run the version upgrade process
pub async fn upgrade_version(
    config: &mut FerriteConfig,
    target_version: Option<String>,
) -> Result<()> {
    let current_version = &config.ferium.game_versions[0];

    let target_version = match target_version {
        Some(v) => v,
        None => pick_minecraft_version().await?,
    };

    if &target_version == current_version {
        println!("{}", "You're already on this version!".yellow());
        return Ok(());
    }

    println!(
        "\n{} {} → {}",
        "Upgrading Minecraft version:".bold(),
        current_version.red(),
        target_version.green()
    );

    // Check mod loader support
    let loader = &config.ferium.mod_loaders[0];
    println!(
        "\n{} Checking {} support for {}...",
        "●".cyan(),
        loader,
        target_version
    );

    let loader_supported = check_loader_support(loader, &target_version).await?;
    if !loader_supported {
        return Err(UpdateVersionError::LoaderNotSupported(
            loader.to_string(),
            target_version.clone(),
        ));
    }
    println!("{} {} supports {}", "✓".green(), loader, target_version);

    println!("\n{} Checking mod compatibility...", "●".cyan());
    let profile: Profile = config.clone().into();

    let mut incompatible_mods = Vec::new();
    let mut check_errors = Vec::new();

    for mod_ in &profile.mods {
        match check_mod_compatibility(mod_, &target_version, loader).await {
            Ok(ModCompatibility::Compatible) => {
                println!("  {} {} is compatible", "✓".green(), mod_.name);
            }
            Ok(ModCompatibility::Uncompatible) => {
                println!("  {} {} is not compatible", "✗".red(), mod_.name);
                incompatible_mods.push(mod_.name.clone());
            }
            Ok(ModCompatibility::Unknown) => {
                println!("  {} {} - compatibility unknown", "?".dimmed(), mod_.name);
            }
            Err(e) => {
                println!(
                    "  {} {} - couldn't check compatibility",
                    "✗".dimmed(),
                    mod_.name
                );
                check_errors.push((mod_.name.clone(), e.to_string()));
            }
        }
    }

    if !incompatible_mods.is_empty() {
        println!(
            "\n{} The following mods may not support {}:",
            "⚠ Warning:".yellow().bold(),
            target_version
        );
        for mod_name in &incompatible_mods {
            println!("  - {}", mod_name.red());
        }
    }

    if !check_errors.is_empty() {
        println!(
            "\n{} Could not verify compatibility for some mods:",
            "⚠".yellow()
        );
        for (mod_name, error) in &check_errors {
            println!("  - {}: {}", mod_name, error.dimmed());
        }
    }

    let confirm = inquire::Confirm::new(&format!(
        "Do you want to proceed with upgrading to {}?",
        target_version.green()
    ))
    .with_default(false)
    .prompt()?;

    if !confirm {
        println!("{}", "Upgrade cancelled.".yellow());
        return Ok(());
    }

    println!("\n{} Updating configuration...", "●".cyan());
    config.ferium.game_versions = vec![target_version.clone()];

    println!("{} Downloading new server jar...", "●".cyan());
    let server_installation = get_server_jar(&target_version, loader).await?;
    config.server.executable = server_installation.executable;
    config.server.wrapper = server_installation.wrapper;

    config.write_config()?;
    println!("{} Configuration updated", "✓".green());

    println!("\n{} Upgrading mods...", "●".cyan().bold());
    upgrade::upgrade(&profile, true, &config.ferium.overrides).await?;

    println!("\n{}", "✓ Upgrade complete!".green().bold());

    Ok(())
}
