use crate::grammar::{
    Grammar, HasId, IsCheckable,
    context::{
        MatchResultMultiple, MatchResultOptional, MatchResultSingle, MatcherContext, ParserContext,
    },
    get_next_id,
    matcher::Matcher,
    parser::Parser,
};
use std::{marker::PhantomData, rc::Rc};

pub trait Property<T, V, MContext> {
    fn put_in_context(&self, context: &mut MContext, value: V);
}

#[derive(Clone, Copy)]
pub struct SingleProperty<V, MRes, F> {
    setter: F,
    _phantom: PhantomData<(V, MRes)>,
}

impl<V, MRes, F> SingleProperty<V, MRes, F>
where
    MRes: MatchResultSingle,
    F: Fn(&mut MRes) -> &mut Option<V>,
{
    pub fn new(setter: F) -> Self {
        Self {
            setter,
            _phantom: PhantomData,
        }
    }
}

impl<T, V, MResSingle, MResMultiple, MResOptional, F>
    Property<T, V, MatcherContext<T, MResSingle, MResMultiple, MResOptional>>
    for SingleProperty<V, MResSingle, F>
where
    MResSingle: MatchResultSingle,
    MResMultiple: MatchResultMultiple,
    MResOptional: MatchResultOptional,
    F: Fn(&mut MResSingle) -> &mut Option<V>,
{
    fn put_in_context(
        &self,
        context: &mut MatcherContext<T, MResSingle, MResMultiple, MResOptional>,
        value: V,
    ) {
        let property_slot = (self.setter)(&mut context.match_result_single);
        if property_slot.is_some() {
            panic!("SingleProperty already set");
        }
        *property_slot = Some(value);
    }
}

#[derive(Clone, Copy)]
pub struct MultipleProperty<V, MRes, F> {
    setter: F,
    _phantom: PhantomData<(V, MRes)>,
}

impl<V, MRes, F> MultipleProperty<V, MRes, F>
where
    MRes: MatchResultMultiple,
    F: Fn(&mut MRes) -> &mut Vec<V>,
{
    pub fn new(setter: F) -> Self {
        Self {
            setter,
            _phantom: PhantomData,
        }
    }
}

impl<T, V, MResSingle, MResMultiple, MResOptional, F>
    Property<T, V, MatcherContext<T, MResSingle, MResMultiple, MResOptional>>
    for MultipleProperty<V, MResMultiple, F>
where
    MResSingle: MatchResultSingle,
    MResMultiple: MatchResultMultiple,
    MResOptional: MatchResultOptional,
    F: Fn(&mut MResMultiple) -> &mut Vec<V>,
{
    fn put_in_context(
        &self,
        context: &mut MatcherContext<T, MResSingle, MResMultiple, MResOptional>,
        value: V,
    ) {
        let property_slot = (self.setter)(&mut context.match_result_multiple);
        property_slot.push(value);
    }
}

#[derive(Clone, Copy)]
pub struct OptionalProperty<V, MRes, F> {
    setter: F,
    _phantom: PhantomData<(V, MRes)>,
}

impl<V, MRes, F> OptionalProperty<V, MRes, F>
where
    MRes: MatchResultOptional,
    F: Fn(&mut MRes) -> &mut Option<V>,
{
    pub fn new(setter: F) -> Self {
        Self {
            setter,
            _phantom: PhantomData,
        }
    }
}

impl<T, V, MResSingle, MResMultiple, MResOptional, F>
    Property<T, V, MatcherContext<T, MResSingle, MResMultiple, MResOptional>>
    for OptionalProperty<V, MResOptional, F>
where
    MResSingle: MatchResultSingle,
    MResMultiple: MatchResultMultiple,
    MResOptional: MatchResultOptional,
    F: Fn(&mut MResOptional) -> &mut Option<V>,
{
    fn put_in_context(
        &self,
        context: &mut MatcherContext<T, MResSingle, MResMultiple, MResOptional>,
        value: V,
    ) {
        let property_slot = (self.setter)(&mut context.match_result_optional);
        if property_slot.is_some() {
            panic!("OptionalProperty already set");
        }
        *property_slot = Some(value);
    }
}

pub struct Capture<T, Out, MResSingle, MResMultiple, MResOptional, Match, F> {
    matcher: Match,
    constructor: F,
    id: usize,
    _phantom: PhantomData<(T, MResSingle, MResMultiple, MResOptional, Out)>,
}

impl<T, Out, MResSingle, MResMultiple, MResOptional, Match, F>
    Capture<T, Out, MResSingle, MResMultiple, MResOptional, Match, F>
where
    MResSingle: MatchResultSingle,
    MResMultiple: MatchResultMultiple,
    MResOptional: MatchResultOptional,
    Match: Matcher<T, MatcherContext<T, MResSingle, MResMultiple, MResOptional>>
        + HasId
        + IsCheckable<T>,
    F: Fn(MResSingle, MResMultiple, MResOptional) -> Out,
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

