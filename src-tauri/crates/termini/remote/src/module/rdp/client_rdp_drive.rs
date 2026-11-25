use std::fs::File;
use std::fs::FileTimes;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::io::{Read, Seek, SeekFrom, Write};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use crate::module::rdp::protocol::core::rdpdr::DeviceIoRequest;
use crate::module::rdp::protocol::core::stream::{ReadCursor, WriteCursor};
use crate::module::rdp::protocol::core::error::{RdpProtocolError, RdpResult};

const IRP_MJ_CREATE: u32 = 0x0000_0000;
const IRP_MJ_CLOSE: u32 = 0x0000_0002;
const IRP_MJ_READ: u32 = 0x0000_0003;
const IRP_MJ_WRITE: u32 = 0x0000_0004;
const IRP_MJ_QUERY_INFORMATION: u32 = 0x0000_0005;
const IRP_MJ_SET_INFORMATION: u32 = 0x0000_0006;
const IRP_MJ_FLUSH_BUFFERS: u32 = 0x0000_0009;
const IRP_MJ_QUERY_VOLUME_INFORMATION: u32 = 0x0000_000A;
const IRP_MJ_SET_VOLUME_INFORMATION: u32 = 0x0000_000B;
const IRP_MJ_DIRECTORY_CONTROL: u32 = 0x0000_000C;
const IRP_MJ_DEVICE_CONTROL: u32 = 0x0000_000E;
const IRP_MJ_SHUTDOWN: u32 = 0x0000_0010;
const IRP_MJ_LOCK_CONTROL: u32 = 0x0000_0011;
const IRP_MJ_CLEANUP: u32 = 0x0000_0012;
const IRP_MJ_QUERY_SECURITY: u32 = 0x0000_0014;
const IRP_MJ_SET_SECURITY: u32 = 0x0000_0015;

const IRP_MN_QUERY_DIRECTORY: u32 = 0x0000_0001;

const FILE_SUPERSEDE: u32 = 0x0000_0000;
const FILE_OPEN: u32 = 0x0000_0001;
const FILE_CREATE: u32 = 0x0000_0002;
const FILE_OPEN_IF: u32 = 0x0000_0003;
const FILE_OVERWRITE: u32 = 0x0000_0004;
const FILE_OVERWRITE_IF: u32 = 0x0000_0005;

const FILE_SUPERSEDED: u8 = 0x00;
const FILE_OPENED: u8 = 0x01;
const FILE_OVERWRITTEN: u8 = 0x03;

const FILE_DIRECTORY_FILE: u32 = 0x0000_0001;

const GENERIC_WRITE: u32 = 0x4000_0000;
const GENERIC_ALL: u32 = 0x1000_0000;
const FILE_WRITE_DATA: u32 = 0x0000_0002;
const FILE_APPEND_DATA: u32 = 0x0000_0004;
const WRITE_ACCESS_MASK: u32 = GENERIC_WRITE | GENERIC_ALL | FILE_WRITE_DATA | FILE_APPEND_DATA;

const FILE_BASIC_INFORMATION: u32 = 4;
const FILE_STANDARD_INFORMATION: u32 = 5;
const FILE_RENAME_INFORMATION: u32 = 10;
const FILE_DISPOSITION_INFORMATION: u32 = 13;
const FILE_ALLOCATION_INFORMATION: u32 = 19;
const FILE_END_OF_FILE_INFORMATION: u32 = 20;
const FILE_ATTRIBUTE_TAG_INFORMATION: u32 = 35;

const FILE_DIRECTORY_INFORMATION: u32 = 1;
const FILE_FULL_DIRECTORY_INFORMATION: u32 = 2;
const FILE_BOTH_DIRECTORY_INFORMATION: u32 = 3;
const FILE_NAMES_INFORMATION: u32 = 12;

const FILE_FS_VOLUME_INFORMATION: u32 = 1;
const FILE_FS_SIZE_INFORMATION: u32 = 3;
const FILE_FS_DEVICE_INFORMATION: u32 = 4;
const FILE_FS_ATTRIBUTE_INFORMATION: u32 = 5;
const FILE_FS_FULL_SIZE_INFORMATION: u32 = 7;

