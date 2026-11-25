use std::sync::Arc;
use std::collections::VecDeque;
use std::sync::mpsc as std_mpsc;
use summer::tracing::{info, warn};
use parking_lot::Mutex as SyncMutex;
use crate::module::rdp::protocol::core::mcs;
use crate::module::rdp::protocol::core::pdu;
use crate::module::rdp::protocol::core::disp;
use crate::module::rdp::client_rdp::ClientRdp;
use crate::module::rdp::protocol::core::logon;
use crate::module::rdp::protocol::core::rdpdr;
use crate::module::rdp::protocol::core::rdpgfx;
use crate::module::rdp::protocol::core::rdpsnd;
use crate::module::rdp::protocol::core::update;
use crate::module::rdp::protocol::gdi::painter;
use crate::module::rdp::protocol::core::channel;
use crate::module::rdp::protocol::core::cliprdr;
use crate::module::rdp::protocol::core::drdynvc;
use crate::module::rdp::protocol::core::errinfo;
use crate::module::rdp::protocol::core::surface;
use crate::module::rdp::client_rdp_drive as drive;
use crate::module::rdp::protocol::core::activation;
use crate::module::rdp::protocol::core::msgchannel;
use crate::module::rdp::protocol::core::redirection;
use tokio::io::{AsyncWriteExt, ReadHalf, WriteHalf};
use crate::module::common::util::task_hub::TaskGroup;
use crate::module::rdp::client_rdp_drive::DriveState;
use crate::module::rdp::protocol::core::capabilities;
use tokio::sync::{mpsc, oneshot, Mutex as AsyncMutex};
use crate::module::rdp::protocol::gdi::gfx::GfxSurfaces;
use crate::service::service_clipboard::ClipboardService;
use crate::module::rdp::client_rdp_clipboard as clipboard;
use crate::module::rdp::protocol::core::rdpdr::RdpdrState;
use crate::module::rdp::protocol::codec::bulk::BulkContext;
use crate::module::rdp::protocol::core::stream::ReadCursor;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use crate::module::rdp::protocol::codec::avc::OutputEncoder;
use crate::module::rdp::protocol::core::rdpsnd::RdpsndState;
use crate::module::rdp::protocol::core::update::UpdateState;
use crate::module::rdp::protocol::core::update::DecodedUpdate;
use crate::module::rdp::protocol::core::surface::SurfaceCommand;
use crate::module::rdp::protocol::core::redirection::Redirection;
use crate::module::rdp::protocol::codec::zgfx::{self, ZgfxContext};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering};
use crate::module::rdp::protocol::core::msgchannel::MessageChannelState;
use crate::module::rdp::protocol::core::capabilities::ClientCapabilities;
use crate::module::rdp::protocol::core::error::{RdpProtocolError, RdpResult};
use crate::module::rdp::protocol::gdi::framebuffer::{self, Framebuffer, Rect};
use crate::module::rdp::protocol::core::transport::{self, RdpFrame, RdpStream};
use crate::module::rdp::client_rdp_session::{RdpSettings, RdpState, CLIENT_NAME};
use crate::model::resp::event::resp_event_rdp::{RespEventRdp, RespRdpDesktopSize};
use crate::module::rdp::protocol::core::fastpath::{FastPathAssembler, FastPathUpdate};

pub(crate) struct FrameSyncState {
    pending: AtomicU64,
    last_bars: AtomicU8,
    last_received: AtomicU64,
    last_frame_duration: AtomicU64,
}

impl FrameSyncState {
    pub(crate) fn new() -> Arc<Self> {
        Arc::new(Self { pending: AtomicU64::new(0), last_frame_duration: AtomicU64::new(0), last_received: AtomicU64::new(0), last_bars: AtomicU8::new(0) })
    }

    pub(crate) fn ack(&self, timestamp: u64) -> Option<u8> {
        let now = unix_ms();
        let last_received = self.last_received.load(Ordering::SeqCst);
        if timestamp >= last_received {
            self.last_received.store(timestamp, Ordering::SeqCst);
            let frame_duration = now.saturating_sub(timestamp);
            let last_frame_duration = self.last_frame_duration.load(Ordering::SeqCst);
            let estimated_rtt = framebuffer::smooth_frame_duration(last_frame_duration, frame_duration);
            self.last_frame_duration.store(estimated_rtt, Ordering::SeqCst);
        }
        let pending = self.pending.load(Ordering::SeqCst);
        if pending != 0 && timestamp >= pending {
            self.pending.store(0, Ordering::SeqCst);
        }
        let bars = framebuffer::network_bars(self.last_frame_duration.load(Ordering::SeqCst));
        let prev = self.last_bars.swap(bars, Ordering::SeqCst);
        if prev == bars {
            None
        } else {
            Some(bars)
        }
    }

    pub(crate) fn reset_network(&self) {
        self.last_bars.store(0, Ordering::SeqCst);
    }

    fn mark_sent(&self, timestamp: u64) {
        self.pending.store(timestamp, Ordering::SeqCst);
    }

    fn rtt(&self) -> u64 {
        self.last_frame_duration.load(Ordering::SeqCst)
    }
}

pub(crate) struct FocusState {
    generation: AtomicU64,
    rect: SyncMutex<Option<Rect>>,
}

impl FocusState {
    pub(crate) fn new() -> Arc<Self> {
        Arc::new(Self { generation: AtomicU64::new(0), rect: SyncMutex::new(None) })
    }

    pub(crate) fn set(&self, rect: Option<Rect>) {
        *self.rect.lock() = rect;
        self.generation.fetch_add(1, Ordering::SeqCst);
    }

    fn snapshot(&self) -> (u64, Option<Rect>) {
        let rect = *self.rect.lock();
        (self.generation.load(Ordering::SeqCst), rect)
    }
}

fn unix_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).expect("system time is before UNIX_EPOCH").as_millis() as u64
}

