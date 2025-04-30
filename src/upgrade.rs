use crate::download::{clean, download};
use anyhow::{Result, anyhow, bail};
use colored::Colorize as _;
use indicatif::{ProgressBar, ProgressStyle};
use libium::{
    config::{
        filters::ProfileParameters as _,
        structs::{Mod, ModIdentifier, ModLoader, Profile},
    },
    upgrade::{DownloadData, mod_downloadable},
};
use parking_lot::Mutex;
use std::{
    collections::HashMap,
    fs::{self, read_dir},
    mem::take,
    sync::{Arc, mpsc},
    time::Duration,
};
use tokio::task::JoinSet;

/// Get the latest compatible downloadable for the mods in `profile`
///
/// If an error occurs with a resolving task, instead of failing immediately,
/// resolution will continue and the error return flag is set to true.
pub async fn get_platform_downloadables(
    profile: &Profile,
    user: bool,
    overrides: &HashMap<String, ModIdentifier>,
) -> Result<(Vec<DownloadData>, bool)> {
    let style = ProgressStyle::default_bar()
        .template("{spinner} {elapsed} [{wide_bar:.cyan/blue}] {pos:.cyan}/{len:.blue}")
        .expect("Progress bar template parse failure")
        .progress_chars("#>-");
    let progress_bar = Arc::new(Mutex::new(ProgressBar::new(0).with_style(style)));
    let mut tasks = JoinSet::new();
    let mut done_mods = Vec::new();
    let (mod_sender, mod_rcvr) = mpsc::channel();

    // Wrap it again in an Arc so that I can count the references to it,
    // because I cannot drop the main thread's sender due to the recursion
    let mod_sender = Arc::new(mod_sender);

    if user {
        println!("{}\n", "Determining the Latest Compatible Versions".bold());
    }
    progress_bar
        .lock()
        .enable_steady_tick(Duration::from_millis(100));
    let pad_len = profile
        .mods
        .iter()
        .map(|m| m.name.len())
        .max()
        .unwrap_or(20)
        .clamp(20, 50);

    for mod_ in profile.mods.clone() {
        mod_sender.send(mod_)?;
    }

    let mut initial = true;

    // A race condition exists where if the last task drops its sender before this thread receives the message,
    // that particular message will get ignored. I used the ostrich algorithm to solve this.

    // `initial` accounts for the edge case where at first,
    // no tasks have been spawned yet but there are messages in the channel
    // TODO: Fix bug where if mods is empty initial will never be false and this loop will run for
    // ever
    while Arc::strong_count(&mod_sender) > 1 || initial {
        if let Ok(mod_) = mod_rcvr.try_recv() {
            initial = false;

            if done_mods.contains(&mod_.identifier) {
                continue;
            }

            done_mods.push(mod_.identifier.clone());
            progress_bar.lock().inc_length(1);

            let filters = profile.filters.clone();
            let overrides = overrides.clone();
            let dep_sender = Arc::clone(&mod_sender);
            let progress_bar = Arc::clone(&progress_bar);

            tasks.spawn(async move {
                let result = mod_.fetch_download_file(filters).await;

                progress_bar.lock().inc(1);
                match result {
                    Ok(mut download_file) => {
                        progress_bar.lock().println(format!(
                            "{} {:pad_len$}  {}",
                            "✓".green(),
                            mod_.name,
                            download_file.filename().dimmed()
                        ));
                        for dep in take(&mut download_file.dependencies) {
                            let override_identifier = match &dep {
                                ModIdentifier::ModrinthProject(id) => id.clone(),
                                ModIdentifier::CurseForgeProject(id) => id.to_string(),
                                ModIdentifier::GitHubRepository(user, repo) => {
                                    format!("{}/{}", user, repo)
                                }
                                _ => todo!(),
                            };

                            let mut identifier = dep;
                            if let Some(override_) = overrides.get(&override_identifier) {
                                identifier = override_.clone();
                            };

                            dep_sender.send(Mod::new(
                                format!(
                                    "Dependency: {}",
                                    match &identifier {
                                        ModIdentifier::CurseForgeProject(id) => id.to_string(),
                                        ModIdentifier::ModrinthProject(id)
                                        | ModIdentifier::PinnedModrinthProject(id, _) =>
                                            id.to_owned(),
                                        _ => unreachable!(),
                                    }
                                ),
                                identifier,
                                vec![],
                                false,
                            ))?;
                        }
                        Ok(Some(download_file))
                    }
                    Err(err) => {
                        if let mod_downloadable::Error::ModrinthError(
                            ferinth::Error::RateLimitExceeded(_),
                        ) = err
                        {
                            // Immediately fail if the rate limit has been exceeded
                            progress_bar.lock().finish_and_clear();
                            bail!(err);
                        }
                        progress_bar.lock().println(format!(
                            "{}",
                            format!("× {:pad_len$}  {err}", mod_.name).red()
                        ));
                        Ok(None)
                    }
                }
            });
        }
    }

    Arc::try_unwrap(progress_bar)
        .map_err(|_| anyhow!("Failed to run threads to completion"))?
        .into_inner()
        .finish_and_clear();

    let tasks = tasks
        .join_all()
        .await
        .into_iter()
        .collect::<Result<Vec<_>>>()?;

    let error = tasks.iter().any(Option::is_none);
    let to_download = tasks.into_iter().flatten().collect();

    Ok((to_download, error))
}

pub async fn upgrade(
    profile: &Profile,
    user: bool,
    overrides: &HashMap<String, ModIdentifier>,
) -> Result<()> {
    let (mut to_download, error) = get_platform_downloadables(profile, user, overrides).await?;
    let mut to_install = Vec::new();
    if profile.output_dir.join("user").exists()
        && profile.filters.mod_loader() != Some(&ModLoader::Quilt)
    {
        for file in read_dir(profile.output_dir.join("user"))? {
            let file = file?;
            let path = file.path();
            if path.is_file()
                && path
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("jar"))
            {
                to_install.push((file.file_name(), path));
            }
        }
    }

    if !profile.output_dir.exists() {
        fs::create_dir(&profile.output_dir)?;
    }

    let disabled_slugs = profile
        .disabled
        .iter()
        .map(|m| m.slug.clone().unwrap())
        .collect::<Vec<_>>();

    for entry in read_dir(&profile.output_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext.eq_ignore_ascii_case("jar") {
                    if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {
                        if disabled_slugs.contains(&filename.to_string()) {
                            let new_path = path.with_file_name(format!("{}.disabled", filename));
                            fs::rename(&path, new_path)?;
                        }
                    }
                } else if ext.eq_ignore_ascii_case(".disabled") {
                    if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {
                        if !disabled_slugs.contains(&filename.to_string()) {
                            fs::remove_file(path)?;
                        }
                    }
                }
            }
        }
    }

    clean(&profile.output_dir, &mut to_download, &mut to_install).await?;
    to_download
        .iter_mut()
        // Download directly to the output directory
        .map(|thing| thing.output = thing.filename().into())
        .for_each(drop); // Doesn't drop any data, just runs the iterator
    if to_download.is_empty() && to_install.is_empty() {
        println!("\n{}", "All up to date!".bold());
    } else {
        println!("\n{}\n", "Downloading Mod Files".bold());
        download(profile.output_dir.clone(), to_download, to_install).await?;
    }

    // TODO: Fix error logging
    if error && false {
        Err(anyhow!(
            "\nCould not get the latest compatible version of some mods"
        ))
    } else {
        Ok(())
    }
}
