use crate::entropy::FastAdaptiveModel;

const TOP: u32 = 1 << 24;
const BOTTOM: u32 = 1 << 16;

pub struct FastRangeEncoder {
    low: u32,
    range: u32,
    output: Vec<u8>,
}

impl FastRangeEncoder {
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            low: 0,
            range: 0xFFFF_FFFF,
            output: Vec::with_capacity(cap),
        }
    }

    pub fn encode(&mut self, symbol: u8, model: &mut FastAdaptiveModel) {
        let sym = symbol as usize;
        let cum_low = model.cum[sym];
        let freq = model.freq[sym];
        let total = model.total;

        // Core arithmetic encoding step with wrapping arithmetic for debug safety
        self.range /= total;
        let add_val = cum_low.wrapping_mul(self.range);
        self.low = self.low.wrapping_add(add_val);
        self.range = self.range.wrapping_mul(freq);

        // Renormalization loop
        while self.range < TOP {
            if self.low ^ (self.low.wrapping_add(self.range)) >= TOP {
                if self.range < BOTTOM {
                    let mask = BOTTOM - 1;
                    self.range = (!self.low & mask).wrapping_add(1);
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
    low: u32,
    range: u32,
    code: u32,
    data: &'a [u8],
    pos: usize,
}

impl<'a> FastRangeDecoder<'a> {
    pub fn new(data: &'a [u8]) -> Option<Self> {
        if data.len() < 4 { return None; }
        
        let mut code = 0;
        for i in 0..4 {
            code = (code << 8) | (data[i] as u32);
        }
        
        Some(Self {
            low: 0,
            range: 0xFFFF_FFFF,
            code,
            data,
            pos: 4,
        })
    }

    pub fn decode(&mut self, model: &mut FastAdaptiveModel) -> u8 {
        let total = model.total;
        self.range /= total;
        
        let count = (self.code.wrapping_sub(self.low)) / self.range;

        let mut sym = 0;
        while sym < 256 && model.cum[sym + 1] <= count {
            sym += 1;
        }

        let add_val = model.cum[sym].wrapping_mul(self.range);
        self.low = self.low.wrapping_add(add_val);
        self.range = self.range.wrapping_mul(model.freq[sym]);

        while self.range < TOP {
            if self.low ^ (self.low.wrapping_add(self.range)) >= TOP {
                if self.range < BOTTOM {
                    let mask = BOTTOM - 1;
                    self.range = (!self.low & mask).wrapping_add(1);
                } else {
                    break;
                }
            }
            self.code = (self.code << 8) | (self.read_byte() as u32);
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