const H264_ALIGN: u16 = 16;
const H264_MIN_DIRTY_AREA: u32 = 131_072;
const FRAME_TICK: Duration = Duration::from_millis(16);
const ENCODE_IDLE: Duration = Duration::from_millis(33);

struct EncodeJob {
    desktop: Rect,
    quality: u8,
    bitrate: u32,
    patches: Vec<(Rect, Vec<u8>)>,
    nv12: Option<(Vec<u8>, Vec<u8>, u16, u16)>,
    focus: Option<(Rect, Vec<u8>)>,
}

struct PendingOutput {
    dest: Rect,
    focus: Option<Vec<u8>>,
}

fn send_frame(event_tx: &mpsc::Sender<RespEventRdp>, frame_sync: &FrameSyncState, body: Vec<u8>, _decode_ms: u64, _encode_ms: u64) -> bool {
    if body.is_empty() {
        return true;
    }
    let timestamp = unix_ms();
    let mut payload = Vec::with_capacity(8 + body.len());
    payload.extend_from_slice(&timestamp.to_le_bytes());
    payload.extend_from_slice(&body);
    match event_tx.try_send(RespEventRdp::OnFrame(payload.into())) {
        Ok(()) => {
            frame_sync.mark_sent(timestamp);
            true
        }
        Err(mpsc::error::TrySendError::Full(_)) => false,
        Err(mpsc::error::TrySendError::Closed(_)) => false,
    }
}

fn emit_event(event_tx: &mpsc::Sender<RespEventRdp>, event: RespEventRdp) -> RdpResult<()> {
    match event_tx.try_send(event) {
        Ok(()) => Ok(()),
        Err(mpsc::error::TrySendError::Full(_)) => Ok(()),
        Err(mpsc::error::TrySendError::Closed(_)) => Err(RdpProtocolError::protocol("rdp event channel closed".to_string())),
    }
}

fn emit_required(event_tx: &mpsc::Sender<RespEventRdp>, event: RespEventRdp) -> RdpResult<()> {
    event_tx.blocking_send(event).map_err(|_| RdpProtocolError::protocol("rdp event channel closed".to_string()))
}

struct EncodeWorker {
    desktop: Rect,
    queued: Arc<AtomicBool>,
    encode_started: Instant,
    pending_dest: VecDeque<PendingOutput>,
    encoder: Option<OutputEncoder>,
    last_decode_ms: Arc<AtomicU64>,
    frame_sync: Arc<FrameSyncState>,
    event_tx: mpsc::Sender<RespEventRdp>,
}

impl EncodeWorker {
    fn run(mut self, encode_rx: std_mpsc::Receiver<EncodeJob>) {
        loop {
            match encode_rx.recv_timeout(ENCODE_IDLE) {
                Ok(job) => {
                    self.queued.store(false, Ordering::Release);
                    self.handle(job);
                    self.drain(Duration::ZERO);
                }
                Err(std_mpsc::RecvTimeoutError::Timeout) => self.drain(Duration::ZERO),
                Err(std_mpsc::RecvTimeoutError::Disconnected) => return,
            }
        }
    }

    fn handle(&mut self, job: EncodeJob) {
        self.desktop = job.desktop;
        let focus = self.encode_focus(job.focus);
        if let Some((y, uv, width, height)) = job.nv12 {
            self.submit_nv12(width, height, y, uv, job.bitrate, focus);
            return;
        }
        self.send_patches(&job.patches, job.quality);
        self.send_focus(focus);
    }

    fn encode_focus(&self, focus: Option<(Rect, Vec<u8>)>) -> Option<Vec<u8>> {
        let (rect, rgba) = focus?;
        match Framebuffer::encode_lossless_patch(rect, &rgba) {
            Ok(body) => Some(body),
            Err(error) => {
                warn!("[rdp] encode-failed path=focus err={}", error.message);
                None
            }
        }
    }

    fn send_focus(&mut self, focus: Option<Vec<u8>>) {
        let Some(body) = focus else {
            return;
        };
        send_frame(&self.event_tx, &self.frame_sync, body, self.last_decode_ms.load(Ordering::Relaxed), 0);
    }

    fn submit_nv12(&mut self, width: u16, height: u16, y: Vec<u8>, uv: Vec<u8>, bitrate: u32, focus: Option<Vec<u8>>) {
        if let Err(error) = self.prepare_encoder(width, height, bitrate) {
            self.fail("encoder-create", error);
            return;
        }
        let Some(encoder) = self.encoder.as_mut() else {
            return;
        };
        self.encode_started = Instant::now();
        if let Err(error) = encoder.submit_nv12(&y, &uv, unix_ms()) {
            self.fail("submit-nv12", error);
            return;
        }
        self.pending_dest.push_back(PendingOutput { dest: self.desktop, focus });
    }

    fn prepare_encoder(&mut self, width: u16, height: u16, bitrate: u32) -> RdpResult<()> {
        if let Some(encoder) = self.encoder.as_mut() {
            if encoder.matches(width, height) {
                encoder.set_bitrate(bitrate)?;
                return Ok(());
            }
        }
        self.drain(ENCODE_IDLE);
        self.encoder = None;
        self.pending_dest.clear();
        let encoder = OutputEncoder::with_bitrate(width, height, bitrate)?;
        info!("[rdp] encoder backend={} size={}x{} bitrate={}", encoder.backend_name(), width, height, bitrate);
        self.encoder = Some(encoder);
        Ok(())
    }

