use crate::grammar::{
    Grammar, HasId, IsCheckable,
    context::{
        MatchResult, MatchResultMultiple, MatchResultOptional, MatchResultSingle, MatcherContext,
        ParserContext,
    },
    error_handler::ErrorHandler,
    get_next_id,
    matcher::Matcher,
    parser::Parser,
};
use std::{fmt::Debug, marker::PhantomData};

pub trait Property<Value, MatchResult> {
    fn put_in_result(&self, result: &mut MatchResult, value: Value);
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
        GF: Fn(MResSingle::Properties, MResMultiple::Properties, MResOptional::Properties) -> Match,
    >(
        grammar_factory: GF,
        constructor: F,
    ) -> Self {
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
        let mut context = MatcherContext::new(
            context,
            MResSingle::new(),
            MResMultiple::new(),
            MResOptional::new(),
        );
        self.matcher.match_pattern(&mut context, pos)?;
        let (res_single, res_multiple, res_optional) = context.match_result;
        Ok((self.constructor)(
            res_single.as_output(),
            res_multiple,
            res_optional,
        ))
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

pub struct CaptureProperty<Pars, Prop> {
    parser: Pars,
    property: Prop,
}

impl<Pars, Prop> CaptureProperty<Pars, Prop> {
    pub fn new(parser: Pars, property: Prop) -> Self {
        Self { parser, property }
    }
}

pub fn capture_property<MContext, Pars, Prop>(
    parser: Pars,
    property: Prop,
) -> CaptureProperty<Pars, Prop> {
    CaptureProperty::new(parser, property)
}

impl<Pars, Prop> HasId for CaptureProperty<Pars, Prop>
where
    Pars: HasId,
{
    fn id(&self) -> usize {
        self.parser.id()
    }
}

impl<Pars, Prop, Token> IsCheckable<Token> for CaptureProperty<Pars, Prop>
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

impl<Token, MRes, Pars, Prop> Matcher<Token, MRes> for CaptureProperty<Pars, Prop>
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

impl MatchResultSingle for () {
    type Properties = ();
    type Output = ();

    fn new() -> Self {}

    fn new_properties() -> Self::Properties {}
    fn as_output(self) -> Self::Output {}
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
            fn as_output(self) -> Self::Output {
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
