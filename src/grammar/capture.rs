use crate::grammar::{
    context::{
        MatchResult, MatchResultMultiple, MatchResultOptional, MatchResultSingle, ParserContext,
    },
    error_handler::{ErrorHandler, ParserError},
    matcher::{DirectMatchRunner, MatchRunner, Matcher, NoMemoizeBacktrackingRunner},
    parser::Parser,
    // span::Span,
};
use std::marker::PhantomData;

pub trait Property<Value, MatchResult> {
    fn put_in_result(&self, result: &mut MatchResult, value: Value);
    fn bind_result(&self, value: Value) -> (Value, Self)
    where
        Self: Sized,
        Self: Clone,
    {
        (value, self.clone())
    }
}

#[derive(Clone, Copy)]
pub struct SingleProperty<F> {
    setter: F,
}

impl<F> SingleProperty<F> {
    pub fn new(setter: F) -> Self {
        Self { setter }
    }
}

impl<V, MRes, F> Property<V, MRes> for SingleProperty<F>
where
    MRes: MatchResult,
    F: Fn(&mut MRes::Single) -> &mut Option<V>,
{
    fn put_in_result(&self, result: &mut MRes, value: V) {
        let property_slot = (self.setter)(result.single());
        if property_slot.is_some() {
            panic!("SingleProperty already set");
        }
        *property_slot = Some(value);
    }
}

#[derive(Clone, Copy)]
pub struct MultipleProperty<F> {
    setter: F,
}

impl<F> MultipleProperty<F> {
    pub fn new(setter: F) -> Self {
        Self { setter }
    }
}

impl<V, MRes, F> Property<V, MRes> for MultipleProperty<F>
where
    MRes: MatchResult,
    F: Fn(&mut MRes::Multiple) -> &mut Vec<V>,
{
    fn put_in_result(&self, result: &mut MRes, value: V) {
        let property_slot = (self.setter)(result.multiple());
        property_slot.push(value);
    }
}

#[derive(Clone, Copy)]
pub struct OptionalProperty<F> {
    setter: F,
}

impl<F> OptionalProperty<F> {
    pub fn new(setter: F) -> Self {
        Self { setter }
    }
}

impl<V, MRes, F> Property<V, MRes> for OptionalProperty<F>
where
    MRes: MatchResult,
    F: Fn(&mut MRes::Optional) -> &mut Option<V>,
{
    fn put_in_result(&self, result: &mut MRes, value: V) {
        let property_slot = (self.setter)(result.optional());
        if property_slot.is_some() {
            panic!("OptionalProperty already set");
        }
        *property_slot = Some(value);
    }
}

pub struct Capture<MRes, Match, F> {
    matcher: Match,
    constructor: F,
    _phantom: PhantomData<MRes>,
}

impl<Out, MResSingle, MResMultiple, MResOptional, Match, F>
    Capture<(MResSingle, MResMultiple, MResOptional), Match, F>
where
    MResSingle: MatchResultSingle,
    MResMultiple: MatchResultMultiple,
    MResOptional: MatchResultOptional,
    F: Fn(MResSingle::Output, MResMultiple, MResOptional) -> Out,
{
    pub fn new<
        'a,
        'ctx: 'a,
        GF: Fn(MResSingle::Properties, MResMultiple::Properties, MResOptional::Properties) -> Match,
        Token: 'ctx,
    >(
        grammar_factory: GF,
        constructor: F,
    ) -> Self
    where
        Match: Matcher<Token, (MResSingle, MResMultiple, MResOptional)>,
    {
        let properties_single = MResSingle::new_properties();
        let properties_multiple = MResMultiple::new_properties();
        let properties_optional = MResOptional::new_properties();
        Self {
            matcher: grammar_factory(properties_single, properties_multiple, properties_optional),
            constructor,
            _phantom: PhantomData,
        }
    }
}

impl<Token, Out, MResSingle, MResMultiple, MResOptional, Match, F> Parser<Token>
    for Capture<(MResSingle, MResMultiple, MResOptional), Match, F>