    fn drain(&mut self, wait: Duration) {
        loop {
            let Some(encoder) = self.encoder.as_mut() else {
                return;
            };
            if encoder.pending() == 0 {
                return;
            }
            match encoder.take_output(wait) {
                Ok(Some(bytes)) => {
                    let Some(pending) = self.pending_dest.pop_front() else {
                        self.fail("pack", RdpProtocolError::protocol("h264 output missing dest".to_string()));
                        return;
                    };
                    let encode_ms = self.encode_started.elapsed().as_millis() as u64;
                    match Framebuffer::pack_patch(framebuffer::FRAME_KIND_H264, pending.dest, &bytes) {
                        Ok(body) => {
                            if !send_frame(&self.event_tx, &self.frame_sync, body, self.last_decode_ms.load(Ordering::Relaxed), encode_ms) {
                                if let Some(encoder) = self.encoder.as_mut() {
                                    encoder.mark_idr();
                                }
                            }
                            self.send_focus(pending.focus);
                        }
                        Err(error) => {
                            self.fail("pack", error);
                            return;
                        }
                    }
                }
                Ok(None) => return,
                Err(error) => {
                    self.fail("output", error);
                    return;
                }
            }
        }
    }

    fn send_patches(&mut self, patches: &[(Rect, Vec<u8>)], quality: u8) {
        let started = Instant::now();
        match Framebuffer::encode_rgba_patches(patches, quality) {
            Ok(body) => {
                let encode_ms = started.elapsed().as_millis() as u64;
                if !send_frame(&self.event_tx, &self.frame_sync, body, self.last_decode_ms.load(Ordering::Relaxed), encode_ms) {
                    if let Some(encoder) = self.encoder.as_mut() {
                        encoder.mark_idr();
                    }
                }
            }
            Err(error) => {
                warn!("[rdp] encode-failed path=rgba-patch err={}", error.message);
                let _ = self.event_tx.try_send(RespEventRdp::OnError(error.message));
            }
        }
    }

    fn fail(&mut self, stage: &str, error: RdpProtocolError) {
        self.encoder = None;
        self.pending_dest.clear();
        warn!("[rdp] encode-failed stage={} err={}", stage, error.message);
        let _ = self.event_tx.try_send(RespEventRdp::OnError(error.message));
    }
}

enum DecodeJob {
    Gfx { payload: Vec<u8> },
    Pointer { body: Vec<u8> },
    Resize { width: u16, height: u16 },
    Persist(oneshot::Sender<[Vec<u64>; 5]>),
    Slowpath { update_type: u16, body: Vec<u8> },
    FastPath { updates: Vec<FastPathUpdate>, surface: Rect },
}

enum DecodeEvent {
    GfxAcks(Vec<Vec<u8>>),
    SurfaceAck(u32),
    Desktop(RespRdpDesktopSize),
    Error(RdpProtocolError),
}

struct DecodeWorker {
    color_depth: u16,
    focus_seen: u64,
    zgfx: ZgfxContext,
    focus_pending: bool,
    avc_passthrough: bool,
    last_gfx_codec: u16,
    surfaces: GfxSurfaces,
    focus_raw: Option<Rect>,
    focus: Arc<FocusState>,
    framebuffer: Framebuffer,
    update_state: UpdateState,
    last_decode_ms: Arc<AtomicU64>,
    encode_queued: Arc<AtomicBool>,
    frame_sync: Arc<FrameSyncState>,
    event_tx: mpsc::Sender<RespEventRdp>,
    encode_tx: std_mpsc::SyncSender<EncodeJob>,
    event_out: mpsc::UnboundedSender<DecodeEvent>,
}

impl DecodeWorker {
    fn run(mut self, jobs: std_mpsc::Receiver<DecodeJob>) {
        let mut last_flush = Instant::now();
        loop {
            let wait = FRAME_TICK.saturating_sub(last_flush.elapsed());
            match jobs.recv_timeout(wait) {
                Ok(job) => {
                    if let Err(error) = self.handle(job) {
                        let _ = self.event_out.send(DecodeEvent::Error(error));
                        return;
                    }
                    if last_flush.elapsed() >= FRAME_TICK && !self.flush() {
                        return;
                    }
                    if last_flush.elapsed() >= FRAME_TICK {
                        last_flush = Instant::now();
                    }
                }
                Err(std_mpsc::RecvTimeoutError::Timeout) => {
                    if !self.flush() {
                        return;
                    }
                    last_flush = Instant::now();
                }
                Err(std_mpsc::RecvTimeoutError::Disconnected) => return,
            }
        }
    }

    fn handle(&mut self, job: DecodeJob) -> RdpResult<()> {
        let started = Instant::now();
        match job {
            DecodeJob::FastPath { updates, surface } => {
                for update in updates {
                    let Some(decoded) = update::decode_fastpath(&update, &mut self.update_state, surface)? else {
                        continue;
                    };
                    self.apply(decoded)?;
                }
            }
            DecodeJob::Slowpath { update_type, body } => {
                let mut cursor = ReadCursor::new(&body);
                if let Some(decoded) = update::decode_slowpath(update_type, &mut cursor, &mut self.update_state)? {
                    self.apply(decoded)?;
                }
            }
            DecodeJob::Pointer { body } => {
                let mut cursor = ReadCursor::new(&body);
                if let Some(decoded) = update::decode_pointer(&mut cursor, &mut self.update_state)? {
                    self.apply(decoded)?;
                }
            }
            DecodeJob::Gfx { payload } => {
                let payload = zgfx::decompress(&mut self.zgfx, &payload)?;
                let outcome = rdpgfx::process(&mut self.surfaces, &mut self.framebuffer, &payload)?;
                if self.surfaces.last_codec_id != 0 && self.surfaces.last_codec_id != self.last_gfx_codec {
                    self.last_gfx_codec = self.surfaces.last_codec_id;
                    info!("[rdp] gfx-codec=0x{:04x}", self.last_gfx_codec);
                }
                if !outcome.responses.is_empty() && self.event_out.send(DecodeEvent::GfxAcks(outcome.responses)).is_err() {
                    return Err(RdpProtocolError::protocol("rdp decode event channel closed".to_string()));
                }
                if let Some(desktop) = outcome.desktop {
                    let desktop = RespRdpDesktopSize { width: desktop.width, height: desktop.height };
                    if self.framebuffer.resize(desktop.width, desktop.height) {
                        emit_required(&self.event_tx, RespEventRdp::OnDesktopResize(desktop.clone()))?;
                    }
                    if self.event_out.send(DecodeEvent::Desktop(desktop)).is_err() {
                        return Err(RdpProtocolError::protocol("rdp decode event channel closed".to_string()));
                    }
                }
            }
            DecodeJob::Resize { width, height } => {
                if self.framebuffer.resize(width, height) {
                    let desktop = RespRdpDesktopSize { width, height };
                    emit_required(&self.event_tx, RespEventRdp::OnDesktopResize(desktop))?;
                }
            }
            DecodeJob::Persist(reply) => {
                if reply.send(self.update_state.bitmaps.persist_cells()).is_err() {
                    return Err(RdpProtocolError::protocol("rdp persist reply channel closed".to_string()));
                }
            }
        }
        self.last_decode_ms.store(started.elapsed().as_millis() as u64, Ordering::Relaxed);
        Ok(())
    }