const FILE_ATTRIBUTE_READONLY: u32 = 0x0000_0001;
const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x0000_0010;
const FILE_ATTRIBUTE_ARCHIVE: u32 = 0x0000_0020;
const FILE_ATTRIBUTE_NORMAL: u32 = 0x0000_0080;

const FILE_CASE_SENSITIVE_SEARCH: u32 = 0x0000_0001;
const FILE_CASE_PRESERVED_NAMES: u32 = 0x0000_0002;
const FILE_UNICODE_ON_DISK: u32 = 0x0000_0004;
const FILE_DEVICE_DISK: u32 = 0x0000_0007;
const MAX_COMPONENT_NAME_LENGTH: u32 = 260;

const STATUS_SUCCESS: u32 = 0x0000_0000;
const STATUS_NO_MORE_FILES: u32 = 0x8000_0006;
const STATUS_UNSUCCESSFUL: u32 = 0xC000_0001;
const STATUS_NO_SUCH_FILE: u32 = 0xC000_000F;
const STATUS_ACCESS_DENIED: u32 = 0xC000_0022;
const STATUS_OBJECT_NAME_COLLISION: u32 = 0xC000_0035;
const STATUS_NOT_SUPPORTED: u32 = 0xC000_00BB;

const FILETIME_EPOCH_OFFSET_SECONDS: u64 = 11_644_473_600;
const FILETIME_TICKS_PER_SECOND: u64 = 10_000_000;
const BYTES_PER_SECTOR: u32 = 512;
const SECTORS_PER_CLUSTER: u32 = 8;
const VOLUME_LABEL: &str = "termini";
const FILE_SYSTEM_NAME: &str = "NTFS";

struct DriveFile {
    path: PathBuf,
    handle: Option<File>,
    directory: bool,
    delete_pending: bool,
    entries: Vec<DirectoryEntry>,
    cursor: usize,
}

struct DirectoryEntry {
    name: String,
    size: u64,
    attributes: u32,
    write_time: u64,
}

pub struct DriveState {
    root: PathBuf,
    next_id: u32,
    files: HashMap<u32, DriveFile>,
}

impl DriveState {
    pub fn new(root: &str) -> Self {
        Self { root: PathBuf::from(root), next_id: 1, files: HashMap::new() }
    }
}

pub fn handle(state: &mut DriveState, request: &DeviceIoRequest) -> RdpResult<(u32, Vec<u8>)> {
    let mut payload = ReadCursor::new(&request.payload);
    match request.major {
        IRP_MJ_CREATE => create(state, &mut payload),
        IRP_MJ_CLOSE => Ok(close(state, request.file_id)),
        IRP_MJ_READ => read(state, request.file_id, &mut payload),
        IRP_MJ_WRITE => write(state, request.file_id, &mut payload),
        IRP_MJ_FLUSH_BUFFERS => flush(state, request.file_id),
        IRP_MJ_QUERY_INFORMATION => query_information(state, request.file_id, &mut payload),
        IRP_MJ_SET_INFORMATION => set_information(state, request.file_id, &mut payload),
        IRP_MJ_QUERY_VOLUME_INFORMATION => query_volume_information(state, &mut payload),
        IRP_MJ_SET_VOLUME_INFORMATION => Ok((STATUS_NOT_SUPPORTED, length_only_body())),
        IRP_MJ_DIRECTORY_CONTROL => directory_control(state, request.file_id, request.minor, &mut payload),
        IRP_MJ_DEVICE_CONTROL => Ok((STATUS_SUCCESS, device_control_body())),
        IRP_MJ_SHUTDOWN => Ok((STATUS_SUCCESS, vec![0u8; 5])),
        IRP_MJ_LOCK_CONTROL => Ok((STATUS_SUCCESS, vec![0u8; 5])),
        IRP_MJ_CLEANUP => Ok(cleanup(state, request.file_id)),
        IRP_MJ_QUERY_SECURITY => query_security(state, request.file_id, &mut payload),
        IRP_MJ_SET_SECURITY => set_security(&mut payload),
        other => Err(RdpProtocolError::unsupported(format!("drive IRP not implemented: 0x{:08x}", other))),
    }
}

