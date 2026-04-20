use std::marker::PhantomData;
use std::panic::Location;

use crate::{
    error::{FurthestFailError, error_handler::ErrorHandler},
    input::{InputFamily, InputStream},
    matcher::{internal::MatcherImpl, runner::MatchRunner},
    parser::Parser,
};

use super::property::{BindDebugInfo, Property};

/// [`crate::matcher::Matcher`] that runs parser `Pars` and stores its output with `Prop`.
pub struct ResultBinder<Pars, Prop, InpFam> where InpFam: InputFamily + ?Sized {
    pub(super) parser: Pars,
    pub(super) property: Prop,
    pub(super) debug: Option<BindDebugInfo>,
    pub(super) _phantom: PhantomData<InpFam>,
}

impl<Pars, Prop, InpFam> ResultBinder<Pars, Prop, InpFam>  where InpFam: InputFamily + ?Sized {
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
pub fn bind_result<Pars, Prop, InpFam>(
    parser: Pars,
    property: Prop,
) -> ResultBinder<Pars, Prop, InpFam> where InpFam: InputFamily + ?Sized {
    bind_result_with_unknown_debug(parser, property)
}

/// Like [`bind_result`] but attaches file/line/column from the caller for panic messages.
#[track_caller]
pub fn bind_result_with_unknown_debug<Pars, Prop, InpFam>(
    parser: Pars,
    property: Prop,
) -> ResultBinder<Pars, Prop, InpFam>  where InpFam: InputFamily + ?Sized {
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
pub fn bind_result_with_debug<Pars, Prop, InpFam>(
    parser: Pars,
    property: Prop,
    debug: BindDebugInfo,
) -> ResultBinder<Pars, Prop, InpFam> where InpFam: InputFamily + ?Sized {
    ResultBinder::new(parser, property, Some(debug))
}

impl<InpFam, Pars, Prop, MRes> MatcherImpl<InpFam, MRes>
    for ResultBinder<Pars, Prop, InpFam>
where
    InpFam: InputFamily + ?Sized,
    Pars: Parser<InpFam>,
    Prop: for<'src> Property<Pars::Output<'src>, MRes> + Clone,
{
    const CAN_MATCH_DIRECTLY: bool = true;
    const HAS_PROPERTY: bool = true;
    const CAN_FAIL: bool = Pars::CAN_FAIL;

    fn match_with_runner<'a, 'src, Runner>(
        &'a self,
        runner: &mut Runner,
        error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, InpFam::In<'src>>,
    ) -> Result<bool, FurthestFailError>
    where
        Runner: MatchRunner<'a, 'src, InpFam, MRes = MRes>,
        'src: 'a,
    {
        if let Some(result) = self
            .parser
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
