use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use futures_util::StreamExt;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::models::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopUpdatePlatform {
    MacArm64,
    MacX64,
    Windows,
    Linux,
    Unsupported,
}

pub type OpenInstaller = Box<dyn Fn(&Path) -> Result<(), String> + Send + Sync>;

pub struct AppUpdateService {
    client: reqwest::Client,
    api_url: String,
    platform: DesktopUpdatePlatform,
    updates_directory: PathBuf,
    user_agent: String,
    open_installer: OpenInstaller,
}

pub const LATEST_RELEASE_API: &str = "https://api.github.com/repos/malinatrash/camunda-support-desktop/releases/latest";

const SHA256_LENGTH: usize = 64;
const PROGRESS_INTERVAL: Duration = Duration::from_millis(100);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(10 * 60);

impl AppUpdateService {
    pub fn new(app_version: &str) -> Self {
        let platform = detect_platform();
        let client = reqwest::Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .build()
            .unwrap_or_default();
        Self::with_options(
            client,
            LATEST_RELEASE_API.to_string(),
            platform,
            std::env::temp_dir().join("camunda-support-updates"),
            Box::new(move |installer| open_installer(platform, installer)),
            app_version,
        )
    }

    pub fn with_options(
        client: reqwest::Client,
        api_url: String,
        platform: DesktopUpdatePlatform,
        updates_directory: PathBuf,
        open_installer: OpenInstaller,
        app_version: &str,
    ) -> Self {
        Self {
            client,
            api_url,
            platform,
            updates_directory,
            user_agent: format!("camunda-support-desktop/{app_version}"),
            open_installer,
        }
    }

    pub async fn check_for_update(&self, current_version: &str) -> UpdateCheckResult {
        match self.fetch_update(current_version).await {
            Ok(Some(update)) => UpdateCheckResult::Available { update },
            Ok(None) => UpdateCheckResult::UpToDate,
            Err(message) => UpdateCheckResult::Failure { message },
        }
    }

    async fn fetch_update(&self, current_version: &str) -> Result<Option<AppUpdate>, String> {
        let response = self.api_request(&self.api_url).send().await.map_err(|error| error.to_string())?;
        let status = response.status().as_u16();
        if status != 200 {
            return Err(github_error(status));
        }
        let release: serde_json::Value = response.json().await.map_err(|error| error.to_string())?;
        let tag = required_string(&release, "tag_name")?;
        if !is_newer_version(&tag, current_version) {
            return Ok(None);
        }
        let mut assets = Vec::new();
        for asset in release["assets"].as_array().into_iter().flatten() {
            assets.push(AppUpdateAsset {
                name: required_string(asset, "name")?,
                download_url: required_string(asset, "browser_download_url")?,
                size_bytes: json_i64(&asset["size"]),
                digest: asset["digest"].as_str().map(str::to_string),
            });
        }
        let installer = select_installer(&assets, self.platform);
        let checksum_url = assets
            .iter()
            .find(|asset| asset.name.eq_ignore_ascii_case("SHA256SUMS.txt"))
            .map(|asset| asset.download_url.clone());
        Ok(Some(AppUpdate {
            version: tag.strip_prefix('v').unwrap_or(&tag).to_string(),
            release_url: required_string(&release, "html_url")?,
            notes: release["body"].as_str().unwrap_or_default().to_string(),
            asset: installer,
            checksum_url,
            tag,
        }))
    }

    pub async fn download_and_open(
        &self,
        update: &AppUpdate,
        on_progress: &(dyn Fn(UpdateDownloadProgress) + Send + Sync),
    ) -> UpdateInstallResult {
        let Some(asset) = &update.asset else {
            return UpdateInstallResult::Failure { message: "Для этой платформы установщик не опубликован".into() };
        };
        match self.download_and_open_asset(update, asset, on_progress).await {
            Ok(()) => UpdateInstallResult::InstallerOpened { file_name: asset.name.clone() },
            Err(message) => UpdateInstallResult::Failure { message },
        }
    }

