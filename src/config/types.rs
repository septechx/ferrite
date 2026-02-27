use super::error::Result;
use libium::config::structs::{Mod, ModIdentifier, ModLoader, Profile};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, env, fs, io::Write};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FerriteConfig {
    pub version: i64,
    pub autoupdate: bool,
    pub output_path: String,
    pub key_store: KeyStoreConfig,
    pub server: ServerConfig,
    pub ferium: FeriumConfig,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ServerConfig {
    pub wrapper: String,
    pub executable: String,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub enum KeyStoreConfig {
    Pass,
    DotEnv,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FeriumConfig {
    pub game_versions: Vec<String>,
    pub mod_loaders: Vec<ModLoader>,
    pub overrides: HashMap<String, ModIdentifier>,
    pub mods: Vec<Mod>,
    pub disabled: Vec<Mod>,
}

impl FerriteConfig {
    pub fn new(
        game_versions: Vec<String>,
        mod_loaders: Vec<ModLoader>,
        wrapper: String,
        executable: String,
    ) -> Self {
        let output_path = if mod_loaders.contains(&ModLoader::Velocity) {
            "plugins".to_string()
        } else {
            "mods".to_string()
        };
        Self {
            version: 4,
            autoupdate: true,
            output_path,
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
        let current_dir = env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        Self::new_complete(
            String::from("ferrite"),
            current_dir.join(&config.output_path),
            config.ferium.game_versions,
            config.ferium.mod_loaders,
            config.ferium.mods,
            config.ferium.disabled,
        )
    }
}
