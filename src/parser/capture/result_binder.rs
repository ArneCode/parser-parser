use std::marker::PhantomData;
use std::panic::Location;

use crate::{
    error::{FurthestFailError, error_handler::ErrorHandler},
    matcher::{internal::MatcherImpl, runner::MatchRunner},
    parser::Parser,
};

use super::property::{BindDebugInfo, Property};

pub struct ResultBinder<Pars, Prop, Token> {
    pub(super) parser: Pars,
    pub(super) property: Prop,
    pub(super) debug: Option<BindDebugInfo>,
    pub(super) _phantom: PhantomData<Token>,
}

impl<Pars, Prop, Token> ResultBinder<Pars, Prop, Token> {
    pub fn new(parser: Pars, property: Prop, debug: Option<BindDebugInfo>) -> Self {
        Self {
            parser,
            property,
            debug,
            _phantom: PhantomData,
        }
    }
}

pub fn bind_result<Pars, Prop, Token>(
    parser: Pars,
    property: Prop,
) -> ResultBinder<Pars, Prop, Token> {
    bind_result_with_unknown_debug(parser, property)
}

#[track_caller]
pub fn bind_result_with_unknown_debug<Pars, Prop, Token>(
    parser: Pars,
    property: Prop,
) -> ResultBinder<Pars, Prop, Token> {
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

pub fn bind_result_with_debug<Pars, Prop, Token>(
    parser: Pars,
    property: Prop,
    debug: BindDebugInfo,
) -> ResultBinder<Pars, Prop, Token> {
    ResultBinder::new(parser, property, Some(debug))
}

impl<Pars, Prop, Token, MRes> MatcherImpl<Token, MRes> for ResultBinder<Pars, Prop, Token>
where
    Pars: Parser<Token>,
    Prop: Property<Pars::Output, MRes> + Clone,
{
    const CAN_MATCH_DIRECTLY: bool = true;
    const HAS_PROPERTY: bool = true;
    const CAN_FAIL: bool = Pars::CAN_FAIL;

    fn match_with_runner<'a, 'ctx, Runner>(
        &'a self,
        runner: &mut Runner,
        error_handler: &mut impl ErrorHandler,
        pos: &mut usize,
    ) -> Result<bool, FurthestFailError>
    where
        Runner: MatchRunner<'a, 'ctx, Token = Token, MRes = MRes>,
        'ctx: 'a,
        Token: 'ctx,
    {
        if let Some(result) = self
            .parser
            .parse(runner.get_parser_context(), error_handler, pos)?
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
