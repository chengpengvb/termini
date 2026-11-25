use crate::core::stream::{ReadCursor, WriteCursor};
use crate::core::error::{RdpProtocolError, RdpResult};

const MSG_MONITOR_READY: u16 = 0x0001;
const MSG_FORMAT_LIST: u16 = 0x0002;
const MSG_FORMAT_LIST_RESPONSE: u16 = 0x0003;
const MSG_FORMAT_DATA_REQUEST: u16 = 0x0004;
const MSG_FORMAT_DATA_RESPONSE: u16 = 0x0005;
const MSG_TEMP_DIRECTORY: u16 = 0x0006;
const MSG_CLIP_CAPS: u16 = 0x0007;
const MSG_FILECONTENTS_REQUEST: u16 = 0x0008;
const MSG_FILECONTENTS_RESPONSE: u16 = 0x0009;
const MSG_LOCK_CLIPDATA: u16 = 0x000A;
const MSG_UNLOCK_CLIPDATA: u16 = 0x000B;

const FLAG_RESPONSE_OK: u16 = 0x0001;
const FLAG_RESPONSE_FAIL: u16 = 0x0002;
const FLAG_ASCII_NAMES: u16 = 0x0004;

const CAPSTYPE_GENERAL: u16 = 0x0001;
const CAPSTYPE_GENERAL_LENGTH: u16 = 12;
const CAPS_VERSION_2: u32 = 0x0000_0002;
const USE_LONG_FORMAT_NAMES: u32 = 0x0000_0002;
const STREAM_FILECLIP_ENABLED: u32 = 0x0000_0004;
const FILECLIP_NO_FILE_PATHS: u32 = 0x0000_0008;
const CLIENT_FILE_FLAGS: u32 = STREAM_FILECLIP_ENABLED | FILECLIP_NO_FILE_PATHS;

const CF_TEXT: u32 = 1;
const CF_DIB: u32 = 8;
const CF_UNICODETEXT: u32 = 13;
const CF_DIBV5: u32 = 17;
const TEXT_FORMATS: [u32; 2] = [CF_UNICODETEXT, CF_TEXT];

const FORMAT_ID_FILE_LIST: u32 = 0x0000_C0DE;
const FORMAT_ID_FILE_CONTENTS: u32 = 0x0000_C0E1;
const FORMAT_ID_PNG: u32 = 0x0000_C0DF;
const FORMAT_ID_HTML: u32 = 0x0000_C0E0;
const FORMAT_NAME_FILE_LIST: &str = "FileGroupDescriptorW";
const FORMAT_NAME_FILE_CONTENTS: &str = "FileContents";
const FORMAT_NAME_PNG: &str = "PNG";
const FORMAT_NAME_HTML: &str = "HTML Format";

const FILECONTENTS_SIZE: u32 = 0x0000_0001;
const FILECONTENTS_RANGE: u32 = 0x0000_0002;
const FILECONTENTS_SIZE_LENGTH: u32 = 8;

const FD_ATTRIBUTES: u32 = 0x0000_0004;
const FD_WRITESTIME: u32 = 0x0000_0020;
const FD_FILESIZE: u32 = 0x0000_0040;
const FD_PROGRESSUI: u32 = 0x0000_4000;
const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x0000_0010;
const FILE_ATTRIBUTE_ARCHIVE: u32 = 0x0000_0020;

const BI_RGB: u32 = 0;
const BI_BITFIELDS: u32 = 3;
const BITMAPINFOHEADER_SIZE: u32 = 40;
const BITMAPFILEHEADER_LENGTH: usize = 14;
const DIB_HEADER_MIN: usize = 16;

const HEADER_LENGTH: usize = 8;
const CAPSSET_HEADER_LENGTH: usize = 4;
const SHORT_FORMAT_NAME_LENGTH: usize = 32;
const DESCRIPTOR_LENGTH: usize = 592;
const DESCRIPTOR_NAME_LENGTH: usize = 520;
const TEMP_DIRECTORY_LENGTH: usize = 520;

pub struct FileEntry {
    pub path: String,
    pub name: String,
    pub size: u64,
    pub write_time: u64,
    pub directory: bool,
}

#[derive(Clone, Copy)]
pub struct FileRequest {
    pub stream_id: u32,
    pub index: usize,
    pub size_only: bool,
    pub position: u64,
    pub length: u32,
}

pub struct LocalImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    pub png: Vec<u8>,
}

pub struct RemoteFile {
    pub name: String,
    pub directory: bool,
    pub data: Vec<u8>,
    has_size: bool,
    size: u64,
}

#[derive(Clone, Copy)]
enum Pending {
    Unicode,
    Ansi,
    Html,
    Dib,
    DibV5,
    Png,
    FileList,
    FileSize { stream_id: u32, index: usize },
    FileRange { stream_id: u32, index: usize },
}

