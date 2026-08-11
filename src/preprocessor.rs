use crate::error::Error;

pub trait Preprocessor {
    fn process(&mut self, input: &[u8], output: &mut [u8]) -> (usize, usize);
    fn flush(&mut self, output: &mut [u8]) -> usize;
    fn reverse_process(&mut self, input: &[u8], output: &mut [u8]) -> (usize, usize);
    fn reverse_flush(&mut self, output: &mut [u8]) -> usize {
        let _ = output;
        0
    }
}

// ---------- NoOp ----------
pub struct NoOpPreprocessor;

impl Preprocessor for NoOpPreprocessor {
    fn process(&mut self, input: &[u8], output: &mut [u8]) -> (usize, usize) {
        let len = input.len().min(output.len());
        output[..len].copy_from_slice(&input[..len]);
        (len, len)
    }
    fn flush(&mut self, _output: &mut [u8]) -> usize {
        0
    }
    fn reverse_process(&mut self, input: &[u8], output: &mut [u8]) -> (usize, usize) {
        self.process(input, output)
    }
}

// ---------- Delta Preprocessor (XOR difference, fixed width) ----------
pub struct DeltaPreprocessor {
    width: usize, // 1, 2, 4 or 8
    buffer: [u8; 8],
    buf_len: usize,
    prev: u64,
    initialized: bool,
}

impl DeltaPreprocessor {
    pub fn new(width: usize) -> Self {
        assert!(width == 1 || width == 2 || width == 4 || width == 8);
        Self {
            width,
            buffer: [0; 8],
            buf_len: 0,
            prev: 0,
            initialized: false,
        }
    }

    fn read_word(buf: &[u8], width: usize) -> u64 {
        let mut val: u64 = 0;
        for i in 0..width {
            val |= (buf[i] as u64) << (i * 8);
        }
        val
    }

    fn write_word(val: u64, width: usize, out: &mut [u8]) {
        for i in 0..width {
            out[i] = (val >> (i * 8)) as u8;
        }
    }
}

impl Preprocessor for DeltaPreprocessor {
    fn process(&mut self, input: &[u8], output: &mut [u8]) -> (usize, usize) {
        let mut in_pos = 0;
        let mut out_pos = 0;

        while in_pos < input.len() && out_pos + self.width <= output.len() {
            let take = (self.width - self.buf_len).min(input.len() - in_pos);
            self.buffer[self.buf_len..self.buf_len + take]
                .copy_from_slice(&input[in_pos..in_pos + take]);
            self.buf_len += take;
            in_pos += take;

            if self.buf_len == self.width {
                let cur = Self::read_word(&self.buffer, self.width);
                if !self.initialized {
                    // first value – output raw
                    Self::write_word(cur, self.width, &mut output[out_pos..]);
                    out_pos += self.width;
                    self.prev = cur;
                    self.initialized = true;
                } else {
                    let diff = cur ^ self.prev;
                    Self::write_word(diff, self.width, &mut output[out_pos..]);
                    out_pos += self.width;
                    self.prev = cur;
                }
                self.buf_len = 0;
            }
        }
        (in_pos, out_pos)
    }

    fn flush(&mut self, output: &mut [u8]) -> usize {
        let len = self.buf_len.min(output.len());
        output[..len].copy_from_slice(&self.buffer[..len]);
        self.buf_len = 0;
        len
    }

    fn reverse_process(&mut self, input: &[u8], output: &mut [u8]) -> (usize, usize) {
        let mut in_pos = 0;
        let mut out_pos = 0;

        while in_pos < input.len() && out_pos + self.width <= output.len() {
            let take = (self.width - self.buf_len).min(input.len() - in_pos);
            self.buffer[self.buf_len..self.buf_len + take]
                .copy_from_slice(&input[in_pos..in_pos + take]);
            self.buf_len += take;
            in_pos += take;

            if self.buf_len == self.width {
                let val = Self::read_word(&self.buffer, self.width);
                if !self.initialized {
                    Self::write_word(val, self.width, &mut output[out_pos..]);
                    out_pos += self.width;
                    self.prev = val;
                    self.initialized = true;
                } else {
                    let original = val ^ self.prev;
                    Self::write_word(original, self.width, &mut output[out_pos..]);
                    out_pos += self.width;
                    self.prev = original;
                }
                self.buf_len = 0;
            }
        }
        (in_pos, out_pos)
    }

    fn reverse_flush(&mut self, output: &mut [u8]) -> usize {
        let len = self.buf_len.min(output.len());
        output[..len].copy_from_slice(&self.buffer[..len]);
        self.buf_len = 0;
        len
    }
}

// ---------- Text Preprocessor (placeholder, passthrough) ----------
pub struct TextPreprocessor;

impl TextPreprocessor {
    pub fn new() -> Self {
        Self
    }
}

impl Preprocessor for TextPreprocessor {
    fn process(&mut self, input: &[u8], output: &mut [u8]) -> (usize, usize) {
        let len = input.len().min(output.len());
        output[..len].copy_from_slice(&input[..len]);
        (len, len)
    }
    fn flush(&mut self, _output: &mut [u8]) -> usize {
        0
    }
    fn reverse_process(&mut self, input: &[u8], output: &mut [u8]) -> (usize, usize) {
        self.process(input, output)
    }
}
