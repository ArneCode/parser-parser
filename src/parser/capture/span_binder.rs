use crate::{
    error::{FurthestFailError, error_handler::ErrorHandler},
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

impl<Token, MRes, Match, Prop> MatcherImpl<Token, MRes> for SpanBinder<Match, Prop>
where
    Match: Matcher<Token, MRes>,
    Prop: Property<(usize, usize), MRes> + Clone,
{
    const CAN_MATCH_DIRECTLY: bool = Match::CAN_MATCH_DIRECTLY;
    const HAS_PROPERTY: bool = true;
    const CAN_FAIL: bool = Match::CAN_FAIL;

    fn match_with_runner<'a, 'ctx, Runner>(
        &'a self,
        runner: &mut Runner,
        error_handler: &mut impl ErrorHandler,
        pos: &mut usize,
    ) -> Result<bool, FurthestFailError>
    where
        Runner: MatchRunner<'a, 'ctx, Token = Token, MRes = MRes>,
        'ctx: 'a,
    {
        let start_pos = *pos;
        if !runner.run_match(&self.matcher, error_handler, pos)? {
            return Ok(false);
        }
        let end_pos = *pos;
        let bound: BoundValue<(usize, usize), _> = self.property.bind_result((start_pos, end_pos));
        runner.register_result(bound);
        Ok(true)
    }
}
