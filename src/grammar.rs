use std::{
    array,
    cell::RefCell,
    collections::HashMap,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    rc::Rc,
    sync::atomic::{self, AtomicUsize},
    usize,
};
pub trait HasId {
    fn id(&self) -> usize;
}
pub trait AstNode {}
pub trait Token {}

pub trait IsCheckable<T: Token> {
    fn calc_check(&self, context: &ParserContext<T>, pos: &mut usize) -> bool;
}

pub struct ParserContext<T: Token> {
    pub tokens: Vec<T>,
    pub memo_table: RefCell<HashMap<(usize, usize), Option<usize>>>,
}

impl<T: Token> ParserContext<T> {
    pub fn new(tokens: Vec<T>) -> Self {
        Self {
            tokens,
            memo_table: RefCell::new(HashMap::new()),
        }
    }
}

pub trait Parser<T: Token> {
    type Output: AstNode + ?Sized;
    fn parse(
        &self,
        context: Rc<ParserContext<T>>,
        pos: &mut usize,
    ) -> Result<Box<Self::Output>, String>;
}

pub struct MatchResult<N: AstNode + ?Sized> {
    pub single_matches: Vec<Option<Box<N>>>,
    pub multiple_matches: Vec<Vec<Box<N>>>,
    pub optional_matches: Vec<Option<Box<N>>>,
}

pub struct MatcherContext<T: Token, N: AstNode + ?Sized> {
    pub parser_context: Rc<ParserContext<T>>,
    pub match_result: MatchResult<N>,
}

impl<T: Token, N: AstNode + ?Sized> MatcherContext<T, N> {
    pub fn new(
        parser_context: Rc<ParserContext<T>>,
        n_single: usize,
        n_multiple: usize,
        n_optional: usize,
    ) -> Self {
        Self {
            parser_context,
            match_result: MatchResult {
                single_matches: (0..n_single).map(|_| None).collect(),
                multiple_matches: (0..n_multiple).map(|_| Vec::new()).collect(),
                optional_matches: (0..n_optional).map(|_| None).collect(),
            },
        }
    }
}

impl<T: Token, N: AstNode + ?Sized> Deref for MatcherContext<T, N> {
    type Target = ParserContext<T>;

    fn deref(&self) -> &Self::Target {
        &self.parser_context
    }
}

pub trait Matcher<T: Token> {
    type Output: AstNode + ?Sized;
    fn match_pattern(
        &self,
        context: &mut MatcherContext<T, Self::Output>,
        pos: &mut usize,
    ) -> Result<(), String>;
}

trait Grammar<T: Token> {
    fn check(&self, context: &ParserContext<T>, pos: &mut usize) -> bool;
    fn check_no_advance(&self, context: &ParserContext<T>, pos: &usize) -> bool {
        let mut pos = *pos;
        self.check(context, &mut pos)
    }
}

