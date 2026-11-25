use crate::codec::color;
use crate::gdi::framebuffer::Rect;

pub struct Nv12Image {
    width: u16,
    height: u16,
    y: Vec<u8>,
    uv: Vec<u8>,
}

impl Nv12Image {
    pub fn new(width: u16, height: u16) -> Self {
        let w = width as usize;
        let h = height as usize;
        Self { width, height, y: vec![16u8; w * h], uv: vec![128u8; w * h.div_ceil(2)] }
    }

    pub fn width(&self) -> u16 {
        self.width
    }

    pub fn height(&self) -> u16 {
        self.height
    }

    pub fn y_stride(&self) -> usize {
        self.width as usize
    }

    pub fn uv_stride(&self) -> usize {
        self.width as usize
    }

    pub fn planes(&self) -> (&[u8], &[u8]) {
        (&self.y, &self.uv)
    }

    pub fn copy_planes(&self) -> (Vec<u8>, Vec<u8>) {
        (self.y.clone(), self.uv.clone())
    }

    pub fn copy_planes_aligned(&self, align: u16) -> (Vec<u8>, Vec<u8>, u16, u16) {
        let aligned_width = self.width.div_ceil(align) * align;
        let aligned_height = self.height.div_ceil(align) * align;
        if aligned_width == self.width && aligned_height == self.height {
            return (self.y.clone(), self.uv.clone(), self.width, self.height);
        }
        let src_w = self.width as usize;
        let src_h = self.height as usize;
        let dst_w = aligned_width as usize;
        let dst_h = aligned_height as usize;
        let mut y = vec![16u8; dst_w * dst_h];
        let mut uv = vec![128u8; dst_w * dst_h.div_ceil(2)];
        for row in 0..src_h {
            let origin = row * src_w;
            let target = row * dst_w;
            y[target..target + src_w].copy_from_slice(&self.y[origin..origin + src_w]);
            if src_w < dst_w {
                y[target + src_w..target + dst_w].fill(self.y[origin + src_w - 1]);
            }
        }
        if src_h > 0 {
            let last = (src_h - 1) * dst_w;
            for row in src_h..dst_h {
                let target = row * dst_w;
                y.copy_within(last..last + dst_w, target);
            }
        }
        let src_uv_h = src_h.div_ceil(2);
        let dst_uv_h = dst_h.div_ceil(2);
        for row in 0..src_uv_h {
            let origin = row * src_w;
            let target = row * dst_w;
            uv[target..target + src_w].copy_from_slice(&self.uv[origin..origin + src_w]);
            if src_w + 1 < dst_w {
                let last_u = self.uv[origin + src_w - 2];
                let last_v = self.uv[origin + src_w - 1];
                let mut column = src_w;
                while column + 1 < dst_w {
                    uv[target + column] = last_u;
                    uv[target + column + 1] = last_v;
                    column += 2;
                }
            }
        }
        if src_uv_h > 0 {
            let last = (src_uv_h - 1) * dst_w;
            for row in src_uv_h..dst_uv_h {
                let target = row * dst_w;
                uv.copy_within(last..last + dst_w, target);
            }
        }
        (y, uv, aligned_width, aligned_height)
    }

    pub fn fill_rgb(&mut self, red: u8, green: u8, blue: u8) {
        let (y, u, v) = color::rgb_to_yuv(red, green, blue, self.width >= 1280);
        self.y.fill(y);
        let mut index = 0usize;
        while index + 1 < self.uv.len() {
            self.uv[index] = u;
            self.uv[index + 1] = v;
            index += 2;
        }
    }

    pub fn blit_rgba(&mut self, dest: Rect, rgba: &[u8], stride: usize, source_left: u16, source_top: u16) {
        if dest.is_empty() || self.width == 0 || self.height == 0 {
            return;
        }
        let bt709 = self.width >= 1280;
        let aligned_left = dest.left & !1;
        let aligned_top = dest.top & !1;
        let aligned_right = dest.right.saturating_add(1) & !1;
        let aligned_bottom = dest.bottom.saturating_add(1) & !1;
        let left = aligned_left.min(self.width) as usize;
        let top = aligned_top.min(self.height) as usize;
        let right = aligned_right.min(self.width) as usize;
        let bottom = aligned_bottom.min(self.height) as usize;
        let src_origin_x = source_left.saturating_sub(dest.left.saturating_sub(aligned_left)) as usize;
        let src_origin_y = source_top.saturating_sub(dest.top.saturating_sub(aligned_top)) as usize;
        let y_stride = self.y_stride();
        let uv_stride = self.uv_stride();
        for row in (top..bottom).step_by(2) {
            for column in (left..right).step_by(2) {
                let mut sum_u = 0u32;
                let mut sum_v = 0u32;
                let mut count = 0u32;
                for dy in 0..2 {
                    let dest_row = row + dy;
                    if dest_row >= bottom {
                        continue;
                    }
                    for dx in 0..2 {
                        let dest_column = column + dx;
                        if dest_column >= right {
                            continue;
                        }
                        let source_row = src_origin_y + dest_row - top;
                        let source_column = src_origin_x + dest_column - left;
                        let origin = source_row * stride + source_column * 4;
                        if origin + 3 >= rgba.len() {
                            continue;
                        }
                        let (y, u, v) = color::rgb_to_yuv(rgba[origin], rgba[origin + 1], rgba[origin + 2], bt709);
                        self.y[dest_row * y_stride + dest_column] = y;
                        sum_u += u32::from(u);
                        sum_v += u32::from(v);
                        count += 1;
                    }
                }
                if count == 0 {
                    continue;
                }
                let uv_row = (row / 2) * uv_stride + column;
                if uv_row + 1 < self.uv.len() {
                    self.uv[uv_row] = (sum_u / count) as u8;
                    self.uv[uv_row + 1] = (sum_v / count) as u8;
                }
            }
        }
    }

