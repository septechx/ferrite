use anyhow::{Result, bail};
use colored::Colorize as _;
use inquire::MultiSelect;
use libium::{
    config::structs::{ModIdentifier, Profile},
    iter_ext::IterExt as _,
};

/// If `to_disable` is empty, display a list of projects in the profile to select from and disable selected ones
///
/// Else, search the given strings with the projects' name and IDs and disable them
pub fn disable(profile: &mut Profile, to_disable: Vec<String>) -> Result<()> {
    let mut indices_to_disable = if to_disable.is_empty() {
        let mod_info = profile
            .mods
            .iter()
            .map(|mod_| {
                format!(
                    "{:11}  {}",
                    match &mod_.identifier {
                        ModIdentifier::CurseForgeProject(id)
                        | ModIdentifier::PinnedCurseForgeProject(id, _) =>
                            format!("CF {:8}", id.to_string()),
                        ModIdentifier::ModrinthProject(id)
                        | ModIdentifier::PinnedModrinthProject(id, _) =>
                            format!("MR {:8}", id.to_string()),
                        ModIdentifier::GitHubRepository(..)
                        | ModIdentifier::PinnedGitHubRepository(..) => format!("GH {:8}", "…"),
                    },
                    match &mod_.identifier {
                        ModIdentifier::ModrinthProject(_)
                        | ModIdentifier::CurseForgeProject(_)
                        | ModIdentifier::PinnedModrinthProject(_, _)
                        | ModIdentifier::PinnedCurseForgeProject(_, _) => mod_.name.clone(),
                        ModIdentifier::GitHubRepository(owner, repo)
                        | ModIdentifier::PinnedGitHubRepository((owner, repo), _) => {
                            format!("{owner}/{repo}")
                        }
                    },
                )
            })
            .collect_vec();
        MultiSelect::new("Select mods to disable", mod_info.clone())
            .raw_prompt_skippable()?
            .unwrap_or_default()
            .iter()
            .map(|o| o.index)
            .collect_vec()
    } else {
        let mut items_to_disable = Vec::new();
        for to_disable in to_disable {
            if let Some(index) = profile.mods.iter().position(|mod_| {
                mod_.name.eq_ignore_ascii_case(&to_disable)
                    || match &mod_.identifier {
                        ModIdentifier::CurseForgeProject(id)
                        | ModIdentifier::PinnedCurseForgeProject(id, _) => {
                            id.to_string() == to_disable
                        }
                        ModIdentifier::ModrinthProject(id)
                        | ModIdentifier::PinnedModrinthProject(id, _) => id == &to_disable,
                        ModIdentifier::GitHubRepository(owner, name)
                        | ModIdentifier::PinnedGitHubRepository((owner, name), _) => {
                            format!("{owner}/{name}").eq_ignore_ascii_case(&to_disable)
                        }
                    }
                    || mod_
                        .slug
                        .as_ref()
                        .is_some_and(|slug| to_disable.eq_ignore_ascii_case(slug))
            }) {
                items_to_disable.push(index);
            } else {
                bail!("A mod with ID or name {to_disable} is not present in this profile");
            }
        }
        items_to_disable
    };

    // Sort the indices in ascending order to fix moving indices during disabling
    indices_to_disable.sort_unstable();
    indices_to_disable.reverse();

    let mut disabled = Vec::new();
    for index in indices_to_disable {
        let mod_ = profile.mods.swap_remove(index);
        disabled.push(mod_.name.clone());
        profile.disabled.push(mod_);
    }

    if !disabled.is_empty() {
        println!(
            "Disabled {}",
            disabled.iter().map(|txt| txt.bold()).display(", ")
        );
    }

    Ok(())
}
