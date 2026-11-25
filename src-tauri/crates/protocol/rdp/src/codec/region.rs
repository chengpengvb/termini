use crate::gdi::framebuffer::Rect;

#[derive(Default)]
pub struct Region {
    rects: Vec<Rect>,
    extents: Rect,
}

impl Region {
    pub fn rects(&self) -> &[Rect] {
        &self.rects
    }

    pub fn extents(&self) -> Rect {
        self.extents
    }

    pub fn union_rect(&mut self, rect: Rect) {
        if self.rects.is_empty() {
            self.extents = rect;
            self.rects = vec![rect];
            return;
        }

        let source = std::mem::take(&mut self.rects);
        let mut items = Vec::with_capacity(source.len() + 1);

        if rect.top < self.extents.top {
            items.push(Rect { left: rect.left, top: rect.top, right: rect.right, bottom: self.extents.top.min(rect.bottom) });
        }

        let mut current = 0usize;
        while current < source.len() {
            let band = source[current];
            let (next, inter_band) = if band.bottom <= rect.top || rect.bottom <= band.top || contained_in_band(&source, current, &rect) {
                (copy_band_with_union(&mut items, &source, current, band.top, band.bottom, None), rect.top)
            } else {
                let mut merge_top = band.top;
                let mut merge_bottom = band.bottom;
                if rect.top > band.top {
                    copy_band_with_union(&mut items, &source, current, band.top, rect.top, None);
                    merge_top = rect.top;
                }
                if rect.bottom < band.bottom {
                    merge_bottom = rect.bottom;
                }
                let next = copy_band_with_union(&mut items, &source, current, merge_top, merge_bottom, Some(&rect));
                if rect.bottom < band.bottom {
                    copy_band_with_union(&mut items, &source, current, merge_bottom, band.bottom, None);
                }
                (next, band.bottom)
            };

            if next < source.len() && source[next].top != band.bottom && rect.bottom > band.bottom && rect.top < source[next].top {
                items.push(Rect { left: rect.left, top: inter_band, right: rect.right, bottom: source[next].top.min(rect.bottom) });
            }
            current = next;
        }

        if self.extents.bottom < rect.bottom {
            items.push(Rect { left: rect.left, top: self.extents.bottom.max(rect.top), right: rect.right, bottom: rect.bottom });
        }

        self.extents = self.extents.union(&rect);
        self.rects = items;
        simplify_bands(&mut self.rects);
    }

    pub fn intersect_rect(&self, rect: &Rect) -> Region {
        if self.rects.is_empty() {
            return Region::default();
        }

        if self.rects.len() == 1 {
            let common = self.extents.intersect(rect);
            if common.is_empty() {
                return Region::default();
            }
            return Region { rects: vec![common], extents: common };
        }

        let mut items = Vec::with_capacity(self.rects.len());
        let mut extents = Rect::default();
        for source in &self.rects {
            if rect.bottom <= source.top {
                break;
            }
            let common = source.intersect(rect);
            if common.is_empty() {
                continue;
            }
            extents = if extents.is_empty() { common } else { extents.union(&common) };
            items.push(common);
        }

        let mut region = Region { rects: items, extents };
        simplify_bands(&mut region.rects);
        region
    }
}

fn copy_band_with_union(destination: &mut Vec<Rect>, source: &[Rect], mut index: usize, top: u16, bottom: u16, union: Option<&Rect>) -> usize {
    let reference = source[index].top;

    if let Some(union) = union {
        while index < source.len() && source[index].top == reference && source[index].right < union.left {
            destination.push(Rect { left: source[index].left, top, right: source[index].right, bottom });
            index += 1;
        }

        let mut start_overlap = *union;
        let mut end_overlap = *union;

        if index < source.len() && source[index].top == reference && source[index].left < union.left {
            start_overlap = source[index];
        }

        while index < source.len() && source[index].top == reference && source[index].right < union.right {
            index += 1;
        }

        if index < source.len() && source[index].top == reference && source[index].left < union.right {
            end_overlap = source[index];
            index += 1;
        }

        destination.push(Rect { left: start_overlap.left, top, right: end_overlap.right, bottom });
    }

    while index < source.len() && source[index].top == reference {
        destination.push(Rect { left: source[index].left, top, right: source[index].right, bottom });
        index += 1;
    }

    index
}

fn contained_in_band(source: &[Rect], mut index: usize, rect: &Rect) -> bool {
    let reference = source[index].top;
    if source[index].top > rect.top || rect.bottom > source[index].bottom {
        return false;
    }
    while index < source.len() && source[index].top == reference && source[index].left <= rect.left {
        if rect.right <= source[index].right {
            return true;
        }
        index += 1;
    }
    false
}

fn next_band(source: &[Rect], index: usize) -> (usize, usize) {
    let reference = source[index].top;
    let mut end = index;
    while end < source.len() && source[end].top == reference {
        end += 1;
    }
    (end, end - index)
}

fn band_match(source: &[Rect], mut first: usize, second: usize) -> bool {
    let reference = source[second].top;
    let mut cursor = second;
    while first < second && cursor < source.len() && source[cursor].top == reference {
        if source[first].left != source[cursor].left || source[first].right != source[cursor].right {
            return false;
        }
        first += 1;
        cursor += 1;
    }
    if first != second {
        return false;
    }
    cursor == source.len() || source[cursor].top != reference
}

fn simplify_bands(rects: &mut Vec<Rect>) {
    if rects.len() < 2 {
        return;
    }

    let mut band = 0usize;
    loop {
        let (next, count) = next_band(rects, band);
        if next == rects.len() {
            break;
        }
        if rects[band].bottom == rects[next].top && band_match(rects, band, next) {
            let bottom = rects[next].bottom;
            for rect in &mut rects[band..next] {
                rect.bottom = bottom;
            }
            rects.drain(next..next + count);
        } else {
            band = next;
        }
    }
}
