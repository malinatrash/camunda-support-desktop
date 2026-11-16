use std::path::Path;
use std::time::{Duration, Instant};

use futures_util::StreamExt;
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;

use super::AppUpdateService;
use super::checksum::{file_sha256, hex};
use crate::models::*;

impl AppUpdateService {
    pub async fn download_and_install(
        &self,
        update: &AppUpdate,
        on_progress: &(dyn Fn(UpdateDownloadProgress) + Send + Sync),
    ) -> UpdateInstallResult {
        let Some(asset) = &update.asset else {
            return UpdateInstallResult::Failure { message: rust_i18n::t!("core.update.no_installer").to_string() };
        };
        match self.download_and_install_asset(update, asset, on_progress).await {
            Ok(Some(restart)) => UpdateInstallResult::Installed { version: update.version.clone(), restart },
            Ok(None) => UpdateInstallResult::InstallerOpened { file_name: asset.name.clone() },
            Err(message) => UpdateInstallResult::Failure { message },
        }
    }

    async fn download_and_install_asset(
        &self,
        update: &AppUpdate,
        asset: &AppUpdateAsset,
        on_progress: &(dyn Fn(UpdateDownloadProgress) + Send + Sync),
    ) -> Result<Option<RestartPlan>, String> {
        let local_api = self.api_url.starts_with("http://localhost") || self.api_url.starts_with("http://127.0.0.1");
        if !asset.download_url.starts_with("https://") && !local_api {
            return Err(rust_i18n::t!("core.update.insecure_url").to_string());
        }
        let release_directory = self.updates_directory.join(safe_file_name(&update.tag));
        tokio::fs::create_dir_all(&release_directory).await.map_err(|error| error.to_string())?;
        let file_name =
            Path::new(&asset.name).file_name().map(|name| name.to_string_lossy().to_string()).unwrap_or_else(|| asset.name.clone());
        let target = release_directory.join(file_name);
        let destination_path = target.to_string_lossy().to_string();
        let progress = |stage: UpdateDownloadStage, downloaded_bytes: i64, total_bytes: i64, bytes_per_second: i64| {
            on_progress(UpdateDownloadProgress {
                stage,
                downloaded_bytes,
                total_bytes,
                bytes_per_second,
                destination_path: destination_path.clone(),
            });
        };
        progress(UpdateDownloadStage::Preparing, 0, 0, 0);
        let expected_digest = self.expected_digest(update, asset).await?;
        let cached_installer_valid = if target.exists() {
            progress(UpdateDownloadStage::Verifying, 0, 0, 0);
            file_sha256(&target).await? == expected_digest
        } else {
            false
        };
        if !cached_installer_valid {
            let partial = target
                .with_file_name(format!("{}.part", target.file_name().map(|name| name.to_string_lossy().to_string()).unwrap_or_default()));
            let _ = tokio::fs::remove_file(&partial).await;
            let response = self.download_request(&asset.download_url).send().await.map_err(|error| error.to_string())?;
            if !response.status().is_success() {
                return Err(rust_i18n::t!("core.update.download_failed", status = response.status().as_u16()).to_string());
            }
            let total_bytes =
                response.content_length().map(|length| length as i64).filter(|length| *length > 0).unwrap_or(asset.size_bytes);
            let mut output = tokio::fs::File::create(&partial).await.map_err(|error| error.to_string())?;
            let mut digest = Sha256::new();
            let started_at = Instant::now();
            let mut last_reported_at: Option<Instant> = None;
            let mut downloaded_bytes: i64 = 0;
            progress(UpdateDownloadStage::Downloading, 0, total_bytes, 0);
            let mut stream = response.bytes_stream();
            while let Some(chunk) = stream.next().await {
                let chunk = chunk.map_err(|error| error.to_string())?;
                output.write_all(&chunk).await.map_err(|error| error.to_string())?;
                digest.update(&chunk);
                downloaded_bytes += chunk.len() as i64;
                let now = Instant::now();
                if last_reported_at.is_none_or(|last| now.duration_since(last) >= PROGRESS_INTERVAL) {
                    progress(
                        UpdateDownloadStage::Downloading,
                        downloaded_bytes,
                        total_bytes,
                        average_speed(downloaded_bytes, started_at.elapsed()),
                    );
                    last_reported_at = Some(now);
                }
            }
            output.flush().await.map_err(|error| error.to_string())?;
            drop(output);
            progress(
                UpdateDownloadStage::Downloading,
                downloaded_bytes,
                total_bytes,
                average_speed(downloaded_bytes, started_at.elapsed()),
            );
            let actual_digest = hex(&digest.finalize());
            progress(UpdateDownloadStage::Verifying, total_bytes, total_bytes, 0);
            if actual_digest != expected_digest {
                return Err(rust_i18n::t!("core.update.checksum_mismatch").to_string());
            }
            tokio::fs::rename(&partial, &target).await.map_err(|error| error.to_string())?;
        }
        progress(UpdateDownloadStage::Installing, asset.size_bytes, asset.size_bytes, 0);
        (self.install)(&target)
    }
}

