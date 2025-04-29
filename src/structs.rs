use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct FabricLoaderEntry {
    pub loader: FabricLoaderEntryLoader,
}

#[derive(Debug, Deserialize)]
pub struct FabricLoaderEntryLoader {
    pub stable: bool,
    pub version: String,
}

#[derive(Debug, Deserialize)]
pub struct NeoForgeLoaderMetadata {
    pub versioning: NeoForgeLoaderVersioning,
}

#[derive(Debug, Deserialize)]
pub struct NeoForgeLoaderVersioning {
    pub versions: NeoForgeVersions,
}

#[derive(Debug, Deserialize)]
pub struct NeoForgeVersions {
    pub version: Vec<String>,
}