// impl Grammar for all Parsers / Matchers, using memoization to optimize repeated checks
impl<T: Token, G: HasId + IsCheckable<T> + ?Sized> Grammar<T> for G {
    fn check(&self, context: &ParserContext<T>, pos: &mut usize) -> bool {
        let id = self.id();
        if let Some(&result) = context.memo_table.borrow().get(&(id, *pos)) {
            if let Some(new_pos) = result {
                *pos = new_pos; // Update the position to the memoized value
            }
            return result.is_some();
        }
        let old_pos = *pos;
        let result = self.calc_check(context, pos);
        context
            .memo_table
            .borrow_mut()
            .insert((id, old_pos), if result { Some(*pos) } else { None });
        if !result {
            *pos = old_pos; // Reset position on failure
        }
        result
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
static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
fn get_next_id() -> usize {
    NEXT_ID.fetch_add(1, atomic::Ordering::Relaxed)
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
    fn new<
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

pub struct Multiple<T, N, Match>
where
    T: Token,
    N: AstNode + ?Sized,
    Match: Matcher<T, Output = N> + HasId + IsCheckable<T>,
{
    matcher: Match,
    id: usize,
    _phantom: PhantomData<(T, N)>,
}

impl<T, N, Match> Multiple<T, N, Match>
where
    T: Token,
    N: AstNode + ?Sized,
    Match: Matcher<T, Output = N> + HasId + IsCheckable<T>,
{
    fn new(matcher: Match) -> Self {
        Self {
            matcher,
            id: get_next_id(),
            _phantom: PhantomData,
        }
    }
}

pub fn many<T, N, Match>(matcher: Match) -> Multiple<T, N, Match>
where
    T: Token + 'static,
    N: AstNode + ?Sized + 'static,
    Match: Matcher<T, Output = N> + HasId + IsCheckable<T> + 'static,
{
    Multiple::new(matcher)
}
impl<T, N, Match> Matcher<T> for Multiple<T, N, Match>
where
    T: Token,
    N: AstNode + ?Sized,
    Match: Matcher<T, Output = N> + HasId + IsCheckable<T>,
{
    type Output = N;

    fn match_pattern(
        &self,
        context: &mut MatcherContext<T, Self::Output>,
        pos: &mut usize,
    ) -> Result<(), String> {
        while self.matcher.check_no_advance(context, pos) {
            self.matcher.match_pattern(context, pos)?;
        }
        Ok(())
    }
}

impl<T, N, Match> IsCheckable<T> for Multiple<T, N, Match>
where
    T: Token,
    N: AstNode + ?Sized,
    Match: Matcher<T, Output = N> + HasId + IsCheckable<T>,
{
    fn calc_check(&self, context: &ParserContext<T>, pos: &mut usize) -> bool {
        // advance pos
        while self.matcher.check(context, pos) {}
        return true;
    }
}

impl<T, N, Match> HasId for Multiple<T, N, Match>
where
    T: Token,
    N: AstNode + ?Sized,
    Match: Matcher<T, Output = N> + HasId + IsCheckable<T>,
{
    fn id(&self) -> usize {
        self.id
    }
}
pub trait ToMatcher<T: Token, N: AstNode + ?Sized> {
    type MatcherType: Matcher<T, Output = N> + HasId + IsCheckable<T>;
    fn to_matcher(&self) -> Self::MatcherType;
}

pub struct StringMatcher<N: AstNode + ?Sized> {
    expected: String,
    id: usize,
    _phantom: PhantomData<N>,
}

impl<N: AstNode + ?Sized> StringMatcher<N> {
    fn new(expected: String) -> Self {
        Self {
            expected,
            id: get_next_id(),
            _phantom: PhantomData,
        }
    }
}

// impl ToMatcher<char, N> for String {
impl<N: AstNode + ?Sized + 'static> ToMatcher<char, N> for String {
    type MatcherType = StringMatcher<N>;

    fn to_matcher(&self) -> Self::MatcherType {
        StringMatcher::new(self.clone())
    }
}

impl<N: AstNode + ?Sized + 'static> ToMatcher<char, N> for &str {
    type MatcherType = StringMatcher<N>;
    fn to_matcher(&self) -> Self::MatcherType {
        StringMatcher::new(self.to_string())
    }
}

impl<N: AstNode + ?Sized> HasId for StringMatcher<N> {
    fn id(&self) -> usize {
        self.id
    }
}
impl Token for char {}
impl<N: AstNode + ?Sized> IsCheckable<char> for StringMatcher<N> {
    fn calc_check(&self, context: &ParserContext<char>, pos: &mut usize) -> bool {
        let end_pos = *pos + self.expected.len();
        if end_pos > context.tokens.len() {
            return false;
        }
        let slice: String = context.tokens[*pos..end_pos].iter().collect();
        if slice == self.expected {
            *pos = end_pos; // Advance position on success
            true
        } else {
            false
        }
    }
}

impl<N: AstNode + ?Sized> Matcher<char> for StringMatcher<N> {
    type Output = N;
    fn match_pattern(
        &self,
        _context: &mut MatcherContext<char, Self::Output>,
        pos: &mut usize,
    ) -> Result<(), String> {
        if self.check(_context, pos) {
            Ok(())
        } else {
            Err(format!("Expected '{}' at position {}", self.expected, pos))
        }
    }
}

pub struct Sequence<T, N, Tuple>
where
    T: Token,
    N: AstNode + ?Sized,
{
    elements: Tuple,
    id: usize,
    phantom: PhantomData<(T, N)>,
}
impl<T, N, Tuple> Sequence<T, N, Tuple>
where
    T: Token,
    N: AstNode,
{
    fn new(elements: Tuple) -> Self {
        Self {
            elements,
            id: get_next_id(),
            phantom: PhantomData,
        }
    }
}

fn seq<T, N, Tuple>(elements: Tuple) -> Sequence<T, N, Tuple>
where
    T: Token,
    N: AstNode,
{
    Sequence::new(elements)
}

impl<T, N, Tuple> HasId for Sequence<T, N, Tuple>
where
    T: Token,
    N: AstNode,
{
    fn id(&self) -> usize {
        self.id
    }
}

macro_rules! impl_matcher_for_seq_tuples {
    () => {};
    ($head:ident $(,$tail:ident)*) => {
        impl<T, N, $head, $($tail),*> IsCheckable<T> for Sequence<T, N,($head, $($tail,)*)>
        where
            T: Token,
            N: AstNode,
            $head: Grammar<T>,
            $($tail: Grammar<T>,)*
        {
            fn calc_check(&self, context: &ParserContext<T>, pos: &mut usize) -> bool {

                #[allow(non_snake_case)]
                let ($head, $($tail,)*) = &self.elements;

                if !$head.check(context, pos) {
                    return false;
                }

                $(
                    if !$tail.check(context, pos) {
                        return false;
                    }
                )*

                true
            }
        }
        impl<T, N, $head, $($tail),*> Matcher<T> for Sequence<T, N,($head, $($tail,)*)>
        where
            T: Token,
            N: AstNode + ?Sized,
            $head: Matcher<T, Output = N>,
            $($tail: Matcher<T, Output = N>,)*
        {
            type Output = N;

            fn match_pattern(
                &self,
                context: &mut MatcherContext<T, N>,
                pos: &mut usize,
            ) -> Result<(), String> {

                #[allow(non_snake_case)]
                let ($head, $($tail,)*) = &self.elements;

                $head.match_pattern(context, pos)?;

                $(
                $tail.match_pattern(context, pos)?;
                )*

                Ok(())
            }
        }

        impl_matcher_for_seq_tuples!($($tail),*);
    };
}

impl_matcher_for_seq_tuples!(
    T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20
);

pub struct OneOfMatcher<T, N, Tuple>
where
    T: Token,
    N: AstNode + ?Sized,
{
    options: Tuple,
    id: usize,
    phantom: PhantomData<(T, N)>,
}

impl<T, N, Tuple> OneOfMatcher<T, N, Tuple>
where
    T: Token,
    N: AstNode + ?Sized,
    Tuple: IsCheckable<T> + Matcher<T, Output = N>,
{
    fn new(options: Tuple) -> Self {
        Self {
            options,
            id: get_next_id(),
            phantom: PhantomData,
        }
    }
}

impl<T, N, Tuple> HasId for OneOfMatcher<T, N, Tuple>
where
    T: Token,
    N: AstNode + ?Sized,
{
    fn id(&self) -> usize {
        self.id
    }
}

macro_rules! impl_matcher_for_one_of_tuples {
    () => {};
    ($head:ident $(,$tail:ident)*) => {
        impl<T, N, $head, $($tail),*> IsCheckable<T> for OneOfMatcher<T, N,($head, $($tail,)*)>
        where
            T: Token,
            N: AstNode,
            $head: Grammar<T>,
            $($tail: Grammar<T>,)*
        {
            fn calc_check(&self, context: &ParserContext<T>, pos: &mut usize) -> bool {

                #[allow(non_snake_case)]
                let ($head, $($tail,)*) = &self.options;

                if $head.check(context, pos) {
                    return true;
                }

                $(
                    if $tail.check(context, pos) {
                        return true;
                    }
                )*

                false
            }
        }

        impl<T, N, $head, $($tail),*> Matcher<T> for OneOfMatcher<T, N,($head, $($tail,)*)>
        where
            T: Token,
            N: AstNode + ?Sized,
            $head: Matcher<T, Output = N> + Grammar<T>,
            $($tail: Matcher<T, Output = N> + Grammar<T>,)*
        {
            type Output = N;

            fn match_pattern(
                &self,
                context: &mut MatcherContext<T, Self::Output>,
                pos: &mut usize,
            ) -> Result<(), String> {

                #[allow(non_snake_case)]
                let ($head, $($tail,)*) = &self.options;

                if $head.check(context, pos) {
                    return $head.match_pattern(context, pos);
                }

                $(
                    if $tail.check(context, pos) {
                        return $tail.match_pattern(context, pos);
                    }
                )*

                Err(format!("None of the options matched at position {}", pos))
            }
        }

        impl_matcher_for_one_of_tuples!($($tail),*);
    };
}

impl_matcher_for_one_of_tuples!(
    T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20
);

pub struct Optional<T, N, Match>
where
    T: Token,
    N: AstNode + ?Sized,
{
    matcher: Match,
    id: usize,
    _phantom: PhantomData<(T, N)>,
}

impl<T, N, Match> Optional<T, N, Match>
where
    T: Token,
    N: AstNode + ?Sized,
{
    fn new(matcher: Match) -> Self {
        Self {
            matcher,
            id: get_next_id(),
            _phantom: PhantomData,
        }
    }
}

pub fn optional<T, N, Match>(matcher: Match) -> Optional<T, N, Match>
where
    T: Token + 'static,
    N: AstNode + ?Sized + 'static,
    Match: Matcher<T, Output = N> + HasId + IsCheckable<T> + 'static,
{
    Optional::new(matcher)
}

impl<T, N, Match> Matcher<T> for Optional<T, N, Match>
where
    T: Token,
    N: AstNode + ?Sized,
    Match: Matcher<T, Output = N> + Grammar<T>,
{
    type Output = N;

    fn match_pattern(
        &self,
        context: &mut MatcherContext<T, Self::Output>,
        pos: &mut usize,
    ) -> Result<(), String> {
        if self.matcher.check_no_advance(context, pos) {
            self.matcher.match_pattern(context, pos)?;
        }
        Ok(())
    }
}

impl<T, N, Match> IsCheckable<T> for Optional<T, N, Match>
where
    T: Token,
    N: AstNode + ?Sized,
    Match: Grammar<T>,
{
    fn calc_check(&self, context: &ParserContext<T>, pos: &mut usize) -> bool {
        self.matcher.check(context, pos);
        true
    }
}

impl<T, N, Match> HasId for Optional<T, N, Match>
where
    T: Token,
    N: AstNode + ?Sized,
    Match: Matcher<T, Output = N>,
{
    fn id(&self) -> usize {
        self.id
    }
}

pub struct OneOrMore<T, N, Match>
where
    T: Token,
    N: AstNode + ?Sized,
    Match: Matcher<T, Output = N> + HasId + IsCheckable<T>,
{
    matcher: Match,
    id: usize,
    _phantom: PhantomData<(T, N)>,
}

impl<T, N, Match> OneOrMore<T, N, Match>
where
    T: Token,
    N: AstNode + ?Sized,
    Match: Matcher<T, Output = N> + HasId + IsCheckable<T>,
{
    pub fn new(matcher: Match) -> Self {
        Self {
            matcher,
            id: get_next_id(),
            _phantom: PhantomData,
        }
    }
}

/// e+  — match one or more repetitions of `matcher`, capturing each occurrence.
pub fn one_or_more<T, N, Match>(matcher: Match) -> OneOrMore<T, N, Match>
where
    T: Token + 'static,
    N: AstNode + ?Sized + 'static,
    Match: Matcher<T, Output = N> + HasId + IsCheckable<T> + 'static,
{
    OneOrMore::new(matcher)
}

impl<T, N, Match> HasId for OneOrMore<T, N, Match>
where
    T: Token,
    N: AstNode + ?Sized,
    Match: Matcher<T, Output = N> + HasId + IsCheckable<T>,
{
    fn id(&self) -> usize {
        self.id
    }
}

impl<T, N, Match> IsCheckable<T> for OneOrMore<T, N, Match>
where
    T: Token,
    N: AstNode + ?Sized,
    Match: Matcher<T, Output = N> + HasId + IsCheckable<T>,
{
    fn calc_check(&self, context: &ParserContext<T>, pos: &mut usize) -> bool {
        // Must consume at least one token.
        if !self.matcher.check(context, pos) {
            return false;
        }
        // Greedily consume the rest (mirrors Multiple).
        while self.matcher.check(context, pos) {}
        true
    }
}

impl<T, N, Match> Matcher<T> for OneOrMore<T, N, Match>
where
    T: Token,
    N: AstNode + ?Sized,
    Match: Matcher<T, Output = N> + HasId + IsCheckable<T>,
{
    type Output = N;

    fn match_pattern(
        &self,
        context: &mut MatcherContext<T, Self::Output>,
        pos: &mut usize,
    ) -> Result<(), String> {
        // First match is mandatory — propagate the error if absent.
        self.matcher.match_pattern(context, pos)?;
        // Remaining matches are optional (same as Multiple).
        while self.matcher.check_no_advance(context, pos) {
            self.matcher.match_pattern(context, pos)?;
        }
        Ok(())
    }
}

pub struct PositiveLookahead<T, N, Check>
where
    T: Token,
    N: AstNode + ?Sized,
{
    checker: Check,
    id: usize,
    _phantom: PhantomData<(T, N)>,
}

impl<T, N, Check> PositiveLookahead<T, N, Check>
where
    T: Token,
    N: AstNode + ?Sized,
{
    pub fn new(checker: Check) -> Self {
        Self {
            checker,
            id: get_next_id(),
            _phantom: PhantomData,
        }
    }
}

/// &e  — positive lookahead. Succeeds without consuming if `e` would match.
pub fn positive_lookahead<T, N, Check>(checker: Check) -> PositiveLookahead<T, N, Check>
where
    T: Token + 'static,
    N: AstNode + ?Sized + 'static,
    Check: HasId + IsCheckable<T> + 'static,
{
    PositiveLookahead::new(checker)
}

impl<T, N, Check> HasId for PositiveLookahead<T, N, Check>
where
    T: Token,
    N: AstNode + ?Sized,
    Check: HasId + IsCheckable<T>,
{
    fn id(&self) -> usize {
        self.id
    }
}

impl<T, N, Check> IsCheckable<T> for PositiveLookahead<T, N, Check>
where
    T: Token,
    N: AstNode + ?Sized,
    Check: HasId + IsCheckable<T>,
{
    fn calc_check(&self, context: &ParserContext<T>, pos: &mut usize) -> bool {
        // Pure peek — pos must not move regardless of outcome.
        self.checker.check_no_advance(context, pos)
    }
}

impl<T, N, Check> Matcher<T> for PositiveLookahead<T, N, Check>
where
    T: Token,
    N: AstNode + ?Sized,
    Check: HasId + IsCheckable<T>,
{
    type Output = N;

    fn match_pattern(
        &self,
        context: &mut MatcherContext<T, Self::Output>,
        pos: &mut usize,
    ) -> Result<(), String> {
        if self.checker.check_no_advance(context, pos) {
            Ok(()) // pos unchanged, nothing captured
        } else {
            Err(format!("positive lookahead failed at position {}", pos))
        }
    }
}

pub struct NegativeLookahead<T, N, Check>
where
    T: Token,
    N: AstNode + ?Sized,
    Check: HasId + IsCheckable<T>,
{
    checker: Check,
    id: usize,
    _phantom: PhantomData<(T, N)>,
}

impl<T, N, Check> NegativeLookahead<T, N, Check>
where
    T: Token,
    N: AstNode + ?Sized,
    Check: HasId + IsCheckable<T>,
{
    pub fn new(checker: Check) -> Self {
        Self {
            checker,
            id: get_next_id(),
            _phantom: PhantomData,
        }
    }
}

/// !e  — negative lookahead. Succeeds without consuming if `e` would *not* match.
pub fn negative_lookahead<T, N, Check>(checker: Check) -> NegativeLookahead<T, N, Check>
where
    T: Token + 'static,
    N: AstNode + ?Sized + 'static,
    Check: HasId + IsCheckable<T> + 'static,
{
    NegativeLookahead::new(checker)
}

impl<T, N, Check> HasId for NegativeLookahead<T, N, Check>
where
    T: Token,
    N: AstNode + ?Sized,
    Check: HasId + IsCheckable<T>,
{
    fn id(&self) -> usize {
        self.id
    }
}

impl<T, N, Check> IsCheckable<T> for NegativeLookahead<T, N, Check>
where
    T: Token,
    N: AstNode + ?Sized,
    Check: HasId + IsCheckable<T>,
{
    fn calc_check(&self, context: &ParserContext<T>, pos: &mut usize) -> bool {
        // Peek — pos must not move.  Success means the inner check *failed*.
        !self.checker.check_no_advance(context, pos)
    }
}

impl<T, N, Check> Matcher<T> for NegativeLookahead<T, N, Check>
where
    T: Token,
    N: AstNode + ?Sized,
    Check: HasId + IsCheckable<T>,
{
    type Output = N;

    fn match_pattern(
        &self,
        context: &mut MatcherContext<T, Self::Output>,
        pos: &mut usize,
    ) -> Result<(), String> {
        if !self.checker.check_no_advance(context, pos) {
            Ok(()) // pos unchanged, nothing captured
        } else {
            Err(format!(
                "negative lookahead failed: forbidden pattern matched at position {}",
                pos
            ))
        }
    }
}

pub struct AnyToken<T, N>
where
    T: Token,
    N: AstNode + ?Sized,
{
    id: usize,
    _phantom: PhantomData<(T, N)>,
}

impl<T, N> AnyToken<T, N>
where
    T: Token,
    N: AstNode + ?Sized,
{
    pub fn new() -> Self {
        Self {
            id: get_next_id(),
            _phantom: PhantomData,
        }
    }
}

/// `.`  — match any single token without inspecting its value.
pub fn any_token<T, N>() -> AnyToken<T, N>
where
    T: Token,
    N: AstNode + ?Sized,
{
    AnyToken::new()
}

impl<T, N> HasId for AnyToken<T, N>
where
    T: Token,
    N: AstNode + ?Sized,
{
    fn id(&self) -> usize {
        self.id
    }
}

impl<T, N> IsCheckable<T> for AnyToken<T, N>
where
    T: Token,
    N: AstNode + ?Sized,
{
    fn calc_check(&self, context: &ParserContext<T>, pos: &mut usize) -> bool {
        if *pos < context.tokens.len() {
            *pos += 1;
            true
        } else {
            false
        }
    }
}

impl<T, N> Matcher<T> for AnyToken<T, N>
where
    T: Token,
    N: AstNode + ?Sized,
{
    type Output = N;

    fn match_pattern(
        &self,
        context: &mut MatcherContext<T, Self::Output>,
        pos: &mut usize,
    ) -> Result<(), String> {
        if *pos < context.tokens.len() {
            *pos += 1;
            Ok(())
        } else {
            Err(format!(
                "expected any token at position {} but reached end of input",
                pos
            ))
        }
    }
}

pub struct TokenParser<T, N, CheckF, ParseF>
where
    T: Token,
    N: AstNode + ?Sized,
    // CheckF: Fn(&T) -> bool,
    // ParseF: Fn(&T) -> Box<N>,
{
    check_fn: CheckF,
    parse_fn: ParseF,
    id: usize,
    _phantom: PhantomData<(T, N)>,
}

impl<T, N, CheckF, ParseF> TokenParser<T, N, CheckF, ParseF>
where
    T: Token,
    N: AstNode + ?Sized,
    // CheckF: Fn(&T) -> bool,
    // ParseF: Fn(&T) -> Box<N>,
{
    pub fn new(check_fn: CheckF, parse_fn: ParseF) -> Self {
        Self {
            check_fn,
            parse_fn,
            id: get_next_id(),
            _phantom: PhantomData,
        }
    }
}

impl<T, N, CheckF, ParseF> HasId for TokenParser<T, N, CheckF, ParseF>
where
    T: Token,
    N: AstNode + ?Sized,
{
    fn id(&self) -> usize {
        self.id
    }
}

impl<T, N, CheckF, ParseF> IsCheckable<T> for TokenParser<T, N, CheckF, ParseF>
where
    T: Token,
    N: AstNode + ?Sized,
    CheckF: Fn(&T) -> bool,
{
    fn calc_check(&self, context: &ParserContext<T>, pos: &mut usize) -> bool {
        if *pos < context.tokens.len() {
            let token = &context.tokens[*pos];
            *pos += 1; // Advance position on success
            (self.check_fn)(token)
        } else {
            false
        }
    }
}

impl<T, N, CheckF, ParseF> Parser<T> for TokenParser<T, N, CheckF, ParseF>
where
    T: Token,
    N: AstNode + ?Sized,
    CheckF: Fn(&T) -> bool,
    ParseF: Fn(&T) -> Box<N>,
{
    type Output = N;

    fn parse(
        &self,
        context: Rc<ParserContext<T>>,
        pos: &mut usize,
    ) -> Result<Box<Self::Output>, String> {
        if *pos < context.tokens.len() {
            if self.check_no_advance(&context, pos) {
                let token = &context.tokens[*pos];
                *pos += 1; // Advance position on success
                Ok((self.parse_fn)(token))
            } else {
                Err(format!(
                    "token did not satisfy check function at position {}",
                    pos
                ))
            }
        } else {
            Err(format!(
                "expected token at position {} but reached end of input",
                pos
            ))
        }
    }
}

struct MultipleParser<T, NodeIn, NodeOut, Pars, CombF>
where
    NodeOut: ?Sized,
{
    parser: Pars,
    combine_fn: CombF,
    id: usize,
    _phantom: PhantomData<(T, NodeIn, NodeOut)>,
}

impl<T, NodeIn, NodeOut, Pars, CombF> MultipleParser<T, NodeIn, NodeOut, Pars, CombF>
where
    T: Token,
    NodeIn: AstNode,
    NodeOut: AstNode + ?Sized,
    Pars: Parser<T, Output = NodeIn>,
    CombF: Fn(Vec<NodeIn>) -> NodeOut,
{
    pub fn new(parser: Pars, combine_fn: CombF) -> Self {
        Self {
            parser,
            combine_fn,
            id: get_next_id(),
            _phantom: PhantomData,
        }
    }
}

impl<T, NodeIn, NodeOut, Pars, CombF> HasId for MultipleParser<T, NodeIn, NodeOut, Pars, CombF>
where
    NodeOut: ?Sized,
{
    fn id(&self) -> usize {
        self.id
    }
}

impl<T, NodeIn, NodeOut, Pars, CombF> IsCheckable<T>
    for MultipleParser<T, NodeIn, NodeOut, Pars, CombF>
where
    T: Token,
    NodeOut: ?Sized,
    Pars: Parser<T, Output = NodeIn> + Grammar<T>,
{
    fn calc_check(&self, context: &ParserContext<T>, pos: &mut usize) -> bool {
        while self.parser.check(context, pos) {}
        true
    }
}

impl<T, NodeIn, NodeOut, Pars, CombF> Parser<T> for MultipleParser<T, NodeIn, NodeOut, Pars, CombF>
where
    T: Token,
    NodeIn: AstNode,
    NodeOut: AstNode,
    Pars: Parser<T, Output = NodeIn> + Grammar<T>,
    CombF: Fn(Vec<NodeIn>) -> NodeOut,
{
    type Output = NodeOut;

    fn parse(
        &self,
        context: Rc<ParserContext<T>>,
        pos: &mut usize,
    ) -> Result<Box<Self::Output>, String> {
        let mut results = Vec::new();
        while self.parser.check_no_advance(&context, pos) {
            results.push(*self.parser.parse(context.clone(), pos)?);
        }
        Ok(Box::new((self.combine_fn)(results)))
    }
}

struct OneOrMoreParser<T, NodeIn, NodeOut, Pars, CombF>
where
    NodeOut: ?Sized,
{
    parser: Pars,
    combine_fn: CombF,
    id: usize,
    _phantom: PhantomData<(T, NodeIn, NodeOut)>,
}

impl<T, NodeIn, NodeOut, Pars, CombF> OneOrMoreParser<T, NodeIn, NodeOut, Pars, CombF>
where
    T: Token,
    NodeIn: AstNode,
    NodeOut: AstNode + ?Sized,
    Pars: Parser<T, Output = NodeIn>,
    CombF: Fn(Vec<NodeIn>) -> NodeOut,
{
    pub fn new(parser: Pars, combine_fn: CombF) -> Self {
        Self {
            parser,
            combine_fn,
            id: get_next_id(),
            _phantom: PhantomData,
        }
    }
}

impl<T, NodeIn, NodeOut, Pars, CombF> HasId for OneOrMoreParser<T, NodeIn, NodeOut, Pars, CombF>
where
    NodeOut: ?Sized,
{
    fn id(&self) -> usize {
        self.id
    }
}

impl<T, NodeIn, NodeOut, Pars, CombF> IsCheckable<T>
    for OneOrMoreParser<T, NodeIn, NodeOut, Pars, CombF>
where
    T: Token,
    NodeOut: ?Sized,
    Pars: Parser<T, Output = NodeIn> + Grammar<T>,
{
    fn calc_check(&self, context: &ParserContext<T>, pos: &mut usize) -> bool {
        if !self.parser.check(context, pos) {
            return false;
        }
        while self.parser.check(context, pos) {}
        true
    }
}

impl<T, NodeIn, NodeOut, Pars, CombF> Parser<T> for OneOrMoreParser<T, NodeIn, NodeOut, Pars, CombF>
where
    T: Token,
    NodeIn: AstNode,
    NodeOut: AstNode,
    Pars: Parser<T, Output = NodeIn> + Grammar<T>,
    CombF: Fn(Vec<NodeIn>) -> NodeOut,
{
    type Output = NodeOut;

    fn parse(
        &self,
        context: Rc<ParserContext<T>>,
        pos: &mut usize,
    ) -> Result<Box<Self::Output>, String> {
        let mut results = Vec::new();
        // First match is mandatory — propagate the error if absent.
        results.push(*self.parser.parse(context.clone(), pos)?);
        // Remaining matches are optional (same as Multiple).
        while self.parser.check_no_advance(&context, pos) {
            results.push(*self.parser.parse(context.clone(), pos)?);
        }
        Ok(Box::new((self.combine_fn)(results)))
    }
}

// impl Parser for all Rc<Parser>
impl<T, N, P> Parser<T> for Rc<P>
where
    T: Token,
    N: AstNode + ?Sized,
    P: Parser<T, Output = N>,
{
    type Output = N;

    fn parse(
        &self,
        context: Rc<ParserContext<T>>,
        pos: &mut usize,
    ) -> Result<Box<Self::Output>, String> {
        (**self).parse(context, pos)
    }
}

// impl Matcher for all Rc<Matcher>
impl<T, N, M> Matcher<T> for Rc<M>
where
    T: Token,
    N: AstNode + ?Sized,
    M: Matcher<T, Output = N>,
{
    type Output = N;

    fn match_pattern(
        &self,
        context: &mut MatcherContext<T, Self::Output>,
        pos: &mut usize,
    ) -> Result<(), String> {
        (**self).match_pattern(context, pos)
    }
}

// impl IsCheckable for all Rc<IsCheckable>
impl<T, C> IsCheckable<T> for Rc<C>
where
    T: Token,
    C: IsCheckable<T>,
{
    fn calc_check(&self, context: &ParserContext<T>, pos: &mut usize) -> bool {
        (**self).calc_check(context, pos)
    }
}

// impl HasId for all Rc<HasId>
impl<H> HasId for Rc<H>
where
    H: HasId,
{
    fn id(&self) -> usize {
        (**self).id()
    }
}

trait MyAstNode: AstNode {}

struct Node {
    value: String,
}
impl AstNode for Node {}
impl MyAstNode for Node {}
#[macro_export]
macro_rules! bind {
    ($($tokens:tt)*) => {
        compile_error!("The `bind!` macro can only be used inside a `capture!` block.");
    };
}
// fn boxed<T: Token, N: AstNode + ?Sized, M: Matcher<T, Output = N> + 'static>(
//     m: M,
// ) -> Box<dyn Matcher<T, Output = N>> {
//     Box::new(m)
// }
#[cfg(test)]
mod tests {
    use macros::capture;

