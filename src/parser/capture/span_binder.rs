use crate::{
    error::{FurthestFailError, error_handler::ErrorHandler},
    input::{Input, InputStream},
    matcher::{Matcher, internal::MatcherImpl, runner::MatchRunner},
};

use super::{bound::BoundValue, property::Property};

/// Runs `matcher`, then records the span `(start, end)` byte/char indices via `property`.
pub struct SpanBinder<Match, Prop> {
    pub(super) matcher: Match,
    pub(super) property: Prop,
}

impl<Match, Prop> SpanBinder<Match, Prop> {
    /// See [`bind_span`].
    pub fn new(matcher: Match, property: Prop) -> Self {
        Self { matcher, property }
    }
}

/// Convenience constructor for [`SpanBinder`].
pub fn bind_span<Match, Prop>(matcher: Match, property: Prop) -> SpanBinder<Match, Prop> {
    SpanBinder::new(matcher, property)
}

impl<'src, Inp: Input<'src>, MRes, Match, Prop> MatcherImpl<'src, Inp, MRes>
    for SpanBinder<Match, Prop>
where
    Match: Matcher<'src, Inp, MRes>,
    Inp: Input<'src>,
    Prop: Property<(Inp::Pos, Inp::Pos), MRes> + Clone + 'src,
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
        let bound: BoundValue<(Inp::Pos, Inp::Pos), _> =
            self.property.bind_result((start_pos, end_pos));
        runner.register_result(bound);
        Ok(true)
    }
}
