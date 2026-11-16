use super::{AppUpdateService, is_newer_version, select_installer};
use crate::models::*;

impl AppUpdateService {
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
        let checksum_url =
            assets.iter().find(|asset| asset.name.eq_ignore_ascii_case("SHA256SUMS.txt")).map(|asset| asset.download_url.clone());
        Ok(Some(AppUpdate {
            version: tag.strip_prefix('v').unwrap_or(&tag).to_string(),
            release_url: required_string(&release, "html_url")?,
            notes: release["body"].as_str().unwrap_or_default().to_string(),
            asset: installer,
            checksum_url,
            tag,
        }))
    }
}

fn github_error(status: u16) -> String {
    match status {
        403 | 429 => rust_i18n::t!("core.update.rate_limited").to_string(),
        404 => rust_i18n::t!("core.update.no_release").to_string(),
        other => rust_i18n::t!("core.update.http_error", status = other).to_string(),
    }
}

fn required_string(value: &serde_json::Value, key: &str) -> Result<String, String> {
    value[key]
        .as_str()
        .filter(|text| !text.trim().is_empty())
        .map(str::to_string)
        .ok_or_else(|| rust_i18n::t!("core.update.missing_field", key = key).to_string())
}

fn json_i64(value: &serde_json::Value) -> i64 {
    value.as_i64().or_else(|| value.as_str().and_then(|text| text.parse().ok())).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;
    use crate::updater::download::safe_file_name;
    use crate::updater::{DesktopUpdatePlatform, LATEST_RELEASE_API};

    #[test]
    fn maps_github_statuses_to_messages() {
        assert_eq!(github_error(403), "GitHub temporarily limited update checks");
        assert_eq!(github_error(404), "No release is published on GitHub yet");
        assert_eq!(github_error(500), "GitHub returned HTTP 500");
        assert_eq!(safe_file_name("v9.0.0/beta x"), "v9.0.0_beta_x");
    }

    #[tokio::test]
    async fn reports_up_to_date_and_github_errors() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/latest"))
            .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"tag_name":"v1.0.0","html_url":"x"}"#))
            .mount(&server)
            .await;
        Mock::given(method("GET")).and(path("/limited")).respond_with(ResponseTemplate::new(403)).mount(&server).await;
        let service = |path: &str| {
            AppUpdateService::with_options(
                reqwest::Client::new(),
                format!("{}{path}", server.uri()),
                DesktopUpdatePlatform::Linux,
                std::env::temp_dir(),
                Box::new(|_| Ok(None)),
                "1.0.0",
            )
        };
        assert_eq!(service("/latest").check_for_update("1.0.0").await, UpdateCheckResult::UpToDate);
        assert_eq!(
            service("/limited").check_for_update("1.0.0").await,
            UpdateCheckResult::Failure { message: "GitHub temporarily limited update checks".into() }
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
            Box::new(|_| Ok(None)),
            "1.0.0",
        );
        assert_eq!(
            https_only.download_and_install(&insecure, &|_| {}).await,
            UpdateInstallResult::Failure { message: "GitHub returned an insecure installer link".into() }
        );
    }
}
