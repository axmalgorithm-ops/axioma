// ---------- Trait ----------

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
    fn flush(&mut self, _output: &mut [u8]) -> usize { 0 }
    fn reverse_process(&mut self, input: &[u8], output: &mut [u8]) -> (usize, usize) {
        self.process(input, output)
    }
}

// ---------- Delta ----------

pub struct DeltaPreprocessor {
    width: usize,
    buffer: [u8; 8],
    buf_len: usize,
    prev: u64,
    initialized: bool,
}

impl DeltaPreprocessor {
    pub fn new(width: usize) -> Self {
        assert!(width == 1 || width == 2 || width == 4 || width == 8);
        Self { width, buffer: [0; 8], buf_len: 0, prev: 0, initialized: false }
    }
    fn read_word(buf: &[u8], width: usize) -> u64 {
        let mut val = 0u64;
        for (i, &b) in buf[..width].iter().enumerate() {
            val |= (b as u64) << (i * 8);
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
        self.flush(output)
    }
}

// ---------- Text (passthrough) ----------

pub struct TextPreprocessor;

impl TextPreprocessor {
    pub fn new() -> Self { Self }
}

impl Preprocessor for TextPreprocessor {
    fn process(&mut self, input: &[u8], output: &mut [u8]) -> (usize, usize) {
        let len = input.len().min(output.len());
        output[..len].copy_from_slice(&input[..len]);
        (len, len)
    }
    fn flush(&mut self, _output: &mut [u8]) -> usize { 0 }
    fn reverse_process(&mut self, input: &[u8], output: &mut [u8]) -> (usize, usize) {
        self.process(input, output)
    }
}

// ========== Unified LZ77 Preprocessor (encode + decode) ==========

const WINDOW_SIZE: usize = 32768;
const MIN_MATCH: usize = 4;
const MAX_MATCH: usize = 258;

pub struct Lz77Preprocessor {
    window: Vec<u8>,
    pos: usize,
    lookahead: Vec<u8>,
    lookahead_pos: usize,
    cmdbuf: Vec<u8>,
    dec_state: DecodeState,
}

enum DecodeState {
    NeedCommand,
    MatchCopy { dist: usize, len: usize, copied: usize },
}

impl Lz77Preprocessor {
    pub fn new() -> Self {
        Self {
            window: vec![0u8; WINDOW_SIZE],
            pos: 0,
            lookahead: Vec::new(),
            lookahead_pos: 0,
            cmdbuf: Vec::new(),
            dec_state: DecodeState::NeedCommand,
        }
    }

    fn push_raw(&mut self, b: u8) {
        let idx = self.pos & (WINDOW_SIZE - 1);
        self.window[idx] = b;
        self.pos += 1;
    }

    fn find_match(&self, data: &[u8]) -> (usize, usize) {
        if data.len() < MIN_MATCH {
            return (0, 0);
        }
        let search_start = self.pos.saturating_sub(WINDOW_SIZE);
        let mut best_len = 0;
        let mut best_dist = 0;
        for start in search_start..self.pos {
            let mut len = 0;
            while len < data.len() && len < MAX_MATCH
                && start + len < self.pos
                && self.window[(start + len) & (WINDOW_SIZE - 1)] == data[len]
            {
                len += 1;
            }
            if len > best_len {
                best_len = len;
                best_dist = self.pos - start;
                if best_len >= MAX_MATCH {
                    break;
                }
            }
        }
        (best_len, best_dist)
    }

    fn encode_command(&mut self, output: &mut [u8]) -> (usize, bool) {
        if self.lookahead_pos >= self.lookahead.len() {
            return (0, true);
        }
        let lookahead_len = self.lookahead.len();
        if self.lookahead_pos + MIN_MATCH <= lookahead_len {
            let remaining = self.lookahead[self.lookahead_pos..].to_vec();
            let (match_len, dist) = self.find_match(&remaining);
            if match_len >= MIN_MATCH {
                let need = 5;
                if output.len() < need {
                    return (0, false);
                }
                output[0] = 0xFF;
                output[1..3].copy_from_slice(&(match_len as u16).to_le_bytes());
                output[3..5].copy_from_slice(&(dist as u16).to_le_bytes());
                for &b in &remaining[..match_len] {
                    self.push_raw(b);
                }
                self.lookahead_pos += match_len;
                return (need, false);
            }
        }

        let mut lit_len = 0;
        while self.lookahead_pos + lit_len < lookahead_len && lit_len < 254 {
            let pos = self.lookahead_pos + lit_len;
            if pos + MIN_MATCH <= lookahead_len {
                let test = self.lookahead[pos..].to_vec();
                let (ml, _) = self.find_match(&test);
                if ml >= MIN_MATCH {
                    break;
                }
            }
            lit_len += 1;
        }
        if lit_len == 0 {
            lit_len = 1;
        }
        let need = 1 + lit_len;
        if output.len() < need {
            return (0, false);
        }

        let lit_bytes = self.lookahead[self.lookahead_pos..self.lookahead_pos + lit_len].to_vec();
        output[0] = lit_len as u8;
        output[1..1 + lit_len].copy_from_slice(&lit_bytes);
        
        for &b in &lit_bytes {
            self.push_raw(b);
        }
        self.lookahead_pos += lit_len;
        (need, false)
    }
}

impl Preprocessor for Lz77Preprocessor {
    fn process(&mut self, input: &[u8], output: &mut [u8]) -> (usize, usize) {
        self.lookahead.extend_from_slice(input);
        let mut out_pos = 0;
        loop {
            let (produced, finished) = self.encode_command(&mut output[out_pos..]);
            out_pos += produced;
            if finished || out_pos == output.len() {
                break;
            }
        }
        self.lookahead.drain(..self.lookahead_pos);
        self.lookahead_pos = 0;
        (input.len(), out_pos)
    }