    use super::*;

    #[test]
    fn test_capture_macro() {
        let word_parser = Rc::new(TokenParser::new(
            |token: &char| token.is_alphabetic(),
            |token: &char| {
                Box::new(Node {
                    value: token.to_string(),
                })
            },
        ));
        let digit_parser = Rc::new(TokenParser::new(
            |token: &char| token.is_digit(10),
            |token: &char| {
                Box::new(Node {
                    value: token.to_string(),
                })
            },
        ));

        let number_parser = capture!(
            {
                seq((
                    bind!(digit_parser.clone(), *digits),
                    many(bind!(digit_parser.clone(), *digits)),
                ))
            } => {
                // In scope:
                //   digits: Vec<Box<N>>
                Box::new(Node {
                    value: digits.into_iter().map(|d| d.value).collect(),
                })
            }
        );

        let func_parser = capture!(
            {
                seq((
                    "fn".to_matcher(),
                    bind!(word_parser.clone(), name),
                    "(".to_matcher(),
                    bind!(word_parser.clone(), *params),
                    many(seq((
                        ",".to_matcher(),
                        bind!(word_parser.clone(), *params),
                    ))),
                    ")".to_matcher(),
                    bind!(word_parser.clone(), ?body),
                ))
            } => {
                // In scope:
                //   name:   Box<N>
                //   params: Vec<Box<N>>
                //   body:   Option<Box<N>>
                Box::new(Node {
                    value: format!(
                        "Function: name={}, params=[{}], body={}",
                        name.value,
                        params.into_iter().map(|p| p.value).collect::<Vec<_>>().join(", "),
                        body.map_or("None".to_string(), |b| b.value)
                    ),
                })
            }
        );

        assert_eq!(
            number_parser
                .parse(Rc::new(ParserContext::new(vec!['1', '2', '3'])), &mut 0)
                .unwrap()
                .value,
            "123"
        );

        assert_eq!(
            func_parser
                .parse(
                    Rc::new(ParserContext::new(vec![
                        'f', 'n', ' ', 'x', ' ', '(', 'y', ',', 'z', ')', ' ', 'b', 'o', 'd', 'y'
                    ])),
                    &mut 0
                )
                .unwrap()
                .value,
            "Function: name=x, params=[y, z], body=body"
        );

        /*
                What this should expand to:
                let func_parser = Capture::new::<1, 1, 1, _>(
            // grammar_factory: property arrays destructured into named bindings
            |[name]:   [SingleProperty;   1],
             [params]: [MultipleProperty; 1],
             [body]:   [OptionalProperty; 1]| {
                //                  ↓ `as name`   replaced     ↓ `as *params` replaced (×2, Copy)
                Sequence::new(vec![
                    StringMatcher::new("fn"),
                    CaptureProperty::new(word_parser, name),
                    StringMatcher::new("("),
                    CaptureProperty::new(expression_parser, params),
                    many(
                        Sequence::new(vec![
                            StringMatcher::new(","),
                            CaptureProperty::new(expression_parser, params), // params copied
                        ])
                    ),
                    StringMatcher::new(")"),
                    CaptureProperty::new(block_parser, body),
                    //                             ↑ `as ?body` replaced
                ])
            },
            // constructor: extract → run user block
            |mut __ctx| {
                let name   = __ctx.match_result.single_matches[0]
                    .take()
                    .expect("capture!: single capture `name` was never set\n...");
                let params = ::std::mem::take(&mut __ctx.match_result.multiple_matches[0]);
                let body   = __ctx.match_result.optional_matches[0].take();

                Box::new(FuncDefNode::new(name, params, body))
            },
        );
                 */
    }
}