#[derive(Default)]
pub struct ClipboardState {
    ready: bool,
    long_format_names: bool,
    file_clip: bool,
    pending: Option<Pending>,
    local_text: String,
    local_html: String,
    local_files: Vec<FileEntry>,
    local_image: Option<LocalImage>,
    remote_files: Vec<RemoteFile>,
    fetch_index: usize,
    next_stream_id: u32,
}

impl ClipboardState {
    pub fn new(local_text: String) -> Self {
        Self { local_text, next_stream_id: 1, ..Default::default() }
    }

    pub fn file_entry(&self, index: usize) -> Option<&FileEntry> {
        self.local_files.get(index)
    }
}

#[derive(Default)]
pub struct ClipboardOutcome {
    pub responses: Vec<Vec<u8>>,
    pub text: Option<String>,
    pub html: Option<String>,
    pub image: Option<Vec<u8>>,
    pub files: Option<Vec<RemoteFile>>,
    pub file_request: Option<FileRequest>,
}

pub fn process(state: &mut ClipboardState, message: &[u8]) -> RdpResult<ClipboardOutcome> {
    let mut cursor = ReadCursor::new(message);
    let msg_type = cursor.read_u16_le()?;
    let msg_flags = cursor.read_u16_le()?;
    let data_len = cursor.read_u32_le()? as usize;
    let mut body = cursor.sub_cursor(data_len)?;
    match msg_type {
        MSG_CLIP_CAPS => {
            read_capabilities(state, &mut body)?;
            Ok(ClipboardOutcome::default())
        }
        MSG_MONITOR_READY => {
            state.ready = true;
            Ok(ClipboardOutcome { responses: vec![build_capabilities(state)?, build_format_list(state)?], ..Default::default() })
        }
        MSG_FORMAT_LIST => read_format_list(state, &mut body, msg_flags),
        MSG_FORMAT_LIST_RESPONSE => Ok(ClipboardOutcome::default()),
        MSG_FORMAT_DATA_REQUEST => read_data_request(state, &mut body),
        MSG_FORMAT_DATA_RESPONSE => read_data_response(state, body.rest(), msg_flags),
        MSG_FILECONTENTS_REQUEST => read_file_contents_request(state, &mut body),
        MSG_FILECONTENTS_RESPONSE => read_file_contents_response(state, body.rest(), msg_flags),
        MSG_LOCK_CLIPDATA | MSG_UNLOCK_CLIPDATA => {
            body.read_u32_le()?;
            Ok(ClipboardOutcome::default())
        }
        MSG_TEMP_DIRECTORY => {
            body.skip(TEMP_DIRECTORY_LENGTH)?;
            Ok(ClipboardOutcome::default())
        }
        other => Err(RdpProtocolError::unsupported(format!("cliprdr message 0x{:04x} not implemented", other))),
    }
}

pub fn set_local(state: &mut ClipboardState, text: String, files: Vec<FileEntry>, html: String, image: Option<LocalImage>) -> RdpResult<Option<Vec<u8>>> {
    if let Some(image) = &image {
        let expected = image.width as usize * image.height as usize * 4;
        if image.rgba.len() != expected {
            return Err(RdpProtocolError::encoding(format!("cliprdr local image RGBA length mismatch: {}x{} need {} have {}", image.width, image.height, expected, image.rgba.len())));
        }
        if image.width == 0 || image.height == 0 {
            return Err(RdpProtocolError::encoding(format!("cliprdr local image size is zero: {}x{}", image.width, image.height)));
        }
        if image.png.is_empty() {
            return Err(RdpProtocolError::encoding("cliprdr local image has no PNG data".to_string()));
        }
    }
    let same_text = state.local_text == text;
    let same_html = state.local_html == html;
    let same_files = state.local_files.len() == files.len() && state.local_files.iter().zip(&files).all(|(current, next)| current.path == next.path && current.size == next.size);
    let same_image = match (&state.local_image, &image) {
        (None, None) => true,
        (Some(current), Some(next)) => current.png == next.png,
        _ => false,
    };
    if same_text && same_html && same_files && same_image {
        return Ok(None);
    }
    state.local_text = text;
    state.local_html = html;
    state.local_files = files;
    state.local_image = image;
    if !state.ready {
        return Ok(None);
    }
    Ok(Some(build_format_list(state)?))
}

pub fn build_file_contents_response(stream_id: u32, data: &[u8]) -> RdpResult<Vec<u8>> {
    let mut body = WriteCursor::with_capacity(data.len() + 4);
    body.write_u32_le(stream_id);
    body.write_slice(data);
    build_message(MSG_FILECONTENTS_RESPONSE, FLAG_RESPONSE_OK, body.as_slice())
}

pub fn build_file_contents_failure(stream_id: u32) -> RdpResult<Vec<u8>> {
    let mut body = WriteCursor::with_capacity(4);
    body.write_u32_le(stream_id);
    build_message(MSG_FILECONTENTS_RESPONSE, FLAG_RESPONSE_FAIL, body.as_slice())
}