    fn apply(&mut self, decoded: DecodedUpdate) -> RdpResult<()> {
        match decoded {
            DecodedUpdate::Bitmap(rectangles) => {
                for rect in rectangles {
                    self.framebuffer.blit_bitmap(&rect)?;
                }
                Ok(())
            }
            DecodedUpdate::Orders(orders) => {
                for order in orders {
                    let Some(draw) = update::consume_order(order, &mut self.update_state)? else {
                        continue;
                    };
                    painter::apply(&mut self.framebuffer, &draw, self.color_depth, &self.update_state.palette, &self.update_state.bitmaps, &self.update_state.brushes, &mut self.update_state.glyphs)?;
                }
                Ok(())
            }
            DecodedUpdate::Cursor(payload) => emit_event(&self.event_tx, RespEventRdp::OnCursor(payload)),
            DecodedUpdate::Surface(commands) => {
                for command in commands {
                    match command {
                        SurfaceCommand::Bits(rect) => self.framebuffer.blit_bitmap(&rect)?,
                        SurfaceCommand::FrameMarker { action, frame_id } if action == surface::SURFACECMD_FRAMEACTION_END => {
                            if self.event_out.send(DecodeEvent::SurfaceAck(frame_id)).is_err() {
                                return Err(RdpProtocolError::protocol("rdp decode event channel closed".to_string()));
                            }
                        }
                        SurfaceCommand::FrameMarker { .. } => {}
                    }
                }
                Ok(())
            }
        }
    }

    fn sync_focus(&mut self) {
        let (generation, rect) = self.focus.snapshot();
        if generation == self.focus_seen {
            return;
        }
        self.focus_seen = generation;
        self.focus_raw = rect;
        self.focus_pending = true;
    }

    fn focus_region(&self, extents: Option<Rect>) -> Option<Rect> {
        if self.avc_passthrough {
            return None;
        }
        let focus = self.focus_raw?.intersect(&self.framebuffer.surface());
        if focus.is_empty() {
            return None;
        }
        if self.focus_pending {
            return Some(focus);
        }
        let hit = extents?.intersect(&focus);
        if hit.is_empty() {
            return None;
        }
        Some(hit)
    }

    fn flush(&mut self) -> bool {
        if self.surfaces.in_frame() {
            return true;
        }
        let avc_patches = self.surfaces.take_avc_patches();
        if !avc_patches.is_empty() {
            self.avc_passthrough = true;
            let mut body = Vec::new();
            for (rect, payload) in avc_patches {
                match Framebuffer::pack_patch(framebuffer::FRAME_KIND_AVC444, rect, &payload) {
                    Ok(bytes) => body.extend_from_slice(&bytes),
                    Err(error) => {
                        let _ = self.event_tx.try_send(RespEventRdp::OnError(error.message));
                        return false;
                    }
                }
            }
            send_frame(&self.event_tx, &self.frame_sync, body, self.last_decode_ms.load(Ordering::Relaxed), 0);
        }
        let h264_patches = self.surfaces.take_h264_patches();
        if !h264_patches.is_empty() {
            self.avc_passthrough = true;
            let mut body = Vec::new();
            for (rect, payload) in h264_patches {
                match Framebuffer::pack_patch(framebuffer::FRAME_KIND_H264, rect, &payload) {
                    Ok(bytes) => body.extend_from_slice(&bytes),
                    Err(error) => {
                        let _ = self.event_tx.try_send(RespEventRdp::OnError(error.message));
                        return false;
                    }
                }
            }
            send_frame(&self.event_tx, &self.frame_sync, body, self.last_decode_ms.load(Ordering::Relaxed), 0);
        }
        if self.encode_queued.load(Ordering::Acquire) {
            return true;
        }
        self.sync_focus();
        let extents = self.framebuffer.take_dirty_extents();
        let focus = self.focus_region(extents).map(|rect| (rect, self.framebuffer.copy_rgba_rect(&rect)));
        if extents.is_none() && focus.is_none() {
            self.focus_pending = false;
            return true;
        }
        let desktop = self.framebuffer.surface();
        let rtt = self.frame_sync.rtt();
        let quality = framebuffer::suggest_quality(rtt);
        let bitrate = framebuffer::suggest_bitrate(rtt);
        let job = match extents {
            Some(extents) if u32::from(extents.width()) * u32::from(extents.height()) >= H264_MIN_DIRTY_AREA => {
                let (y, uv, width, height) = self.framebuffer.copy_nv12_aligned(H264_ALIGN);
                EncodeJob { desktop, quality, bitrate, nv12: Some((y, uv, width, height)), patches: Vec::new(), focus }
            }
            Some(extents) => EncodeJob { desktop, quality, bitrate, nv12: None, patches: vec![(extents, self.framebuffer.copy_rgba_rect(&extents))], focus },
            None => EncodeJob { desktop, quality, bitrate, nv12: None, patches: Vec::new(), focus },
        };
        self.encode_queued.store(true, Ordering::Release);
        match self.encode_tx.try_send(job) {
            Ok(()) => {
                self.focus_pending = false;
                true
            }
            Err(std_mpsc::TrySendError::Full(_)) => {
                self.encode_queued.store(false, Ordering::Release);
                if let Some(extents) = extents {
                    self.framebuffer.invalidate(extents);
                }
                true
            }
            Err(std_mpsc::TrySendError::Disconnected(_)) => false,
        }
    }
}

