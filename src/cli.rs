use clap::{Parser, Subcommand, ValueHint};
use libium::config::structs::ModLoader;
use std::path::PathBuf;

#[derive(Clone, Debug, Parser)]
pub struct Ferrite {
    #[clap(subcommand)]
    pub subcommand: SubCommands,
}

#[derive(Clone, Debug, Subcommand)]
pub enum SubCommands {
    Init {
        #[clap(long, short = 'v')]
        game_version: Vec<String>,

        #[clap(long, short)]
        #[clap(value_enum)]
        mod_loader: Option<ModLoader>,

        #[clap(long, short)]
        name: Option<String>,

        #[clap(long, short)]
        #[clap(value_hint(ValueHint::DirPath))]
        output_dir: Option<PathBuf>,
    },
    Add {
        #[clap(required = true)]
        identifiers: Vec<String>,
    },
    #[clap(visible_alias = "ls")]
    List {
        #[clap(long, short)]
        verbose: bool,
    },
    #[clap(visible_alias = "rm")]
    Remove { mod_names: Vec<String> },
    #[clap(visible_aliases = ["update"])]
    Upgrade,
}