fn read_capabilities(state: &mut ClipboardState, body: &mut ReadCursor<'_>) -> RdpResult<()> {
    let count = body.read_u16_le()?;
    body.skip(2)?;
    for _ in 0..count {
        let set_type = body.read_u16_le()?;
        let length = body.read_u16_le()? as usize;
        if length < CAPSSET_HEADER_LENGTH {
            return Err(RdpProtocolError::protocol(format!("cliprdr capability set too short: {}", length)));
        }
        let mut set = body.sub_cursor(length - CAPSSET_HEADER_LENGTH)?;
        if set_type != CAPSTYPE_GENERAL {
            return Err(RdpProtocolError::unsupported(format!("cliprdr capability set {} not implemented", set_type)));
        }
        set.skip(4)?;
        let flags = set.read_u32_le()?;
        state.long_format_names = flags & USE_LONG_FORMAT_NAMES != 0;
        state.file_clip = flags & STREAM_FILECLIP_ENABLED != 0;
    }
    Ok(())
}

fn read_format_list(state: &mut ClipboardState, body: &mut ReadCursor<'_>, msg_flags: u16) -> RdpResult<ClipboardOutcome> {
    let ascii_names = msg_flags & FLAG_ASCII_NAMES != 0;
    let mut formats = Vec::new();
    while body.remaining() > 0 {
        let format = body.read_u32_le()?;
        let name = match state.long_format_names {
            true => read_long_name(body)?,
            false if ascii_names => read_short_ascii_name(body)?,
            false => read_short_unicode_name(body)?,
        };
        formats.push((format, name));
    }
    state.pending = None;
    state.remote_files.clear();
    state.fetch_index = 0;
    let mut responses = vec![build_message(MSG_FORMAT_LIST_RESPONSE, FLAG_RESPONSE_OK, &[])?];
    if let Some(pending) = pick_format(state, &formats) {
        let format_id = request_format_id(pending, &formats)?;
        state.pending = Some(pending);
        let mut request = WriteCursor::with_capacity(4);
        request.write_u32_le(format_id);
        responses.push(build_message(MSG_FORMAT_DATA_REQUEST, 0, request.as_slice())?);
    }
    Ok(ClipboardOutcome { responses, ..Default::default() })
}

fn pick_format(state: &ClipboardState, formats: &[(u32, String)]) -> Option<Pending> {
    if state.file_clip && formats.iter().any(|(_, name)| name == FORMAT_NAME_FILE_LIST) {
        return Some(Pending::FileList);
    }
    if formats.iter().any(|(format, _)| *format == CF_DIB) {
        return Some(Pending::Dib);
    }
    if formats.iter().any(|(_, name)| name == FORMAT_NAME_PNG) {
        return Some(Pending::Png);
    }
    if formats.iter().any(|(format, _)| *format == CF_DIBV5) {
        return Some(Pending::DibV5);
    }
    if formats.iter().any(|(_, name)| name == FORMAT_NAME_HTML) {
        return Some(Pending::Html);
    }
    if formats.iter().any(|(format, _)| *format == CF_UNICODETEXT) {
        return Some(Pending::Unicode);
    }
    if formats.iter().any(|(format, _)| *format == CF_TEXT) {
        return Some(Pending::Ansi);
    }
    None
}

fn request_format_id(pending: Pending, formats: &[(u32, String)]) -> RdpResult<u32> {
    let found = match pending {
        Pending::Unicode => formats.iter().find(|(format, _)| *format == CF_UNICODETEXT).map(|(format, _)| *format),
        Pending::Ansi => formats.iter().find(|(format, _)| *format == CF_TEXT).map(|(format, _)| *format),
        Pending::Html => formats.iter().find(|(_, name)| name == FORMAT_NAME_HTML).map(|(format, _)| *format),
        Pending::Dib => Some(CF_DIB),
        Pending::DibV5 => Some(CF_DIBV5),
        Pending::Png => formats.iter().find(|(_, name)| name == FORMAT_NAME_PNG).map(|(format, _)| *format),
        Pending::FileList => formats.iter().find(|(_, name)| name == FORMAT_NAME_FILE_LIST).map(|(format, _)| *format),
        Pending::FileSize { .. } | Pending::FileRange { .. } => None,
    };
    found.ok_or_else(|| RdpProtocolError::protocol("cliprdr selected format is not in the format list".to_string()))
}

fn read_data_request(state: &ClipboardState, body: &mut ReadCursor<'_>) -> RdpResult<ClipboardOutcome> {
    let format = body.read_u32_le()?;
    let data = match format {
        FORMAT_ID_FILE_LIST => build_file_descriptors(state)?,
        FORMAT_ID_FILE_CONTENTS => None,
        CF_DIB => state.local_image.as_ref().map(encode_dib).transpose()?,
        FORMAT_ID_PNG => state.local_image.as_ref().map(|image| image.png.clone()),
        FORMAT_ID_HTML => encode_html_format(&state.local_html),
        _ => encode_text(format, &state.local_text),
    };
    let response = match data {
        Some(data) => build_message(MSG_FORMAT_DATA_RESPONSE, FLAG_RESPONSE_OK, &data)?,
        None => build_message(MSG_FORMAT_DATA_RESPONSE, FLAG_RESPONSE_FAIL, &[])?,
    };
    Ok(ClipboardOutcome { responses: vec![response], ..Default::default() })
}