where
    MResSingle: MatchResultSingle,
    MResMultiple: MatchResultMultiple,
    MResOptional: MatchResultOptional,
    // Token: 'ctx,
    Match: Matcher<Token, (MResSingle, MResMultiple, MResOptional)>,
    F: Fn(MResSingle::Output, MResMultiple, MResOptional) -> Out,
{
    type Output = Out;
    const CAN_FAIL: bool = Match::CAN_FAIL;

    fn parse<'ctx>(
        &self,
        context: &mut ParserContext<'ctx, Token>,
        error_handler: &mut impl ErrorHandler,
        pos: &mut usize,
    ) -> Result<Option<Self::Output>, ParserError> {
        // TODO: match_start logic is a bit wrong, maybe remove overall?
        let old_match_start = context.match_start;
        context.match_start = *pos;
        if Match::CAN_MATCH_DIRECTLY {
            let mut runner = DirectMatchRunner::new(
                context,
                (MResSingle::new(), MResMultiple::new(), MResOptional::new()),
            );
            if runner.run_match(&self.matcher, error_handler, pos)? {
                let (res_single, res_multiple, res_optional) = runner.get_match_result();
                let result = (self.constructor)(res_single.to_output(), res_multiple, res_optional);
                context.match_start = old_match_start;
                Ok(Some(result))
            } else {
                drop(runner);
                context.match_start = old_match_start;
                Ok(None)
            }
        } else {
            let mut runner = NoMemoizeBacktrackingRunner::new(context);
            if runner.run_match(&self.matcher, error_handler, pos)? {
                let (res_single, res_multiple, res_optional) = runner.get_match_result();
                let result = (self.constructor)(res_single.to_output(), res_multiple, res_optional);
                context.match_start = old_match_start;
                Ok(Some(result))
            } else {
                drop(runner);
                context.match_start = old_match_start;
                Ok(None)
            }
        }
    }
}
pub trait BoundResult<MRes> {
    fn put_in_result(self, result: &mut MRes);
    fn put_boxed_in_result(self: Box<Self>, result: &mut MRes);
}

impl<Value, MRes, Prop> BoundResult<MRes> for (Value, Prop)
where
    Prop: Property<Value, MRes>,
{
    fn put_in_result(self, result: &mut MRes) {
        let (value, property) = self;
        property.put_in_result(result, value);
    }
    fn put_boxed_in_result(self: Box<Self>, result: &mut MRes) {
        (*self).put_in_result(result)
    }
}

pub struct ResultBinder<Pars, Prop, Token> {
    parser: Pars,
    property: Prop,
    _phantom: PhantomData<Token>,
}

impl<Pars, Prop, Token> ResultBinder<Pars, Prop, Token> {
    pub fn new(parser: Pars, property: Prop) -> Self {
        Self {
            parser,
            property,
            _phantom: PhantomData,
        }
    }
}

pub fn bind_result<Pars, Prop, Token>(
    parser: Pars,
    property: Prop,
) -> ResultBinder<Pars, Prop, Token> {
    ResultBinder::new(parser, property)
}

impl<Pars, Prop, Token, MRes> Matcher<Token, MRes> for ResultBinder<Pars, Prop, Token>
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
    ) -> Result<bool, ParserError>
    where
        Runner: MatchRunner<'a, 'ctx, Token = Token, MRes = MRes>,
        'ctx: 'a,
        Token: 'ctx,
    {
        if let Some(result) = self
            .parser
            .parse(runner.get_parser_context(), error_handler, pos)?
        {
            let bound = self.property.bind_result(result);
            runner.register_result(bound);
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

/// Binds the span (start and end positions) of a match to a property in the match result.
pub struct SpanBinder<Match, Prop> {
    matcher: Match,
    property: Prop,
}
impl<Match, Prop> SpanBinder<Match, Prop> {
    pub fn new(matcher: Match, property: Prop) -> Self {
        Self { matcher, property }
    }
}
pub fn bind_span<Match, Prop>(matcher: Match, property: Prop) -> SpanBinder<Match, Prop> {
    SpanBinder::new(matcher, property)
}

impl<Token, MRes, Match, Prop> Matcher<Token, MRes> for SpanBinder<Match, Prop>
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
    ) -> Result<bool, ParserError>
    where
        Runner: MatchRunner<'a, 'ctx, Token = Token, MRes = MRes>,
        'ctx: 'a,
    {
        let start_pos = *pos;
        if !runner.run_match(&self.matcher, error_handler, pos)? {
            return Ok(false);
        }
        let end_pos = *pos;
        let bound = self.property.bind_result((start_pos, end_pos));
        runner.register_result(bound);
        Ok(true)
    }
}

impl MatchResultSingle for () {
    type Properties = ();
    type Output = ();

    fn new() -> Self {}

    fn new_properties() -> Self::Properties {}
    fn to_output(self) -> Self::Output {}
}

impl MatchResultMultiple for () {
    type Properties = ();

    fn new() -> Self {}

    fn new_properties() -> Self::Properties {}
}

impl MatchResultOptional for () {
    type Properties = ();

    fn new() -> Self {}

    fn new_properties() -> Self::Properties {}
}

fn unwrap_single<T>(option: Option<T>) -> T {
    option.expect("Expected single match result to be set, but it was not")
}

