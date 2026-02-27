use libium::config::structs::{Mod, ModIdentifier, ModLoader};
use thiserror::Error;

use crate::config::FerriteConfig;

#[derive(Debug, Error)]
pub enum ScriptError {
    #[error("Invalid script: '{0}'. Available scripts: setup:quilt, setup:sinytra")]
    InvalidScript(String),
}

pub type Result<T> = std::result::Result<T, ScriptError>;

pub fn run(config: &mut FerriteConfig, script: &str) -> Result<()> {
    match script {
        "setup:quilt" => {
            config.ferium.overrides.insert(
                String::from("P7dR8mSH"),
                ModIdentifier::ModrinthProject(String::from("qvIfYCYJ"), None),
            );
            config.ferium.mod_loaders.push(ModLoader::Fabric);
        }
        "setup:sinytra" => {
            config.ferium.overrides.insert(
                String::from("P7dR8mSH"),
                ModIdentifier::ModrinthProject(String::from("Aqlf1Shp"), None),
            );
            config.ferium.mods.push(Mod::new(
                String::from("Connector Extras"),
                ModIdentifier::ModrinthProject(String::from("FYpiwiBR"), None),
                vec![],
                false,
            ));
            config.ferium.mod_loaders.push(ModLoader::Fabric);
        }
        _ => return Err(ScriptError::InvalidScript(script.to_string())),
    }

    Ok(())
}