fn read_data_response(state: &mut ClipboardState, data: &[u8], msg_flags: u16) -> RdpResult<ClipboardOutcome> {
    let Some(pending) = state.pending.take() else {
        return Ok(ClipboardOutcome::default());
    };
    if msg_flags & FLAG_RESPONSE_OK == 0 {
        return Ok(ClipboardOutcome::default());
    }
    match pending {
        Pending::Unicode => {
            let text = decode_unicode_text(data)?;
            state.local_text = text.clone();
            Ok(ClipboardOutcome { text: Some(text), ..Default::default() })
        }
        Pending::Ansi => {
            let text = decode_ansi_text(data)?;
            state.local_text = text.clone();
            Ok(ClipboardOutcome { text: Some(text), ..Default::default() })
        }
        Pending::Html => {
            let html = decode_html_format(data)?;
            state.local_html = html.clone();
            Ok(ClipboardOutcome { html: Some(html), ..Default::default() })
        }
        Pending::Dib | Pending::DibV5 => {
            let image = dib_to_bmp(data)?;
            Ok(ClipboardOutcome { image: Some(image), ..Default::default() })
        }
        Pending::Png => {
            if data.is_empty() {
                return Err(RdpProtocolError::protocol("cliprdr PNG data is empty".to_string()));
            }
            Ok(ClipboardOutcome { image: Some(data.to_vec()), ..Default::default() })
        }
        Pending::FileList => {
            state.remote_files = parse_file_list(data)?;
            state.fetch_index = 0;
            advance_file_fetch(state)
        }
        Pending::FileSize { .. } | Pending::FileRange { .. } => Err(RdpProtocolError::protocol("cliprdr received FORMAT_DATA_RESPONSE during file contents phase".to_string())),
    }
}

fn read_file_contents_request(state: &ClipboardState, body: &mut ReadCursor<'_>) -> RdpResult<ClipboardOutcome> {
    let stream_id = body.read_u32_le()?;
    let index = body.read_u32_le()? as usize;
    let flags = body.read_u32_le()?;
    let position_low = body.read_u32_le()?;
    let position_high = body.read_u32_le()?;
    let length = body.read_u32_le()?;
    let size_only = match flags {
        FILECONTENTS_SIZE => true,
        FILECONTENTS_RANGE => false,
        other => return Err(RdpProtocolError::protocol(format!("cliprdr invalid file contents request flags: 0x{:08x}", other))),
    };
    if size_only && (length != FILECONTENTS_SIZE_LENGTH || position_low != 0 || position_high != 0) {
        return Err(RdpProtocolError::protocol(format!("cliprdr invalid file size request: length={} position={}:{}", length, position_high, position_low)));
    }
    if state.local_files.get(index).is_none() {
        return Ok(ClipboardOutcome { responses: vec![build_file_contents_failure(stream_id)?], ..Default::default() });
    }
    let position = (u64::from(position_high) << 32) | u64::from(position_low);
    Ok(ClipboardOutcome { file_request: Some(FileRequest { stream_id, index, size_only, position, length }), ..Default::default() })
}

fn read_file_contents_response(state: &mut ClipboardState, data: &[u8], msg_flags: u16) -> RdpResult<ClipboardOutcome> {
    let pending = match state.pending {
        Some(Pending::FileSize { stream_id, index }) => Pending::FileSize { stream_id, index },
        Some(Pending::FileRange { stream_id, index }) => Pending::FileRange { stream_id, index },
        _ => return Err(RdpProtocolError::protocol("cliprdr received unrequested file contents".to_string())),
    };
    if msg_flags & FLAG_RESPONSE_OK == 0 {
        return Err(RdpProtocolError::protocol("cliprdr file contents request failed".to_string()));
    }
    if data.len() < 4 {
        return Err(RdpProtocolError::underrun(format!("cliprdr file contents response is missing streamId: {}", data.len())));
    }
    let stream_id = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    let payload = &data[4..];
    match pending {
        Pending::FileSize { stream_id: expected, index } => {
            if stream_id != expected {
                return Err(RdpProtocolError::protocol(format!("cliprdr file size streamId mismatch: {} != {}", stream_id, expected)));
            }
            if payload.len() != FILECONTENTS_SIZE_LENGTH as usize {
                return Err(RdpProtocolError::protocol(format!("cliprdr invalid file size payload length: {}", payload.len())));
            }
            let size = u64::from_le_bytes(payload.try_into().map_err(|_| RdpProtocolError::protocol("cliprdr file size is not 8 bytes".to_string()))?);
            let file = state.remote_files.get_mut(index).ok_or_else(|| RdpProtocolError::protocol(format!("cliprdr file size index out of range: {}", index)))?;
            file.size = size;
            file.has_size = true;
            advance_file_fetch(state)
        }
        Pending::FileRange { stream_id: expected, index } => {
            if stream_id != expected {
                return Err(RdpProtocolError::protocol(format!("cliprdr file contents streamId mismatch: {} != {}", stream_id, expected)));
            }
            if payload.is_empty() {
                return Err(RdpProtocolError::protocol("cliprdr file contents response is empty".to_string()));
            }
            let file = state.remote_files.get_mut(index).ok_or_else(|| RdpProtocolError::protocol(format!("cliprdr file contents index out of range: {}", index)))?;
            file.data.extend_from_slice(payload);
            if file.data.len() as u64 > file.size {
                return Err(RdpProtocolError::protocol(format!("cliprdr file contents exceed declared size: {} > {}", file.data.len(), file.size)));
            }
            advance_file_fetch(state)
        }
        _ => Err(RdpProtocolError::protocol("cliprdr invalid file contents state".to_string())),
    }
}

