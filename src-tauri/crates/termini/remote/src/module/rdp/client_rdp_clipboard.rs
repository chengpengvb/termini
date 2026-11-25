use anyhow::anyhow;
use std::path::Path;
use std::io::{Read, Seek, SeekFrom};
use crate::model::dto::clipboard::ClipboardSnapshot;
use crate::module::rdp::protocol::core::cliprdr::{FileEntry, FileRequest, LocalImage, RemoteFile};

const FILETIME_EPOCH_OFFSET_SECONDS: u64 = 11_644_473_600;
const FILETIME_TICKS_PER_SECOND: u64 = 10_000_000;

#[cfg(any(target_os = "android", target_os = "ios"))]
pub(crate) fn write_text(_text: &str) -> anyhow::Result<()> {
    Err(anyhow!("clipboard is not supported on the current platform"))
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub(crate) fn write_text(text: &str) -> anyhow::Result<()> {
    use clipboard_rs::{Clipboard, ClipboardContext};
    let context = ClipboardContext::new().map_err(|error| anyhow!("{}", error))?;
    context.set_text(to_local(text)).map_err(|error| anyhow!("{}", error))
}

#[cfg(any(target_os = "android", target_os = "ios"))]
pub(crate) fn write_html(_html: &str) -> anyhow::Result<()> {
    Err(anyhow!("clipboard is not supported on the current platform"))
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub(crate) fn write_html(html: &str) -> anyhow::Result<()> {
    use clipboard_rs::{Clipboard, ClipboardContext};
    let context = ClipboardContext::new().map_err(|error| anyhow!("{}", error))?;
    context.set_html(html.to_string()).map_err(|error| anyhow!("{}", error))
}

#[cfg(any(target_os = "android", target_os = "ios"))]
pub(crate) fn write_image(_data: &[u8]) -> anyhow::Result<()> {
    Err(anyhow!("clipboard is not supported on the current platform"))
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub(crate) fn write_image(data: &[u8]) -> anyhow::Result<()> {
    use clipboard_rs::{Clipboard, ClipboardContext};
    use clipboard_rs::common::{RustImage, RustImageData};
    let context = ClipboardContext::new().map_err(|error| anyhow!("{}", error))?;
    let image = RustImageData::from_bytes(data).map_err(|error| anyhow!("{}", error))?;
    context.set_image(image).map_err(|error| anyhow!("{}", error))
}

#[cfg(any(target_os = "android", target_os = "ios"))]
pub(crate) fn write_remote_files(_files: &[RemoteFile]) -> anyhow::Result<()> {
    Err(anyhow!("clipboard is not supported on the current platform"))
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub(crate) fn write_remote_files(files: &[RemoteFile]) -> anyhow::Result<()> {
    use clipboard_rs::{Clipboard, ClipboardContext};
    let root = clip_staging_dir()?;
    let mut tops = Vec::new();
    for file in files {
        let path = staging_path(&root, &file.name)?;
        if file.directory {
            std::fs::create_dir_all(&path).map_err(|error| anyhow!("failed to create clipboard directory {:?}: {}", path, error))?;
        } else {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|error| anyhow!("failed to create clipboard directory {:?}: {}", parent, error))?;
            }
            std::fs::write(&path, &file.data).map_err(|error| anyhow!("failed to write clipboard file {:?}: {}", path, error))?;
        }
        let top = file.name.split(['\\', '/']).next().ok_or_else(|| anyhow!("cliprdr file name has no top-level component: {}", file.name))?;
        let top_path = root.join(top).to_string_lossy().to_string();
        if !tops.contains(&top_path) {
            tops.push(top_path);
        }
    }
    let context = ClipboardContext::new().map_err(|error| anyhow!("{}", error))?;
    context.set_files(tops).map_err(|error| anyhow!("{}", error))
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn clip_staging_dir() -> anyhow::Result<std::path::PathBuf> {
    const DATA_DIR_ENV: &str = "TERMINI_DATA_DIR";
    let data_dir = std::env::var(DATA_DIR_ENV).map_err(|error| anyhow!("failed to read environment variable {}: {}", DATA_DIR_ENV, error))?;
    let path = std::path::PathBuf::from(data_dir).join("rdp").join("clip");
    if path.exists() {
        std::fs::remove_dir_all(&path).map_err(|error| anyhow!("failed to clean clipboard directory {:?}: {}", path, error))?;
    }
    std::fs::create_dir_all(&path).map_err(|error| anyhow!("failed to create clipboard directory {:?}: {}", path, error))?;
    Ok(path)
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn staging_path(root: &std::path::Path, name: &str) -> anyhow::Result<std::path::PathBuf> {
    let mut path = root.to_path_buf();
    for component in name.split(['\\', '/']) {
        if component.is_empty() || component == "." || component == ".." {
            return Err(anyhow!("invalid cliprdr file name: {}", name));
        }
        path.push(component);
    }
    if !path.starts_with(root) {
        return Err(anyhow!("cliprdr file path escapes the staging directory: {}", name));
    }
    Ok(path)
}

fn to_wire(text: String) -> String {
    if cfg!(windows) {
        return text;
    }
    let mut converted = String::with_capacity(text.len());
    let mut previous = '\0';
    for current in text.chars() {
        match current {
            '\r' => converted.push_str("\r\n"),
            '\n' if previous != '\r' => converted.push_str("\r\n"),
            '\n' => {}
            _ => converted.push(current),
        }
        previous = current;
    }
    converted
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn to_local(text: &str) -> String {
    if cfg!(windows) {
        return text.to_string();
    }
    let mut converted = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(current) = chars.next() {
        if current != '\r' {
            converted.push(current);
            continue;
        }
        converted.push('\n');
        if chars.peek() == Some(&'\n') {
            chars.next();
        }
    }
    converted
}

pub(crate) fn from_snapshot(snapshot: &ClipboardSnapshot) -> (String, Vec<FileEntry>, String, Option<LocalImage>) {
    let text = to_wire(snapshot.text.clone());
    let files = files_from_raw(snapshot.files.clone());
    let html = snapshot.html.clone();
    let image = snapshot.image.as_ref().map(|image| LocalImage { width: image.width, height: image.height, rgba: image.rgba.clone(), png: image.png.clone() });
    (text, files, html, image)
}

pub(crate) fn files_from_raw(raw: Vec<String>) -> Vec<FileEntry> {
    let mut entries = Vec::new();
    for item in raw {
        let path = std::path::PathBuf::from(crate::util::clipboard_file_util::strip_file_uri(item.as_str()));
        let Some(name) = path.file_name().map(|name| name.to_string_lossy().to_string()) else {
            continue;
        };
        collect_entry(&path, &name, &mut entries);
    }
    entries
}

fn collect_entry(path: &Path, name: &str, entries: &mut Vec<FileEntry>) {
    let Ok(metadata) = path.metadata() else {
        return;
    };
    let write_time =
        metadata.modified().ok().and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok()).map(|elapsed| (elapsed.as_secs() + FILETIME_EPOCH_OFFSET_SECONDS) * FILETIME_TICKS_PER_SECOND);
    let Some(write_time) = write_time else {
        return;
    };
    let directory = metadata.is_dir();
    entries.push(FileEntry { path: path.to_string_lossy().to_string(), name: name.to_string(), size: metadata.len(), write_time, directory });
    if !directory {
        return;
    }
    let Ok(children) = std::fs::read_dir(path) else {
        return;
    };
    for child in children.flatten() {
        let child_name = format!("{}\\{}", name, child.file_name().to_string_lossy());
        collect_entry(&child.path(), &child_name, entries);
    }
}

pub(crate) fn read_file_range(path: &str, request: &FileRequest) -> anyhow::Result<Vec<u8>> {
    let metadata = std::fs::metadata(path).map_err(|error| anyhow!("failed to read file metadata {}: {}", path, error))?;
    if request.size_only {
        return Ok(metadata.len().to_le_bytes().to_vec());
    }
    let mut file = std::fs::File::open(path).map_err(|error| anyhow!("failed to open file {}: {}", path, error))?;
    file.seek(SeekFrom::Start(request.position)).map_err(|error| anyhow!("failed to seek file {}: {}", path, error))?;
    let remaining = metadata.len().saturating_sub(request.position).min(u64::from(request.length)) as usize;
    let mut data = vec![0u8; remaining];
    file.read_exact(&mut data).map_err(|error| anyhow!("failed to read file {}: {}", path, error))?;
    Ok(data)
}
