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

#[derive(Debug, Deserialize)]
pub struct VelocityVersions {
    pub versions: Vec<VelocityVersion>,
}

#[derive(Debug, Deserialize)]
pub struct VelocityVersion {
    pub version: VelocityVersionInner,
}

#[derive(Debug, Deserialize)]
pub struct VelocityVersionInner {
    pub id: String,
}

#[derive(Debug, Deserialize)]
pub struct VelocityVersionBuild {
    pub downloads: VelocityVersionDownloads,
}

#[derive(Debug, Deserialize)]
pub struct VelocityVersionDownloads {
    #[serde(rename = "server:default")]
    pub server_default: VelocityVersionDownload,
}

#[derive(Debug, Deserialize)]
pub struct VelocityVersionDownload {
    pub url: String,
}
