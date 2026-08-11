use crate::error::Error;
use alloc::vec::Vec;

pub struct RangeEncoder<'a> {
    low: u64,
    range: u32,
    buffer: Vec<u8>,
    output: &'a mut dyn FnMut(&[u8]) -> Result<(), Error>,
}

impl<'a> RangeEncoder<'a> {
    pub fn new(output: &'a mut dyn FnMut(&[u8]) -> Result<(), Error>) -> Self {
        Self {
            low: 0,
            range: 0xFFFF_FFFF,
            buffer: Vec::new(),
            output,
        }
    }

    fn emit_byte(&mut self, byte: u8, carry: bool) {
        if carry {
            let mut i = self.buffer.len();
            while i > 0 {
                i -= 1;
                if self.buffer[i] == 0xFF {
                    self.buffer[i] = 0x00;
                } else {
                    self.buffer[i] = self.buffer[i].wrapping_add(1);
                    break;
                }
            }
        }
        self.buffer.push(byte);
    }

    pub fn encode_bit(&mut self, bit: bool, prob: u8) -> Result<(), Error> {
        let split = ((self.range as u64 * prob as u64) >> 8) as u32;
        if bit {
            self.low += split as u64;
            self.range -= split;
        } else {
            self.range = split;
        }

        while self.range < 0x0100_0000 {
            let byte = (self.low >> 24) as u8;
            let carry = (self.low >> 32) > 0;
            self.emit_byte(byte, carry);
            self.low = (self.low & 0x00FF_FFFF) << 8;
            self.range <<= 8;
        }
        Ok(())
    }

    pub fn finish(&mut self) -> Result<(), Error> {
        for _ in 0..5 {
            let byte = (self.low >> 24) as u8;
            let carry = (self.low >> 32) > 0;
            self.emit_byte(byte, carry);
            self.low = (self.low & 0x00FF_FFFF) << 8;
        }
        (self.output)(&self.buffer)?;
        Ok(())
    }
}

pub struct RangeDecoder<'a> {
    range: u32,
    code: u32,
    input: &'a mut dyn FnMut(&mut [u8]) -> Result<usize, Error>,
}

impl<'a> RangeDecoder<'a> {
    pub fn new(input: &'a mut dyn FnMut(&mut [u8]) -> Result<usize, Error>) -> Result<Self, Error> {
        let mut dec = Self {
            range: 0xFFFF_FFFF,
            code: 0,
            input,
        };
        for _ in 0..4 {
            dec.code = (dec.code << 8) | (dec.read_byte()? as u32);
        }
        Ok(dec)
    }

    fn read_byte(&mut self) -> Result<u8, Error> {
        let mut buf = [0u8; 1];
        let n = (self.input)(&mut buf)?;
        if n == 0 {
            Ok(0)
        } else {
            Ok(buf[0])
        }
    }

    pub fn decode_bit(&mut self, prob: u8) -> Result<bool, Error> {
        let split = ((self.range as u64 * prob as u64) >> 8) as u32;
        let bit = self.code >= split;
        if bit {
            self.code -= split;
            self.range -= split;
        } else {
            self.range = split;
        }
        while self.range < 0x0100_0000 {
            self.range <<= 8;
            self.code = (self.code << 8) | (self.read_byte()? as u32);
        }
        Ok(bit)
    }
}