    async fn download_and_open_asset(
        &self,
        update: &AppUpdate,
        asset: &AppUpdateAsset,
        on_progress: &(dyn Fn(UpdateDownloadProgress) + Send + Sync),
    ) -> Result<(), String> {
        let local_api = self.api_url.starts_with("http://localhost") || self.api_url.starts_with("http://127.0.0.1");
        if !asset.download_url.starts_with("https://") && !local_api {
            return Err("GitHub вернул небезопасную ссылку на установщик".into());
        }
        let release_directory = self.updates_directory.join(safe_file_name(&update.tag));
        tokio::fs::create_dir_all(&release_directory).await.map_err(|error| error.to_string())?;
        let file_name = Path::new(&asset.name)
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| asset.name.clone());
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
            let partial = target.with_file_name(format!(
                "{}.part",
                target.file_name().map(|name| name.to_string_lossy().to_string()).unwrap_or_default()
            ));
            let _ = tokio::fs::remove_file(&partial).await;
            let response = self
                .download_request(&asset.download_url)
                .send()
                .await
                .map_err(|error| error.to_string())?;
            if !response.status().is_success() {
                return Err(format!("GitHub не отдал установщик: HTTP {}", response.status().as_u16()));
            }
            let total_bytes = response
                .content_length()
                .map(|length| length as i64)
                .filter(|length| *length > 0)
                .unwrap_or(asset.size_bytes);
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
                if last_reported_at.map_or(true, |last| now.duration_since(last) >= PROGRESS_INTERVAL) {
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
                return Err("Контрольная сумма установщика не совпала".into());
            }
            tokio::fs::rename(&partial, &target).await.map_err(|error| error.to_string())?;
        }
        progress(UpdateDownloadStage::OpeningInstaller, asset.size_bytes, asset.size_bytes, 0);
        (self.open_installer)(&target)
    }

    async fn expected_digest(&self, update: &AppUpdate, asset: &AppUpdateAsset) -> Result<String, String> {
        let asset_digest = asset
            .digest
            .as_deref()
            .filter(|digest| digest.len() >= 7 && digest[..7].eq_ignore_ascii_case("sha256:"))
            .map(|digest| digest[7..].to_lowercase())
            .filter(|digest| digest.len() == SHA256_LENGTH);
        if let Some(digest) = asset_digest {
            return Ok(digest);
        }
        let checksum_url = update.checksum_url.as_deref().ok_or("В релизе отсутствует SHA256SUMS.txt")?;
        let response = self
            .download_request(checksum_url)
            .send()
            .await
            .map_err(|error| error.to_string())?;
        if !response.status().is_success() {
            return Err("Не удалось получить контрольные суммы обновления".into());
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
            .ok_or_else(|| format!("Для {} не найдена контрольная сумма", asset.name))
    }

    fn api_request(&self, url: &str) -> reqwest::RequestBuilder {
        self.client
            .get(url)
            .timeout(REQUEST_TIMEOUT)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("User-Agent", &self.user_agent)
    }

    fn download_request(&self, url: &str) -> reqwest::RequestBuilder {
        self.client.get(url).timeout(DOWNLOAD_TIMEOUT).header("User-Agent", &self.user_agent)
    }
}

pub fn detect_platform() -> DesktopUpdatePlatform {
    let arch = std::env::consts::ARCH;
    match std::env::consts::OS {
        "macos" if arch == "aarch64" || arch == "arm64" => DesktopUpdatePlatform::MacArm64,
        "macos" => DesktopUpdatePlatform::MacX64,
        "windows" => DesktopUpdatePlatform::Windows,
        "linux" => DesktopUpdatePlatform::Linux,
        _ => DesktopUpdatePlatform::Unsupported,
    }
}

pub fn select_installer(assets: &[AppUpdateAsset], platform: DesktopUpdatePlatform) -> Option<AppUpdateAsset> {
    let with_extension = |extension: &str| assets.iter().find(|asset| ends_with_ignore_case(&asset.name, extension));
    let dmg_with_marker = |markers: &[&str]| {
        assets.iter().find(|asset| {
            let name = asset.name.to_lowercase();
            name.ends_with(".dmg") && markers.iter().any(|marker| name.contains(marker))
        })
    };
    match platform {
        DesktopUpdatePlatform::MacArm64 => dmg_with_marker(&["aarch64", "arm64"]).or_else(|| with_extension(".dmg")),
        DesktopUpdatePlatform::MacX64 => {
            dmg_with_marker(&["x64", "x86_64", "intel"]).or_else(|| with_extension(".dmg"))
        }
        DesktopUpdatePlatform::Windows => with_extension(".exe").or_else(|| with_extension(".msi")),
        DesktopUpdatePlatform::Linux => with_extension(".deb"),
        DesktopUpdatePlatform::Unsupported => None,
    }
    .cloned()
}