fn create(state: &mut DriveState, payload: &mut ReadCursor<'_>) -> RdpResult<(u32, Vec<u8>)> {
    let desired_access = payload.read_u32_le()?;
    let allocation_size = payload.read_u32_le()? as u64 | ((payload.read_u32_le()? as u64) << 32);
    payload.read_u32_le()?;
    payload.read_u32_le()?;
    let disposition = payload.read_u32_le()?;
    let options = payload.read_u32_le()?;
    let path_length = payload.read_u32_le()? as usize;
    let raw = payload.read_slice(path_length)?;

    let Some(path) = resolve_path(&state.root, &decode_utf16(raw))? else {
        return Ok((STATUS_ACCESS_DENIED, create_body(0, 0)));
    };
    let exists = path.exists();
    let directory = options & FILE_DIRECTORY_FILE != 0 || (exists && path.is_dir());

    if exists && disposition == FILE_CREATE {
        return Ok((STATUS_OBJECT_NAME_COLLISION, create_body(0, 0)));
    }
    if !exists && (disposition == FILE_OPEN || disposition == FILE_OVERWRITE) {
        return Ok((STATUS_NO_SUCH_FILE, create_body(0, 0)));
    }

    let writable = desired_access & WRITE_ACCESS_MASK != 0;
    let handle = match directory {
        true => {
            if !exists {
                std::fs::create_dir_all(&path).map_err(|error| RdpProtocolError::protocol(format!("failed to create directory {:?}: {}", path, error)))?;
            }
            None
        }
        false => {
            let mut options = std::fs::OpenOptions::new();
            options.read(true).write(writable);
            match disposition {
                FILE_CREATE | FILE_OPEN_IF => options.create(true),
                FILE_SUPERSEDE | FILE_OVERWRITE | FILE_OVERWRITE_IF => options.create(true).truncate(writable),
                _ => &mut options,
            };
            match options.open(&path) {
                Ok(handle) => Some(handle),
                Err(_) => return Ok((STATUS_ACCESS_DENIED, create_body(0, 0))),
            }
        }
    };

    let file_id = state.next_id;
    state.next_id += 1;
    let information = match disposition {
        FILE_OPEN_IF => FILE_OPENED,
        FILE_OVERWRITE_IF => FILE_OVERWRITTEN,
        _ => FILE_SUPERSEDED,
    };
    if let (Some(handle), true) = (handle.as_ref(), allocation_size > 0) {
        handle.set_len(allocation_size).map_err(|error| RdpProtocolError::protocol(format!("failed to set file length: {}", error)))?;
    }
    state.files.insert(file_id, DriveFile { path, handle, directory, delete_pending: false, entries: Vec::new(), cursor: 0 });
    Ok((STATUS_SUCCESS, create_body(file_id, information)))
}

fn close(state: &mut DriveState, file_id: u32) -> (u32, Vec<u8>) {
    let Some(file) = state.files.remove(&file_id) else {
        return (STATUS_UNSUCCESSFUL, vec![0u8; 5]);
    };
    if !file.delete_pending {
        return (STATUS_SUCCESS, vec![0u8; 5]);
    }
    let removed = match file.directory {
        true => std::fs::remove_dir_all(&file.path),
        false => std::fs::remove_file(&file.path),
    };
    match removed {
        Ok(()) => (STATUS_SUCCESS, vec![0u8; 5]),
        Err(_) => (STATUS_UNSUCCESSFUL, vec![0u8; 5]),
    }
}

fn read(state: &mut DriveState, file_id: u32, payload: &mut ReadCursor<'_>) -> RdpResult<(u32, Vec<u8>)> {
    let length = payload.read_u32_le()? as usize;
    let offset = read_u64(payload)?;
    let Some(file) = state.files.get_mut(&file_id) else {
        return Ok((STATUS_UNSUCCESSFUL, read_body(&[])));
    };
    let Some(handle) = file.handle.as_mut() else {
        return Ok((STATUS_UNSUCCESSFUL, read_body(&[])));
    };
    handle.seek(SeekFrom::Start(offset)).map_err(|error| RdpProtocolError::protocol(format!("failed to seek file: {}", error)))?;
    let mut data = vec![0u8; length];
    let mut filled = 0usize;
    while filled < length {
        let count = handle.read(&mut data[filled..]).map_err(|error| RdpProtocolError::protocol(format!("failed to read file: {}", error)))?;
        if count == 0 {
            break;
        }
        filled += count;
    }
    data.truncate(filled);
    Ok((STATUS_SUCCESS, read_body(&data)))
}

