use anyhow::{Result, bail};
use libium::config::structs::{ModIdentifier, ModLoader};

use crate::FerriteConfig;

pub fn run(config: &mut FerriteConfig, script: &str) -> Result<()> {
    match script {
        "setup:quilt" => {
            config.ferium.overrides.insert(
                String::from("P7dR8mSH"),
                ModIdentifier::ModrinthProject(String::from("qvIfYCYJ")),
            );
            config.ferium.mod_loaders.push(ModLoader::Fabric);
        }
        _ => bail!("Invalid script"),
    }

    Ok(())
}
