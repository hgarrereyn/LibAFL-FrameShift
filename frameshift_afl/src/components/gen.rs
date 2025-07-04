use libafl::{inputs::BytesInput, prelude::Generator};

use super::structured_input::StructuredInput;


pub struct GrammarGenerator;

impl<S> Generator<StructuredInput,S> for GrammarGenerator {
    fn generate(&mut self, _state: &mut S) -> Result<StructuredInput, libafl::Error> {
        Ok(StructuredInput::new_raw(b"aaaaaaaa"))
    }
}


pub struct BytesGenerator;

impl<S> Generator<BytesInput,S> for BytesGenerator {
    fn generate(&mut self, _state: &mut S) -> Result<BytesInput, libafl::Error> {
        Ok(BytesInput::new(b"aaaaaaaa".to_vec()))
    }
}
