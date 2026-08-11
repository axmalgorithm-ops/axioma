use crate::preprocessor::Preprocessor;
use crate::context::ContextModel;
use crate::range_coder::RangeEncoder;
use crate::error::Error;

pub struct Encoder<P: Preprocessor> {
    preprocessor: P,
    model: ContextModel,
}

impl<P: Preprocessor> Encoder<P> {
    pub fn new(preprocessor: P) -> Self {
        Self {
            preprocessor,
            model: ContextModel::new(),
        }
    }

    pub fn compress(
        &mut self,
        input: &[u8],
        write_output: &mut dyn FnMut(&[u8]) -> Result<(), Error>,
    ) -> Result<usize, Error> {
        let mut total_written = 0;
        let mut callback = |data: &[u8]| -> Result<(), Error> {
            total_written += data.len();
            write_output(data)
        };

        let mut encoder = RangeEncoder::new(&mut callback);
        let mut transformed = [0u8; 256];
        let mut in_pos = 0;

        while in_pos < input.len() {
            let (consumed, produced) = self.preprocessor.process(
                &input[in_pos..],
                &mut transformed,
            );
            if consumed == 0 {
                break;
            }
            in_pos += consumed;

            for &symbol in transformed[..produced].iter() {
                self.model.encode_symbol(symbol, |bit, prob| {
                    encoder.encode_bit(bit, prob)
                })?;
            }
        }

        let flushed = self.preprocessor.flush(&mut transformed);
        for &symbol in transformed[..flushed].iter() {
            self.model.encode_symbol(symbol, |bit, prob| {
                encoder.encode_bit(bit, prob)
            })?;
        }

        encoder.finish()?;
        Ok(total_written)
    }
}