fn advance_file_fetch(state: &mut ClipboardState) -> RdpResult<ClipboardOutcome> {
    loop {
        if state.fetch_index >= state.remote_files.len() {
            state.pending = None;
            return Ok(ClipboardOutcome { files: Some(std::mem::take(&mut state.remote_files)), ..Default::default() });
        }
        let index = state.fetch_index;
        let directory = state.remote_files[index].directory;
        let has_size = state.remote_files[index].has_size;
        let size = state.remote_files[index].size;
        let received = state.remote_files[index].data.len() as u64;
        if directory || (has_size && size == 0) {
            state.fetch_index += 1;
            continue;
        }
        if !has_size {
            let stream_id = alloc_stream(state);
            state.pending = Some(Pending::FileSize { stream_id, index });
            return Ok(ClipboardOutcome { responses: vec![build_file_contents_request(stream_id, index as u32, FILECONTENTS_SIZE, 0, FILECONTENTS_SIZE_LENGTH)?], ..Default::default() });
        }
        if received == size {
            state.fetch_index += 1;
            continue;
        }
        let remaining = size - received;
        let length = if remaining > u64::from(u32::MAX) { u32::MAX } else { remaining as u32 };
        let stream_id = alloc_stream(state);
        state.pending = Some(Pending::FileRange { stream_id, index });
        return Ok(ClipboardOutcome { responses: vec![build_file_contents_request(stream_id, index as u32, FILECONTENTS_RANGE, received, length)?], ..Default::default() });
    }
}

fn alloc_stream(state: &mut ClipboardState) -> u32 {
    let stream_id = state.next_stream_id;
    state.next_stream_id = match state.next_stream_id.wrapping_add(1) {
        0 => 1,
        next => next,
    };
    stream_id
}

fn build_file_contents_request(stream_id: u32, index: u32, flags: u32, position: u64, length: u32) -> RdpResult<Vec<u8>> {
    let mut body = WriteCursor::with_capacity(28);
    body.write_u32_le(stream_id);
    body.write_u32_le(index);
    body.write_u32_le(flags);
    body.write_u32_le(position as u32);
    body.write_u32_le((position >> 32) as u32);
    body.write_u32_le(length);
    build_message(MSG_FILECONTENTS_REQUEST, 0, body.as_slice())
}

fn build_capabilities(state: &ClipboardState) -> RdpResult<Vec<u8>> {
    let mut flags = 0u32;
    if state.long_format_names {
        flags |= USE_LONG_FORMAT_NAMES;
    }
    if state.file_clip {
        flags |= CLIENT_FILE_FLAGS;
    }
    let mut body = WriteCursor::with_capacity(CAPSSET_HEADER_LENGTH + CAPSTYPE_GENERAL_LENGTH as usize);
    body.write_u16_le(1);
    body.write_u16_le(0);
    body.write_u16_le(CAPSTYPE_GENERAL);
    body.write_u16_le(CAPSTYPE_GENERAL_LENGTH);
    body.write_u32_le(CAPS_VERSION_2);
    body.write_u32_le(flags);
    build_message(MSG_CLIP_CAPS, 0, body.as_slice())
}

