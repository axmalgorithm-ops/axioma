use crate::entropy::FastAdaptiveModel;

const TOP: u64 = 1 << 24;
const BOTTOM: u64 = 1 << 16;

pub struct FastRangeEncoder {
    low: u64,
    range: u64,
    output: Vec<u8>,
}

impl FastRangeEncoder {
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            low: 0,
            range: 0xFFFF_FFFF_u64,
            output: Vec::with_capacity(cap),
        }
    }

    pub fn encode(&mut self, symbol: u8, model: &mut FastAdaptiveModel) {
        let sym = symbol as usize;
        let cum_low = model.cum[sym] as u64;
        let freq = model.freq[sym] as u64;
        let total = model.total as u64;

        self.range /= total;
        self.low += cum_low * self.range;
        self.range *= freq;

        while self.range < TOP {
            if (self.low ^ (self.low + self.range)) >= TOP {
                if self.range < BOTTOM {
                    self.range = (!self.low & (BOTTOM - 1)) + 1;
                } else {
                    break;
                }
            }
            self.output.push((self.low >> 24) as u8);
            self.range <<= 8;
            self.low <<= 8;
        }

        model.update(symbol);
    }

    pub fn finish(&mut self) -> &[u8] {
        for _ in 0..4 {
            self.output.push((self.low >> 24) as u8);
            self.low <<= 8;
        }
        &self.output
    }

    pub fn output(&self) -> &[u8] {
        &self.output
    }
}

pub struct FastRangeDecoder<'a> {
    low: u64,
    range: u64,
    code: u64,
    data: &'a [u8],
    pos: usize,
}

impl<'a> FastRangeDecoder<'a> {
    pub fn new(data: &'a [u8]) -> Option<Self> {
        if data.len() < 4 { return None; }
        
        let mut code = 0u64;
        for i in 0..4 {
            code = (code << 8) | (data[i] as u64);
        }
        
        Some(Self {
            low: 0,
            range: 0xFFFF_FFFF_u64,
            code,
            data,
            pos: 4,
        })
    }

    pub fn decode(&mut self, model: &mut FastAdaptiveModel) -> u8 {
        let total = model.total as u64;
        self.range /= total;
        
        let count = (self.code - self.low) / self.range;

        let mut sym = 0;
        while sym < 256 && (model.cum[sym + 1] as u64) <= count {
            sym += 1;
        }

        let cum_low = model.cum[sym] as u64;
        let freq = model.freq[sym] as u64;

        self.low += cum_low * self.range;
        self.range *= freq;

        while self.range < TOP {
            if (self.low ^ (self.low + self.range)) >= TOP {
                if self.range < BOTTOM {
                    self.range = (!self.low & (BOTTOM - 1)) + 1;
                } else {
                    break;
                }
            }
            self.code = (self.code << 8) | (self.read_byte() as u64);
            self.range <<= 8;
            self.low <<= 8;
        }

        model.update(sym as u8);
        sym as u8
    }

    fn read_byte(&mut self) -> u8 {
        if self.pos < self.data.len() {
            let b = self.data[self.pos];
            self.pos += 1;
            b
        } else {
            0
        }
    }
}
