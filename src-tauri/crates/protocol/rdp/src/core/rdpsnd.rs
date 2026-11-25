use rodio::{OutputStream, Sink};
use rodio::buffer::SamplesBuffer;
use std::time::{Duration, Instant};
use std::sync::mpsc::{self, Sender};
use std::thread::JoinHandle as OsJoinHandle;
use crate::core::stream::{ReadCursor, WriteCursor};
use crate::core::error::{RdpProtocolError, RdpResult};

const SNDC_CLOSE: u8 = 1;
const SNDC_WAVE: u8 = 2;
const SNDC_SETVOLUME: u8 = 3;
const SNDC_WAVECONFIRM: u8 = 5;
const SNDC_TRAINING: u8 = 6;
const SNDC_FORMATS: u8 = 7;
const SNDC_QUALITYMODE: u8 = 12;
const SNDC_WAVE2: u8 = 13;

const TSSNDCAPS_ALIVE: u32 = 1;
const TSSNDCAPS_VOLUME: u32 = 2;
const HIGH_QUALITY: u16 = 0x0002;
const CHANNEL_VERSION_WIN_7: u16 = 6;
const CHANNEL_VERSION_WIN_MAX: u16 = 8;
const CLIENT_VOLUME: u32 = 0xFFFF_FFFF;

const WAVE_FORMAT_PCM: u16 = 0x0001;
const BEEP_SAMPLE_RATE: u32 = 48000;
const BEEP_MAX_DURATION_MS: u32 = 10_000;
const BEEP_AMPLITUDE: f64 = 0.25;
const OVERRUN_LATENCY_MS: u64 = 0;
const PLAYBACK_READY_TIMEOUT: Duration = Duration::from_millis(800);

struct AudioFormat {
    tag: u16,
    channels: u16,
    samples_per_sec: u32,
    avg_bytes_per_sec: u32,
    block_align: u16,
    bits_per_sample: u16,
    extra: Vec<u8>,
}

struct PcmFrame {
    channels: u16,
    sample_rate: u32,
    samples: Vec<i16>,
}

pub struct RdpsndState {
    formats: Vec<AudioFormat>,
    version: u16,
    block_no: u8,
    timestamp: u16,
    format_no: u16,
    expecting_wave: bool,
    wave_prefix: [u8; 4],
    wave_size: u16,
    playback: Option<Sender<PcmFrame>>,
    arrival: Option<Instant>,
    start_play: Option<Instant>,
    total_play_size: u64,
}

impl Default for RdpsndState {
    fn default() -> Self {
        Self::empty()
    }
}

impl RdpsndState {
    pub fn start() -> (Self, Option<OsJoinHandle<()>>, Option<String>) {
        let (playback, handle, error) = open_playback();
        (
            Self {
                playback,
                version: 0,
                block_no: 0,
                timestamp: 0,
                format_no: 0,
                wave_size: 0,
                arrival: None,
                start_play: None,
                total_play_size: 0,
                formats: Vec::new(),
                wave_prefix: [0; 4],
                expecting_wave: false,
            },
            handle,
            error,
        )
    }

    fn empty() -> Self {
        Self {
            version: 0,
            block_no: 0,
            timestamp: 0,
            format_no: 0,
            wave_size: 0,
            arrival: None,
            playback: None,
            start_play: None,
            total_play_size: 0,
            formats: Vec::new(),
            wave_prefix: [0; 4],
            expecting_wave: false,
        }
    }
}

#[derive(Default)]
pub struct RdpsndOutcome {
    pub responses: Vec<Vec<u8>>,
}

pub fn process(state: &mut RdpsndState, message: &[u8]) -> RdpResult<RdpsndOutcome> {
    if state.expecting_wave {
        return process_wave_body(state, message);
    }
    let mut cursor = ReadCursor::new(message);
    let msg_type = cursor.read_u8()?;
    cursor.skip(1)?;
    let body_size = cursor.read_u16_le()? as usize;
    if msg_type == SNDC_WAVE {
        return process_wave_info(state, &mut cursor, body_size);
    }
    let body = cursor.read_slice(body_size)?;
    let mut body = ReadCursor::new(body);
    match msg_type {
        SNDC_FORMATS => process_formats(state, &mut body),
        SNDC_TRAINING => process_training(&mut body),
        SNDC_WAVE2 => process_wave2(state, &mut body, body_size),
        SNDC_SETVOLUME => {
            body.skip(4)?;
            Ok(RdpsndOutcome::default())
        }
        SNDC_QUALITYMODE => Ok(RdpsndOutcome::default()),
        SNDC_CLOSE => Ok(RdpsndOutcome::default()),
        _other => Ok(RdpsndOutcome::default()),
    }
}

