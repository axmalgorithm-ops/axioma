pub struct FastAdaptiveModel {
    pub freq: Box<[u32; 257]>,
    pub cum: Box<[u32; 258]>,
    pub total: u32,
}

impl FastAdaptiveModel {
    pub fn new() -> Self {
        // Initialize all symbols with an equal base weight
        let freq = Box::new([1u32; 257]);
        let mut cum = Box::new([0u32; 258]);

        let mut acc = 0;
        for i in 0..=256 {
            cum[i] = acc;
            acc += freq[i];
        }
        cum[257] = acc;

        Self {
            freq,
            cum,
            total: acc,
        }
    }

    pub fn update(&mut self, symbol: u8) {
        let sym = symbol as usize;
        self.freq[sym] += 16;
        self.total += 16;

        // Scaling mechanism: when the total frequency mass exceeds the threshold,
        // we halve all frequencies (bitwise right shift) to prevent range overflow.
        // The .max(1) ensures no frequency ever drops to absolute zero.
        if self.total >= 8192 {
            let mut acc = 0;
            for i in 0..=256 {
                self.freq[i] = (self.freq[i] >> 1).max(1);
                self.cum[i] = acc;
                acc += self.freq[i];
            }
            self.cum[257] = acc;
            self.total = acc;
        } else {
            // Fast recalculation of the cumulative distribution
            let mut acc = 0;
            for i in 0..=256 {
                self.cum[i] = acc;
                acc += self.freq[i];
            }
            self.cum[257] = acc;
        }
    }
}
