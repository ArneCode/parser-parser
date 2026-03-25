use crate::grammar::{
    AstNode, Grammar, HasId, IsCheckable, Token,
    context::{MatcherContext, ParserContext},
    get_next_id,
    matcher::Matcher,
    parser::Parser,
};
use std::{array, marker::PhantomData, ops::Deref, rc::Rc};
impl<T: Token, N: AstNode + ?Sized> Deref for MatcherContext<T, N> {
    type Target = ParserContext<T>;

    fn deref(&self) -> &Self::Target {
        &self.parser_context
    }
}

pub trait Property<T: Token, N: AstNode + ?Sized> {
    fn put_in_context(&self, context: &mut MatcherContext<T, N>, value: Box<N>);
}

#[derive(Clone, Copy)]
pub struct SingleProperty {
    property_pos: usize,
}
impl<T: Token, N: AstNode + ?Sized> Property<T, N> for SingleProperty {
    fn put_in_context(&self, context: &mut MatcherContext<T, N>, value: Box<N>) {
        if self.property_pos < context.match_result.single_matches.len() {
            context.match_result.single_matches[self.property_pos] = Some(value);
        } else {
            panic!("SingleProperty position out of bounds");
        }
    }
}
#[derive(Clone, Copy)]
pub struct MultipleProperty {
    property_pos: usize,
}
impl<T: Token, N: AstNode + ?Sized> Property<T, N> for MultipleProperty {
    fn put_in_context(&self, context: &mut MatcherContext<T, N>, value: Box<N>) {
        if self.property_pos < context.match_result.multiple_matches.len() {
            context.match_result.multiple_matches[self.property_pos].push(value);
        } else {
            panic!("MultipleProperty position out of bounds");
        }
    }
}
#[derive(Clone, Copy)]
pub struct OptionalProperty {
    property_pos: usize,
}
impl<T: Token, N: AstNode + ?Sized> Property<T, N> for OptionalProperty {
    fn put_in_context(&self, context: &mut MatcherContext<T, N>, value: Box<N>) {
        if self.property_pos < context.match_result.optional_matches.len() {
            context.match_result.optional_matches[self.property_pos] = Some(value);
        } else {
            panic!("OptionalProperty position out of bounds");
        }
    }
}

pub struct Capture<T, N, Match, F>
where
    T: Token,
    N: AstNode + ?Sized,
    Match: Matcher<T, Output = N> + HasId + IsCheckable<T>,
    F: Fn(MatcherContext<T, N>) -> Box<N>,
{
    matcher: Match,
    constructor: F,
    n_single: usize,
    n_multiple: usize,
    n_optional: usize,
    id: usize,
    _phantom: PhantomData<(T, N)>,
}

impl<T, N, Match, F> Capture<T, N, Match, F>
where
    T: Token,
    N: AstNode + ?Sized,
    Match: Matcher<T, Output = N> + HasId + IsCheckable<T>,
    F: Fn(MatcherContext<T, N>) -> Box<N>,
{
    pub fn new<
        const N_SINGLE: usize,
        const N_MULTIPLE: usize,
        const N_OPTIONAL: usize,
        GF: Fn(
            [SingleProperty; N_SINGLE],
            [MultipleProperty; N_MULTIPLE],
            [OptionalProperty; N_OPTIONAL],
        ) -> Match,
    >(
        grammar_factory: GF,
        constructor: F,
    ) -> Self {
        Self {
            matcher: grammar_factory(
                array::from_fn(|i| SingleProperty { property_pos: i }),
                array::from_fn(|i| MultipleProperty { property_pos: i }),
                array::from_fn(|i| OptionalProperty { property_pos: i }),
            ),
            constructor,
            n_single: N_SINGLE,
            n_multiple: N_MULTIPLE,
            n_optional: N_OPTIONAL,
            id: get_next_id(),
            _phantom: PhantomData,
        }
    }
}

impl<T, N, Match, F> Parser<T> for Capture<T, N, Match, F>
where
    T: Token,
    N: AstNode + ?Sized,
    Match: Matcher<T, Output = N> + HasId + IsCheckable<T>,
    F: Fn(MatcherContext<T, N>) -> Box<N>,
{
    type Output = N;

    fn parse(
        &self,
        context: Rc<ParserContext<T>>,
        pos: &mut usize,
    ) -> Result<Box<Self::Output>, String> {
        let mut context =
            MatcherContext::new(context, self.n_single, self.n_multiple, self.n_optional);
        self.matcher.match_pattern(&mut context, pos)?;
        Ok((self.constructor)(context))
    }
}

impl<T, N, Match, F> IsCheckable<T> for Capture<T, N, Match, F>
where
    T: Token,
    N: AstNode + ?Sized,
    Match: Matcher<T, Output = N> + HasId + IsCheckable<T>,
    F: Fn(MatcherContext<T, N>) -> Box<N>,
{
    fn calc_check(&self, context: &ParserContext<T>, pos: &mut usize) -> bool {
        self.matcher.check(context, pos)
    }
}

impl<T, N, Match, F> HasId for Capture<T, N, Match, F>
where
    T: Token,
    N: AstNode + ?Sized,
    Match: Matcher<T, Output = N> + HasId + IsCheckable<T>,
    F: Fn(MatcherContext<T, N>) -> Box<N>,
{
    fn id(&self) -> usize {
        self.id
    }
}
pub struct CaptureProperty<
    T: Token,
    N: AstNode + ?Sized,
    Pars: Parser<T, Output = N>,
    Prop: Property<T, N>,
> {
    parser: Pars,
    property: Prop,
    id: usize,
    _phantom: PhantomData<(T, N)>,
}

impl<T: Token, N: AstNode + ?Sized, Pars: Parser<T, Output = N>, Prop: Property<T, N>>
    CaptureProperty<T, N, Pars, Prop>
{
    pub fn new(parser: Pars, property: Prop) -> Self {
        Self {
            parser,
            property,
            id: get_next_id(),
            _phantom: PhantomData,
        }
    }
}

pub fn capture_property<
    T: Token + 'static,
    N: AstNode + ?Sized + 'static,
    Pars: Parser<T, Output = N> + 'static,
    Prop: Property<T, N> + 'static,
>(
    parser: Pars,
    property: Prop,
) -> CaptureProperty<T, N, Pars, Prop> {
    CaptureProperty::new(parser, property)
}

impl<T: Token, N: AstNode + ?Sized, Pars: Parser<T, Output = N>, Prop: Property<T, N>> HasId
    for CaptureProperty<T, N, Pars, Prop>
{
    fn id(&self) -> usize {
        return self.id;
    }
}
impl<T, N, Pars, Prop> IsCheckable<T> for CaptureProperty<T, N, Pars, Prop>
where
    T: Token,
    N: AstNode + ?Sized,
    Pars: Parser<T, Output = N> + Grammar<T>,
    Prop: Property<T, N>,
{
    fn calc_check(&self, context: &ParserContext<T>, pos: &mut usize) -> bool {
        self.parser.check(context, pos)
    }
}
impl<T: Token, N: AstNode + ?Sized, Pars: Parser<T, Output = N>, Prop: Property<T, N>> Matcher<T>
    for CaptureProperty<T, N, Pars, Prop>
{
    type Output = N;

    fn match_pattern(
        &self,
        context: &mut MatcherContext<T, Self::Output>,
        pos: &mut usize,
    ) -> Result<(), String> {
        let result = self.parser.parse(context.parser_context.clone(), pos)?;
        self.property.put_in_context(context, result);
        Ok(())
    }
}