fn write(state: &mut DriveState, file_id: u32, payload: &mut ReadCursor<'_>) -> RdpResult<(u32, Vec<u8>)> {
    let length = payload.read_u32_le()? as usize;
    let offset = read_u64(payload)?;
    payload.skip(20)?;
    let data = payload.read_slice(length)?;
    let Some(file) = state.files.get_mut(&file_id) else {
        return Ok((STATUS_UNSUCCESSFUL, write_body(0)));
    };
    let Some(handle) = file.handle.as_mut() else {
        return Ok((STATUS_UNSUCCESSFUL, write_body(0)));
    };
    handle.seek(SeekFrom::Start(offset)).map_err(|error| RdpProtocolError::protocol(format!("failed to seek file: {}", error)))?;
    handle.write_all(data).map_err(|error| RdpProtocolError::protocol(format!("failed to write file: {}", error)))?;
    Ok((STATUS_SUCCESS, write_body(length as u32)))
}

fn flush(state: &mut DriveState, file_id: u32) -> RdpResult<(u32, Vec<u8>)> {
    let Some(file) = state.files.get_mut(&file_id) else {
        return Ok((STATUS_UNSUCCESSFUL, Vec::new()));
    };
    let Some(handle) = file.handle.as_mut() else {
        return Ok((STATUS_SUCCESS, Vec::new()));
    };
    handle.sync_all().map_err(|error| RdpProtocolError::protocol(format!("failed to flush file: {}", error)))?;
    Ok((STATUS_SUCCESS, Vec::new()))
}

fn cleanup(state: &DriveState, file_id: u32) -> (u32, Vec<u8>) {
    if state.files.contains_key(&file_id) {
        (STATUS_SUCCESS, vec![0u8; 5])
    } else {
        (STATUS_UNSUCCESSFUL, vec![0u8; 5])
    }
}

fn query_security(state: &DriveState, file_id: u32, payload: &mut ReadCursor<'_>) -> RdpResult<(u32, Vec<u8>)> {
    payload.read_u32_le()?;
    if !state.files.contains_key(&file_id) {
        return Ok((STATUS_UNSUCCESSFUL, length_only_body()));
    }
    Ok((STATUS_NOT_SUPPORTED, length_only_body()))
}

fn set_security(payload: &mut ReadCursor<'_>) -> RdpResult<(u32, Vec<u8>)> {
    payload.read_u32_le()?;
    let length = payload.read_u32_le()?;
    payload.skip(24)?;
    payload.read_slice(length as usize)?;
    Ok((STATUS_NOT_SUPPORTED, set_information_body(length)))
}

fn query_information(state: &mut DriveState, file_id: u32, payload: &mut ReadCursor<'_>) -> RdpResult<(u32, Vec<u8>)> {
    let class = payload.read_u32_le()?;
    let Some(file) = state.files.get(&file_id) else {
        return Ok((STATUS_UNSUCCESSFUL, length_only_body()));
    };
    let Ok(metadata) = file.path.metadata() else {
        return Ok((STATUS_UNSUCCESSFUL, length_only_body()));
    };
    let attributes = attributes_of(&metadata);
    let write_time = filetime_of(&metadata);
    let mut body = WriteCursor::with_capacity(48);
    match class {
        FILE_BASIC_INFORMATION => {
            body.write_u32_le(36);
            body.write_slice(&write_time.to_le_bytes());
            body.write_slice(&write_time.to_le_bytes());
            body.write_slice(&write_time.to_le_bytes());
            body.write_slice(&write_time.to_le_bytes());
            body.write_u32_le(attributes);
        }
        FILE_STANDARD_INFORMATION => {
            body.write_u32_le(22);
            body.write_slice(&metadata.len().to_le_bytes());
            body.write_slice(&metadata.len().to_le_bytes());
            body.write_u32_le(1);
            body.write_u8(u8::from(file.delete_pending));
            body.write_u8(u8::from(metadata.is_dir()));
        }
        FILE_ATTRIBUTE_TAG_INFORMATION => {
            body.write_u32_le(8);
            body.write_u32_le(attributes);
            body.write_u32_le(0);
        }
        _ => {
            body.write_u32_le(0);
            return Ok((STATUS_UNSUCCESSFUL, body.into_vec()));
        }
    }
    Ok((STATUS_SUCCESS, body.into_vec()))
}