pub(crate) struct ReaderContext {
    bulk: BulkContext,
    device: RdpdrState,
    sound: RdpsndState,
    drive: Option<DriveState>,
    settings: Arc<RdpSettings>,
    message: MessageChannelState,
    assembler: FastPathAssembler,
    state: Arc<SyncMutex<RdpState>>,
    redirection: Option<Redirection>,
    clipboard_service: ClipboardService,
    event_tx: mpsc::Sender<RespEventRdp>,
    decode_tx: std_mpsc::SyncSender<DecodeJob>,
    writer: Arc<AsyncMutex<WriteHalf<RdpStream>>>,
    decode_event_rx: mpsc::UnboundedReceiver<DecodeEvent>,
}

impl ReaderContext {
    pub(crate) fn new(
        settings: Arc<RdpSettings>,
        state: Arc<SyncMutex<RdpState>>,
        writer: Arc<AsyncMutex<WriteHalf<RdpStream>>>,
        event_tx: mpsc::Sender<RespEventRdp>,
        desktop: &RespRdpDesktopSize,
        bulk: BulkContext,
        message: MessageChannelState,
        multifragment_update: Option<u32>,
        clipboard_service: ClipboardService,
        frame_sync: Arc<FrameSyncState>,
        focus: Arc<FocusState>,
        tasks: &TaskGroup,
    ) -> Self {
        let mut assembler = FastPathAssembler::default();
        assembler.set_max_request_size(multifragment_update.unwrap_or_default());
        let last_decode_ms = Arc::new(AtomicU64::new(0));
        let (encode_tx, encode_rx) = std_mpsc::sync_channel::<EncodeJob>(1);
        let encode_queued = Arc::new(AtomicBool::new(false));
        let worker = EncodeWorker {
            encoder: None,
            desktop: Rect::default(),
            event_tx: event_tx.clone(),
            queued: encode_queued.clone(),
            pending_dest: VecDeque::new(),
            frame_sync: frame_sync.clone(),
            encode_started: Instant::now(),
            last_decode_ms: last_decode_ms.clone(),
        };
        tasks.spawn_os_required("rdp-encode", move || worker.run(encode_rx));
        let (decode_tx, decode_rx) = std_mpsc::sync_channel::<DecodeJob>(4);
        let (decode_event_tx, decode_event_rx) = mpsc::unbounded_channel();
        let decode_worker = DecodeWorker {
            zgfx: ZgfxContext::default(),
            surfaces: GfxSurfaces::default(),
            framebuffer: Framebuffer::new(desktop.width, desktop.height),
            update_state: UpdateState::default(),
            color_depth: settings.color_depth,
            focus,
            focus_seen: 0,
            focus_raw: None,
            focus_pending: false,
            avc_passthrough: false,
            last_gfx_codec: 0,
            encode_queued,
            encode_tx,
            event_tx: event_tx.clone(),
            event_out: decode_event_tx,
            frame_sync: frame_sync.clone(),
            last_decode_ms,
        };
        tasks.spawn_os_required("rdp-decode", move || decode_worker.run(decode_rx));
        let (sound, sound_handle, sound_error) = RdpsndState::start();
        if let Some(error) = sound_error {
            warn!("[rdp] Rdp 音频输出不可用, 远端声音将被丢弃: {}", error);
        }
        if let Some(handle) = sound_handle {
            tasks.adopt_os(handle);
        }
        Self { bulk, state, writer, message, event_tx, settings, assembler, decode_tx, decode_event_rx, drive: None, redirection: None, clipboard_service, device: RdpdrState::default(), sound }
    }

    fn take_redirection(&mut self) -> Option<Redirection> {
        self.redirection.take()
    }

    async fn emit_error(&self, error: RdpProtocolError) {
        warn!("[rdp] Rdp 会话读取中断: {}", error.message);
        let _ = self.event_tx.send(RespEventRdp::OnError(error.message)).await;
    }

    fn submit_decode(&self, job: DecodeJob) -> RdpResult<()> {
        self.decode_tx.send(job).map_err(|_| RdpProtocolError::protocol("rdp decode channel closed".to_string()))
    }

    async fn apply_decode_event(&mut self, event: DecodeEvent) -> RdpResult<()> {
        match event {
            DecodeEvent::GfxAcks(acks) => {
                for ack in acks {
                    self.write_dynamic(rdpgfx::CHANNEL_NAME, &ack).await?;
                }
                Ok(())
            }
            DecodeEvent::SurfaceAck(frame_id) => self.acknowledge_frame(frame_id).await,
            DecodeEvent::Desktop(desktop) => {
                self.state.lock().desktop = desktop;
                Ok(())
            }
            DecodeEvent::Error(error) => Err(error),
        }
    }

    async fn write_channel(&self, body: &[u8]) -> RdpResult<()> {
        let channel = self.state.lock().channel;
        self.write_to_channel(channel.global_channel_id, body).await
    }

    async fn write_to_channel(&self, channel_id: u16, body: &[u8]) -> RdpResult<()> {
        let (user_id, wrapped) = {
            let mut state = self.state.lock();
            let wrapped = state.security.wrap(0, body)?;
            (state.channel.user_id, wrapped)
        };
        let bytes = mcs::build_send_data_request(user_id, channel_id, &wrapped)?;
        let mut writer = self.writer.lock().await;
        writer.write_all(&bytes).await.map_err(|error| RdpProtocolError::protocol(format!("failed to send data: {}", error)))?;
        writer.flush().await.map_err(|error| RdpProtocolError::protocol(format!("failed to flush data: {}", error)))?;
        Ok(())
    }

