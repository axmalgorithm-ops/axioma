use crate::error::Error;
use alloc::boxed::Box;
use alloc::vec::Vec;

#[derive(Clone, Debug)]
pub struct BitTree {
    probs: [u8; 256],
}

impl BitTree {
    pub fn new() -> Self {
        Self { probs: [128u8; 256] }
    }

    pub fn encode_symbol<F>(&mut self, symbol: u8, mut encode_bit: F) -> Result<(), Error>
    where
        F: FnMut(bool, u8) -> Result<(), Error>,
    {
        let mut ctx = 0usize;
        for i in (0..8).rev() {
            let bit = (symbol >> i) & 1;
            let prob = self.probs[ctx];
            encode_bit(bit != 0, prob)?;
            self.probs[ctx] = update_probability(prob, bit != 0);
            ctx = (ctx << 1) + 1 + bit as usize;
        }
        Ok(())
    }

    pub fn decode_symbol<F>(&mut self, mut decode_bit: F) -> Result<u8, Error>
    where
        F: FnMut(u8) -> Result<bool, Error>,
    {
        let mut ctx = 0usize;
        let mut symbol = 0u8;
        for i in (0..8).rev() {
            let prob = self.probs[ctx];
            let bit = decode_bit(prob)?;
            if bit {
                symbol |= 1 << i;
            }
            self.probs[ctx] = update_probability(prob, bit);
            ctx = (ctx << 1) + 1 + bit as usize;
        }
        Ok(symbol)
    }
}

impl Default for BitTree {
    fn default() -> Self {
        Self::new()
    }
}

/// Order-1 context model.
/// Memory is allocated directly on the heap to prevent stack overflow.
pub struct ContextModel {
    trees: Box<[BitTree; 256]>,
    history: u8,
}

impl ContextModel {
    pub fn new() -> Self {
        let mut v: Vec<BitTree> = Vec::with_capacity(256);
        for _ in 0..256 {
            v.push(BitTree::new());
        }
        let trees: Box<[BitTree; 256]> = v
            .into_boxed_slice()
            .try_into()
            .expect("Vec length is exactly 256");
        Self {
            trees,
            history: 0,
        }
    }

    pub fn encode_symbol<F>(&mut self, symbol: u8, encode_bit: F) -> Result<(), Error>
    where
        F: FnMut(bool, u8) -> Result<(), Error>,
    {
        let ctx = self.history as usize;
        self.trees[ctx].encode_symbol(symbol, encode_bit)?;
        self.history = symbol;
        Ok(())
    }

    pub fn decode_symbol<F>(&mut self, decode_bit: F) -> Result<u8, Error>
    where
        F: FnMut(u8) -> Result<bool, Error>,
    {
        let ctx = self.history as usize;
        let symbol = self.trees[ctx].decode_symbol(decode_bit)?;
        self.history = symbol;
        Ok(symbol)
    }
}

impl Default for ContextModel {
    fn default() -> Self {
        Self::new()
    }
}

fn update_probability(prob: u8, bit: bool) -> u8 {
    if bit {
        prob.saturating_add(16).min(250)
    } else {
        prob.saturating_sub(16).max(5)
    }
}
