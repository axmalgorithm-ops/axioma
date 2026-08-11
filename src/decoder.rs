use crate::preprocessor::Preprocessor;
use crate::context::ContextModel;
use crate::range_coder::RangeDecoder;
use crate::error::Error;

pub struct Decoder<P: Preprocessor> {
    preprocessor: P,
    model: ContextModel,
}

impl<P: Preprocessor> Decoder<P> {
    pub fn new(preprocessor: P) -> Self {
        Self {
            preprocessor,
            model: ContextModel::new(),
        }
    }

    pub fn decompress(
        &mut self,
        compressed: &[u8],
        output: &mut [u8],
    ) -> Result<usize, Error> {
        let mut read_pos = 0;
        let mut input_fn = |buf: &mut [u8]| -> Result<usize, Error> {
            if read_pos >= compressed.len() {
                return Ok(0);
            }
            let take = buf.len().min(compressed.len() - read_pos);
            buf[..take].copy_from_slice(&compressed[read_pos..read_pos + take]);
            read_pos += take;
            Ok(take)
        };

        let mut decoder = RangeDecoder::new(&mut input_fn)?;
        let mut raw_buf = [0u8; 256];
        let mut out_pos = 0;

        while out_pos < output.len() {
            let chunk_size = 256.min(output.len() - out_pos);
            for item in raw_buf[..chunk_size].iter_mut() {
                *item = self.model.decode_symbol(|prob| decoder.decode_bit(prob))?;
            }

            let mut chunk_pos = 0;
            while chunk_pos < chunk_size {
                let (consumed, produced) = self.preprocessor.reverse_process(
                    &raw_buf[chunk_pos..chunk_size],
                    &mut output[out_pos..],
                );
                if consumed == 0 && produced == 0 {
                    break;
                }
                chunk_pos += consumed;
                out_pos += produced;
            }
        }

        Ok(out_pos)
    }
}
