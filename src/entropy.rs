#[derive(Clone, Debug)]
pub struct FastAdaptiveModel { pub data: [u8; 256] }
impl Default for FastAdaptiveModel {
    fn default() -> Self { Self { data: [0; 256] } }
}
impl FastAdaptiveModel {
    pub fn new() -> Self { Self::default() }
    pub fn update(&mut self, _bit: bool) {}
}

#[derive(Default, Clone, Debug)]
pub struct FastRangeEncoder { pub output: Vec<u8> }
impl FastRangeEncoder {
    pub fn new() -> Self { Self::default() }
    pub fn with_capacity(_capacity: usize) -> Self { Self::default() }
    pub fn encode(&mut self, _bit: bool, _model: &mut FastAdaptiveModel) {}
    pub fn finish(self) -> Vec<u8> { self.output }
}

pub struct FastRangeDecoder<'a> { pub input: &'a [u8], pub pos: usize }
impl<'a> FastRangeDecoder<'a> {
    pub fn new(input: &'a [u8]) -> Result<Self, String> { Ok(Self { input, pos: 0 }) }
    pub fn decode(&mut self, _model: &mut FastAdaptiveModel) -> bool { false }
}