    pub fn blit_tile(&mut self, dest_left: usize, dest_top: usize, clip: Rect, luma: &[u8], chroma: &[u8], tile_size: usize) {
        if clip.is_empty() {
            return;
        }
        let y_stride = self.y_stride();
        let uv_stride = self.uv_stride();
        let clip_left = clip.left as usize;
        let clip_top = clip.top as usize;
        let clip_right = clip.right.min(self.width) as usize;
        let clip_bottom = clip.bottom.min(self.height) as usize;
        let copy_left = clip_left.max(dest_left);
        let copy_top = clip_top.max(dest_top);
        let copy_right = clip_right.min(dest_left.saturating_add(tile_size));
        let copy_bottom = clip_bottom.min(dest_top.saturating_add(tile_size));
        if copy_left >= copy_right || copy_top >= copy_bottom {
            return;
        }
        for row in copy_top..copy_bottom {
            let source_row = row - dest_top;
            let source = source_row * tile_size + (copy_left - dest_left);
            let target = row * y_stride + copy_left;
            let width = copy_right - copy_left;
            if source + width <= luma.len() && target + width <= self.y.len() {
                self.y[target..target + width].copy_from_slice(&luma[source..source + width]);
            }
        }
        if dest_left % 2 != 0 || dest_top % 2 != 0 {
            return;
        }
        let uv_left = copy_left & !1;
        let uv_top = copy_top & !1;
        let uv_right = copy_right & !1;
        let uv_bottom = copy_bottom & !1;
        if uv_right <= uv_left || uv_bottom <= uv_top {
            return;
        }
        for row in (uv_top..uv_bottom).step_by(2) {
            let source_row = (row - dest_top) / 2;
            let source = source_row * tile_size + (uv_left - dest_left);
            let target = (row / 2) * uv_stride + uv_left;
            let width = uv_right - uv_left;
            if source + width <= chroma.len() && target + width <= self.uv.len() {
                self.uv[target..target + width].copy_from_slice(&chroma[source..source + width]);
            }
        }
    }

    pub fn blit_from(&mut self, source: &Nv12Image, dest: Rect, source_left: u16, source_top: u16) {
        if dest.is_empty() {
            return;
        }
        let y_stride = self.y_stride();
        let uv_stride = self.uv_stride();
        let source_y_stride = source.y_stride();
        let source_uv_stride = source.uv_stride();
        let left = dest.left as usize;
        let top = dest.top as usize;
        let width = dest.width() as usize;
        let height = dest.height() as usize;
        let src_left = source_left as usize;
        let src_top = source_top as usize;
        for row in 0..height {
            if top + row >= self.height as usize || src_top + row >= source.height as usize {
                break;
            }
            let copy = width.min(self.width as usize - left).min(source.width as usize - src_left);
            let target = (top + row) * y_stride + left;
            let origin = (src_top + row) * source_y_stride + src_left;
            self.y[target..target + copy].copy_from_slice(&source.y[origin..origin + copy]);
        }
        let uv_left = left & !1;
        let uv_top = top & !1;
        let uv_src_left = src_left & !1;
        let uv_src_top = src_top & !1;
        let uv_width = (width + (left & 1) + 1) & !1;
        let uv_height = (height + (top & 1) + 1) & !1;
        for row in (0..uv_height).step_by(2) {
            let dest_row = uv_top + row;
            let origin_row = uv_src_top + row;
            if dest_row + 1 >= self.height as usize || origin_row + 1 >= source.height as usize {
                break;
            }
            let copy = uv_width.min(self.width as usize - uv_left).min(source.width as usize - uv_src_left) & !1;
            let target = (dest_row / 2) * uv_stride + uv_left;
            let origin = (origin_row / 2) * source_uv_stride + uv_src_left;
            if copy == 0 || target + copy > self.uv.len() || origin + copy > source.uv.len() {
                continue;
            }
            self.uv[target..target + copy].copy_from_slice(&source.uv[origin..origin + copy]);
        }
    }
}
