use clap::{Parser, Subcommand};
use libium::config::structs::ModLoader;

#[derive(Clone, Debug, Parser)]
#[clap(version, about = "Mod manager for Minecraft servers")]
pub struct Ferrite {
    #[clap(subcommand)]
    pub subcommand: SubCommands,
}

#[derive(Clone, Debug, Subcommand)]
pub enum SubCommands {
    #[clap(about = "Initialize a new ferrite.yaml configuration")]
    Init {
        #[clap(
            long,
            short = 'v',
            value_name = "VERSION",
            help = "Minecraft version(s), e.g. 1.20.1"
        )]
        game_versions: Option<Vec<String>>,

        #[clap(
            long,
            short,
            value_name = "LOADER",
            help = "Mod loader(s): quilt, fabric, forge, neoforge, velocity"
        )]
        mod_loaders: Option<Vec<ModLoader>>,
    },

    #[clap(about = "Start the Minecraft server")]
    Start,

    #[clap(visible_aliases = ["i", "install"], about = "Add mods by identifier")]
    Add {
        #[clap(
            required = true,
            value_name = "IDENTIFIER",
            help = "Mod identifier(s): Modrinth slug, CurseForge project ID, or GitHub 'owner/repo'"
        )]
        identifiers: Vec<String>,
    },

    #[clap(visible_alias = "rm", about = "Remove mods by name")]
    Remove {
        #[clap(required = true, value_name = "NAME", help = "Mod name(s) to remove")]
        mod_names: Vec<String>,
    },

    #[clap(about = "Disable mods by name (moves to disabled list)")]
    Disable {
        #[clap(required = true, value_name = "NAME", help = "Mod name(s) to disable")]
        mod_names: Vec<String>,
    },

    #[clap(about = "Override a mod's version or source")]
    Override {
        #[clap(help = "Name of the mod to override")]
        mod_name: String,

        #[clap(
            help = "New identifier: Modrinth slug, CurseForge project ID, or GitHub 'owner/repo'"
        )]
        identifier: String,
    },

    #[clap(about = "Run a setup script")]
    Script {
        #[clap(help = "Script name: setup:quilt, setup:sinytra")]
        script: String,
    },

    #[clap(visible_alias = "ls", about = "List all installed mods")]
    List,

    #[clap(
        visible_alias = "update",
        about = "Upgrade all mods to latest versions"
    )]
    Upgrade,

    #[clap(about = "Upgrade Minecraft version and update mods")]
    UpdateVersion {
        #[clap(
            long,
            short = 'v',
            value_name = "VERSION",
            help = "Target Minecraft version, e.g. 1.20.1"
        )]
        version: Option<String>,
    },
}