fn set_information(state: &mut DriveState, file_id: u32, payload: &mut ReadCursor<'_>) -> RdpResult<(u32, Vec<u8>)> {
    let class = payload.read_u32_le()?;
    let length = payload.read_u32_le()?;
    payload.skip(24)?;
    let root = state.root.clone();
    let Some(file) = state.files.get_mut(&file_id) else {
        return Ok((STATUS_UNSUCCESSFUL, set_information_body(length)));
    };
    let status = match class {
        FILE_BASIC_INFORMATION => set_basic_information(file, payload)?,
        FILE_END_OF_FILE_INFORMATION | FILE_ALLOCATION_INFORMATION => set_end_of_file(file, payload)?,
        FILE_DISPOSITION_INFORMATION => {
            file.delete_pending = true;
            STATUS_SUCCESS
        }
        FILE_RENAME_INFORMATION => set_rename_information(file, &root, payload)?,
        _ => STATUS_NOT_SUPPORTED,
    };
    Ok((status, set_information_body(length)))
}

fn set_basic_information(file: &mut DriveFile, payload: &mut ReadCursor<'_>) -> RdpResult<u32> {
    let _creation = read_u64(payload)?;
    let access = read_u64(payload)?;
    let write = read_u64(payload)?;
    let _change = read_u64(payload)?;
    payload.read_u32_le()?;
    let Some(handle) = file.handle.as_ref() else {
        return Ok(STATUS_SUCCESS);
    };
    let mut times = FileTimes::new();
    if let Some(accessed) = system_time_of(access) {
        times = times.set_accessed(accessed);
    }
    if let Some(modified) = system_time_of(write) {
        times = times.set_modified(modified);
    }
    match handle.set_times(times) {
        Ok(()) => Ok(STATUS_SUCCESS),
        Err(_) => Ok(STATUS_UNSUCCESSFUL),
    }
}

fn set_end_of_file(file: &mut DriveFile, payload: &mut ReadCursor<'_>) -> RdpResult<u32> {
    let size = read_u64(payload)?;
    let Some(handle) = file.handle.as_ref() else {
        return Ok(STATUS_UNSUCCESSFUL);
    };
    match handle.set_len(size) {
        Ok(()) => Ok(STATUS_SUCCESS),
        Err(_) => Ok(STATUS_UNSUCCESSFUL),
    }
}

fn set_rename_information(file: &mut DriveFile, root: &Path, payload: &mut ReadCursor<'_>) -> RdpResult<u32> {
    let replace = payload.read_u8()?;
    payload.read_u8()?;
    let name_length = payload.read_u32_le()? as usize;
    let raw = payload.read_slice(name_length)?;
    let Some(target) = resolve_path(root, &decode_utf16(raw))? else {
        return Ok(STATUS_ACCESS_DENIED);
    };
    if target.exists() && replace == 0 {
        return Ok(STATUS_OBJECT_NAME_COLLISION);
    }
    file.handle = None;
    match std::fs::rename(&file.path, &target) {
        Ok(()) => {
            file.path = target;
            Ok(STATUS_SUCCESS)
        }
        Err(_) => Ok(STATUS_UNSUCCESSFUL),
    }
}

