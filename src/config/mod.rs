mod error;
mod types;
mod upgrade;

pub use error::{ConfigError, Result};
pub use types::{FerriteConfig, KeyStoreConfig};
pub use upgrade::{detect_config_version, needs_upgrade, upgrade_config};

use colored::Colorize;
use dotenvy::dotenv;
use std::{env, fs, io::Write, process::Command};

pub fn load_config() -> Result<FerriteConfig> {
    let mut config_content = fs::read_to_string("ferrite.yaml")?;
    let mut version = detect_config_version(&config_content);
    let original_version = version;

    if needs_upgrade(version) {
        println!(
            "{} Upgrading config from version {} to {}...",
            "⚠".yellow(),
            original_version,
            version
        );
    }

    while needs_upgrade(version) {
        if let Some(upgraded) = upgrade_config(&config_content, version) {
            config_content = upgraded;
            version += 1;
        } else {
            break;
        }
    }

    if version > original_version {
        fs::write("ferrite.yaml", &config_content)?;
        println!("{} Config upgraded to version {}.", "✓".green(), version);
    }

    let config: FerriteConfig =
        serde_norway::from_str(&config_content).map_err(|e| ConfigError::Parse(e.to_string()))?;

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
                .map_err(|e| ConfigError::PassCommand(format!("github_token: {}", e)))?
                .stdout;

            let token_str = String::from_utf8_lossy(&gh_token).trim().to_string();
            unsafe {
                env::set_var("GITHUB_TOKEN", token_str);
            }

            let cf_api_key = Command::new("pass")
                .arg("ferrite/curseforge_api_key")
                .output()
                .map_err(|e| ConfigError::PassCommand(format!("curseforge_api_key: {}", e)))?
                .stdout;

            let key_str = String::from_utf8_lossy(&cf_api_key).trim().to_string();
            unsafe {
                env::set_var("CURSEFORGE_API_KEY", key_str);
            }
        }
    }

    Ok(config)
}