fn open_installer(platform: DesktopUpdatePlatform, installer: &Path) -> Result<(), String> {
    let spawned = match platform {
        DesktopUpdatePlatform::MacArm64 | DesktopUpdatePlatform::MacX64 => Command::new("open").arg(installer).spawn(),
        DesktopUpdatePlatform::Windows => Command::new(installer).spawn(),
        DesktopUpdatePlatform::Linux => Command::new("xdg-open").arg(installer).spawn(),
        DesktopUpdatePlatform::Unsupported => return Err("Автообновление недоступно на этой платформе".into()),
    };
    spawned.map(|_| ()).map_err(|error| error.to_string())
}

fn github_error(status: u16) -> String {
    match status {
        403 | 429 => "GitHub временно ограничил проверку обновлений".into(),
        404 => "В GitHub пока нет опубликованного релиза".into(),
        other => format!("GitHub вернул HTTP {other}"),
    }
}

fn required_string(value: &serde_json::Value, key: &str) -> Result<String, String> {
    value[key]
        .as_str()
        .filter(|text| !text.trim().is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("GitHub не вернул поле {key}"))
}

fn json_i64(value: &serde_json::Value) -> i64 {
    value.as_i64().or_else(|| value.as_str().and_then(|text| text.parse().ok())).unwrap_or(0)
}

fn ends_with_ignore_case(name: &str, suffix: &str) -> bool {
    name.to_lowercase().ends_with(suffix)
}

fn safe_file_name(value: &str) -> String {
    value
        .chars()
        .map(|char| if char.is_alphanumeric() || matches!(char, '.' | '-' | '_') { char } else { '_' })
        .collect()
}

fn release_asset_name(value: &str) -> String {
    value.replace(' ', ".")
}