    async fn handle_frame(&mut self, frame: RdpFrame) -> RdpResult<()> {
        match frame {
            RdpFrame::FastPath { flags, payload } => {
                let payload = self.state.lock().security.unwrap_fastpath(flags, &payload)?;
                let updates = self.assembler.push(&payload, &mut self.bulk)?;
                if updates.is_empty() {
                    return Ok(());
                }
                let desktop = self.state.lock().desktop.clone();
                let surface = Rect::from_size(0, 0, desktop.width, desktop.height)?;
                self.submit_decode(DecodeJob::FastPath { updates, surface })
            }
            RdpFrame::X224(payload) => {
                let indication = mcs::parse_send_data_indication(&payload)?;
                let (channel, payload) = {
                    let mut state = self.state.lock();
                    let payload = state.security.unwrap(&indication.payload)?;
                    (state.channel, payload)
                };
                if indication.channel_id != channel.global_channel_id {
                    if channel.message_channel_id != 0 && indication.channel_id == channel.message_channel_id {
                        return self.handle_message_channel(&payload).await;
                    }
                    return self.handle_channel_data(indication.channel_id, &payload).await;
                }
                let mut cursor = ReadCursor::new(&payload);
                let header = pdu::read_share_control_header(&mut cursor)?;
                match header.pdu_type {
                    pdu::PDU_TYPE_DATA => self.handle_data_pdu(&mut cursor).await,
                    pdu::PDU_TYPE_DEMAND_ACTIVE => {
                        let demand = activation::parse_demand_active(&mut cursor)?;
                        self.reactivate(demand).await
                    }
                    pdu::PDU_TYPE_DEACTIVATE_ALL => Ok(()),
                    pdu::PDU_TYPE_FLOW_RESPONSE | pdu::PDU_TYPE_FLOW_STOP | pdu::PDU_TYPE_FLOW_TEST => Ok(()),
                    pdu::PDU_TYPE_SERVER_REDIRECTION => {
                        self.redirection = redirection::parse(&mut cursor)?;
                        Ok(())
                    }
                    other => Err(RdpProtocolError::protocol(format!("unexpected PDU during session: 0x{:02x}", other))),
                }
            }
        }
    }

    async fn handle_message_channel(&mut self, payload: &[u8]) -> RdpResult<()> {
        let Some(response) = msgchannel::process(&mut self.message, payload)? else {
            return Ok(());
        };
        let channel_id = self.state.lock().channel.message_channel_id;
        self.write_to_channel(channel_id, &response).await
    }

    async fn handle_channel_data(&mut self, channel_id: u16, payload: &[u8]) -> RdpResult<()> {
        let chunk = channel::parse_chunk(payload)?;
        let (name, message) = {
            let mut state = self.state.lock();
            let Some(name) = state.channels.channel_name(channel_id) else {
                return Err(RdpProtocolError::protocol(format!("received data on an unnegotiated static channel: id={}", channel_id)));
            };
            let Some(message) = state.channels.reassemble(channel_id, chunk) else {
                return Ok(());
            };
            (name, message)
        };
        match name {
            channel::DRDYNVC_CHANNEL_NAME => self.handle_dynamic_channel(channel_id, &message).await,
            channel::CLIPRDR_CHANNEL_NAME => self.handle_clipboard(channel_id, &message).await,
            channel::RDPDR_CHANNEL_NAME => self.handle_device_channel(channel_id, &message).await,
            channel::RDPSND_CHANNEL_NAME => self.handle_sound(channel_id, &message).await,
            other => Err(RdpProtocolError::unsupported(format!("static channel {} is not implemented", other))),
        }
    }

    async fn handle_device_channel(&mut self, channel_id: u16, message: &[u8]) -> RdpResult<()> {
        let outcome = match rdpdr::process(&mut self.device, CLIENT_NAME, self.settings.drive_path.as_deref(), message) {
            Ok(outcome) => outcome,
            Err(error) => {
                warn!("[rdp] rdp 通道 {} 处理失败: {}", channel::RDPDR_CHANNEL_NAME, error);
                return Ok(());
            }
        };
        for response in &outcome.responses {
            self.write_static(channel_id, response).await?;
        }
        let Some(request) = outcome.request else {
            return Ok(());
        };
        let drive = self.drive.get_or_insert_with(|| DriveState::new(self.settings.drive_path.as_deref().unwrap_or_default()));
        let (status, body) = match drive::handle(drive, &request) {
            Ok(result) => result,
            Err(error) => {
                warn!("[rdp] rdp 通道 {} 处理失败: {}", channel::RDPDR_CHANNEL_NAME, error);
                return Ok(());
            }
        };
        let completion = rdpdr::build_io_completion(request.device_id, request.completion_id, status, &body);
        self.write_static(channel_id, &completion).await
    }

    async fn handle_sound(&mut self, channel_id: u16, message: &[u8]) -> RdpResult<()> {
        let outcome = match rdpsnd::process(&mut self.sound, message) {
            Ok(outcome) => outcome,
            Err(error) => {
                warn!("[rdp] rdp 通道 {} 处理失败: {}", channel::RDPSND_CHANNEL_NAME, error);
                return Ok(());
            }
        };
        for response in &outcome.responses {
            self.write_static(channel_id, response).await?;
        }
        Ok(())
    }