impl<T, Out, MResSingle, MResMultiple, MResOptional, Match, F> Parser<T>
    for Capture<T, Out, MResSingle, MResMultiple, MResOptional, Match, F>
where
    MResSingle: MatchResultSingle,
    MResMultiple: MatchResultMultiple,
    MResOptional: MatchResultOptional,
    Match: Matcher<T, MatcherContext<T, MResSingle, MResMultiple, MResOptional>>
        + HasId
        + IsCheckable<T>,
    F: Fn(MResSingle, MResMultiple, MResOptional) -> Out,
{
    type Output = Out;

    fn parse(
        &self,
        context: Rc<ParserContext<T>>,
        pos: &mut usize,
    ) -> Result<Self::Output, String> {
        let mut context = MatcherContext::new(
            context,
            MResSingle::new(),
            MResMultiple::new(),
            MResOptional::new(),
        );
        self.matcher.match_pattern(&mut context, pos)?;
        Ok((self.constructor)(
            context.match_result_single,
            context.match_result_multiple,
            context.match_result_optional,
        ))
    }
}

impl<T, Out, MResSingle, MResMultiple, MResOptional, Match, F> IsCheckable<T>
    for Capture<T, Out, MResSingle, MResMultiple, MResOptional, Match, F>
where
    Match: Grammar<T>,
{
    fn calc_check(&self, context: &ParserContext<T>, pos: &mut usize) -> bool {
        self.matcher.check(context, pos)
    }
}

impl<T, Out, MResSingle, MResMultiple, MResOptional, Match, F> HasId
    for Capture<T, Out, MResSingle, MResMultiple, MResOptional, Match, F>
where
    Match: HasId,
{
    fn id(&self) -> usize {
        self.id
    }
}

pub struct CaptureProperty<T, Out, MContext, Pars, Prop> {
    parser: Pars,
    property: Prop,
    _phantom: PhantomData<(T, Out, MContext)>,
}

impl<T, Out, MContext, Pars, Prop> CaptureProperty<T, Out, MContext, Pars, Prop>
where
    Pars: Parser<T, Output = Out>,
    Prop: Property<T, Out, MContext>,
{
    pub fn new(parser: Pars, property: Prop) -> Self {
        Self {
            parser,
            property,
            _phantom: PhantomData,
        }
    }
}

pub fn capture_property<T, Out, MContext, Pars, Prop>(
    parser: Pars,
    property: Prop,
) -> CaptureProperty<T, Out, MContext, Pars, Prop>
where
    Pars: Parser<T, Output = Out> + 'static,
    Prop: Property<T, Out, MContext> + 'static,
{
    CaptureProperty::new(parser, property)
}

impl<T, Out, MContext, Pars, Prop> HasId for CaptureProperty<T, Out, MContext, Pars, Prop>
where
    Pars: HasId,
{
    fn id(&self) -> usize {
        self.parser.id()
    }
}

impl<T, Out, MContext, Pars, Prop> IsCheckable<T> for CaptureProperty<T, Out, MContext, Pars, Prop>
where
    Pars: Grammar<T>,
{
    fn calc_check(&self, context: &ParserContext<T>, pos: &mut usize) -> bool {
        self.parser.check(context, pos)
    }
}

impl<T, Out, MResSingle, MResMultiple, MResOptional, Pars, Prop>
    Matcher<T, MatcherContext<T, MResSingle, MResMultiple, MResOptional>>
    for CaptureProperty<
        T,
        Out,
        MatcherContext<T, MResSingle, MResMultiple, MResOptional>,
        Pars,
        Prop,
    >
where
    Pars: Parser<T, Output = Out>,
    Prop: Property<T, Out, MatcherContext<T, MResSingle, MResMultiple, MResOptional>>,
{
    fn match_pattern(
        &self,
        context: &mut MatcherContext<T, MResSingle, MResMultiple, MResOptional>,
        pos: &mut usize,
    ) -> Result<(), String> {
        let result = self.parser.parse(context.parser_context.clone(), pos)?;
        self.property.put_in_context(context, result);
        Ok(())
    }
}

macro_rules! impl_match_results_for_tuple {
    ( $(($T:ident, $idx:tt)),+ ) => {

        impl<$($T),+> MatchResultSingle for ($(Option<$T>,)+) {
            type Properties = (
                $(SingleProperty<$T, Self, fn(&mut Self) -> &mut Option<$T>>,)+
            );

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
        }

        impl<$($T),+> MatchResultMultiple for ($(Vec<$T>,)+) {
            type Properties = (
                $(MultipleProperty<$T, Self, fn(&mut Self) -> &mut Vec<$T>>,)+
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
                $(OptionalProperty<$T, Self, fn(&mut Self) -> &mut Option<$T>>,)+
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