fn query_volume_information(state: &mut DriveState, payload: &mut ReadCursor<'_>) -> RdpResult<(u32, Vec<u8>)> {
    let class = payload.read_u32_le()?;
    let (total, available) = volume_space(&state.root);
    let cluster = u64::from(BYTES_PER_SECTOR * SECTORS_PER_CLUSTER);
    let mut body = WriteCursor::with_capacity(64);
    match class {
        FILE_FS_VOLUME_INFORMATION => {
            let label = encode_utf16(VOLUME_LABEL);
            body.write_u32_le(u32::try_from(17 + label.len()).map_err(|_| RdpProtocolError::encoding("volume label is too long".to_string()))?);
            body.write_slice(&filetime_now().to_le_bytes());
            body.write_u32_le(0);
            body.write_u32_le(u32::try_from(label.len()).map_err(|_| RdpProtocolError::encoding("volume label is too long".to_string()))?);
            body.write_u8(0);
            body.write_slice(&label);
        }
        FILE_FS_SIZE_INFORMATION => {
            body.write_u32_le(24);
            body.write_slice(&(total / cluster).to_le_bytes());
            body.write_slice(&(available / cluster).to_le_bytes());
            body.write_u32_le(SECTORS_PER_CLUSTER);
            body.write_u32_le(BYTES_PER_SECTOR);
        }
        FILE_FS_FULL_SIZE_INFORMATION => {
            body.write_u32_le(32);
            body.write_slice(&(total / cluster).to_le_bytes());
            body.write_slice(&(available / cluster).to_le_bytes());
            body.write_slice(&(available / cluster).to_le_bytes());
            body.write_u32_le(SECTORS_PER_CLUSTER);
            body.write_u32_le(BYTES_PER_SECTOR);
        }
        FILE_FS_ATTRIBUTE_INFORMATION => {
            let name = encode_utf16(FILE_SYSTEM_NAME);
            body.write_u32_le(u32::try_from(12 + name.len()).map_err(|_| RdpProtocolError::encoding("file system name is too long".to_string()))?);
            body.write_u32_le(FILE_CASE_SENSITIVE_SEARCH | FILE_CASE_PRESERVED_NAMES | FILE_UNICODE_ON_DISK);
            body.write_u32_le(MAX_COMPONENT_NAME_LENGTH);
            body.write_u32_le(u32::try_from(name.len()).map_err(|_| RdpProtocolError::encoding("file system name is too long".to_string()))?);
            body.write_slice(&name);
        }
        FILE_FS_DEVICE_INFORMATION => {
            body.write_u32_le(8);
            body.write_u32_le(FILE_DEVICE_DISK);
            body.write_u32_le(0);
        }
        _ => {
            body.write_u32_le(0);
            return Ok((STATUS_UNSUCCESSFUL, body.into_vec()));
        }
    }
    Ok((STATUS_SUCCESS, body.into_vec()))
}

fn directory_control(state: &mut DriveState, file_id: u32, minor: u32, payload: &mut ReadCursor<'_>) -> RdpResult<(u32, Vec<u8>)> {
    if minor != IRP_MN_QUERY_DIRECTORY {
        return Ok((STATUS_NOT_SUPPORTED, length_only_body()));
    }
    let class = payload.read_u32_le()?;
    let initial = payload.read_u8()?;
    let path_length = payload.read_u32_le()? as usize;
    payload.skip(23)?;
    let raw = payload.read_slice(path_length)?;
    let root = state.root.clone();
    let Some(file) = state.files.get_mut(&file_id) else {
        return Ok((STATUS_UNSUCCESSFUL, directory_failure_body()));
    };
    let pattern = decode_utf16(raw);
    if initial != 0 {
        let Some(entries) = list_entries(&root, &pattern)? else {
            return Ok((STATUS_ACCESS_DENIED, directory_failure_body()));
        };
        file.entries = entries;
        file.cursor = 0;
    }
    let Some(entry) = file.entries.get(file.cursor) else {
        return Ok((STATUS_NO_MORE_FILES, directory_failure_body()));
    };
    file.cursor += 1;
    let name = encode_utf16(&entry.name);
    let mut body = WriteCursor::with_capacity(name.len() + 104);
    match class {
        FILE_DIRECTORY_INFORMATION => {
            body.write_u32_le(u32::try_from(64 + name.len()).map_err(|_| RdpProtocolError::encoding("directory entry is too long".to_string()))?);
            write_directory_common(&mut body, entry, &name);
        }
        FILE_FULL_DIRECTORY_INFORMATION => {
            body.write_u32_le(u32::try_from(68 + name.len()).map_err(|_| RdpProtocolError::encoding("directory entry is too long".to_string()))?);
            write_directory_common(&mut body, entry, &[]);
            body.write_u32_le(0);
            body.write_slice(&name);
        }
        FILE_BOTH_DIRECTORY_INFORMATION => {
            body.write_u32_le(u32::try_from(93 + name.len()).map_err(|_| RdpProtocolError::encoding("directory entry is too long".to_string()))?);
            write_directory_common(&mut body, entry, &[]);
            body.write_u32_le(0);
            body.write_u8(0);
            body.write_zeros(24);
            body.write_slice(&name);
        }
        FILE_NAMES_INFORMATION => {
            body.write_u32_le(u32::try_from(12 + name.len()).map_err(|_| RdpProtocolError::encoding("directory entry is too long".to_string()))?);
            body.write_u32_le(0);
            body.write_u32_le(0);
            body.write_u32_le(u32::try_from(name.len()).map_err(|_| RdpProtocolError::encoding("directory entry is too long".to_string()))?);
            body.write_slice(&name);
        }
        _ => return Ok((STATUS_NOT_SUPPORTED, directory_failure_body())),
    }
    Ok((STATUS_SUCCESS, body.into_vec()))
}