fn build_format_list(state: &ClipboardState) -> RdpResult<Vec<u8>> {
    let mut body = WriteCursor::with_capacity(8 + SHORT_FORMAT_NAME_LENGTH * 4);
    if state.file_clip && !state.local_files.is_empty() {
        write_format(&mut body, state, FORMAT_ID_FILE_LIST, FORMAT_NAME_FILE_LIST)?;
        write_format(&mut body, state, FORMAT_ID_FILE_CONTENTS, FORMAT_NAME_FILE_CONTENTS)?;
    } else {
        if state.local_image.is_some() {
            write_format(&mut body, state, CF_DIB, "")?;
            write_format(&mut body, state, FORMAT_ID_PNG, FORMAT_NAME_PNG)?;
        }
        if !state.local_html.is_empty() {
            write_format(&mut body, state, FORMAT_ID_HTML, FORMAT_NAME_HTML)?;
        }
        if !state.local_text.is_empty() {
            for format in TEXT_FORMATS {
                write_format(&mut body, state, format, "")?;
            }
        }
    }
    build_message(MSG_FORMAT_LIST, 0, body.as_slice())
}

fn write_format(body: &mut WriteCursor, state: &ClipboardState, format: u32, name: &str) -> RdpResult<()> {
    body.write_u32_le(format);
    match state.long_format_names {
        true => {
            body.write_utf16_le(name);
            body.write_u16_le(0);
        }
        false => body.write_utf16_le_padded(name, SHORT_FORMAT_NAME_LENGTH)?,
    }
    Ok(())
}

fn build_file_descriptors(state: &ClipboardState) -> RdpResult<Option<Vec<u8>>> {
    if state.local_files.is_empty() {
        return Ok(None);
    }
    let count = u32::try_from(state.local_files.len()).map_err(|_| RdpProtocolError::encoding(format!("cliprdr too many files: {}", state.local_files.len())))?;
    let mut body = WriteCursor::with_capacity(4 + state.local_files.len() * DESCRIPTOR_LENGTH);
    body.write_u32_le(count);
    for entry in &state.local_files {
        body.write_u32_le(FD_ATTRIBUTES | FD_WRITESTIME | FD_FILESIZE | FD_PROGRESSUI);
        body.write_zeros(32);
        body.write_u32_le(match entry.directory {
            true => FILE_ATTRIBUTE_DIRECTORY,
            false => FILE_ATTRIBUTE_ARCHIVE,
        });
        body.write_zeros(16);
        body.write_u32_le(entry.write_time as u32);
        body.write_u32_le((entry.write_time >> 32) as u32);
        body.write_u32_le((entry.size >> 32) as u32);
        body.write_u32_le(entry.size as u32);
        body.write_utf16_le_padded(&entry.name, DESCRIPTOR_NAME_LENGTH)?;
    }
    Ok(Some(body.into_vec()))
}

fn parse_file_list(data: &[u8]) -> RdpResult<Vec<RemoteFile>> {
    let mut cursor = ReadCursor::new(data);
    let count = cursor.read_u32_le()? as usize;
    if cursor.remaining() != count * DESCRIPTOR_LENGTH {
        return Err(RdpProtocolError::protocol(format!("cliprdr file list length mismatch: count={} remaining={}", count, cursor.remaining())));
    }
    let mut files = Vec::with_capacity(count);
    for _ in 0..count {
        let flags = cursor.read_u32_le()?;
        cursor.skip(32)?;
        let attributes = cursor.read_u32_le()?;
        cursor.skip(16)?;
        let write_low = cursor.read_u32_le()?;
        let write_high = cursor.read_u32_le()?;
        let size_high = cursor.read_u32_le()?;
        let size_low = cursor.read_u32_le()?;
        let name = read_utf16_fixed(&mut cursor, DESCRIPTOR_NAME_LENGTH)?;
        if name.is_empty() {
            return Err(RdpProtocolError::protocol("cliprdr file descriptor name is empty".to_string()));
        }
        let _write_time = (u64::from(write_high) << 32) | u64::from(write_low);
        let has_size = flags & FD_FILESIZE != 0;
        let size = (u64::from(size_high) << 32) | u64::from(size_low);
        files.push(RemoteFile { name, directory: attributes & FILE_ATTRIBUTE_DIRECTORY != 0, data: Vec::new(), has_size, size });
    }
    Ok(files)
}

fn build_message(msg_type: u16, msg_flags: u16, body: &[u8]) -> RdpResult<Vec<u8>> {
    let data_len = u32::try_from(body.len()).map_err(|_| RdpProtocolError::encoding(format!("cliprdr message body too long: {}", body.len())))?;
    let mut cursor = WriteCursor::with_capacity(HEADER_LENGTH + body.len());
    cursor.write_u16_le(msg_type);
    cursor.write_u16_le(msg_flags);
    cursor.write_u32_le(data_len);
    cursor.write_slice(body);
    Ok(cursor.into_vec())
}

fn read_long_name(body: &mut ReadCursor<'_>) -> RdpResult<String> {
    let mut units = Vec::new();
    loop {
        let unit = body.read_u16_le()?;
        if unit == 0 {
            break;
        }
        units.push(unit);
    }
    String::from_utf16(&units).map_err(|error| RdpProtocolError::encoding(format!("cliprdr invalid long format name: {}", error)))
}

fn read_short_unicode_name(body: &mut ReadCursor<'_>) -> RdpResult<String> {
    read_utf16_fixed(body, SHORT_FORMAT_NAME_LENGTH)
}