    async fn handle_dynamic_channel(&mut self, channel_id: u16, message: &[u8]) -> RdpResult<()> {
        let outcome = match drdynvc::process(&mut self.state.lock().dynamic, message) {
            Ok(outcome) => outcome,
            Err(error) => {
                warn!("[rdp] rdp 通道 {} 处理失败: {}", channel::DRDYNVC_CHANNEL_NAME, error);
                return Ok(());
            }
        };
        for response in &outcome.responses {
            self.write_static(channel_id, response).await?;
        }
        if outcome.created == Some(rdpgfx::CHANNEL_NAME) {
            #[cfg(any(target_os = "android", target_os = "ios"))]
            let capabilities = rdpgfx::build_mobile_caps_advertise();
            #[cfg(not(any(target_os = "android", target_os = "ios")))]
            let capabilities = rdpgfx::build_caps_advertise();
            self.write_dynamic(rdpgfx::CHANNEL_NAME, &capabilities).await?;
        }
        let Some(message) = outcome.message else {
            return Ok(());
        };
        self.handle_dynamic_message(message).await
    }

    async fn handle_clipboard(&mut self, channel_id: u16, message: &[u8]) -> RdpResult<()> {
        let outcome = match cliprdr::process(&mut self.state.lock().clipboard, message) {
            Ok(outcome) => outcome,
            Err(error) => {
                warn!("[rdp] rdp 通道 {} 处理失败: {}", channel::CLIPRDR_CHANNEL_NAME, error);
                return Ok(());
            }
        };
        for response in &outcome.responses {
            self.write_static(channel_id, response).await?;
        }
        if outcome.text.is_some() || outcome.html.is_some() || outcome.image.is_some() || outcome.files.is_some() {
            self.clipboard_service.mark_self_write();
        }
        if let Some(request) = outcome.file_request {
            let response = self.read_clipboard_file(&request)?;
            self.write_static(channel_id, &response).await?;
        }
        if let Some(text) = outcome.text {
            let _ = tokio::task::spawn_blocking(move || {
                if let Err(error) = clipboard::write_text(&text) {
                    warn!("[rdp] rdp 剪贴板写入本机失败: {}", error);
                }
            })
            .await;
        }
        if let Some(html) = outcome.html {
            let _ = tokio::task::spawn_blocking(move || {
                if let Err(error) = clipboard::write_html(&html) {
                    warn!("[rdp] rdp 剪贴板写入本机 HTML 失败: {}", error);
                }
            })
            .await;
        }
        if let Some(image) = outcome.image {
            let _ = tokio::task::spawn_blocking(move || {
                if let Err(error) = clipboard::write_image(&image) {
                    warn!("[rdp] rdp 剪贴板写入本机图像失败: {}", error);
                }
            })
            .await;
        }
        if let Some(files) = outcome.files {
            let _ = tokio::task::spawn_blocking(move || {
                if let Err(error) = clipboard::write_remote_files(&files) {
                    warn!("[rdp] rdp 剪贴板写入本机文件失败: {}", error);
                }
            })
            .await;
        }
        Ok(())
    }

    fn read_clipboard_file(&self, request: &cliprdr::FileRequest) -> RdpResult<Vec<u8>> {
        let path = {
            let state = self.state.lock();
            let Some(entry) = state.clipboard.file_entry(request.index) else {
                return cliprdr::build_file_contents_failure(request.stream_id);
            };
            entry.path.clone()
        };
        match clipboard::read_file_range(&path, request) {
            Ok(data) => cliprdr::build_file_contents_response(request.stream_id, &data),
            Err(error) => {
                warn!("[rdp] rdp 剪贴板文件读取失败: {}", error);
                cliprdr::build_file_contents_failure(request.stream_id)
            }
        }
    }

    async fn handle_dynamic_message(&mut self, message: drdynvc::DynamicMessage) -> RdpResult<()> {
        if message.name == disp::CHANNEL_NAME {
            self.state.lock().max_num_monitors = disp::parse_caps(&message.payload)?;
            return Ok(());
        }
        if message.name != rdpgfx::CHANNEL_NAME {
            return Err(RdpProtocolError::unsupported(format!("dynamic channel {} is not implemented", message.name)));
        }
        self.submit_decode(DecodeJob::Gfx { payload: message.payload })
    }

    async fn write_dynamic(&self, name: &str, payload: &[u8]) -> RdpResult<()> {
        let (channel_id, body) = {
            let state = self.state.lock();
            let dynamic_id = state.dynamic.channel_id(name).ok_or_else(|| RdpProtocolError::protocol(format!("dynamic channel {} is not established", name)))?;
            let channel_id = state.channels.channel_id(channel::DRDYNVC_CHANNEL_NAME).ok_or_else(|| RdpProtocolError::protocol("drdynvc was not negotiated for dynamic channels".to_string()))?;
            (channel_id, drdynvc::build_data(dynamic_id, payload)?)
        };
        self.write_static(channel_id, &body).await
    }

    async fn write_static(&self, channel_id: u16, body: &[u8]) -> RdpResult<()> {
        let packets = self.state.lock().channels.split(channel_id, body)?;
        for packet in packets {
            self.write_to_channel(channel_id, &packet).await?;
        }
        Ok(())
    }

