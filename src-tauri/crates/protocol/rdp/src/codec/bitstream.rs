pub struct BitStream<'a> {
    data: &'a [u8],
    position: usize,
}

impl<'a> BitStream<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, position: 0 }
    }

    pub fn remaining(&self) -> usize {
        (self.data.len() * 8).saturating_sub(self.position)
    }

    pub fn accumulator(&self) -> u32 {
        let byte = self.position / 8;
        let bit = self.position % 8;
        let mut window = 0u64;
        for offset in 0..5 {
            window <<= 8;
            if let Some(value) = self.data.get(byte + offset) {
                window |= u64::from(*value);
            }
        }
        (window >> (8 - bit)) as u32
    }

    pub fn shift(&mut self, count: usize) {
        self.position += count;
    }

    pub fn read_bits(&mut self, count: u32) -> u32 {
        if count == 0 {
            return 0;
        }
        let value = self.accumulator() >> (32 - count);
        self.shift(count as usize);
        value
    }

    pub fn count_leading(&mut self, ones: bool) -> u32 {
        let mut count = self.leading(ones);
        let mut total = count;
        while count == 32 && self.remaining() > 0 {
            self.shift(32);
            count = self.leading(ones);
            total += count;
        }
        self.shift((total % 32) as usize);
        total
    }

    fn leading(&self, ones: bool) -> u32 {
        let accumulator = if ones { !self.accumulator() } else { self.accumulator() };
        accumulator.leading_zeros().min(self.remaining() as u32)
    }
}