pub fn play_beep(state: &mut RdpsndState, frequency: u32, duration: u32) -> RdpResult<()> {
    if duration == 0 {
        return Ok(());
    }
    let samples = beep_pcm(frequency, duration)?;
    let Some(playback) = state.playback.as_ref() else {
        return Ok(());
    };
    let _ = playback.send(PcmFrame { channels: 1, sample_rate: BEEP_SAMPLE_RATE, samples });
    Ok(())
}

pub fn beep_pcm(frequency: u32, duration: u32) -> RdpResult<Vec<i16>> {
    if frequency == 0 || frequency >= BEEP_SAMPLE_RATE / 2 {
        return Err(RdpProtocolError::protocol(format!("PLAY_SOUND invalid frequency: {} Hz", frequency)));
    }
    if duration > BEEP_MAX_DURATION_MS {
        return Err(RdpProtocolError::protocol(format!("PLAY_SOUND invalid duration: {} ms", duration)));
    }
    let count = usize::try_from(u64::from(duration) * u64::from(BEEP_SAMPLE_RATE) / 1000).map_err(|_| RdpProtocolError::protocol(format!("PLAY_SOUND sample count overflow: {} ms", duration)))?;
    let omega = 2.0 * std::f64::consts::PI * f64::from(frequency) / f64::from(BEEP_SAMPLE_RATE);
    let peak = BEEP_AMPLITUDE * f64::from(i16::MAX);
    Ok((0..count).map(|index| (omega * index as f64).sin() * peak).map(|sample| sample as i16).collect())
}

fn process_formats(state: &mut RdpsndState, body: &mut ReadCursor<'_>) -> RdpResult<RdpsndOutcome> {
    body.skip(14)?;
    let count = body.read_u16_le()?;
    state.block_no = body.read_u8()?;
    state.version = body.read_u16_le()?;
    body.skip(1)?;
    let mut selected = Vec::new();
    for _ in 0..count {
        let format = read_format(body)?;
        if format.tag == WAVE_FORMAT_PCM {
            selected.push(format);
        }
    }
    state.formats = selected;
    let mut responses = vec![build_client_formats(&state.formats)?];
    if state.version >= CHANNEL_VERSION_WIN_7 {
        responses.push(build_quality_mode());
    }
    Ok(RdpsndOutcome { responses })
}

fn process_training(body: &mut ReadCursor<'_>) -> RdpResult<RdpsndOutcome> {
    let timestamp = body.read_u16_le()?;
    let pack_size = body.read_u16_le()?;
    Ok(RdpsndOutcome { responses: vec![build_training(timestamp, pack_size)] })
}

fn process_wave_info(state: &mut RdpsndState, body: &mut ReadCursor<'_>, body_size: usize) -> RdpResult<RdpsndOutcome> {
    if body_size < 8 {
        return Err(RdpProtocolError::protocol(format!("rdpsnd Wave too short: {}", body_size)));
    }
    state.arrival = Some(Instant::now());
    state.timestamp = body.read_u16_le()?;
    state.format_no = body.read_u16_le()?;
    state.block_no = body.read_u8()?;
    body.skip(3)?;
    let prefix = body.read_slice(4)?;
    state.wave_prefix.copy_from_slice(prefix);
    state.wave_size = u16::try_from(body_size - 8).map_err(|_| RdpProtocolError::protocol(format!("rdpsnd Wave data too long: {}", body_size)))?;
    ensure_format(state)?;
    state.expecting_wave = true;
    Ok(RdpsndOutcome::default())
}