    fn flush(&mut self, output: &mut [u8]) -> usize {
        self.lookahead_pos = 0;
        let mut out_pos = 0;
        loop {
            let (produced, finished) = self.encode_command(&mut output[out_pos..]);
            out_pos += produced;
            if finished || out_pos == output.len() {
                break;
            }
        }
        self.lookahead.clear();
        self.lookahead_pos = 0;
        out_pos
    }

    fn reverse_process(&mut self, input: &[u8], output: &mut [u8]) -> (usize, usize) {
        self.cmdbuf.extend_from_slice(input);
        let mut out_pos = 0;
        loop {
            match self.dec_state {
                DecodeState::NeedCommand => {
                    if self.cmdbuf.is_empty() {
                        break;
                    }
                    let cmd = self.cmdbuf[0];
                    if cmd == 0xFF {
                        if self.cmdbuf.len() < 5 {
                            break;
                        }
                        let len = u16::from_le_bytes([self.cmdbuf[1], self.cmdbuf[2]]) as usize;
                        let dist = u16::from_le_bytes([self.cmdbuf[3], self.cmdbuf[4]]) as usize;
                        self.cmdbuf.drain(..5);
                        self.dec_state = DecodeState::MatchCopy { dist, len, copied: 0 };
                    } else {
                        let lit_len = cmd as usize;
                        if self.cmdbuf.len() < 1 + lit_len {
                            break;
                        }
                        let lit_bytes = self.cmdbuf[1..1 + lit_len].to_vec();
                        self.cmdbuf.drain(..1 + lit_len);
                        for &b in &lit_bytes {
                            if out_pos >= output.len() {
                                let mut undo = vec![cmd];
                                undo.extend_from_slice(&lit_bytes);
                                for byte in undo.into_iter().rev() {
                                    self.cmdbuf.insert(0, byte);
                                }
                                self.dec_state = DecodeState::NeedCommand;
                                return (input.len(), out_pos);
                            }
                            output[out_pos] = b;
                            out_pos += 1;
                            self.push_raw(b);
                        }
                        self.dec_state = DecodeState::NeedCommand;
                    }
                }
                DecodeState::MatchCopy { dist, len, copied } => {
                    let remaining = len - copied;
                    let to_copy = remaining.min(output.len() - out_pos);
                    for _ in 0..to_copy {
                        let idx = (self.pos - dist) & (WINDOW_SIZE - 1);
                        let b = self.window[idx];
                        output[out_pos] = b;
                        out_pos += 1;
                        self.push_raw(b);
                    }
                    let new_copied = copied + to_copy;
                    if new_copied == len {
                        self.dec_state = DecodeState::NeedCommand;
                    } else {
                        self.dec_state = DecodeState::MatchCopy { dist, len, copied: new_copied };
                    }
                    if out_pos == output.len() {
                        break;
                    }
                }
            }
        }
        (input.len(), out_pos)
    }

    fn reverse_flush(&mut self, output: &mut [u8]) -> usize {
        let mut out_pos = 0;
        loop {
            match self.dec_state {
                DecodeState::NeedCommand => {
                    if self.cmdbuf.is_empty() {
                        break;
                    }
                    let cmd = self.cmdbuf[0];
                    if cmd == 0xFF {
                        if self.cmdbuf.len() < 5 {
                            break;
                        }
                        let len = u16::from_le_bytes([self.cmdbuf[1], self.cmdbuf[2]]) as usize;
                        let dist = u16::from_le_bytes([self.cmdbuf[3], self.cmdbuf[4]]) as usize;
                        self.cmdbuf.drain(..5);
                        self.dec_state = DecodeState::MatchCopy { dist, len, copied: 0 };
                    } else {
                        let lit_len = cmd as usize;
                        if self.cmdbuf.len() < 1 + lit_len {
                            break;
                        }
                        let lit_bytes: Vec<u8> = self.cmdbuf[1..1 + lit_len].to_vec();
                        self.cmdbuf.drain(..1 + lit_len);
                        for &b in &lit_bytes {
                            if out_pos >= output.len() {
                                let mut undo = vec![cmd];
                                undo.extend_from_slice(&lit_bytes);
                                for byte in undo.into_iter().rev() {
                                    self.cmdbuf.insert(0, byte);
                                }
                                return out_pos;
                            }
                            output[out_pos] = b;
                            out_pos += 1;
                            self.push_raw(b);
                        }
                        self.dec_state = DecodeState::NeedCommand;
                    }
                }
                DecodeState::MatchCopy { dist, len, copied } => {
                    let remaining = len - copied;
                    let to_copy = remaining.min(output.len() - out_pos);
                    for _ in 0..to_copy {
                        let idx = (self.pos - dist) & (WINDOW_SIZE - 1);
                        let b = self.window[idx];
                        output[out_pos] = b;
                        out_pos += 1;
                        self.push_raw(b);
                    }
                    let new_copied = copied + to_copy;
                    if new_copied == len {
                        self.dec_state = DecodeState::NeedCommand;
                    } else {
                        self.dec_state = DecodeState::MatchCopy { dist, len, copied: new_copied };
                    }
                }
            }
            if out_pos == output.len() {
                break;
            }
        }
        out_pos
    }
}
