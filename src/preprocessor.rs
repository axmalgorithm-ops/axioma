use crate::error::Error;
pub trait Preprocessor {
    fn process(&mut self, input: &[u8], output: &mut [u8]) -> (usize, usize);
    fn flush(&mut self, output: &mut [u8]) -> usize;
    fn reverse_process(&mut self, input: &[u8], output: &mut [u8]) -> (usize, usize);
}

pub struct DeltaPreprocessor { last_byte: u8 }
impl DeltaPreprocessor { pub fn new() -> Self { Self { last_byte: 0 } } }
impl Default for DeltaPreprocessor { fn default() -> Self { Self::new() } }

impl Preprocessor for DeltaPreprocessor {
    fn process(&mut self, input: &[u8], output: &mut [u8]) -> (usize, usize) {
        let len = input.len().min(output.len());
        for i in 0..len {
            let current = input[i];
            output[i] = current.wrapping_sub(self.last_byte);
            self.last_byte = current;
        }
        (len, len)
    }
    fn flush(&mut self, _output: &mut [u8]) -> usize { 0 }
    fn reverse_process(&mut self, input: &[u8], output: &mut [u8]) -> (usize, usize) {
        let len = input.len().min(output.len());
        for i in 0..len {
            let delta = input[i];
            let current = delta.wrapping_add(self.last_byte);
            output[i] = current;
            self.last_byte = current;
        }
        (len, len)
    }
}
