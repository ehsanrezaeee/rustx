//! Bounded logical asset names shared by browser and native applications.
use crate::{HostError, HttpClient, Scope, MAX_HTTP_BYTES};
use serde::Serialize;
use std::{collections::BTreeMap, fmt};

pub const MAX_ASSET_ENTRIES: usize = 256;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum AssetKind {
    PngImage,
    Font,
    Bytes,
}
#[derive(Clone, Debug, Serialize)]
pub struct AssetEntry {
    pub id: String,
    pub url: String,
    pub revision: String,
    pub kind: AssetKind,
    pub max_bytes: usize,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AssetError {
    InvalidManifest,
    DuplicateId,
    UnknownId,
    Transport(HostError),
    Decode,
    ResourceLimit,
    Unsupported,
}
#[derive(Clone)]
pub struct AssetData {
    pub entry: AssetEntry,
    pub bytes: Vec<u8>,
}
impl fmt::Debug for AssetData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AssetData")
            .field("id", &self.entry.id)
            .field("revision", &self.entry.revision)
            .field("bytes", &self.bytes.len())
            .finish()
    }
}
#[derive(Clone, Debug)]
pub struct AssetManifest {
    entries: BTreeMap<String, AssetEntry>,
}
impl AssetManifest {
    pub fn new(entries: Vec<AssetEntry>) -> Result<Self, AssetError> {
        if entries.len() > MAX_ASSET_ENTRIES {
            return Err(AssetError::InvalidManifest);
        }
        let mut manifest = BTreeMap::new();
        for entry in entries {
            if entry.id.is_empty()
                || entry.id.len() > 128
                || entry.id.starts_with('/')
                || entry
                    .id
                    .split('/')
                    .any(|p| p.is_empty() || p == "." || p == "..")
                || !entry
                    .id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"/_-.".contains(&b))
                || entry.revision.is_empty()
                || entry.revision.len() > 64
                || !entry
                    .revision
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
                || entry.url.is_empty()
                || entry.url.len() > 2048
                || entry.url.contains(['\\', '\r', '\n', '\0', '#'])
                || entry.url.starts_with("//")
                || !(entry.url.starts_with('/')
                    || entry.url.starts_with("https://")
                    || entry.url.starts_with("http://"))
                || !(1..=MAX_HTTP_BYTES).contains(&entry.max_bytes)
            {
                return Err(AssetError::InvalidManifest);
            }
            if manifest.insert(entry.id.clone(), entry).is_some() {
                return Err(AssetError::DuplicateId);
            }
        }
        Ok(Self { entries: manifest })
    }
    pub fn get(&self, id: &str) -> Option<&AssetEntry> {
        self.entries.get(id)
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    /// Revision participates in the fetch URL. This is cache invalidation, not a
    /// cryptographic integrity guarantee; callers may supply content-hash URLs.
    pub fn request_url(&self, id: &str) -> Result<String, AssetError> {
        let entry = self.get(id).ok_or(AssetError::UnknownId)?;
        Ok(format!(
            "{}{}rustx_revision={}",
            entry.url,
            if entry.url.contains('?') { '&' } else { '?' },
            entry.revision
        ))
    }
    pub fn load(
        &self,
        http: &HttpClient,
        scope: &Scope,
        id: &str,
        complete: impl FnOnce(Result<AssetData, AssetError>) + 'static,
    ) -> Result<u64, AssetError> {
        let entry = self.get(id).ok_or(AssetError::UnknownId)?.clone();
        let url = self.request_url(id)?;
        Ok(http
            .request("GET", url)
            .response_limit(entry.max_bytes)
            .send_callback(scope, move |result| {
                complete(
                    result
                        .and_then(|r| r.error_for_status())
                        .map(|r| AssetData {
                            entry,
                            bytes: r.body,
                        })
                        .map_err(AssetError::Transport),
                );
            }))
    }
}
