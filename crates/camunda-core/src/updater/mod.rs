mod check;
mod checksum;
mod download;
mod install;
mod platform;
mod restart;
mod version;

use std::path::{Path, PathBuf};
use std::time::Duration;

pub use platform::{DesktopUpdatePlatform, detect_platform, select_installer};
pub use restart::restart;
pub use version::is_newer_version;

use crate::models::RestartPlan;
use install::install_package;

pub struct AppUpdateService {
    client: reqwest::Client,
    api_url: String,
    platform: DesktopUpdatePlatform,
    updates_directory: PathBuf,
    user_agent: String,
    install: Install,
}

// Ставит скачанный пакет; None — установка не удалась тихо и открыт обычный установщик.
pub type Install = Box<dyn Fn(&Path) -> Result<Option<RestartPlan>, String> + Send + Sync>;

pub const LATEST_RELEASE_API: &str = "https://api.github.com/repos/malinatrash/camunda-support-desktop/releases/latest";

impl AppUpdateService {
    pub fn new(app_version: &str) -> Self {
        let platform = detect_platform();
        let client = reqwest::Client::builder().connect_timeout(CONNECT_TIMEOUT).build().unwrap_or_default();
        Self::with_options(
            client,
            LATEST_RELEASE_API.to_string(),
            platform,
            std::env::temp_dir().join("camunda-support-updates"),
            Box::new(move |package| install_package(platform, package)),
            app_version,
        )
    }

    pub fn with_options(
        client: reqwest::Client,
        api_url: String,
        platform: DesktopUpdatePlatform,
        updates_directory: PathBuf,
        install: Install,
        app_version: &str,
    ) -> Self {
        Self { client, api_url, platform, updates_directory, user_agent: format!("camunda-support-desktop/{app_version}"), install }
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

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(10 * 60);
