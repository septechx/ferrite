mod download;
mod installers;

pub use download::*;
pub use installers::*;

use anyhow::Result;
use indicatif::ProgressBar;
use libium::config::structs::ModLoader;

/// Represents a server installation result
pub struct ServerInstallation {
    pub executable: String,
    pub wrapper: String,
}

/// Downloads and installs a server for the given game version and mod loader
pub async fn get_server_jar(
    game_version: &str,
    mod_loader: &ModLoader,
) -> Result<ServerInstallation> {
    let progress_bar = create_progress_bar(&format!(
        "Downloading server jar for {} ({})",
        game_version, mod_loader
    ));

    match mod_loader {
        ModLoader::Fabric => install_fabric_server(game_version, &progress_bar).await,
        ModLoader::Forge => install_forge_server(game_version, &progress_bar).await,
        ModLoader::Quilt => install_quilt_server(game_version, &progress_bar).await,
        ModLoader::NeoForge => install_neoforge_server(game_version, &progress_bar).await,
        ModLoader::Velocity => install_velocity_proxy(game_version, &progress_bar).await,
    }
}

/// Creates a progress bar with a spinner and message
fn create_progress_bar(message: &str) -> ProgressBar {
    let style = indicatif::ProgressStyle::default_spinner()
        .template("{spinner} {msg}")
        .expect("Progress bar template parse failure");
    let progress_bar = ProgressBar::new_spinner().with_style(style);
    progress_bar.set_message(message.to_string());
    progress_bar.enable_steady_tick(std::time::Duration::from_millis(100));
    progress_bar
}
