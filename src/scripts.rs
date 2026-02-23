use anyhow::{Result, bail};
use libium::config::structs::{Mod, ModIdentifier, ModLoader};

use crate::config::FerriteConfig;

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
        _ => bail!("Invalid script"),
    }

    Ok(())
}
