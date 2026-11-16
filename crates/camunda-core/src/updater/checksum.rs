use std::path::Path;

use sha2::{Digest, Sha256};
use tokio::io::AsyncReadExt;

use super::AppUpdateService;
use crate::models::{AppUpdate, AppUpdateAsset};

impl AppUpdateService {
    pub(super) async fn expected_digest(&self, update: &AppUpdate, asset: &AppUpdateAsset) -> Result<String, String> {
        let asset_digest = asset
            .digest
            .as_deref()
            .filter(|digest| digest.len() >= 7 && digest[..7].eq_ignore_ascii_case("sha256:"))
            .map(|digest| digest[7..].to_lowercase())
            .filter(|digest| digest.len() == SHA256_LENGTH);
        if let Some(digest) = asset_digest {
            return Ok(digest);
        }
        let checksum_url = update.checksum_url.as_deref().ok_or_else(|| rust_i18n::t!("core.update.no_checksums").to_string())?;
        let response = self.download_request(checksum_url).send().await.map_err(|error| error.to_string())?;
        if !response.status().is_success() {
            return Err(rust_i18n::t!("core.update.checksums_failed").to_string());
        }
        let body = response.text().await.map_err(|error| error.to_string())?;
        let expected_name = release_asset_name(&asset.name);
        body.lines()
            .find_map(|line| {
                let hash: String = line.chars().take(SHA256_LENGTH).collect::<String>().to_lowercase();
                let file_name = line.chars().skip(SHA256_LENGTH).collect::<String>();
                let file_name = file_name.trim();
                let file_name = file_name.strip_prefix('*').unwrap_or(file_name);
                let file_name = file_name.strip_prefix("./").unwrap_or(file_name);
                (hash.len() == SHA256_LENGTH && release_asset_name(file_name) == expected_name).then_some(hash)
            })
            .ok_or_else(|| rust_i18n::t!("core.update.checksum_missing", name = asset.name).to_string())
    }
}

pub(super) async fn file_sha256(path: &Path) -> Result<String, String> {
    let mut file = tokio::fs::File::open(path).await.map_err(|error| error.to_string())?;
    let mut digest = Sha256::new();
    let mut buffer = vec![0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer).await.map_err(|error| error.to_string())?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(hex(&digest.finalize()))
}

pub(super) fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn release_asset_name(value: &str) -> String {
    value.replace(' ', ".")
}

const SHA256_LENGTH: usize = 64;
