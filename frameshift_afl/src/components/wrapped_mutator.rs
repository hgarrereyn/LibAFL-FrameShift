use std::{borrow::Cow, marker::PhantomData, rc::Rc};

use libafl::{prelude::{MutationResult, Mutator}, state::{HasRand, State, UsesState}};
use libafl_bolts::{rands::Rand, Named};

use super::structured_input::StructuredInput;


pub struct WrappedMutator<M,S> {
    mutator: M,
    name: Cow<'static, str>,
    _state: PhantomData<S>,
}

impl<M,S> WrappedMutator<M,S>
where 
    M: Named
{
    pub fn new(mutator: M) -> Self {
        Self {
            name: Cow::from(format!("wrapped<{}>", mutator.name())),
            mutator,
            _state: PhantomData,
        }
    }
}

impl<M,S> Named for WrappedMutator<M,S>
where
    M: Named
{
    fn name(&self) -> &Cow<'static, str> {
        &self.name
    }
}

impl<M,S> UsesState for WrappedMutator<M,S>
where 
    S: State
{
    type State = S;
}

impl<M,S> Mutator<StructuredInput, S> for WrappedMutator<M,S>
where 
    M: Mutator<StructuredInput, S>,
    S: HasRand
{
    fn mutate(&mut self, state: &mut S, input: &mut StructuredInput) -> Result<MutationResult, libafl::Error> {

        let seed = state.rand_mut().next();
        input.set_seed(seed);

        let res = self.mutator.mutate(state, input)?;
        if res == MutationResult::Skipped {
            return Ok(res);
        }

        input.input.sanitize();

        Ok(res)
    }
}