macro_rules! impl_match_results_for_tuple {
    ( $(($T:ident, $idx:tt)),+ ) => {

        impl<$($T),+> MatchResultSingle for ($(Option<$T>,)+)
        // where $($T: Debug),+
        {
            type Properties = (
                $(SingleProperty<fn(&mut Self) -> &mut Option<$T>>,)+
            );
            type Output = ($($T,)+);

            fn new() -> Self {
                // Block expr per repetition to anchor to $T without PhantomData imports
                ($( { let _: std::marker::PhantomData<$T>; None },)+ )
            }

            fn new_properties() -> Self::Properties {
                ($(
                    SingleProperty::new(
                        (|s: &mut Self| -> &mut Option<$T> { &mut s.$idx })
                            as fn(&mut Self) -> &mut Option<$T>,
                    ),
                )+)
            }
            fn to_output(self) -> Self::Output {
                #[allow(non_snake_case)]
                let ($( $T, )+) = self;
                ($(unwrap_single($T),)+)
            }
        }

        impl<$($T),+> MatchResultMultiple for ($(Vec<$T>,)+) {
            type Properties = (
                $(MultipleProperty<fn(&mut Self) -> &mut Vec<$T>>,)+
            );

            fn new() -> Self {
                ($( { let _: std::marker::PhantomData<$T>; Vec::new() },)+ )
            }

            fn new_properties() -> Self::Properties {
                ($(
                    MultipleProperty::new(
                        (|s: &mut Self| -> &mut Vec<$T> { &mut s.$idx })
                            as fn(&mut Self) -> &mut Vec<$T>,
                    ),
                )+)
            }
        }

        impl<$($T),+> MatchResultOptional for ($(Option<$T>,)+) {
            type Properties = (
                $(OptionalProperty<fn(&mut Self) -> &mut Option<$T>>,)+
            );

            fn new() -> Self {
                ($( { let _: std::marker::PhantomData<$T>; None },)+ )
            }

            fn new_properties() -> Self::Properties {
                ($(
                    OptionalProperty::new(
                        (|s: &mut Self| -> &mut Option<$T> { &mut s.$idx })
                            as fn(&mut Self) -> &mut Option<$T>,
                    ),
                )+)
            }
        }
    };
}

impl_match_results_for_tuple!((T0, 0));
impl_match_results_for_tuple!((T0, 0), (T1, 1));
impl_match_results_for_tuple!((T0, 0), (T1, 1), (T2, 2));
impl_match_results_for_tuple!((T0, 0), (T1, 1), (T2, 2), (T3, 3));
impl_match_results_for_tuple!((T0, 0), (T1, 1), (T2, 2), (T3, 3), (T4, 4));
impl_match_results_for_tuple!((T0, 0), (T1, 1), (T2, 2), (T3, 3), (T4, 4), (T5, 5));
impl_match_results_for_tuple!(
    (T0, 0),
    (T1, 1),
    (T2, 2),
    (T3, 3),
    (T4, 4),
    (T5, 5),
    (T6, 6)
);
impl_match_results_for_tuple!(
    (T0, 0),
    (T1, 1),
    (T2, 2),
    (T3, 3),
    (T4, 4),
    (T5, 5),
    (T6, 6),
    (T7, 7)
);
impl_match_results_for_tuple!(
    (T0, 0),
    (T1, 1),
    (T2, 2),
    (T3, 3),
    (T4, 4),
    (T5, 5),
    (T6, 6),
    (T7, 7),
    (T8, 8)
);
impl_match_results_for_tuple!(
    (T0, 0),
    (T1, 1),
    (T2, 2),
    (T3, 3),
    (T4, 4),
    (T5, 5),
    (T6, 6),
    (T7, 7),
    (T8, 8),
    (T9, 9)
);
impl_match_results_for_tuple!(
    (T0, 0),
    (T1, 1),
    (T2, 2),
    (T3, 3),
    (T4, 4),
    (T5, 5),
    (T6, 6),
    (T7, 7),
    (T8, 8),
    (T9, 9),
    (T10, 10)
);
impl_match_results_for_tuple!(
    (T0, 0),
    (T1, 1),
    (T2, 2),
    (T3, 3),
    (T4, 4),
    (T5, 5),
    (T6, 6),
    (T7, 7),
    (T8, 8),
    (T9, 9),
    (T10, 10),
    (T11, 11)
);
pub trait HasProperty {}
pub trait HasNoProperty {}
pub trait CanNotFail {}

impl<Match, Prop> HasNoProperty for SpanBinder<Match, Prop> {}

impl<Pars, Prop, Token> HasProperty for ResultBinder<Pars, Prop, Token> {}
