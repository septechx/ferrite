mod download;
mod installers;

pub use download::*;
pub use installers::*;

use anyhow::Result;
use indicatif::ProgressBar;
use libium::config::structs::ModLoader;

pub struct ServerInstallation {
    pub executable: String,
    pub wrapper: String,
}

pub async fn get_server_jar(
    game_version: &str,
    mod_loader: &ModLoader,
) -> Result<ServerInstallation> {
    let progress_bar = create_progress_bar(&format!(
        "Downloading server jar for {game_version} ({mod_loader})"
    ));

    match mod_loader {
        ModLoader::Fabric => FabricInstaller::install(game_version, &progress_bar).await,
        ModLoader::Forge => ForgeInstaller::install(game_version, &progress_bar).await,
        ModLoader::Quilt => QuiltInstaller::install(game_version, &progress_bar).await,
        ModLoader::NeoForge => NeoForgeInstaller::install(game_version, &progress_bar).await,
        ModLoader::Velocity => VelocityInstaller::install(game_version, &progress_bar).await,
    }
}

fn create_progress_bar(message: &str) -> ProgressBar {
    let style = indicatif::ProgressStyle::default_spinner()
        .template("{spinner} {msg}")
        .expect("Progress bar template parse failure");
    let progress_bar = ProgressBar::new_spinner().with_style(style);
    progress_bar.set_message(message.to_string());
    progress_bar.enable_steady_tick(std::time::Duration::from_millis(100));
    progress_bar
}