fn write_directory_common(body: &mut WriteCursor, entry: &DirectoryEntry, name: &[u8]) {
    body.write_u32_le(0);
    body.write_u32_le(0);
    body.write_slice(&entry.write_time.to_le_bytes());
    body.write_slice(&entry.write_time.to_le_bytes());
    body.write_slice(&entry.write_time.to_le_bytes());
    body.write_slice(&entry.write_time.to_le_bytes());
    body.write_slice(&entry.size.to_le_bytes());
    body.write_slice(&entry.size.to_le_bytes());
    body.write_u32_le(entry.attributes);
    body.write_u32_le(name_length(entry));
    body.write_slice(name);
}

fn name_length(entry: &DirectoryEntry) -> u32 {
    (entry.name.encode_utf16().count() * 2) as u32
}

fn list_entries(root: &Path, pattern: &str) -> RdpResult<Option<Vec<DirectoryEntry>>> {
    let parts: Vec<&str> = pattern.split(['\\', '/']).filter(|part| !part.is_empty()).collect();
    let (last, prefix) = match parts.split_last() {
        None => ("*", Vec::new()),
        Some((last, rest)) => (*last, rest.to_vec()),
    };
    if last == ".." {
        return Ok(None);
    }
    let Some(parent) = resolve_path(root, &prefix.join("\\"))? else {
        return Ok(None);
    };
    if last == "." {
        let Ok(metadata) = parent.metadata() else {
            return Ok(Some(Vec::new()));
        };
        return Ok(Some(vec![DirectoryEntry { name: ".".to_string(), size: 0, attributes: attributes_of(&metadata), write_time: filetime_of(&metadata) }]));
    }
    if last != "*" && last != "*.*" {
        let Some(path) = resolve_path(&parent, last)? else {
            return Ok(None);
        };
        let Ok(metadata) = path.metadata() else {
            return Ok(Some(Vec::new()));
        };
        return Ok(Some(vec![DirectoryEntry { name: last.to_string(), size: metadata.len(), attributes: attributes_of(&metadata), write_time: filetime_of(&metadata) }]));
    }
    let mut entries = Vec::new();
    for name in [".", ".."] {
        entries.push(DirectoryEntry { name: name.to_string(), size: 0, attributes: FILE_ATTRIBUTE_DIRECTORY, write_time: filetime_now() });
    }
    let children = std::fs::read_dir(&parent).map_err(|error| RdpProtocolError::protocol(format!("failed to enumerate directory {:?}: {}", parent, error)))?;
    for child in children.flatten() {
        let name = child.file_name().to_string_lossy().to_string();
        let Ok(metadata) = child.metadata() else {
            continue;
        };
        entries.push(DirectoryEntry { name, size: metadata.len(), attributes: attributes_of(&metadata), write_time: filetime_of(&metadata) });
    }
    Ok(Some(entries))
}

fn resolve_path(root: &Path, relative: &str) -> RdpResult<Option<PathBuf>> {
    let mut path = root.to_path_buf();
    for part in relative.split(['\\', '/']) {
        if part.is_empty() || part == "." {
            continue;
        }
        if part == ".." {
            return Ok(None);
        }
        path.push(part);
    }
    Ok(Some(path))
}