fn read_short_ascii_name(body: &mut ReadCursor<'_>) -> RdpResult<String> {
    let bytes = body.read_slice(SHORT_FORMAT_NAME_LENGTH)?;
    let end = bytes.iter().position(|byte| *byte == 0).unwrap_or(bytes.len());
    let name = std::str::from_utf8(&bytes[..end]).map_err(|error| RdpProtocolError::encoding(format!("cliprdr invalid ASCII format name: {}", error)))?;
    Ok(name.to_string())
}

fn read_utf16_fixed(body: &mut ReadCursor<'_>, byte_len: usize) -> RdpResult<String> {
    let bytes = body.read_slice(byte_len)?;
    if bytes.len() % 2 != 0 {
        return Err(RdpProtocolError::encoding(format!("cliprdr invalid utf16 field length: {}", bytes.len())));
    }
    let units = bytes.chunks_exact(2).map(|pair| u16::from_le_bytes([pair[0], pair[1]])).take_while(|unit| *unit != 0).collect::<Vec<u16>>();
    String::from_utf16(&units).map_err(|error| RdpProtocolError::encoding(format!("cliprdr invalid utf16 text: {}", error)))
}

fn encode_text(format: u32, text: &str) -> Option<Vec<u8>> {
    if text.is_empty() {
        return None;
    }
    match format {
        CF_UNICODETEXT => {
            let mut cursor = WriteCursor::with_capacity(text.len() * 2 + 2);
            cursor.write_utf16_le(text);
            cursor.write_u16_le(0);
            Some(cursor.into_vec())
        }
        CF_TEXT => {
            let mut data = text.as_bytes().to_vec();
            data.push(0);
            Some(data)
        }
        _ => None,
    }
}

fn decode_unicode_text(data: &[u8]) -> RdpResult<String> {
    if data.len() % 2 != 0 {
        return Err(RdpProtocolError::encoding(format!("cliprdr invalid unicode text length: {}", data.len())));
    }
    let units = data.chunks_exact(2).map(|pair| u16::from_le_bytes([pair[0], pair[1]])).take_while(|unit| *unit != 0).collect::<Vec<u16>>();
    String::from_utf16(&units).map_err(|error| RdpProtocolError::encoding(format!("cliprdr invalid unicode text: {}", error)))
}

fn decode_ansi_text(data: &[u8]) -> RdpResult<String> {
    let end = data.iter().position(|byte| *byte == 0).unwrap_or(data.len());
    let text = std::str::from_utf8(&data[..end]).map_err(|error| RdpProtocolError::encoding(format!("cliprdr invalid text: {}", error)))?;
    Ok(text.to_string())
}

fn encode_html_format(html: &str) -> Option<Vec<u8>> {
    if html.is_empty() {
        return None;
    }
    let mut header = String::from("Version:0.9\r\nStartHTML:0000000000\r\nEndHTML:0000000000\r\nStartFragment:0000000000\r\nEndFragment:0000000000\r\n");
    let start_html = header.len();
    if !html.starts_with("<html>") {
        header.push_str("<html>\r\n<body>\r\n<!--StartFragment-->");
    }
    let start_fragment = header.len();
    header.push_str(html);
    let end_fragment = header.len();
    if !html.ends_with("</html>") {
        header.push_str("<!--EndFragment-->\r\n</body>\r\n</html>");
    }
    let end_html = header.len();
    header.replace_range(23..33, &format!("{start_html:0>10}"));
    header.replace_range(43..53, &format!("{end_html:0>10}"));
    header.replace_range(69..79, &format!("{start_fragment:0>10}"));
    header.replace_range(93..103, &format!("{end_fragment:0>10}"));
    Some(header.into_bytes())
}

fn decode_html_format(data: &[u8]) -> RdpResult<String> {
    let text = if data.starts_with(&[0xFF, 0xFE]) {
        decode_unicode_text(&data[2..])?
    } else {
        std::str::from_utf8(data).map_err(|error| RdpProtocolError::encoding(format!("cliprdr HTML Format is not UTF-8: {}", error)))?.to_string()
    };
    let bytes = text.as_bytes();
    let start = html_header_offset(bytes, b"StartHTML")?;
    let end = html_header_offset(bytes, b"EndHTML")?;
    if start > end || end > bytes.len() {
        return Err(RdpProtocolError::protocol(format!("cliprdr invalid HTML offsets: {}..{} len={}", start, end, bytes.len())));
    }
    let html = std::str::from_utf8(&bytes[start..end]).map_err(|error| RdpProtocolError::encoding(format!("cliprdr invalid HTML fragment: {}", error)))?;
    Ok(html.to_string())
}

