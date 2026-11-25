use std::fs;
use std::time::Instant;
use std::process::Command;
use std::path::{Path, PathBuf};
use crate::module::rdp::protocol::codec::jpeg;
use crate::module::rdp::protocol::gdi::framebuffer::{Framebuffer, Rect, FRAME_KIND_JPEG};

const VIDEO: &str = r"C:\Users\chengpeng\Desktop\李嘉誠：感覺回到了20歲！「回春藥」研究獲突破，領袖说「活到150歲」不是開玩笑！【文昭思緒飛揚534】.mp4";
const WIDTH: u16 = 1280;
const HEIGHT: u16 = 720;
const FRAMES: usize = 12;
const QUALITY: u8 = 90;
const MIN_FPS: f64 = 60.0;

fn extract_frames(dir: &Path) -> Vec<Vec<u8>> {
    if !Path::new(VIDEO).is_file() {
        panic!("视频不存在: {}", VIDEO);
    }
    let raw = dir.join("frames.rgba");
    let status = Command::new("ffmpeg")
        .args([
            "-y",
            "-hide_banner",
            "-loglevel",
            "error",
            "-ss",
            "1",
            "-i",
            VIDEO,
            "-frames:v",
            &FRAMES.to_string(),
            "-vf",
            &format!("scale={WIDTH}:{HEIGHT}"),
            "-f",
            "rawvideo",
            "-pix_fmt",
            "rgba",
            raw.to_str().expect("raw path"),
        ])
        .status()
        .expect("启动 ffmpeg");
    if !status.success() {
        panic!("ffmpeg 抽帧失败: {:?}", status.code());
    }
    let bytes = fs::read(&raw).expect("读 rgba");
    let frame_len = WIDTH as usize * HEIGHT as usize * 4;
    if bytes.len() != frame_len * FRAMES {
        panic!("rgba 长度异常: need {} have {}", frame_len * FRAMES, bytes.len());
    }
    bytes.chunks_exact(frame_len).map(|chunk| chunk.to_vec()).collect()
}

fn rect_count(body: &[u8]) -> usize {
    let mut offset = 0usize;
    let mut count = 0usize;
    while offset + 13 <= body.len() {
        let len = u32::from_le_bytes([body[offset + 9], body[offset + 10], body[offset + 11], body[offset + 12]]) as usize;
        offset = offset + 13 + len;
        count += 1;
    }
    count
}

fn measure_fps(frames: usize, work: impl FnMut()) -> f64 {
    let mut work = work;
    work();
    let started = Instant::now();
    for _ in 0..frames {
        work();
    }
    let ms = started.elapsed().as_secs_f64() * 1000.0;
    (frames as f64) * 1000.0 / ms.max(0.001)
}

fn scatter_dirty(framebuffer: &mut Framebuffer, frame: &[u8]) {
    let stride = WIDTH as usize * 4;
    let mut top = 0u16;
    while top < HEIGHT {
        let mut left = 0u16;
        while left < WIDTH {
            let width = 160u16.min(WIDTH - left);
            let height = 180u16.min(HEIGHT - top);
            let rect = Rect::from_size(left, top, width, height).expect("scatter rect");
            framebuffer.blit_surface(rect, frame, stride, left, top).expect("scatter blit");
            left += 160;
        }
        top += 180;
    }
}

#[test]
fn video_encode_decode_fps() {
    let dir = PathBuf::from(r"c:\project\rust\termix-project\termini\src-tauri\target\rdp-codec-fps");
    fs::create_dir_all(&dir).expect("workdir");
    let frames = extract_frames(&dir);
    let sample = &frames[0];

    let jpeg_fps = measure_fps(frames.len(), || {
        let _ = jpeg::encode(sample, WIDTH, HEIGHT, QUALITY).expect("jpeg");
    });
    if jpeg_fps < MIN_FPS {
        panic!("JPEG 编码未到 {MIN_FPS}fps: {jpeg_fps:.2}");
    }

    let rect = Rect::from_size(0, 0, WIDTH, HEIGHT).expect("rect");
    let mut framebuffer = Framebuffer::new(WIDTH, HEIGHT);
    let mut index = 0usize;
    let mut last_kind = 0u8;
    let mut last_rects = 0usize;
    let take_fps = measure_fps(frames.len(), || {
        let frame = &frames[index % frames.len()];
        framebuffer.blit_surface(rect, frame, WIDTH as usize * 4, 0, 0).expect("blit");
        let body = framebuffer.take_frame(QUALITY).expect("take_frame").expect("dirty");
        last_kind = body[0];
        last_rects = rect_count(&body);
        index += 1;
    });
    if last_kind != FRAME_KIND_JPEG {
        panic!("全帧脏区应为 JPEG, kind={last_kind}");
    }
    if last_rects != 1 {
        panic!("全帧脏区应为 1 个矩形, rects={last_rects}");
    }
    if take_fps < MIN_FPS {
        panic!("take_frame 全帧未到 {MIN_FPS}fps: {take_fps:.2}");
    }

    let mut scattered = Framebuffer::new(WIDTH, HEIGHT);
    index = 0;
    last_kind = 0;
    last_rects = 0;
    let scatter_fps = measure_fps(frames.len(), || {
        let frame = &frames[index % frames.len()];
        scatter_dirty(&mut scattered, frame);
        if scattered.dirty_len() == 0 {
            panic!("scatter 未产生脏区");
        }
        let body = scattered.take_frame(QUALITY).expect("take_frame").expect("dirty");
        last_kind = body[0];
        last_rects = rect_count(&body);
        index += 1;
    });
    if last_kind != FRAME_KIND_JPEG {
        panic!("多脏区合并后应为 JPEG, kind={last_kind}");
    }
    if last_rects != 1 {
        panic!("多脏区应合并为 1 个矩形, rects={last_rects}");
    }
    if scatter_fps < MIN_FPS {
        panic!("take_frame 多脏区未到 {MIN_FPS}fps: {scatter_fps:.2}");
    }
}
