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
