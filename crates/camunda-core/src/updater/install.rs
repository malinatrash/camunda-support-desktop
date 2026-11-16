use std::path::{Path, PathBuf};
use std::process::Command;

use super::DesktopUpdatePlatform;
use crate::models::RestartPlan;

pub(super) fn install_package(platform: DesktopUpdatePlatform, package: &Path) -> Result<Option<RestartPlan>, String> {
    let package_path = package.to_string_lossy().to_string();
    match platform {
        DesktopUpdatePlatform::MacArm64 | DesktopUpdatePlatform::MacX64 => match replace_app_bundle(package) {
            Ok(app) => Ok(Some(RestartPlan::Relaunch { app: app.to_string_lossy().to_string() })),
            Err(error) => {
                eprintln!("тихая установка не удалась, открываем DMG: {error}");
                Command::new("open").arg(package).spawn().map(|_| None).map_err(|error| error.to_string())
            }
        },
        DesktopUpdatePlatform::Windows => Ok(Some(RestartPlan::SilentInstaller { installer: package_path })),
        DesktopUpdatePlatform::Linux => Ok(Some(RestartPlan::DebPackage { package: package_path })),
        DesktopUpdatePlatform::Unsupported => Err(rust_i18n::t!("core.update.unsupported_platform").to_string()),
    }
}

// Бандл .app, из которого запущен процесс.
fn running_app_bundle() -> Result<PathBuf, String> {
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    executable
        .ancestors()
        .find(|path| path.extension().is_some_and(|extension| extension == "app"))
        .map(Path::to_path_buf)
        .ok_or_else(|| rust_i18n::t!("core.update.not_app_bundle").to_string())
}

// Монтирует DMG, копирует .app рядом с текущим и атомарно подменяет его.
fn replace_app_bundle(dmg: &Path) -> Result<PathBuf, String> {
    let app = running_app_bundle()?;
    let parent = app.parent().ok_or_else(|| rust_i18n::t!("core.update.no_bundle_parent").to_string())?;
    let mount = tempfile_dir("camunda-support-mount")?;
    run(Command::new("hdiutil").args(["attach", "-nobrowse", "-noautoopen", "-readonly", "-mountpoint"]).arg(&mount).arg(dmg))?;
    let copied = copy_bundle_from_mount(&mount, parent);
    let _ = run(Command::new("hdiutil").args(["detach", "-force"]).arg(&mount));
    let _ = std::fs::remove_dir(&mount);
    let staged = copied?;
    let _ = run(Command::new("xattr").args(["-dr", "com.apple.quarantine"]).arg(&staged));
    let retired = parent.join(format!(".{}.old", app.file_name().map(|name| name.to_string_lossy().to_string()).unwrap_or_default()));
    let _ = std::fs::remove_dir_all(&retired);
    std::fs::rename(&app, &retired)
        .map_err(|error| rust_i18n::t!("core.update.replace_denied", path = app.display(), error = error).to_string())?;
    if let Err(error) = std::fs::rename(&staged, &app) {
        let _ = std::fs::rename(&retired, &app);
        return Err(error.to_string());
    }
    let _ = std::fs::remove_dir_all(&retired);
    Ok(app)
}

fn copy_bundle_from_mount(mount: &Path, target_directory: &Path) -> Result<PathBuf, String> {
    let source = std::fs::read_dir(mount)
        .map_err(|error| error.to_string())?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| path.extension().is_some_and(|extension| extension == "app"))
        .ok_or_else(|| rust_i18n::t!("core.update.no_app_in_dmg").to_string())?;
    let staged = target_directory.join(".camunda-support-update.app");
    let _ = std::fs::remove_dir_all(&staged);
    run(Command::new("ditto").arg(&source).arg(&staged))?;
    Ok(staged)
}

fn tempfile_dir(prefix: &str) -> Result<PathBuf, String> {
    let path = std::env::temp_dir().join(format!("{prefix}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).map_err(|error| error.to_string())?;
    Ok(path)
}

fn run(command: &mut Command) -> Result<(), String> {
    let output = command.output().map_err(|error| error.to_string())?;
    if output.status.success() {
        return Ok(());
    }
    Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
}
