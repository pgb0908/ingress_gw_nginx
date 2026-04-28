use crate::models::{
    GatewayDocument, PluginChainDocument, PolicyDocument, PolicyEntry, RevisionBundle, RevisionManifest, RouterDocument,
    ServiceDocument,
};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    let bytes = fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_slice(&bytes).with_context(|| format!("failed to parse {}", path.display()))
}

pub fn load_revision_bundle(path: &Path) -> Result<RevisionBundle> {
    let root = path.canonicalize().with_context(|| format!("failed to resolve {}", path.display()))?;
    let manifest: RevisionManifest = read_json(&root.join("revision.json"))?;
    let plugin_chain: PluginChainDocument = read_json(&root.join("plugin-chain.json"))?;
    let gateway: GatewayDocument = read_json(&root.join("gateway.json"))?;
    let listener = read_json(&root.join("listener.json"))?;

    let mut routers = Vec::new();
    for file in sorted_glob(&root, "router-")? {
        routers.push(read_json::<RouterDocument>(&file)?);
    }

    let mut services = HashMap::new();
    for file in sorted_glob(&root, "service-")? {
        let service: ServiceDocument = read_json(&file)?;
        services.insert(service.metadata.name.clone(), service);
    }

    let mut policies = Vec::new();
    for file in sorted_glob(&root, "policy-")? {
        policies.push(PolicyEntry {
            document: read_json::<PolicyDocument>(&file)?,
            source_file: file,
        });
    }
    policies.sort_by_key(|entry| entry.document.spec.order);

    Ok(RevisionBundle {
        root,
        manifest,
        gateway,
        listener,
        routers,
        services,
        policies,
        plugin_chain: plugin_chain.plugins,
    })
}

fn sorted_glob(root: &Path, prefix: &str) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in fs::read_dir(root)? {
        let path = entry?.path();
        if path.is_file() {
            if let Some(name) = path.file_name().and_then(|v| v.to_str()) {
                if name.starts_with(prefix) && name.ends_with(".json") {
                    files.push(path);
                }
            }
        }
    }
    files.sort();
    Ok(files)
}

