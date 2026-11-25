use std::fs;
use std::path::PathBuf;

fn main() {
    inject_app_toml_env();
    inject_account_api_url();
    ensure_runtime_app_toml();

    let mut attributes = tauri_build::Attributes::new();
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os == "windows" {
        attributes = attributes.windows_attributes(tauri_build::WindowsAttributes::new_without_app_manifest());
        embed_windows_common_controls_manifest();
    }
    if target_os == "macos" || target_os == "ios" {
        println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/lib/swift");
    }
    if target_os == "android" {
        println!("cargo:rustc-link-arg=-Wl,-z,max-page-size=16384");
    }
    tauri_build::try_build(attributes).expect("tauri_build failed");
}

fn embed_windows_common_controls_manifest() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR")).join("windows-app-manifest.xml");
    println!("cargo:rerun-if-changed={}", manifest.display());
    println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
    println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", manifest.display());
}

fn inject_app_toml_env() {
    let toml_path = "src/base/config/app.toml";
    println!("cargo:rerun-if-changed={}", toml_path);
    let text = match fs::read_to_string(toml_path) {
        Ok(t) => t,
        Err(_) => return,
    };
    if let Some(port) = read_toml_number(&text, "frontend", "dev_port") {
        println!("cargo:rustc-env=FRONTEND_DEV_PORT={}", port);
    }
    if let Some(port) = read_toml_number(&text, "web", "port") {
        println!("cargo:rustc-env=BACKEND_PORT={}", port);
    }
    let require_prefixes = read_toml_array(&text, "auth", "require_prefixes").unwrap_or_default();
    println!("cargo:rustc-env=AUTH_REQUIRE_PREFIXES={}", require_prefixes.join(","));
    let require_paths = read_toml_array(&text, "auth", "require_paths").unwrap_or_default();
    println!("cargo:rustc-env=AUTH_REQUIRE_PATHS={}", require_paths.join(","));
    if let Some(paths) = read_toml_array(&text, "auth", "warmup_allow_paths") {
        println!("cargo:rustc-env=AUTH_WARMUP_ALLOW_PATHS={}", paths.join(","));
    }
    if let Some(origins) = read_toml_array(&text, "web", "allow_origins") {
        println!("cargo:rustc-env=CORS_ALLOW_ORIGINS={}", origins.join(","));
    }
}

fn inject_account_api_url() {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let root = manifest_dir.parent().expect("workspace root");
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

fn read_toml_number(text: &str, section: &str, key: &str) -> Option<u32> {
    let section_marker = format!("[{}]", section);
    let after_section = text.split_once(&section_marker)?.1;
    let section_body = after_section.split("\n[").next()?;
    for line in section_body.lines() {
        let trimmed = line.trim();
        let Some(rest) = trimmed.strip_prefix(key) else {
            continue;
        };
        let rest = rest.trim_start();
        let rest = rest.strip_prefix('=')?.trim_start();
        let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if !digits.is_empty() {
            return digits.parse().ok();
        }
    }
    None
}

fn read_toml_array(text: &str, section: &str, key: &str) -> Option<Vec<String>> {
    let section_marker = format!("[{}]", section);
    let after_section = text.split_once(&section_marker)?.1;
    let section_body = after_section.split("\n[").next()?;
    let mut in_array = false;
    let mut values: Vec<String> = Vec::new();
    for line in section_body.lines() {
        let trimmed = line.trim();
        if !in_array {
            let Some(rest) = trimmed.strip_prefix(key) else { continue };
            let rest = rest.trim_start().strip_prefix('=')?.trim_start();
            let rest = rest.strip_prefix('[')?.trim();
            if rest.starts_with(']') {
                return Some(values);
            }
            in_array = true;
            let v = rest.trim_end_matches(',').trim().trim_matches('"').trim_matches('\'');
            if !v.is_empty() {
                values.push(v.to_string());
            }
        } else {
            if trimmed.starts_with(']') {
                return Some(values);
            }
            let v = trimmed.trim_end_matches(',').trim().trim_matches('"').trim_matches('\'');
            if !v.is_empty() {
                values.push(v.to_string());
            }
        }
    }
    if in_array && !values.is_empty() {
        Some(values)
    } else {
        None
    }
}

fn ensure_runtime_app_toml() {
    let manifest_dir = match std::env::var("CARGO_MANIFEST_DIR") {
        Ok(v) => v,
        Err(_) => return,
    };

    let src = PathBuf::from(&manifest_dir).join("src").join("base").join("config").join("app.toml");
    let dst = PathBuf::from(&manifest_dir).join("config").join("app.toml");

    println!("cargo:rerun-if-changed={}", src.display());

    let src_bytes = match fs::read(&src) {
        Ok(v) => v,
        Err(_) => return,
    };
    if let Ok(dst_bytes) = fs::read(&dst) {
        if dst_bytes == src_bytes {
            return;
        }
    }

    let Some(parent) = dst.parent() else { return };
    if fs::create_dir_all(parent).is_err() {
        return;
    }
    let _ = fs::write(dst, src_bytes);
}
