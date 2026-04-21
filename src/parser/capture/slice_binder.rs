use crate::{
    error::{FurthestFailError, error_handler::ErrorHandler},
    input::{InputStream, SliceableInput},
    matcher::{MatchRunner, Matcher, internal::MatcherImpl},
    parser::{BoundValue, Property},
};

pub struct SliceBinder<Match, Prop> {
    pub(super) matcher: Match,
    pub(super) property: Prop,
}

impl<Match, Prop> SliceBinder<Match, Prop> {
    /// See [`bind_slice`].
    pub fn new(matcher: Match, property: Prop) -> Self {
        Self { matcher, property }
    }
}

/// Convenience constructor for [`SpanBinder`].
pub fn bind_slice<Match, Prop>(matcher: Match, property: Prop) -> SliceBinder<Match, Prop> {
    SliceBinder::new(matcher, property)
}

impl<'src, Inp: SliceableInput<'src>, MRes, Match, Prop> MatcherImpl<'src, Inp, MRes>
    for SliceBinder<Match, Prop>
where
    Match: Matcher<'src, Inp, MRes>,
    Prop: Property<Inp::Slice, MRes> + Clone,
{
    const CAN_MATCH_DIRECTLY: bool = Match::CAN_MATCH_DIRECTLY;
    const HAS_PROPERTY: bool = true;
    const CAN_FAIL: bool = Match::CAN_FAIL;

    fn match_with_runner<'a, Runner>(
        &'a self,
        runner: &mut Runner,
        error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, Inp>,
    ) -> Result<bool, FurthestFailError>
    where
        Runner: MatchRunner<'a, 'src, Inp, MRes = MRes>,
        'src: 'a,
    {
        let start_pos = input.get_pos();
        if !runner.run_match(&self.matcher, error_handler, input)? {
            return Ok(false);
        }
        let end_pos = input.get_pos();
        let slice = input.slice(start_pos..end_pos);
        let bound: BoundValue<Inp::Slice, _> = self.property.bind_result(slice);
        runner.register_result(bound);
        Ok(true)
    }
}