fn volume_space(root: &Path) -> (u64, u64) {
    let disks = sysinfo::Disks::new_with_refreshed_list();
    let mut best: Option<(usize, u64, u64)> = None;
    for disk in disks.list() {
        let mount = disk.mount_point();
        if !root.starts_with(mount) {
            continue;
        }
        let depth = mount.components().count();
        if best.is_none_or(|(current, _, _)| depth > current) {
            best = Some((depth, disk.total_space(), disk.available_space()));
        }
    }
    match best {
        Some((_, total, available)) => (total.max(1), available),
        None => (1, 0),
    }
}

fn attributes_of(metadata: &std::fs::Metadata) -> u32 {
    let mut attributes = match metadata.is_dir() {
        true => FILE_ATTRIBUTE_DIRECTORY,
        false => FILE_ATTRIBUTE_ARCHIVE,
    };
    if metadata.permissions().readonly() {
        attributes |= FILE_ATTRIBUTE_READONLY;
    }
    if attributes == FILE_ATTRIBUTE_ARCHIVE {
        attributes |= FILE_ATTRIBUTE_NORMAL;
    }
    attributes
}

fn filetime_of(metadata: &std::fs::Metadata) -> u64 {
    metadata.modified().ok().and_then(|time| time.duration_since(UNIX_EPOCH).ok()).map(|elapsed| (elapsed.as_secs() + FILETIME_EPOCH_OFFSET_SECONDS) * FILETIME_TICKS_PER_SECOND).unwrap_or_default()
}

fn filetime_now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|elapsed| (elapsed.as_secs() + FILETIME_EPOCH_OFFSET_SECONDS) * FILETIME_TICKS_PER_SECOND).unwrap_or_default()
}

fn system_time_of(filetime: u64) -> Option<SystemTime> {
    if filetime == 0 {
        return None;
    }
    let seconds = filetime / FILETIME_TICKS_PER_SECOND;
    let nanos = (filetime % FILETIME_TICKS_PER_SECOND) * 100;
    seconds.checked_sub(FILETIME_EPOCH_OFFSET_SECONDS).map(|elapsed| UNIX_EPOCH + Duration::new(elapsed, nanos as u32))
}

fn read_u64(payload: &mut ReadCursor<'_>) -> RdpResult<u64> {
    let low = payload.read_u32_le()? as u64;
    let high = payload.read_u32_le()? as u64;
    Ok((high << 32) | low)
}

fn encode_utf16(value: &str) -> Vec<u8> {
    value.encode_utf16().flat_map(|unit| unit.to_le_bytes()).collect()
}

fn decode_utf16(raw: &[u8]) -> String {
    let units = raw.chunks_exact(2).map(|pair| u16::from_le_bytes([pair[0], pair[1]])).take_while(|unit| *unit != 0).collect::<Vec<u16>>();
    String::from_utf16_lossy(&units)
}

fn create_body(file_id: u32, information: u8) -> Vec<u8> {
    let mut body = WriteCursor::with_capacity(5);
    body.write_u32_le(file_id);
    body.write_u8(information);
    body.into_vec()
}

fn read_body(data: &[u8]) -> Vec<u8> {
    let mut body = WriteCursor::with_capacity(data.len() + 4);
    body.write_u32_le(data.len() as u32);
    body.write_slice(data);
    body.into_vec()
}

fn write_body(length: u32) -> Vec<u8> {
    let mut body = WriteCursor::with_capacity(5);
    body.write_u32_le(length);
    body.write_u8(0);
    body.into_vec()
}

fn set_information_body(length: u32) -> Vec<u8> {
    let mut body = WriteCursor::with_capacity(4);
    body.write_u32_le(length);
    body.into_vec()
}

fn length_only_body() -> Vec<u8> {
    let mut body = WriteCursor::with_capacity(4);
    body.write_u32_le(0);
    body.into_vec()
}

fn directory_failure_body() -> Vec<u8> {
    let mut body = WriteCursor::with_capacity(5);
    body.write_u32_le(0);
    body.write_u8(0);
    body.into_vec()
}

fn device_control_body() -> Vec<u8> {
    let mut body = WriteCursor::with_capacity(4);
    body.write_u32_le(0);
    body.into_vec()
}
