use std::{borrow::Cow, collections::HashSet, marker::PhantomData};

use libafl::{corpus::Corpus, events::{Event, EventFirer}, inputs::UsesInput, prelude::{AggregatorOps, Executor, HasObservers, MapObserver, ObserversTuple, UserStats, UserStatsValue}, stages::Stage, state::{HasCorpus, State, UsesState}, Error, HasMetadata};
use libafl_bolts::{prelude::OwnedSlice, tuples::{Handle, Handled}, AsIter, AsSlice, ErrorBacktrace, Named};

use crate::core::search::{SearchContext, SearchOptions};

use super::{search_metadata::SearchMetadata, structured_input::{InputStatus, StructuredInput}};


#[derive(Clone, Debug)]
pub struct SearchStageArgs {
    pub options: SearchOptions
}

pub struct SearchStage<S,C,O> {
    pub map_handle: Handle<C>,
    pub args: SearchStageArgs,
    _phantom: PhantomData<(S,O)>,
}

impl<S,C,O> SearchStage<S,C,O>
where
    S: State + UsesInput<Input = StructuredInput>,
    O: MapObserver + for<'it> AsIter<'it, Item = u8> + for<'it> AsSlice<'it, SliceRef = &'it [u8]>,
    C: Named + AsMut<O> + AsRef<O>
{
    pub fn new(observer: &C, args: SearchStageArgs) -> Self {
        Self {
            map_handle: observer.handle(),
            args,
            _phantom: PhantomData,
        }
    }

    pub fn get_coverage_slice<'a, E,EM,Z,OT>(&self, fuzzer: &mut Z, executor: &mut E, state: &mut S, mgr: &mut EM, input: &[u8]) -> &'a [u8]
    where
        E: Executor<EM,Z,State = S> + HasObservers<Observers = OT>,
        Z: UsesState<State = E::State>,
        EM: UsesState<State = E::State>,
        OT: ObserversTuple<E::State>
    {
        {
            let mut ot = executor.observers_mut();
            let obs = ot[&self.map_handle].as_mut();
            obs.reset_map().unwrap();
        }
        let _exit_kind = executor.run_target(fuzzer, state, mgr, &StructuredInput::new_raw(input));
        let ot = executor.observers();
        let obs = ot[&self.map_handle].as_ref();

        // Convert to static lifetime - this is unsafe but needed for the oracle
        let slice = obs.as_slice();
        unsafe { std::mem::transmute::<&[u8], &'a [u8]>(slice) }
    }
}

impl<S,C,O> Named for SearchStage<S,C,O> {
    fn name(&self) -> &Cow<'static, str> {
        &Cow::Borrowed("SearchStage")
    }
}

impl<S,C,O> UsesState for SearchStage<S,C,O>
where
    S: State
{
    type State = S;
}

