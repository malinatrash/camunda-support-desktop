use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppUpdate {
    pub version: String,
    pub tag: String,
    pub release_url: String,
    pub notes: String,
    pub asset: Option<AppUpdateAsset>,
    pub checksum_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppUpdateAsset {
    pub name: String,
    pub download_url: String,
    pub size_bytes: i64,
    pub digest: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum UpdateCheckResult {
    UpToDate,
    Available { update: AppUpdate },
    Failure { message: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum UpdateInstallResult {
    Installed { version: String, restart: RestartPlan },
    InstallerOpened { file_name: String },
    Failure { message: String },
}

// Как довести установку до конца при перезапуске приложения.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum RestartPlan {
    Relaunch { app: String },
    SilentInstaller { installer: String },
    DebPackage { package: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UpdateDownloadStage {
    Preparing,
    Downloading,
    Verifying,
    Installing,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateDownloadProgress {
    pub stage: UpdateDownloadStage,
    pub downloaded_bytes: i64,
    pub total_bytes: i64,
    pub bytes_per_second: i64,
    pub destination_path: String,
}
