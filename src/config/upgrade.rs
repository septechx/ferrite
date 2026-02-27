const LATEST_CONFIG_VERSION: i64 = 4;

pub fn detect_config_version(content: &str) -> i64 {
    serde_norway::from_str::<serde_norway::Value>(content)
        .ok()
        .and_then(|v| v.get("version").and_then(|vv| vv.as_i64()))
        .unwrap_or(0)
}

pub fn upgrade_config(content: &str, from_version: i64) -> Option<String> {
    match from_version {
        v if v < 3 => Some(upgrade_config_to_v3(content)),
        3 => Some(upgrade_config_to_v4(content)),
        _ => None,
    }
}

pub fn needs_upgrade(version: i64) -> bool {
    version < LATEST_CONFIG_VERSION
}

fn parse_yaml(content: &str) -> serde_norway::Value {
    serde_norway::from_str(content).unwrap_or(serde_norway::Value::Null)
}

fn upgrade_config_to_v3(content: &str) -> String {
    let yaml = parse_yaml(content);

    fn convert_identifier_to_tagged(value: &serde_norway::Value) -> serde_norway::Value {
        match value {
            serde_norway::Value::String(s) => serde_norway::Value::Mapping(
                [(
                    serde_norway::Value::String("ModrinthProject".to_string()),
                    serde_norway::Value::String(s.clone()),
                )]
                .into_iter()
                .collect(),
            ),
            serde_norway::Value::Number(n) => serde_norway::Value::Mapping(
                [(
                    serde_norway::Value::String("CurseForgeProject".to_string()),
                    serde_norway::Value::Number(n.clone()),
                )]
                .into_iter()
                .collect(),
            ),
            serde_norway::Value::Sequence(seq) if seq.len() == 2 => serde_norway::Value::Mapping(
                [(
                    serde_norway::Value::String("GitHubRepository".to_string()),
                    serde_norway::Value::Sequence(seq.clone()),
                )]
                .into_iter()
                .collect(),
            ),
            serde_norway::Value::Mapping(map)
                if map.contains_key(serde_norway::Value::String("github".to_string())) =>
            {
                if let Some(serde_norway::Value::Mapping(github_map)) =
                    map.get(serde_norway::Value::String("github".to_string()))
                {
                    let owner = github_map
                        .get(serde_norway::Value::String("owner".to_string()))
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let repo = github_map
                        .get(serde_norway::Value::String("repo".to_string()))
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    serde_norway::Value::Mapping(
                        [(
                            serde_norway::Value::String("GitHubRepository".to_string()),
                            serde_norway::Value::Sequence(vec![
                                serde_norway::Value::String(owner.to_string()),
                                serde_norway::Value::String(repo.to_string()),
                            ]),
                        )]
                        .into_iter()
                        .collect(),
                    )
                } else {
                    value.clone()
                }
            }
            _ => value.clone(),
        }
    }

    fn convert_mod_list(mods: &[serde_norway::Value]) -> Vec<serde_norway::Value> {
        mods.iter()
            .map(|m| {
                if let serde_norway::Value::Mapping(map) = m {
                    let mut new_map = map.clone();
                    if let Some(identifier) =
                        map.get(serde_norway::Value::String("identifier".to_string()))
                    {
                        new_map.insert(
                            serde_norway::Value::String("identifier".to_string()),
                            convert_identifier_to_tagged(identifier),
                        );
                    }
                    serde_norway::Value::Mapping(new_map)
                } else {
                    m.clone()
                }
            })
            .collect()
    }

    if let serde_norway::Value::Mapping(root) = &yaml {
        let mut new_root = root.clone();

        if let Some(ferium) = root.get(serde_norway::Value::String("ferium".to_string()))
            && let serde_norway::Value::Mapping(ferium_map) = ferium
        {
            let mut new_ferium = ferium_map.clone();

            if let Some(overrides) =
                ferium_map.get(serde_norway::Value::String("overrides".to_string()))
                && let serde_norway::Value::Mapping(overrides_map) = overrides
            {
                let mut new_overrides = serde_norway::Mapping::new();
                for (key, value) in overrides_map {
                    new_overrides.insert(key.clone(), convert_identifier_to_tagged(value));
                }
                new_ferium.insert(
                    serde_norway::Value::String("overrides".to_string()),
                    serde_norway::Value::Mapping(new_overrides),
                );
            }

            if let Some(mods) = ferium_map.get(serde_norway::Value::String("mods".to_string()))
                && let serde_norway::Value::Sequence(mods_seq) = mods
            {
                new_ferium.insert(
                    serde_norway::Value::String("mods".to_string()),
                    serde_norway::Value::Sequence(convert_mod_list(mods_seq)),
                );
            }

            if let Some(disabled) =
                ferium_map.get(serde_norway::Value::String("disabled".to_string()))
                && let serde_norway::Value::Sequence(disabled_seq) = disabled
            {
                new_ferium.insert(
                    serde_norway::Value::String("disabled".to_string()),
                    serde_norway::Value::Sequence(convert_mod_list(disabled_seq)),
                );
            }

            new_root.insert(
                serde_norway::Value::String("ferium".to_string()),
                serde_norway::Value::Mapping(new_ferium),
            );
        }

        new_root.insert(
            serde_norway::Value::String("version".to_string()),
            serde_norway::Value::Number(serde_norway::Number::from(3)),
        );

        serde_norway::to_string(&serde_norway::Value::Mapping(new_root))
            .unwrap_or_else(|_| content.to_string())
    } else {
        content.to_string()
    }
}

fn upgrade_config_to_v4(content: &str) -> String {
    let yaml = parse_yaml(content);

    if let serde_norway::Value::Mapping(root) = &yaml {
        let mut new_root = root.clone();

        let has_velocity = root
            .get(serde_norway::Value::String("ferium".to_string()))
            .and_then(|ferium| ferium.get("mod_loaders"))
            .and_then(|loaders| loaders.as_sequence())
            .map(|loaders| {
                loaders.iter().any(|l| {
                    l.as_str()
                        .map(|s| s.to_lowercase() == "velocity")
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false);

        let output_path = if has_velocity { "plugins" } else { "mods" };

        new_root.insert(
            serde_norway::Value::String("output_path".to_string()),
            serde_norway::Value::String(output_path.to_string()),
        );

        new_root.insert(
            serde_norway::Value::String("version".to_string()),
            serde_norway::Value::Number(serde_norway::Number::from(4)),
        );

        serde_norway::to_string(&serde_norway::Value::Mapping(new_root))
            .unwrap_or_else(|_| content.to_string())
    } else {
        content.to_string()
    }
}