fn process_wave_body(state: &mut RdpsndState, message: &[u8]) -> RdpResult<RdpsndOutcome> {
    state.expecting_wave = false;
    if message.len() < 4 {
        return Err(RdpProtocolError::protocol(format!("rdpsnd Wave body too short: {}", message.len())));
    }
    let mut data = message.to_vec();
    data[..4].copy_from_slice(&state.wave_prefix);
    if data.len() < state.wave_size as usize {
        return Err(RdpProtocolError::protocol(format!("rdpsnd Wave body underrun: need {} have {}", state.wave_size, data.len())));
    }
    treat_wave(state, &data[..state.wave_size as usize])
}

fn process_wave2(state: &mut RdpsndState, body: &mut ReadCursor<'_>, body_size: usize) -> RdpResult<RdpsndOutcome> {
    if body_size < 12 {
        return Err(RdpProtocolError::protocol(format!("rdpsnd Wave2 too short: {}", body_size)));
    }
    state.arrival = Some(Instant::now());
    state.timestamp = body.read_u16_le()?;
    state.format_no = body.read_u16_le()?;
    state.block_no = body.read_u8()?;
    body.skip(3)?;
    body.skip(4)?;
    ensure_format(state)?;
    treat_wave(state, body.rest())
}

fn treat_wave(state: &mut RdpsndState, data: &[u8]) -> RdpResult<RdpsndOutcome> {
    let first = build_wave_confirm(state.timestamp, state.block_no);
    let dropped = detect_overrun(state, data.len());
    if !dropped {
        play_wave(state, data)?;
    }
    let Some(arrival) = state.arrival else {
        return Err(RdpProtocolError::protocol("rdpsnd WAVE is missing arrival time".to_string()));
    };
    let diff_ms = arrival.elapsed().as_millis() as u64;
    let timestamp = ((u64::from(state.timestamp) + diff_ms) % 65536) as u16;
    let second = build_wave_confirm(timestamp, state.block_no);
    Ok(RdpsndOutcome { responses: vec![first, second] })
}

fn ensure_format(state: &RdpsndState) -> RdpResult<()> {
    if (state.format_no as usize) >= state.formats.len() {
        return Err(RdpProtocolError::protocol(format!("rdpsnd format number out of range: {}", state.format_no)));
    }
    Ok(())
}

fn detect_overrun(state: &mut RdpsndState, size: usize) -> bool {
    let format = &state.formats[state.format_no as usize];
    let bpf = u64::from(format.channels) * u64::from(format.bits_per_sample) * u64::from(format.samples_per_sec) / 8;
    if bpf == 0 {
        return false;
    }
    let duration = 1000 * size as u64 / bpf;
    let total_duration = 1000 * state.total_play_size / bpf;
    let now = Instant::now();
    let Some(start) = state.start_play else {
        state.start_play = Some(now);
        state.total_play_size = size as u64;
        return false;
    };
    let elapsed = u64::try_from(now.duration_since(start).as_millis()).expect("rdpsnd playback clock exceeds u64 milliseconds");
    if elapsed > total_duration + 10 {
        state.start_play = Some(now);
        state.total_play_size = size as u64;
        return false;
    }
    let remaining = if elapsed > total_duration { 0 } else { total_duration - elapsed };
    let max_duration = duration * 2 + OVERRUN_LATENCY_MS;
    let drop = remaining + duration > max_duration;
    if drop {
        return true;
    }
    state.total_play_size += size as u64;
    false
}

fn play_wave(state: &mut RdpsndState, data: &[u8]) -> RdpResult<()> {
    let format = &state.formats[state.format_no as usize];
    if format.tag != WAVE_FORMAT_PCM {
        return Err(RdpProtocolError::unsupported(format!("rdpsnd format 0x{:04x} not implemented", format.tag)));
    }
    let samples = decode_pcm(format, data)?;
    let Some(playback) = state.playback.as_ref() else {
        return Ok(());
    };
    let _ = playback.send(PcmFrame { channels: format.channels, sample_rate: format.samples_per_sec, samples });
    Ok(())
}

fn decode_pcm(format: &AudioFormat, data: &[u8]) -> RdpResult<Vec<i16>> {
    match format.bits_per_sample {
        16 => {
            if data.len() % 2 != 0 {
                return Err(RdpProtocolError::protocol(format!("rdpsnd PCM16 length is odd: {}", data.len())));
            }
            Ok(data.chunks_exact(2).map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]])).collect())
        }
        8 => Ok(data.iter().map(|sample| ((*sample as i16) - 128) << 8).collect()),
        other => Err(RdpProtocolError::unsupported(format!("rdpsnd PCM bit depth not implemented: {}", other))),
    }
}

