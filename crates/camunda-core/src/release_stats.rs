use std::collections::HashSet;
use std::time::Duration;

use tokio::sync::Mutex;

use crate::models::ReleaseDownloadStats;

pub const RELEASES_API: &str = "https://api.github.com/repos/malinatrash/camunda-support-desktop/releases?per_page=100";
pub const CACHE_TTL_MILLIS: i64 = 6 * 60 * 60 * 1_000;

const MAX_PAGES: usize = 100;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);

pub type NowMillis = Box<dyn Fn() -> i64 + Send + Sync>;

pub struct ReleaseDownloadStatsService {
    client: reqwest::Client,
    releases_api_url: String,
    user_agent: String,
    now_millis: NowMillis,
    cache_ttl_millis: i64,
    cached: Mutex<Option<CachedStats>>,
}

struct CachedStats {
    stats: ReleaseDownloadStats,
    loaded_at_millis: i64,
}

impl ReleaseDownloadStatsService {
    pub fn new(app_version: &str) -> Self {
        let client = reqwest::Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .build()
            .unwrap_or_default();
        Self::with_options(client, RELEASES_API.to_string(), Box::new(current_millis), CACHE_TTL_MILLIS, app_version)
    }

    pub fn with_options(
        client: reqwest::Client,
        releases_api_url: String,
        now_millis: NowMillis,
        cache_ttl_millis: i64,
        app_version: &str,
    ) -> Self {
        Self {
            client,
            releases_api_url,
            user_agent: format!("camunda-support-desktop/{app_version}"),
            now_millis,
            cache_ttl_millis,
            cached: Mutex::new(None),
        }
    }

    pub async fn load(&self, force_refresh: bool) -> Result<ReleaseDownloadStats, String> {
        let mut cached = self.cached.lock().await;
        let now = (self.now_millis)();
        if let Some(entry) = cached.as_ref() {
            if !force_refresh && now - entry.loaded_at_millis < self.cache_ttl_millis {
                return Ok(entry.stats.clone());
            }
        }
        let stats = self.fetch_all_pages().await?;
        *cached = Some(CachedStats { stats: stats.clone(), loaded_at_millis: (self.now_millis)() });
        Ok(stats)
    }

    async fn fetch_all_pages(&self) -> Result<ReleaseDownloadStats, String> {
        let mut next_url = Some(self.releases_api_url.clone());
        let mut visited_urls = HashSet::new();
        let mut page_count = 0;
        let mut release_count: i32 = 0;
        let mut installer_asset_count: i32 = 0;
        let mut mac_os_downloads: i64 = 0;
        let mut windows_downloads: i64 = 0;
        let mut linux_downloads: i64 = 0;

        while let Some(url) = next_url {
            if page_count >= MAX_PAGES {
                return Err("GitHub вернул слишком много страниц релизов".into());
            }
            page_count += 1;
            if !visited_urls.insert(url.clone()) {
                return Err("GitHub вернул циклическую пагинацию релизов".into());
            }
            let response = self
                .client
                .get(&url)
                .timeout(REQUEST_TIMEOUT)
                .header("Accept", "application/vnd.github+json")
                .header("X-GitHub-Api-Version", "2022-11-28")
                .header("User-Agent", &self.user_agent)
                .send()
                .await
                .map_err(|error| error.to_string())?;
            let status = response.status().as_u16();
            if status != 200 {
                return Err(github_error(status));
            }
            next_url = response
                .headers()
                .get("Link")
                .and_then(|value| value.to_str().ok())
                .and_then(next_page_url);
            let releases: serde_json::Value = response.json().await.map_err(|error| error.to_string())?;
            let releases = releases.as_array().ok_or("GitHub вернул неожиданный формат списка релизов")?;
            release_count += releases.len() as i32;

            for release in releases {
                for asset in release["assets"].as_array().into_iter().flatten() {
                    let name = asset["name"].as_str().unwrap_or_default().to_lowercase();
                    let downloads = asset["download_count"].as_i64().unwrap_or(0);
                    if name.ends_with(".dmg") {
                        mac_os_downloads += downloads;
                    } else if name.ends_with(".exe") || name.ends_with(".msi") {
                        windows_downloads += downloads;
                    } else if name.ends_with(".deb") {
                        linux_downloads += downloads;
                    } else {
                        continue;
                    }
                    installer_asset_count += 1;
                }
            }
        }

        Ok(ReleaseDownloadStats {
            total_downloads: mac_os_downloads + windows_downloads + linux_downloads,
            mac_os_downloads,
            windows_downloads,
            linux_downloads,
            release_count,
            installer_asset_count,
        })
    }
}