impl<S,C,O,E,EM,Z> Stage<E,EM,Z> for SearchStage<S,C,O> 
where
    S: State + HasCorpus + HasMetadata + UsesInput<Input = StructuredInput>,
    C: Named + AsMut<O> + AsRef<O>,
    O: MapObserver + for<'it> AsIter<'it, Item = u8> + for<'it> AsSlice<'it, SliceRef = &'it [u8]>,
    E: Executor<EM,Z> + UsesState<State = S> + HasObservers,
    Z: UsesState<State = S>,
    EM: UsesState<State = S> + EventFirer
{
    fn restart_progress_should_run(&mut self, _state: &mut Self::State) -> Result<bool, libafl::Error> {
        Ok(true)
    }

    fn clear_restart_progress(&mut self, _state: &mut Self::State) -> Result<(), libafl::Error> {
        Ok(())
    }

    fn perform(
        &mut self,
        fuzzer: &mut Z,
        executor: &mut E,
        state: &mut Self::State,
        manager: &mut EM,
    ) -> Result<(), libafl::Error> {
        let corpus_idx = state.corpus().current().ok_or(Error::Empty("missing current".to_string(), ErrorBacktrace {}))?;

        // Fetch the testcase
        let input = state.corpus().get(corpus_idx).unwrap().borrow();
        let inner = input.input().as_ref().unwrap();

        let will_search = match inner.status {
            // If the input is marked as searched, we only need to search if it has been mutated since the last search.
            InputStatus::Searched(id) => id != corpus_idx,

            // If the input is new or mutated, we always search it.
            InputStatus::New | InputStatus::Mutated => true,

            // If the input is in progress, it crashed during the last search, so we skip it.
            InputStatus::InProgress => false,
        };

        if !will_search {
            return Ok(());
        }

        drop(input);

        // Otherwise, we need to search this input. Mark as in progress and perform the search.
        let mut input = state.corpus().get(corpus_idx).unwrap().borrow().clone();
        input.input_mut().as_mut().unwrap().status = InputStatus::InProgress;

        let testcase = input.input().as_ref().unwrap().input.clone();
        state.corpus_mut().replace(corpus_idx, input)?;

        // Set up the oracle
        let mut oracle = |input: &[u8]| {
            self.get_coverage_slice(fuzzer, executor, state, manager, input)
        };

        let res = SearchContext::search(&testcase, &mut oracle, self.args.options.clone());

        if self.args.options.verbose {
            println!("{:?}", res.input);
        }

        // Update the testcase with the new grammar
        {
            let mut other = state.corpus().get(corpus_idx).unwrap().borrow().clone();
            other.input_mut().as_mut().unwrap().input = res.input.clone();
            other.input_mut().as_mut().unwrap().status = InputStatus::Searched(corpus_idx);
            
            state.corpus_mut().replace(corpus_idx, other)?;

            println!("  ({}) [searched]", corpus_idx);
        }

        // Ensure we have the state metadata
        if !state.has_metadata::<SearchMetadata>() {
            state.add_metadata(SearchMetadata::new());
        }

        // Update metadata
        let (num_searched, num_found, search_tests, target_time_ms, total_time_ms) = {
            let metadata = state.metadata_mut::<SearchMetadata>().unwrap();
            metadata.num_searched += 1;
            metadata.num_found += if res.found_any {1} else {0};
            metadata.search_tests += res.test_count;
            metadata.target_time_ms += res.target_test_ms;
            metadata.total_time_ms += res.total_test_ms;
            (metadata.num_searched, metadata.num_found, metadata.search_tests, metadata.target_time_ms, metadata.total_time_ms)
        };

        // Update stats
        manager.fire(state, Event::UpdateUserStats {
            name: Cow::Borrowed("searched"),
            value: UserStats::new(UserStatsValue::Number(num_searched as u64), AggregatorOps::None),
            phantom: PhantomData,
        })?;

        manager.fire(state, Event::UpdateUserStats {
            name: Cow::Borrowed("found"),
            value: UserStats::new(UserStatsValue::Ratio(num_found as u64, num_searched as u64), AggregatorOps::None),
            phantom: PhantomData,
        })?;

        manager.fire(state, Event::UpdateUserStats {
            name: Cow::Borrowed("search_tests"),
            value: UserStats::new(UserStatsValue::Number(search_tests as u64), AggregatorOps::None),
            phantom: PhantomData,
        })?;

        manager.fire(state, Event::UpdateUserStats {
            name: Cow::Borrowed("target_time_ms"),
            value: UserStats::new(UserStatsValue::Number(target_time_ms as u64), AggregatorOps::None),
            phantom: PhantomData,
        })?;

        manager.fire(state, Event::UpdateUserStats {
            name: Cow::Borrowed("total_time_ms"),
            value: UserStats::new(UserStatsValue::Number(total_time_ms as u64), AggregatorOps::None),
            phantom: PhantomData,
        })?;

        Ok(())
    }

    // fn perform(
    //     &mut self,
    //     fuzzer: &mut Z,
    //     executor: &mut E,
    //     state: &mut Self::State,
    //     manager: &mut EM,
    // ) -> Result<(), libafl::Error> {
    //     let corpus_idx = state.corpus().current().ok_or(Error::Empty("missing current".to_string(), ErrorBacktrace {}))?;

    //     // Ensure we have the state metadata
    //     if !state.has_metadata::<SearchMetadata>() {
    //         state.add_metadata(SearchMetadata::new());
    //     }

    //     // Update metadata on new inputs
    //     {
    //         let input = state.corpus().get(corpus_idx).unwrap().borrow();
    //         let inner = input.input().as_ref().unwrap();

    //         let mut update = None;

    //         match inner.status {
    //             InputStatus::Searched(id) => {
    //                 if id == corpus_idx {
    //                     // We've already searched this input.
    //                     return Ok(())
    //                 } else {
    //                     // This is a newly derived input.
    //                     let mut other = input.clone();
    //                     other.input_mut().as_mut().unwrap().status = InputStatus::Mutated;
    //                     update = Some(other);

    //                     println!("({}) [mut] from={:?}", corpus_idx, input.parent_id());
    //                 }
    //             }
    //             _ => {}
    //         }

    //         drop(input);

    //         if let Some(other) = update {
    //             state.corpus_mut().replace(corpus_idx, other)?;
    //         }
    //     }

    //     // Check if we should search this input
    //     let raw = {
    //         let input = state.corpus().get(corpus_idx).unwrap().borrow();
    //         let inner = input.input().as_ref().unwrap();

    //         let mut update = None;

    //         match inner.status {
    //             InputStatus::New | InputStatus::Mutated => {
    //                 // always search
    //                 println!("({}) searching [new]", corpus_idx);

    //                 // Mark as in progress
    //                 let mut other = input.clone();
    //                 other.input_mut().as_mut().unwrap().status = InputStatus::InProgress;
    //                 update = Some(other);
    //             },
    //             _ => {
    //                 return Ok(())
    //             }
    //         }

    //         let raw = inner.input.get_raw().to_vec();

    //         drop(input);

    //         if let Some(other) = update {
    //             state.corpus_mut().replace(corpus_idx, other)?;
    //         }

    //         raw
    //     };

    //     let mut oracle = |input: &[u8], mask: &[usize]| {
    //         self.get_coverage_masked(fuzzer, executor, state, manager, input, mask)
    //     };

    //     let res = SearchContext::search(&raw, &mut oracle, self.args.options.clone());

    //     if self.args.options.verbose {
    //         println!("{:?}", res.input);
    //     }

    //     // Update the testcase with the new grammar
    //     {
    //         let mut other = state.corpus().get(corpus_idx).unwrap().borrow().clone();
    //         other.input_mut().as_mut().unwrap().input = res.input.clone();
    //         other.input_mut().as_mut().unwrap().status = InputStatus::Searched(corpus_idx);
            
    //         state.corpus_mut().replace(corpus_idx, other)?;

    //         println!("  ({}) [searched]", corpus_idx);
    //     }

    //     // Update metadata
    //     let (num_searched, num_found, search_tests, target_time_ms, total_time_ms) = {
    //         let metadata = state.metadata_mut::<SearchMetadata>().unwrap();
    //         metadata.num_searched += 1;
    //         metadata.num_found += if res.found_any {1} else {0};
    //         metadata.search_tests += res.test_count;
    //         metadata.target_time_ms += res.target_test_ms;
    //         metadata.total_time_ms += res.total_test_ms;
    //         (metadata.num_searched, metadata.num_found, metadata.search_tests, metadata.target_time_ms, metadata.total_time_ms)
    //     };

    //     // Update stats
    //     manager.fire(state, Event::UpdateUserStats {
    //         name: Cow::Borrowed("searched"),
    //         value: UserStats::new(UserStatsValue::Number(num_searched as u64), AggregatorOps::None),
    //         phantom: PhantomData,
    //     })?;

    //     manager.fire(state, Event::UpdateUserStats {
    //         name: Cow::Borrowed("found"),
    //         value: UserStats::new(UserStatsValue::Ratio(num_found as u64, num_searched as u64), AggregatorOps::None),
    //         phantom: PhantomData,
    //     })?;

    //     manager.fire(state, Event::UpdateUserStats {
    //         name: Cow::Borrowed("search_tests"),
    //         value: UserStats::new(UserStatsValue::Number(search_tests as u64), AggregatorOps::None),
    //         phantom: PhantomData,
    //     })?;

    //     manager.fire(state, Event::UpdateUserStats {
    //         name: Cow::Borrowed("target_time_ms"),
    //         value: UserStats::new(UserStatsValue::Number(target_time_ms as u64), AggregatorOps::None),
    //         phantom: PhantomData,
    //     })?;

    //     manager.fire(state, Event::UpdateUserStats {
    //         name: Cow::Borrowed("total_time_ms"),
    //         value: UserStats::new(UserStatsValue::Number(total_time_ms as u64), AggregatorOps::None),
    //         phantom: PhantomData,
    //     })?;

    //     Ok(())
    // }
}