fn html_header_offset(data: &[u8], key: &[u8]) -> RdpResult<usize> {
    let mut pattern = Vec::with_capacity(key.len() + 1);
    pattern.extend_from_slice(key);
    pattern.push(b':');
    let start =
        data.windows(pattern.len()).position(|window| window == pattern.as_slice()).ok_or_else(|| RdpProtocolError::protocol(format!("cliprdr HTML is missing {}", String::from_utf8_lossy(key))))?;
    let value = &data[start + pattern.len()..];
    let end =
        value.iter().position(|byte| *byte == b'\r' || *byte == b'\n').ok_or_else(|| RdpProtocolError::protocol(format!("cliprdr HTML {} value is not terminated", String::from_utf8_lossy(key))))?;
    let digits = std::str::from_utf8(&value[..end]).map_err(|error| RdpProtocolError::encoding(format!("cliprdr HTML offset is not ASCII: {}", error)))?;
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(RdpProtocolError::protocol(format!("cliprdr invalid HTML offset: {}", digits)));
    }
    if digits.bytes().all(|byte| byte == b'0') {
        return Ok(0);
    }
    digits.trim_start_matches('0').parse::<usize>().map_err(|error| RdpProtocolError::encoding(format!("cliprdr HTML offset cannot be parsed: {}", error)))
}

fn encode_dib(image: &LocalImage) -> RdpResult<Vec<u8>> {
    let width = i32::try_from(image.width).map_err(|_| RdpProtocolError::encoding(format!("cliprdr image width cannot be encoded: {}", image.width)))?;
    let height = i32::try_from(image.height).map_err(|_| RdpProtocolError::encoding(format!("cliprdr image height cannot be encoded: {}", image.height)))?;
    let stride = image.width as usize * 4;
    let mut body = WriteCursor::with_capacity(BITMAPINFOHEADER_SIZE as usize + stride * image.height as usize);
    body.write_u32_le(BITMAPINFOHEADER_SIZE);
    body.write_u32_le(width as u32);
    body.write_u32_le(height as u32);
    body.write_u16_le(1);
    body.write_u16_le(32);
    body.write_u32_le(BI_RGB);
    body.write_u32_le((stride * image.height as usize) as u32);
    body.write_u32_le(0);
    body.write_u32_le(0);
    body.write_u32_le(0);
    body.write_u32_le(0);
    for row in (0..image.height as usize).rev() {
        let origin = row * stride;
        for column in 0..image.width as usize {
            let pixel = origin + column * 4;
            body.write_u8(image.rgba[pixel + 2]);
            body.write_u8(image.rgba[pixel + 1]);
            body.write_u8(image.rgba[pixel]);
            body.write_u8(image.rgba[pixel + 3]);
        }
    }
    Ok(body.into_vec())
}

fn dib_to_bmp(dib: &[u8]) -> RdpResult<Vec<u8>> {
    if dib.len() < DIB_HEADER_MIN {
        return Err(RdpProtocolError::underrun(format!("cliprdr DIB too short: {}", dib.len())));
    }
    let header_size = u32::from_le_bytes([dib[0], dib[1], dib[2], dib[3]]) as usize;
    if header_size < BITMAPINFOHEADER_SIZE as usize || header_size > dib.len() {
        return Err(RdpProtocolError::protocol(format!("cliprdr invalid DIB header length: {}", header_size)));
    }
    let bit_count = u16::from_le_bytes([dib[14], dib[15]]);
    let compression = u32::from_le_bytes([dib[16], dib[17], dib[18], dib[19]]);
    let clr_used = u32::from_le_bytes([dib[32], dib[33], dib[34], dib[35]]);
    let extra = dib_palette_bytes(header_size, bit_count, compression, clr_used)?;
    let off_bits = BITMAPFILEHEADER_LENGTH + header_size + extra;
    if off_bits > BITMAPFILEHEADER_LENGTH + dib.len() {
        return Err(RdpProtocolError::protocol(format!("cliprdr DIB pixel offset out of bounds: off={} len={}", off_bits, dib.len())));
    }
    let total = BITMAPFILEHEADER_LENGTH + dib.len();
    let mut bmp = Vec::with_capacity(total);
    bmp.extend_from_slice(&[b'B', b'M']);
    bmp.extend_from_slice(&(total as u32).to_le_bytes());
    bmp.extend_from_slice(&[0, 0, 0, 0]);
    bmp.extend_from_slice(&(off_bits as u32).to_le_bytes());
    bmp.extend_from_slice(dib);
    Ok(bmp)
}

fn dib_palette_bytes(header_size: usize, bit_count: u16, compression: u32, clr_used: u32) -> RdpResult<usize> {
    match compression {
        BI_RGB => {
            if bit_count <= 8 {
                let colors = if clr_used == 0 { 1usize << bit_count } else { clr_used as usize };
                Ok(colors * 4)
            } else {
                Ok(0)
            }
        }
        BI_BITFIELDS => {
            if header_size >= 52 {
                Ok(0)
            } else {
                Ok(12)
            }
        }
        other => Err(RdpProtocolError::unsupported(format!("cliprdr DIB compression not implemented: {}", other))),
    }
}