    async fn handle_data_pdu(&mut self, cursor: &mut ReadCursor<'_>) -> RdpResult<()> {
        let header = pdu::read_share_data_header(cursor)?;
        let body = pdu::read_data_pdu_body(cursor, &header, &mut self.bulk)?;
        let mut body = ReadCursor::new(&body);
        match header.pdu_type2 {
            pdu::PDU_TYPE2_UPDATE => {
                let update_type = body.read_u16_le()?;
                self.submit_decode(DecodeJob::Slowpath { update_type, body: body.rest().to_vec() })
            }
            pdu::PDU_TYPE2_POINTER => self.submit_decode(DecodeJob::Pointer { body: body.rest().to_vec() }),
            pdu::PDU_TYPE2_SET_ERROR_INFO => {
                let code = body.read_u32_le()?;
                if code == 0 {
                    Ok(())
                } else {
                    Err(RdpProtocolError::rejected(errinfo::describe(code)))
                }
            }
            pdu::PDU_TYPE2_SAVE_SESSION_INFO => {
                let cookie = logon::parse_save_session_info(&mut body)?;
                if cookie.is_some() {
                    self.state.lock().auto_reconnect = cookie;
                }
                Ok(())
            }
            pdu::PDU_TYPE2_STATUS_INFO => {
                info!("[rdp] Rdp 服务端状态: 0x{:08x}", logon::parse_status_info(&mut body)?);
                Ok(())
            }
            pdu::PDU_TYPE2_PLAY_SOUND => {
                let sound = logon::parse_play_sound(&mut body)?;
                rdpsnd::play_beep(&mut self.sound, sound.frequency, sound.duration)
            }
            pdu::PDU_TYPE2_MONITOR_LAYOUT => {
                info!("[rdp] Rdp 服务端显示器布局: {} 台", logon::parse_monitor_layout(&mut body)?);
                Ok(())
            }
            _ => Ok(()),
        }
    }

    async fn acknowledge_frame(&self, frame_id: u32) -> RdpResult<()> {
        let (channel, frame_acknowledge) = {
            let state = self.state.lock();
            (state.channel, state.frame_acknowledge)
        };
        if !frame_acknowledge {
            return Ok(());
        }
        self.write_channel(&activation::build_frame_acknowledge(channel.share_id, channel.user_id, frame_id)?).await
    }

    async fn reactivate(&mut self, demand: activation::DemandActive) -> RdpResult<()> {
        let desktop = RespRdpDesktopSize { width: demand.capabilities.desktop_width, height: demand.capabilities.desktop_height };
        let input_flags = capabilities::negotiate_input_flags(demand.capabilities.input_flags);
        self.assembler.set_max_request_size(demand.capabilities.multifragment_update.unwrap_or_default());
        let (channel, chunk_size) = {
            let mut state = self.state.lock();
            state.channel.share_id = demand.share_id;
            state.refresh_rect = demand.capabilities.refresh_rect;
            state.suppress_output = demand.capabilities.suppress_output;
            state.frame_acknowledge = demand.capabilities.frame_acknowledge.is_some_and(|value| value > 0);
            state.input_flags = input_flags;
            state.desktop = desktop.clone();
            state.channels.set_chunk_size(demand.capabilities.virtual_channel_chunk_size);
            state.security.set_salted_checksum(demand.capabilities.salted_checksum);
            (state.channel, state.channels.chunk_size())
        };
        let capabilities = ClientCapabilities {
            node_id: channel.user_id,
            color_depth: self.settings.color_depth,
            keyboard_layout: self.settings.keyboard_layout,
            desktop_width: desktop.width,
            desktop_height: desktop.height,
            virtual_channel_chunk_size: chunk_size,
            input_flags,
            order_support: capabilities::negotiate_order_support(&demand.capabilities.order_support),
            large_pointer: demand.capabilities.large_pointer,
            multifragment_update: demand.capabilities.multifragment_update,
            surface_commands: demand.capabilities.surface_commands,
            bitmap_codecs: demand.capabilities.bitmap_codecs,
            frame_acknowledge: Some(2),
        };
        self.write_channel(&activation::build_confirm_active(channel.share_id, channel.user_id, &capabilities)?).await?;
        self.write_channel(&activation::build_synchronize(channel.share_id, channel.user_id, channel.global_channel_id)?).await?;
        self.write_channel(&activation::build_control(channel.share_id, channel.user_id, pdu::CTRL_ACTION_COOPERATE)?).await?;
        self.write_channel(&activation::build_control(channel.share_id, channel.user_id, pdu::CTRL_ACTION_REQUEST_CONTROL)?).await?;
        self.submit_decode(DecodeJob::Resize { width: desktop.width, height: desktop.height })?;
        let (persist_tx, persist_rx) = oneshot::channel();
        self.submit_decode(DecodeJob::Persist(persist_tx))?;
        let cells = persist_rx.await.map_err(|_| RdpProtocolError::protocol("rdp persist reply channel closed".to_string()))?;
        self.write_channel(&activation::build_persistent_key_list(channel.share_id, channel.user_id, &cells)?).await?;
        self.write_channel(&activation::build_font_list(channel.share_id, channel.user_id)?).await?;
        info!("[rdp] Rdp 会话重激活. 桌面: {}x{}", desktop.width, desktop.height);
        Ok(())
    }
}

pub(crate) fn spawn_reader(client: Arc<ClientRdp>, mut reader: ReadHalf<RdpStream>, mut context: ReaderContext, mut shutdown_rx: oneshot::Receiver<()>, tasks: &TaskGroup) {
    tasks.spawn("rdp-reader", async move {
        loop {
            tokio::select! {
                _ = &mut shutdown_rx => {
                    return;
                }
                frame = transport::read_frame(&mut reader) => {
                    let frame = match frame {
                        Ok(frame) => frame,
                        Err(error) => {
                            context.emit_error(error).await;
                            break;
                        }
                    };
                    if let Err(error) = context.handle_frame(frame).await {
                        context.emit_error(error).await;
                        break;
                    }
                    if let Some(redirection) = context.take_redirection() {
                        if let Err(error) = client.redirect(redirection).await {
                            warn!("[rdp] rdp 会话重定向失败: {}", error);
                        }
                        return;
                    }
                }
                event = context.decode_event_rx.recv() => {
                    let Some(event) = event else {
                        context.emit_error(RdpProtocolError::protocol("rdp decode worker disconnected".to_string())).await;
                        break;
                    };
                    if let Err(error) = context.apply_decode_event(event).await {
                        context.emit_error(error).await;
                        break;
                    }
                }
            }
        }
        if let Err(error) = client.close().await {
            warn!("[rdp] rdp 连接断开后清理失败: {}", error);
        }
    });
}