// Достаёт URL с rel="next" из заголовка Link вида `<url>; rel="next", <url>; rel="last"`.
fn next_page_url(link_header: &str) -> Option<String> {
    link_header.split(',').find_map(|part| {
        let part = part.trim();
        let rest = part.strip_prefix('<')?;
        let end = rest.find('>')?;
        let url = &rest[..end];
        let rel = rest[end + 1..].trim_start().strip_prefix(';')?.trim_start().strip_prefix("rel=\"")?;
        let rel = &rel[..rel.find('"')?];
        (rel == "next").then(|| url.to_string())
    })
}

fn github_error(status: u16) -> String {
    match status {
        403 | 429 => "GitHub временно ограничил загрузку статистики".into(),
        404 => "GitHub не нашёл опубликованные релизы".into(),
        other => format!("GitHub вернул HTTP {other} при загрузке статистики"),
    }
}

fn current_millis() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

#[cfg(test)]
mod tests {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;

    #[test]
    fn parses_next_page_from_link_header() {
        let header = r#"<https://api.github.com/releases?page=2>; rel="next", <https://api.github.com/releases?page=5>; rel="last""#;
        assert_eq!(next_page_url(header).as_deref(), Some("https://api.github.com/releases?page=2"));
        assert_eq!(next_page_url(r#"<https://api.github.com/releases?page=1>; rel="prev""#), None);
        assert_eq!(next_page_url("garbage"), None);
    }

    #[tokio::test]
    async fn sums_installers_from_every_release_page_and_caches_result() {
        let server = MockServer::start().await;
        let base_url = server.uri();
        Mock::given(method("GET"))
            .and(path("/releases"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("Link", format!("<{base_url}/releases-page-2>; rel=\"next\"").as_str())
                    .set_body_string(
                        r#"[{"assets":[
                            {"name":"Camunda Support-1.0.0.dmg","download_count":2},
                            {"name":"Camunda Support-1.0.0.exe","download_count":3},
                            {"name":"SHA256SUMS.txt","download_count":99}
                        ]}]"#,
                    ),
            )
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/releases-page-2"))
            .respond_with(ResponseTemplate::new(200).set_body_string(
                r#"[{"assets":[
                    {"name":"Camunda Support-0.9.0.msi","download_count":4},
                    {"name":"Camunda Support-0.9.0.deb","download_count":5},
                    {"name":"source.zip","download_count":50}
                ]}]"#,
            ))
            .mount(&server)
            .await;

        let service = ReleaseDownloadStatsService::with_options(
            reqwest::Client::new(),
            format!("{base_url}/releases"),
            Box::new(|| 1_000),
            CACHE_TTL_MILLIS,
            "1.0.0",
        );
        let requests = || async { server.received_requests().await.map(|requests| requests.len()).unwrap_or(0) };

        let first = service.load(false).await.expect("статистика загружена");
        assert_eq!(first.total_downloads, 14);
        assert_eq!(first.mac_os_downloads, 2);
        assert_eq!(first.windows_downloads, 7);
        assert_eq!(first.linux_downloads, 5);
        assert_eq!(first.release_count, 2);
        assert_eq!(first.installer_asset_count, 4);
        assert_eq!(requests().await, 2);

        assert_eq!(service.load(false).await.as_ref(), Ok(&first));
        assert_eq!(requests().await, 2, "Повторная загрузка должна использовать шестичасовой кэш");

        assert_eq!(service.load(true).await.as_ref(), Ok(&first));
        assert_eq!(requests().await, 4, "Ручное обновление должно заново пройти все страницы");
    }

    #[tokio::test]
    async fn reports_github_errors_and_pagination_loops() {
        let server = MockServer::start().await;
        let base_url = server.uri();
        Mock::given(method("GET"))
            .and(path("/missing"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/loop"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("Link", format!("<{base_url}/loop>; rel=\"next\"").as_str())
                    .set_body_string("[]"),
            )
            .mount(&server)
            .await;
        let service = |path: &str| {
            ReleaseDownloadStatsService::with_options(
                reqwest::Client::new(),
                format!("{base_url}{path}"),
                Box::new(|| 1_000),
                CACHE_TTL_MILLIS,
                "1.0.0",
            )
        };
        assert_eq!(service("/missing").load(false).await, Err("GitHub не нашёл опубликованные релизы".into()));
        assert_eq!(service("/loop").load(false).await, Err("GitHub вернул циклическую пагинацию релизов".into()));
    }
}