pub(super) fn safe_file_name(value: &str) -> String {
    value.chars().map(|char| if char.is_alphanumeric() || matches!(char, '.' | '-' | '_') { char } else { '_' }).collect()
}

fn average_speed(downloaded_bytes: i64, elapsed: Duration) -> i64 {
    let nanos = elapsed.as_nanos();
    if nanos == 0 {
        return 0;
    }
    (downloaded_bytes as f64 * 1_000_000_000.0 / nanos as f64) as i64
}

const PROGRESS_INTERVAL: Duration = Duration::from_millis(100);

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};

    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;
    use crate::updater::DesktopUpdatePlatform;

    #[tokio::test]
    async fn checks_downloads_verifies_and_opens_platform_installer() {
        let installer_bytes = "проверенный dmg".as_bytes().to_vec();
        let installer_name = "Camunda.Support-9.0.0.dmg";
        let checksum_file_name = "Camunda Support-9.0.0.dmg";
        let checksum = hex(&Sha256::digest(&installer_bytes));
        let server = MockServer::start().await;
        let base_url = server.uri();
        Mock::given(method("GET"))
            .and(path("/latest"))
            .and(header("Accept", "application/vnd.github+json"))
            .and(header("X-GitHub-Api-Version", "2022-11-28"))
            .and(header("User-Agent", "camunda-support-desktop/1.0.0"))
            .respond_with(ResponseTemplate::new(200).set_body_string(format!(
                r#"{{
                  "tag_name": "v9.0.0",
                  "html_url": "{base_url}/release",
                  "body": "Новая версия",
                  "assets": [
                    {{"name": "{installer_name}", "browser_download_url": "{base_url}/installer", "size": {}, "digest": null}},
                    {{"name": "SHA256SUMS.txt", "browser_download_url": "{base_url}/checksums", "size": 100, "digest": null}}
                  ]
                }}"#,
                installer_bytes.len()
            )))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/installer"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(installer_bytes.clone()))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/checksums"))
            .respond_with(ResponseTemplate::new(200).set_body_string(format!("{checksum}  ./{checksum_file_name}\n")))
            .mount(&server)
            .await;

        let directory = tempfile::tempdir().expect("временный каталог");
        let opened: Arc<Mutex<Option<PathBuf>>> = Arc::new(Mutex::new(None));
        let opened_sink = opened.clone();
        let service = AppUpdateService::with_options(
            reqwest::Client::new(),
            format!("{base_url}/latest"),
            DesktopUpdatePlatform::MacArm64,
            directory.path().to_path_buf(),
            Box::new(move |installer| {
                *opened_sink.lock().expect("lock") = Some(installer.to_path_buf());
                Ok(None)
            }),
            "1.0.0",
        );

        let UpdateCheckResult::Available { update } = service.check_for_update("1.0.0").await else {
            panic!("ожидалось доступное обновление");
        };
        assert_eq!(update.asset.as_ref().map(|asset| asset.name.as_str()), Some(installer_name));
        assert_eq!(update.version, "9.0.0");
        assert_eq!(update.notes, "Новая версия");

        let progress: Mutex<Vec<UpdateDownloadProgress>> = Mutex::new(Vec::new());
        let report = |event: UpdateDownloadProgress| progress.lock().expect("lock").push(event);
        let UpdateInstallResult::InstallerOpened { file_name } = service.download_and_install(&update, &report).await else {
            panic!("ожидалось открытие установщика");
        };
        assert_eq!(file_name, installer_name);

        let progress = progress.into_inner().expect("lock");
        let size = installer_bytes.len() as i64;
        assert_eq!(progress.first().map(|event| event.stage), Some(UpdateDownloadStage::Preparing));
        assert!(progress.iter().any(|event| {
            event.stage == UpdateDownloadStage::Downloading
                && event.downloaded_bytes == size
                && event.total_bytes == size
                && event.bytes_per_second > 0
        }));
        assert!(progress.iter().any(|event| event.stage == UpdateDownloadStage::Verifying));
        let last = progress.last().expect("есть прогресс");
        assert_eq!(last.stage, UpdateDownloadStage::Installing);
        let opened_file = opened.lock().expect("lock").clone().expect("установщик открыт");
        assert_eq!(opened_file.to_string_lossy(), last.destination_path);
        assert_eq!(std::fs::read(&opened_file).expect("файл читается"), installer_bytes);

        let UpdateInstallResult::InstallerOpened { .. } = service.download_and_install(&update, &|_| {}).await else {
            panic!("повторная установка должна использовать кэш");
        };
        assert_eq!(server.received_requests().await.map(|requests| requests.len()), Some(4));
    }
}