fn average_speed(downloaded_bytes: i64, elapsed: Duration) -> i64 {
    let nanos = elapsed.as_nanos();
    if nanos == 0 {
        return 0;
    }
    (downloaded_bytes as f64 * 1_000_000_000.0 / nanos as f64) as i64
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

async fn file_sha256(path: &Path) -> Result<String, String> {
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

// Сравнивает версии вида v1.2.3 / 1.2.3-dev.4: true, если candidate новее current.
pub fn is_newer_version(candidate: &str, current: &str) -> bool {
    fn parts(value: &str) -> Vec<i64> {
        value
            .trim()
            .trim_start_matches('v')
            .split('-')
            .next()
            .unwrap_or("")
            .split('+')
            .next()
            .unwrap_or("")
            .split('.')
            .map(|part| part.parse::<i64>().unwrap_or(0))
            .collect()
    }
    let candidate = parts(candidate);
    let current = parts(current);
    let size = candidate.len().max(current.len()).max(3);
    for index in 0..size {
        let left = candidate.get(index).copied().unwrap_or(0);
        let right = current.get(index).copied().unwrap_or(0);
        if left != right {
            return left > right;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;

    fn asset(name: &str) -> AppUpdateAsset {
        AppUpdateAsset {
            name: name.to_string(),
            download_url: format!("https://example.test/{name}"),
            size_bytes: 1,
            digest: None,
        }
    }

    #[test]
    fn compares_semantic_versions() {
        assert!(is_newer_version("v1.5.0", "1.4.5"));
        assert!(is_newer_version("2.0.0", "1.9.9-dev.3+abc1234"));
        assert!(is_newer_version("v1.2.0", "1.1.9"));
        assert!(is_newer_version("2.0", "1.99.99"));
        assert!(!is_newer_version("1.4.5", "1.4.5"));
        assert!(!is_newer_version("v1.4.4", "1.4.5"));
        assert!(!is_newer_version("1.1.9", "1.2.0"));
        assert!(is_newer_version("1.4.5.1", "1.4.5"));
    }

    #[test]
    fn selects_installer_for_every_supported_desktop_platform() {
        let assets = vec![
            asset("Camunda.Support-9.0.0.msi"),
            asset("Camunda.Support-9.0.0.exe"),
            asset("Camunda.Support-9.0.0.dmg"),
            asset("Camunda.Support-9.0.0.deb"),
        ];
        let name = |platform| select_installer(&assets, platform).map(|asset| asset.name);
        assert_eq!(name(DesktopUpdatePlatform::MacArm64).as_deref(), Some("Camunda.Support-9.0.0.dmg"));
        assert_eq!(name(DesktopUpdatePlatform::MacX64).as_deref(), Some("Camunda.Support-9.0.0.dmg"));
        assert_eq!(name(DesktopUpdatePlatform::Windows).as_deref(), Some("Camunda.Support-9.0.0.exe"));
        assert_eq!(name(DesktopUpdatePlatform::Linux).as_deref(), Some("Camunda.Support-9.0.0.deb"));
        assert_eq!(name(DesktopUpdatePlatform::Unsupported), None);
    }

    #[test]
    fn prefers_architecture_specific_dmg() {
        let assets = vec![asset("Camunda.Support-9.0.0-x64.dmg"), asset("Camunda.Support-9.0.0-aarch64.dmg")];
        let name = |platform| select_installer(&assets, platform).map(|asset| asset.name);
        assert_eq!(name(DesktopUpdatePlatform::MacArm64).as_deref(), Some("Camunda.Support-9.0.0-aarch64.dmg"));
        assert_eq!(name(DesktopUpdatePlatform::MacX64).as_deref(), Some("Camunda.Support-9.0.0-x64.dmg"));
    }

    #[test]
    fn maps_github_statuses_to_messages() {
        assert_eq!(github_error(403), "GitHub временно ограничил проверку обновлений");
        assert_eq!(github_error(404), "В GitHub пока нет опубликованного релиза");
        assert_eq!(github_error(500), "GitHub вернул HTTP 500");
        assert_eq!(safe_file_name("v9.0.0/beta x"), "v9.0.0_beta_x");
    }

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
                Ok(())
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
        let UpdateInstallResult::InstallerOpened { file_name } = service.download_and_open(&update, &report).await
        else {
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
        assert_eq!(last.stage, UpdateDownloadStage::OpeningInstaller);
        let opened_file = opened.lock().expect("lock").clone().expect("установщик открыт");
        assert_eq!(opened_file.to_string_lossy(), last.destination_path);
        assert_eq!(std::fs::read(&opened_file).expect("файл читается"), installer_bytes);

        let UpdateInstallResult::InstallerOpened { .. } = service.download_and_open(&update, &|_| {}).await else {
            panic!("повторная установка должна использовать кэш");
        };
        assert_eq!(server.received_requests().await.map(|requests| requests.len()), Some(4));
    }

    #[tokio::test]
    async fn reports_up_to_date_and_github_errors() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/latest"))
            .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"tag_name":"v1.0.0","html_url":"x"}"#))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/limited"))
            .respond_with(ResponseTemplate::new(403))
            .mount(&server)
            .await;
        let service = |path: &str| {
            AppUpdateService::with_options(
                reqwest::Client::new(),
                format!("{}{path}", server.uri()),
                DesktopUpdatePlatform::Linux,
                std::env::temp_dir(),
                Box::new(|_| Ok(())),
                "1.0.0",
            )
        };
        assert_eq!(service("/latest").check_for_update("1.0.0").await, UpdateCheckResult::UpToDate);
        assert_eq!(
            service("/limited").check_for_update("1.0.0").await,
            UpdateCheckResult::Failure { message: "GitHub временно ограничил проверку обновлений".into() }
        );
        let insecure = AppUpdate {
            version: "9.0.0".into(),
            tag: "v9.0.0".into(),
            release_url: String::new(),
            notes: String::new(),
            asset: Some(AppUpdateAsset {
                name: "app.deb".into(),
                download_url: "http://example.test/app.deb".into(),
                size_bytes: 1,
                digest: None,
            }),
            checksum_url: None,
        };
        let https_only = AppUpdateService::with_options(
            reqwest::Client::new(),
            LATEST_RELEASE_API.into(),
            DesktopUpdatePlatform::Linux,
            std::env::temp_dir(),
            Box::new(|_| Ok(())),
            "1.0.0",
        );
        assert_eq!(
            https_only.download_and_open(&insecure, &|_| {}).await,
            UpdateInstallResult::Failure { message: "GitHub вернул небезопасную ссылку на установщик".into() }
        );
    }
}
