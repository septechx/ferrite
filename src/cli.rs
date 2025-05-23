use clap::{Parser, Subcommand};
use libium::config::structs::ModLoader;

#[derive(Clone, Debug, Parser)]
#[clap(version)]
pub struct Ferrite {
    #[clap(subcommand)]
    pub subcommand: SubCommands,
}

#[derive(Clone, Debug, Subcommand)]
pub enum SubCommands {
    Init {
        #[clap(long, short = 'v')]
        game_versions: Option<Vec<String>>,

        #[clap(long, short)]
        mod_loaders: Option<Vec<ModLoader>>,
    },

    Start,

    #[clap(visible_aliases = ["i", "install"])]
    Add {
        #[clap(required = true)]
        identifiers: Vec<String>,
    },

    #[clap(visible_alias = "rm")]
    Remove {
        #[clap(required = true)]
        mod_names: Vec<String>,
    },

    Disable {
        #[clap(required = true)]
        mod_names: Vec<String>,
    },

    Override {
        #[clap(required = true)]
        mod_override: Vec<String>,
    },

    Script {
        #[clap(required = true)]
        script: String,
    },

    #[clap(visible_alias = "ls")]
    List,

    #[clap(visible_alias = "update")]
    Upgrade,
}
