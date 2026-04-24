use std::marker::PhantomData;
use std::panic::Location;

use crate::{
    error::{FurthestFailError, error_handler::ErrorHandler},
    input::{Input, InputStream},
    matcher::{MatcherCombinator, internal::MatcherImpl, runner::MatchRunner},
    parser::{Parser, ParserCombinator},
};

use super::property::{BindDebugInfo, Property};

/// [`crate::matcher::Matcher`] that runs parser `Pars` and stores its output with `Prop`.
#[derive(Clone)]
pub struct ResultBinder<Pars, Prop, Inp> {
    pub(super) parser: Pars,
    pub(super) property: Prop,
    pub(super) debug: Option<BindDebugInfo>,
    pub(super) _phantom: PhantomData<Inp>,
}

impl<Pars, Prop, Inp> std::fmt::Debug for ResultBinder<Pars, Prop, Inp>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResultBinder")
            .finish()
    }
}

impl<Pars, Prop, Inp> MatcherCombinator for ResultBinder<Pars, Prop, Inp> where
    Pars: ParserCombinator
{
}

impl<Pars, Prop, Inp> ResultBinder<Pars, Prop, Inp> {
    /// Wraps `parser` and `property`; `debug` is passed through on bind (for duplicate detection).
    pub fn new(parser: Pars, property: Prop, debug: Option<BindDebugInfo>) -> Self {
        Self {
            parser,
            property,
            debug,
            _phantom: PhantomData,
        }
    }
}

/// Same as [`bind_result_with_unknown_debug`] (caller location is used for debug metadata).
pub fn bind_result<Pars, Prop, Inp>(parser: Pars, property: Prop) -> ResultBinder<Pars, Prop, Inp> {
    bind_result_with_unknown_debug(parser, property)
}

/// Like [`bind_result`] but attaches file/line/column from the caller for panic messages.
#[track_caller]
pub fn bind_result_with_unknown_debug<Pars, Prop, Inp>(
    parser: Pars,
    property: Prop,
) -> ResultBinder<Pars, Prop, Inp> {
    let location = Location::caller();
    ResultBinder::new(
        parser,
        property,
        Some(BindDebugInfo {
            property_name: "<unknown>",
            file: location.file(),
            line: location.line(),
            column: location.column(),
        }),
    )
}

/// Binds parse output to `property` with explicit [`BindDebugInfo`].
pub fn bind_result_with_debug<Pars, Prop, Inp>(
    parser: Pars,
    property: Prop,
    debug: BindDebugInfo,
) -> ResultBinder<Pars, Prop, Inp> {
    ResultBinder::new(parser, property, Some(debug))
}

impl<'src, Inp: Input<'src> + 'src, Pars, Prop, MRes> MatcherImpl<'src, Inp, MRes>
    for ResultBinder<Pars, Prop, Inp>
where
    Pars: Parser<'src, Inp> + 'src,
    Inp: Input<'src> + Clone,
    Prop: Property<Pars::Output, MRes> + Clone + 'src,
{
    const CAN_MATCH_DIRECTLY: bool = true;
    const HAS_PROPERTY: bool = true;
    const CAN_FAIL: bool = Pars::CAN_FAIL;

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
        if let Some(result) =
            self.parser
                .parse(runner.get_parser_context(), error_handler, input)?
        {
            let bound = if let Some(debug) = self.debug {
                self.property.bind_result_with_debug(result, debug)
            } else {
                self.property.bind_result(result)
            };
            runner.register_result(bound);
            Ok(true)
        } else {
            Ok(false)
        }
    }
}