fn read_format(body: &mut ReadCursor<'_>) -> RdpResult<AudioFormat> {
    let tag = body.read_u16_le()?;
    let channels = body.read_u16_le()?;
    let samples_per_sec = body.read_u32_le()?;
    let avg_bytes_per_sec = body.read_u32_le()?;
    let block_align = body.read_u16_le()?;
    let bits_per_sample = body.read_u16_le()?;
    let extra_len = body.read_u16_le()? as usize;
    let extra = body.read_vec(extra_len)?;
    Ok(AudioFormat { tag, channels, samples_per_sec, avg_bytes_per_sec, block_align, bits_per_sample, extra })
}

fn build_client_formats(formats: &[AudioFormat]) -> RdpResult<Vec<u8>> {
    let extra: usize = formats.iter().map(|format| 18 + format.extra.len()).sum();
    let body = 20 + extra;
    let mut cursor = WriteCursor::with_capacity(4 + body);
    cursor.write_u8(SNDC_FORMATS);
    cursor.write_u8(0);
    cursor.write_u16_le(u16::try_from(body).map_err(|_| RdpProtocolError::encoding(format!("rdpsnd format list too long: {}", body)))?);
    cursor.write_u32_le(TSSNDCAPS_ALIVE | TSSNDCAPS_VOLUME);
    cursor.write_u32_le(CLIENT_VOLUME);
    cursor.write_u32_le(0);
    cursor.write_u16_le(0);
    cursor.write_u16_le(u16::try_from(formats.len()).map_err(|_| RdpProtocolError::encoding(format!("rdpsnd too many formats: {}", formats.len())))?);
    cursor.write_u8(0);
    cursor.write_u16_le(CHANNEL_VERSION_WIN_MAX);
    cursor.write_u8(0);
    for format in formats {
        cursor.write_u16_le(format.tag);
        cursor.write_u16_le(format.channels);
        cursor.write_u32_le(format.samples_per_sec);
        cursor.write_u32_le(format.avg_bytes_per_sec);
        cursor.write_u16_le(format.block_align);
        cursor.write_u16_le(format.bits_per_sample);
        cursor.write_u16_le(u16::try_from(format.extra.len()).map_err(|_| RdpProtocolError::encoding(format!("rdpsnd extra too long: {}", format.extra.len())))?);
        cursor.write_slice(&format.extra);
    }
    Ok(cursor.into_vec())
}

fn build_quality_mode() -> Vec<u8> {
    let mut cursor = WriteCursor::with_capacity(8);
    cursor.write_u8(SNDC_QUALITYMODE);
    cursor.write_u8(0);
    cursor.write_u16_le(4);
    cursor.write_u16_le(HIGH_QUALITY);
    cursor.write_u16_le(0);
    cursor.into_vec()
}

fn build_training(timestamp: u16, pack_size: u16) -> Vec<u8> {
    let mut cursor = WriteCursor::with_capacity(8);
    cursor.write_u8(SNDC_TRAINING);
    cursor.write_u8(0);
    cursor.write_u16_le(4);
    cursor.write_u16_le(timestamp);
    cursor.write_u16_le(pack_size);
    cursor.into_vec()
}

fn build_wave_confirm(timestamp: u16, block_no: u8) -> Vec<u8> {
    let mut cursor = WriteCursor::with_capacity(8);
    cursor.write_u8(SNDC_WAVECONFIRM);
    cursor.write_u8(0);
    cursor.write_u16_le(4);
    cursor.write_u16_le(timestamp);
    cursor.write_u8(block_no);
    cursor.write_u8(0);
    cursor.into_vec()
}

