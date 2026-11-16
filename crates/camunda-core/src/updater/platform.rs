use crate::models::AppUpdateAsset;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopUpdatePlatform {
    MacArm64,
    MacX64,
    Windows,
    Linux,
    Unsupported,
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
        DesktopUpdatePlatform::MacX64 => dmg_with_marker(&["x64", "x86_64", "intel"]).or_else(|| with_extension(".dmg")),
        DesktopUpdatePlatform::Windows => with_extension(".exe").or_else(|| with_extension(".msi")),
        DesktopUpdatePlatform::Linux => with_extension(".deb"),
        DesktopUpdatePlatform::Unsupported => None,
    }
    .cloned()
}

fn ends_with_ignore_case(name: &str, suffix: &str) -> bool {
    name.to_lowercase().ends_with(suffix)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn asset(name: &str) -> AppUpdateAsset {
        AppUpdateAsset { name: name.to_string(), download_url: format!("https://example.test/{name}"), size_bytes: 1, digest: None }
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
}
