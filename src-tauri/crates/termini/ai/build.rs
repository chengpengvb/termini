use std::fs;
use std::path::PathBuf;

fn main() {
    println!("cargo:rustc-check-cfg=cfg(desktop)");
    println!("cargo:rustc-check-cfg=cfg(mobile)");
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os == "android" || target_os == "ios" {
        println!("cargo:rustc-cfg=mobile");
    } else {
        println!("cargo:rustc-cfg=desktop");
    }
    inject_account_api_url();
}

fn inject_account_api_url() {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let root = manifest_dir.parent().and_then(|p| p.parent()).and_then(|p| p.parent()).and_then(|p| p.parent()).expect("workspace root");
    let env_path = root.join(".env");
    let env_dev_path = root.join(".env.development");
    println!("cargo:rerun-if-changed={}", env_path.display());
    println!("cargo:rerun-if-changed={}", env_dev_path.display());
    println!("cargo:rerun-if-env-changed=PROFILE");
    let profile = std::env::var("PROFILE").expect("PROFILE");
    let path = if profile == "release" { &env_path } else { &env_dev_path };
    let url = read_env_value(path, "VITE_ACCOUNT_API_URL").unwrap_or_else(|| panic!("VITE_ACCOUNT_API_URL missing in {}", path.display()));
    println!("cargo:rustc-env=ACCOUNT_API_URL={}", url);
}

fn read_env_value(path: &PathBuf, key: &str) -> Option<String> {
    let text = fs::read_to_string(path).ok()?;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some(rest) = trimmed.strip_prefix(key) else {
            continue;
        };
        let rest = rest.trim_start();
        let rest = rest.strip_prefix('=')?.trim();
        let value = rest.trim_matches('"').trim_matches('\'').trim();
        if value.is_empty() {
            return None;
        }
        return Some(value.to_string());
    }
    None
}
