use crate::grammar::{
    Grammar, HasId, IsCheckable,
    context::{
        MatchResult, MatchResultMultiple, MatchResultOptional, MatchResultSingle, MatcherContext,
        ParserContext,
    },
    error_handler::ErrorHandler,
    get_next_id,
    label::MaybeLabel,
    matcher::{
        CanImplMatchWithRunner, CanMatchWithRunner, DoImplMatchWithNoMoemoizeBacktrackingRunner,
        MatchRunner, Matcher, NoMoemoizeBacktrackingRunner,
    },
    parser::Parser,
    span::Span,
};
use std::{fmt::Debug, marker::PhantomData};

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
    id: usize,
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
        EHandler: ErrorHandler + 'ctx,
    >(
        grammar_factory: GF,
        constructor: F,
    ) -> Self
    where
        Match: CanMatchWithRunner<
            NoMoemoizeBacktrackingRunner<
                'a,
                'ctx,
                Token,
                (MResSingle, MResMultiple, MResOptional),
                EHandler,
            >,
        >, // Matcher<Token, (MResSingle, MResMultiple, MResOptional)> + HasId + IsCheckable<Token>,
    {
        let properties_single = MResSingle::new_properties();
        let properties_multiple = MResMultiple::new_properties();
        let properties_optional = MResOptional::new_properties();
        Self {
            matcher: grammar_factory(properties_single, properties_multiple, properties_optional),
            constructor,
            id: get_next_id(),
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
    Match: Matcher<Token, (MResSingle, MResMultiple, MResOptional)> + HasId + IsCheckable<Token>,
    F: Fn(MResSingle::Output, MResMultiple, MResOptional) -> Out,
{
    type Output = Out;

    fn parse<'ctx>(
        &self,
        context: &mut ParserContext<'ctx, Token, impl ErrorHandler>,
        pos: &mut usize,
    ) -> Result<Self::Output, String> {
        let old_match_start = context.match_start;
        context.match_start = *pos;
        let mut context = MatcherContext::new(
            context,
            MResSingle::new(),
            MResMultiple::new(),
            MResOptional::new(),
        );
        self.matcher.match_pattern(&mut context, pos)?;
        let (res_single, res_multiple, res_optional) = context.match_result;
        let result = (self.constructor)(res_single.to_output(), res_multiple, res_optional);
        context.parser_context.match_start = old_match_start;
        Ok(result)
    }
}

impl<T, MRes, Match, F> IsCheckable<T> for Capture<MRes, Match, F>
where
    Match: Grammar<T>,
{
    fn calc_check(
        &self,
        context: &mut ParserContext<T, impl ErrorHandler>,
        pos: &mut usize,
    ) -> bool {
        self.matcher.check(context, pos)
    }
}

impl<MRes, Match, F> HasId for Capture<MRes, Match, F>
where
    Match: HasId,
{
    fn id(&self) -> usize {
        self.id
    }
}

// pub struct BoundResult<Value, MRes, Prop>
// where
//     Prop: Property<Value, MRes>,
// {
//     property: &'a Prop,
//     value: Value,
//     _phantom: PhantomData<MRes>,
// }
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

impl<Pars, Prop, Token> HasId for ResultBinder<Pars, Prop, Token>
where
    Pars: HasId,
{
    fn id(&self) -> usize {
        self.parser.id()
    }
}

impl<Pars, Prop, Token> IsCheckable<Token> for ResultBinder<Pars, Prop, Token>
where
    Pars: Grammar<Token>,
{
    fn calc_check(
        &self,
        context: &mut ParserContext<Token, impl ErrorHandler>,
        pos: &mut usize,
    ) -> bool {
        self.parser.check(context, pos)
    }
}

impl<Token, MRes, Pars, Prop> Matcher<Token, MRes> for ResultBinder<Pars, Prop, Token>
where
    Pars: Parser<Token>,
    Prop: Property<Pars::Output, MRes>,
{
    fn match_pattern(
        &self,
        context: &mut MatcherContext<Token, MRes, impl ErrorHandler>,
        pos: &mut usize,
    ) -> Result<(), String> {
        let result = self.parser.parse(context.parser_context, pos)?;
        self.property
            .put_in_result(&mut context.match_result, result);
        Ok(())
    }
}

impl<'a, 'ctx, Pars, Prop, Runner, Token> CanImplMatchWithRunner<Runner>
    for ResultBinder<Pars, Prop, Token>
where
    Token: 'ctx,
    Pars: Parser<Token>,
    Pars::Output: 'a,
    Runner: MatchRunner<'a, 'ctx, Token = Token>,
    Prop: Property<Pars::Output, Runner::MRes> + Clone + 'a,
{
    fn impl_match_with_runner(&self, runner: &mut Runner, pos: &mut usize) -> Result<bool, String> {
        let result = self.parser.parse(runner.get_parser_context(), pos)?;
        let bound = self.property.bind_result(result);
        runner.register_result(bound);
        Ok(true)
    }
}

impl<Pars, Prop, Token> DoImplMatchWithNoMoemoizeBacktrackingRunner
    for ResultBinder<Pars, Prop, Token>
{
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
impl<Match, Prop> HasId for SpanBinder<Match, Prop>
where
    Match: HasId,
{
    fn id(&self) -> usize {
        self.matcher.id()
    }
}
impl<Token, Match, Prop> IsCheckable<Token> for SpanBinder<Match, Prop>
where
    Match: Grammar<Token>,
{
    fn calc_check(
        &self,
        context: &mut ParserContext<Token, impl ErrorHandler>,
        pos: &mut usize,
    ) -> bool {
        self.matcher.check(context, pos)
    }
}

impl<Token, MRes, Match, Prop> Matcher<Token, MRes> for SpanBinder<Match, Prop>
where
    Match: Matcher<Token, MRes> + HasId,
    Prop: Property<Span, MRes>,
{
    fn match_pattern(
        &self,
        context: &mut MatcherContext<Token, MRes, impl ErrorHandler>,
        pos: &mut usize,
    ) -> Result<(), String> {
        let start_pos = *pos;
        self.matcher.match_pattern(context, pos)?;
        let end_pos = *pos;
        self.property
            .put_in_result(&mut context.match_result, Span::new(start_pos, end_pos));
        Ok(())
    }
}

impl<'a, 'ctx, Match, Prop, Runner> CanImplMatchWithRunner<Runner> for SpanBinder<Match, Prop>
where
    Match: CanMatchWithRunner<Runner>,
    Runner: MatchRunner<'a, 'ctx>,
    Prop: Property<Span, Runner::MRes> + Clone + 'a,
{
    fn impl_match_with_runner(&self, runner: &mut Runner, pos: &mut usize) -> Result<bool, String> {
        let start_pos = *pos;
        if !runner.run_match(&self.matcher, pos)? {
            return Ok(false);
        }
        let end_pos = *pos;
        let bound = self.property.bind_result(Span::new(start_pos, end_pos));
        runner.register_result(bound);
        Ok(true)
    }
}

impl<Match, Prop> DoImplMatchWithNoMoemoizeBacktrackingRunner for SpanBinder<Match, Prop> where
    Match: DoImplMatchWithNoMoemoizeBacktrackingRunner
{
}

impl<Label, Match, Prop> MaybeLabel<Label> for SpanBinder<Match, Prop> {}

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
        where $($T: Debug),+ {
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

impl<Label, MRes, Match, F> MaybeLabel<Label> for Capture<MRes, Match, F> {}
impl<Label, Pars, Prop, Token> MaybeLabel<Label> for ResultBinder<Pars, Prop, Token> {}
