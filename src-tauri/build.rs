use std::process::Command;

fn main() {
    let describe = git(&["describe", "--tags", "--match", "v[0-9]*", "--long", "--always", "--dirty"]);
    let build = build_label(describe.as_deref());
    println!("cargo:rustc-env=CAMUNDA_SUPPORT_BUILD={build}");
    println!("cargo:rerun-if-changed=../.git/HEAD");
    println!("cargo:rerun-if-changed=../.git/refs/tags");
    tauri_build::build()
}

fn git(args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?.trim().to_string();
    (!text.is_empty()).then_some(text)
}

// Из вывода git describe делает короткую метку сборки: sha плюс -dirty, иначе local.
fn build_label(describe: Option<&str>) -> String {
    let Some(describe) = describe else { return "local".to_string() };
    let dirty = describe.ends_with("-dirty");
    let trimmed = describe.trim_end_matches("-dirty");
    let sha = trimmed.rsplit("-g").next().map(|value| value.to_string()).unwrap_or_default();
    let sha = if sha.len() >= 7 && sha.chars().all(|c| c.is_ascii_hexdigit()) { sha } else { trimmed.to_string() };
    if dirty { format!("{sha}-dirty") } else { sha }
}