fn open_playback() -> (Option<Sender<PcmFrame>>, Option<OsJoinHandle<()>>, Option<String>) {
    #[cfg(target_os = "ios")]
    if let Err(error) = ios_audio::activate_session() {
        return (None, None, Some(error));
    }
    let (tx, rx) = mpsc::channel::<PcmFrame>();
    let (ready_tx, ready_rx) = mpsc::sync_channel::<Option<String>>(1);
    let handle = std::thread::Builder::new()
        .name("rdpsnd".to_string())
        .spawn(move || {
            let stream = match OutputStream::try_default() {
                Ok(stream) => stream,
                Err(error) => {
                    let _ = ready_tx.send(Some(format!("open audio output failed: {}", error)));
                    return;
                }
            };
            let sink = match Sink::try_new(&stream.1) {
                Ok(sink) => sink,
                Err(error) => {
                    let _ = ready_tx.send(Some(format!("create audio sink failed: {}", error)));
                    return;
                }
            };
            let _ = ready_tx.send(None);
            while let Ok(frame) = rx.recv() {
                sink.append(SamplesBuffer::new(frame.channels, frame.sample_rate, frame.samples));
            }
        })
        .ok();
    let Some(handle) = handle else {
        return (None, None, Some("spawn rdpsnd playback thread failed".to_string()));
    };
    match ready_rx.recv_timeout(PLAYBACK_READY_TIMEOUT) {
        Ok(None) => (Some(tx), Some(handle), None),
        Ok(Some(error)) => (None, Some(handle), Some(error)),
        Err(_) => (Some(tx), Some(handle), None),
    }
}

// iOS 上音频输出必须先激活 AVAudioSession, 否则 rodio 拿不到默认输出设备, 远端声音全丢
#[cfg(target_os = "ios")]
mod ios_audio {
    use std::ffi::c_char;

    type ObjcId = *mut core::ffi::c_void;
    type ObjcSel = *const core::ffi::c_void;

    #[link(name = "objc")]
    extern "C" {
        fn objc_getClass(name: *const c_char) -> ObjcId;
        fn sel_registerName(name: *const c_char) -> ObjcSel;
        fn objc_msgSend();
    }

    unsafe fn send_id(target: ObjcId, selector: ObjcSel) -> ObjcId {
        std::mem::transmute::<unsafe extern "C" fn(), unsafe extern "C" fn(ObjcId, ObjcSel) -> ObjcId>(objc_msgSend)(target, selector)
    }

    unsafe fn send_string(target: ObjcId, selector: ObjcSel, text: *const c_char) -> ObjcId {
        std::mem::transmute::<unsafe extern "C" fn(), unsafe extern "C" fn(ObjcId, ObjcSel, *const c_char) -> ObjcId>(objc_msgSend)(target, selector, text)
    }

    unsafe fn send_object_error(target: ObjcId, selector: ObjcSel, object: ObjcId) -> bool {
        std::mem::transmute::<unsafe extern "C" fn(), unsafe extern "C" fn(ObjcId, ObjcSel, ObjcId, *mut ObjcId) -> bool>(objc_msgSend)(target, selector, object, std::ptr::null_mut())
    }

    unsafe fn send_bool_error(target: ObjcId, selector: ObjcSel, value: bool) -> bool {
        std::mem::transmute::<unsafe extern "C" fn(), unsafe extern "C" fn(ObjcId, ObjcSel, bool, *mut ObjcId) -> bool>(objc_msgSend)(target, selector, value, std::ptr::null_mut())
    }

    pub fn activate_session() -> Result<(), String> {
        unsafe {
            let session_class = objc_getClass(b"AVAudioSession\0".as_ptr() as *const c_char);
            if session_class.is_null() {
                return Err("AVAudioSession class is unavailable".to_string());
            }
            let session = send_id(session_class, sel_registerName(b"sharedInstance\0".as_ptr() as *const c_char));
            if session.is_null() {
                return Err("AVAudioSession sharedInstance is null".to_string());
            }
            let string_class = objc_getClass(b"NSString\0".as_ptr() as *const c_char);
            if string_class.is_null() {
                return Err("NSString class is unavailable".to_string());
            }
            let category = send_string(string_class, sel_registerName(b"stringWithUTF8String:\0".as_ptr() as *const c_char), b"AVAudioSessionCategoryPlayback\0".as_ptr() as *const c_char);
            if !send_object_error(session, sel_registerName(b"setCategory:error:\0".as_ptr() as *const c_char), category) {
                return Err("AVAudioSession setCategory:playback failed".to_string());
            }
            if !send_bool_error(session, sel_registerName(b"setActive:error:\0".as_ptr() as *const c_char), true) {
                return Err("AVAudioSession setActive failed".to_string());
            }
            Ok(())
        }
    }
}
