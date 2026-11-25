pub const PANDOC_VERSION: &str = "3.11";
pub const TYPST_VERSION: &str = "0.15.1";
pub const DENO_VERSION: &str = "2.9.6";
pub const UV_VERSION: &str = "0.12.10";
const PANDOC_RELEASE_BASE: &str = "https://github.com/jgm/pandoc/releases/download";
const TYPST_RELEASE_BASE: &str = "https://github.com/typst/typst/releases/download";
const DENO_RELEASE_BASE: &str = "https://github.com/denoland/deno/releases/download";
const UV_RELEASE_BASE: &str = "https://github.com/astral-sh/uv/releases/download";
const PDFIUM_RELEASE_BASE: &str = "https://github.com/bblanchon/pdfium-binaries/releases/latest/download";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExternalTool {
    Pandoc,
    Typst,
    Pdfium,
    Deno,
    Uv,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveKind {
    Zip,
    TarGz,
    TarXz,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Platform {
    pub os: &'static str,
    pub arch: &'static str,
}

impl Platform {
    pub fn current() -> Platform {
        Platform { os: std::env::consts::OS, arch: std::env::consts::ARCH }
    }
}

impl ExternalTool {
    pub fn name(self) -> &'static str {
        match self {
            ExternalTool::Pandoc => "pandoc",
            ExternalTool::Typst => "typst",
            ExternalTool::Pdfium => "pdfium",
            ExternalTool::Deno => "deno",
            ExternalTool::Uv => "uv",
        }
    }

    pub fn version(self) -> &'static str {
        match self {
            ExternalTool::Pandoc => PANDOC_VERSION,
            ExternalTool::Typst => TYPST_VERSION,
            ExternalTool::Deno => DENO_VERSION,
            ExternalTool::Uv => UV_VERSION,
            ExternalTool::Pdfium => "latest",
        }
    }

    pub fn install_dir_name(self) -> String {
        match self {
            ExternalTool::Pdfium => "pdfium".to_string(),
            other => format!("tools/{}-{}", other.name(), other.version()),
        }
    }

    pub fn executable_name(self, platform: Platform) -> &'static str {
        let windows = platform.os == "windows";
        match self {
            ExternalTool::Pandoc => {
                if windows {
                    "pandoc.exe"
                } else {
                    "pandoc"
                }
            }
            ExternalTool::Typst => {
                if windows {
                    "typst.exe"
                } else {
                    "typst"
                }
            }
            ExternalTool::Deno => {
                if windows {
                    "deno.exe"
                } else {
                    "deno"
                }
            }
            ExternalTool::Uv => {
                if windows {
                    "uv.exe"
                } else {
                    "uv"
                }
            }
            ExternalTool::Pdfium => match platform.os {
                "windows" => "pdfium.dll",
                "macos" => "libpdfium.dylib",
                _ => "libpdfium.so",
            },
        }
    }

    pub fn asset_name(self, platform: Platform) -> Result<String, String> {
        let unsupported = || format!("{} has no release package for platform {}/{}", self.name(), platform.os, platform.arch);
        match self {
            ExternalTool::Pandoc => {
                let suffix = match (platform.os, platform.arch) {
                    ("windows", "x86_64") => "windows-x86_64.zip",
                    ("macos", "x86_64") => "x86_64-macOS.zip",
                    ("macos", "aarch64") => "arm64-macOS.zip",
                    ("linux", "x86_64") => "linux-amd64.tar.gz",
                    ("linux", "aarch64") => "linux-arm64.tar.gz",
                    _ => return Err(unsupported()),
                };
                Ok(format!("pandoc-{}-{}", PANDOC_VERSION, suffix))
            }
            ExternalTool::Typst => {
                let target = match (platform.os, platform.arch) {
                    ("windows", "x86_64") => "x86_64-pc-windows-msvc.zip",
                    ("windows", "aarch64") => "aarch64-pc-windows-msvc.zip",
                    ("macos", "x86_64") => "x86_64-apple-darwin.tar.xz",
                    ("macos", "aarch64") => "aarch64-apple-darwin.tar.xz",
                    ("linux", "x86_64") => "x86_64-unknown-linux-musl.tar.xz",
                    ("linux", "aarch64") => "aarch64-unknown-linux-musl.tar.xz",
                    _ => return Err(unsupported()),
                };
                Ok(format!("typst-{}", target))
            }
            ExternalTool::Deno => {
                let target = match (platform.os, platform.arch) {
                    ("windows", "x86_64") => "x86_64-pc-windows-msvc",
                    ("windows", "aarch64") => "aarch64-pc-windows-msvc",
                    ("macos", "x86_64") => "x86_64-apple-darwin",
                    ("macos", "aarch64") => "aarch64-apple-darwin",
                    ("linux", "x86_64") => "x86_64-unknown-linux-gnu",
                    ("linux", "aarch64") => "aarch64-unknown-linux-gnu",
                    _ => return Err(unsupported()),
                };
                Ok(format!("deno-{}.zip", target))
            }
            ExternalTool::Uv => {
                let target = match (platform.os, platform.arch) {
                    ("windows", "x86_64") => "x86_64-pc-windows-msvc.zip",
                    ("windows", "aarch64") => "aarch64-pc-windows-msvc.zip",
                    ("macos", "x86_64") => "x86_64-apple-darwin.tar.gz",
                    ("macos", "aarch64") => "aarch64-apple-darwin.tar.gz",
                    ("linux", "x86_64") => "x86_64-unknown-linux-gnu.tar.gz",
                    ("linux", "aarch64") => "aarch64-unknown-linux-gnu.tar.gz",
                    _ => return Err(unsupported()),
                };
                Ok(format!("uv-{}", target))
            }
            ExternalTool::Pdfium => {
                let tag = match (platform.os, platform.arch) {
                    ("windows", "x86_64") => "win-x64",
                    ("windows", "aarch64") => "win-arm64",
                    ("macos", "x86_64") => "mac-x64",
                    ("macos", "aarch64") => "mac-arm64",
                    ("linux", "x86_64") => "linux-x64",
                    ("linux", "aarch64") => "linux-arm64",
                    _ => return Err(unsupported()),
                };
                Ok(format!("pdfium-{}.tgz", tag))
            }
        }
    }

    pub fn archive_kind(asset_name: &str) -> Result<ArchiveKind, String> {
        if asset_name.ends_with(".zip") {
            Ok(ArchiveKind::Zip)
        } else if asset_name.ends_with(".tar.gz") || asset_name.ends_with(".tgz") {
            Ok(ArchiveKind::TarGz)
        } else if asset_name.ends_with(".tar.xz") {
            Ok(ArchiveKind::TarXz)
        } else {
            Err(format!("unrecognized archive format: {}", asset_name))
        }
    }

    pub fn default_url(self, asset_name: &str) -> String {
        match self {
            ExternalTool::Pandoc => format!("{}/{}/{}", PANDOC_RELEASE_BASE, PANDOC_VERSION, asset_name),
            ExternalTool::Typst => format!("{}/v{}/{}", TYPST_RELEASE_BASE, TYPST_VERSION, asset_name),
            ExternalTool::Deno => format!("{}/v{}/{}", DENO_RELEASE_BASE, DENO_VERSION, asset_name),
            ExternalTool::Uv => format!("{}/{}/{}", UV_RELEASE_BASE, UV_VERSION, asset_name),
            ExternalTool::Pdfium => format!("{}/{}", PDFIUM_RELEASE_BASE, asset_name),
        }
    }

    pub fn download_url(self, mirror: Option<&str>, asset_name: &str) -> String {
        match mirror.map(str::trim).filter(|value| !value.is_empty()) {
            Some(base) => format!("{}/{}/{}", base.trim_end_matches('/'), self.name(), asset_name),
            None => self.default_url(asset_name),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WINDOWS: Platform = Platform { os: "windows", arch: "x86_64" };
    const MAC_ARM: Platform = Platform { os: "macos", arch: "aarch64" };

    #[test]
    fn github_urls_follow_release_layout() {
        let asset = ExternalTool::Pandoc.asset_name(WINDOWS).unwrap();
        assert_eq!(asset, "pandoc-3.11-windows-x86_64.zip");
        assert_eq!(ExternalTool::Pandoc.download_url(None, &asset), "https://github.com/jgm/pandoc/releases/download/3.11/pandoc-3.11-windows-x86_64.zip");
        let typst = ExternalTool::Typst.asset_name(MAC_ARM).unwrap();
        assert_eq!(ExternalTool::Typst.download_url(None, &typst), "https://github.com/typst/typst/releases/download/v0.15.1/typst-aarch64-apple-darwin.tar.xz");
        assert_eq!(ExternalTool::Pdfium.download_url(None, "pdfium-win-x64.tgz"), "https://github.com/bblanchon/pdfium-binaries/releases/latest/download/pdfium-win-x64.tgz");
        let deno = ExternalTool::Deno.asset_name(WINDOWS).unwrap();
        assert_eq!(deno, "deno-x86_64-pc-windows-msvc.zip");
        assert_eq!(ExternalTool::Deno.download_url(None, &deno), "https://github.com/denoland/deno/releases/download/v2.9.6/deno-x86_64-pc-windows-msvc.zip");
    }

    #[test]
    fn mirror_replaces_host_and_keeps_file_name() {
        let asset = ExternalTool::Pandoc.asset_name(WINDOWS).unwrap();
        assert_eq!(ExternalTool::Pandoc.download_url(Some("https://oss.example.com/tools/"), &asset), "https://oss.example.com/tools/pandoc/pandoc-3.11-windows-x86_64.zip");
        assert_eq!(ExternalTool::Pandoc.download_url(Some("   "), &asset), ExternalTool::Pandoc.default_url(&asset));
    }

    #[test]
    fn archive_kind_by_extension() {
        assert_eq!(ExternalTool::archive_kind("a.zip").unwrap(), ArchiveKind::Zip);
        assert_eq!(ExternalTool::archive_kind("a.tgz").unwrap(), ArchiveKind::TarGz);
        assert_eq!(ExternalTool::archive_kind("a.tar.xz").unwrap(), ArchiveKind::TarXz);
        assert!(ExternalTool::archive_kind("a.7z").is_err());
    }
}
