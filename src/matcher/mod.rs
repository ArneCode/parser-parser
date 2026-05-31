//! Matchers: grammar fragments used inside [`crate::parser::capture::Capture`] (via [`crate::capture`])
//! and related runners.
//!
//! # For users
//!
//! - Matchers describe **structure**: sequences (`(a, b)`), [`crate::one_of::one_of`], repetition
//!   ([`multiple::many`], [`one_or_more()`], [`optional()`]), lookahead ([`positive_lookahead()`],
//!   [`negative_lookahead()`]), and [`commit_on()`] for committed sub-rules.
//! - They are composed with parsers through [`crate::capture`]; see [`crate::guide::capture_and_binds`].
//! - Extend matchers with [`MatcherCombinator`] (`with_label`, `try_insert_if_missing`, `unwanted`, …).
//!
//! Concept guides: [`crate::guide::core_concepts`], [`crate::guide::errors_and_recovery`],
//! [`crate::guide::common_patterns`].
//!
//! # Parameters and sealing
//!
//! [`Matcher`] is parameterized by `MRes`, the “match result” type that captures
//! bound values and spans (typically a tuple bucket produced by [`crate::parser::capture::Capture`]).
//! You compose matchers rather than implementing [`Matcher`] yourself: it extends a
//! crate-private implementation trait (not public API).
//!
//! ## Associated constants
//!
//! - `CAN_FAIL` — may return `Ok(false)` on a normal path (no match).
//! - `HAS_PROPERTY` — may write into `MRes` (captures).
//! - `CAN_MATCH_DIRECTLY` — optimization hint for the runner.
//!
//! `CAN_FAIL` does **not** indicate whether `Err` with [`crate::error::MatcherRunError`] is possible.

pub mod any_token;
pub mod commit_matcher;
pub mod err_if;
pub mod error_contextualizer;
pub mod if_error;
/// Parser-as-matcher adapters that discard parser output.
pub mod ignore_result;
pub mod multiple;
pub mod negative_lookahead;
/// Helpers for “match any token except …” patterns.
pub mod none_of;
pub mod one_or_more;
pub mod optional;
pub mod parser_matcher;
pub mod positive_lookahead;
pub(crate) mod runner;
pub mod sequence;
pub mod string;
/// Matcher-to-parser adapters that return a fixed output.
pub mod to_parser;
pub use crate::error::MatcherRunError;
pub use any_token::AnyToken;
pub use commit_matcher::{CommitMatcher, commit_on};
pub use err_if::{
    ErrIfMatchedMatcher, ErrIfNoMatchMatcher, err_if_matched, err_if_no_match,
    try_insert_if_missing, unwanted,
};
pub use error_contextualizer::ErrorContextualizer;
pub use multiple::{Multiple, many};
pub use negative_lookahead::{NegativeLookahead, negative_lookahead};
pub use one_or_more::{OneOrMore, one_or_more};
pub use optional::{Optional, optional};
pub use parser_matcher::ParserMatcher;
pub use positive_lookahead::{PositiveLookahead, positive_lookahead};
pub(crate) use runner::{DirectMatchRunner, MatchRunner, NoMemoizeBacktrackingRunner};
pub use string::StringMatcher;
pub use to_parser::ToParser;

use std::{fmt::Display, ops::Deref, rc::Rc};

use crate::{
    error::{MissingSyntax, UnwantedSyntax, error_handler::ErrorHandler},
    input::{Input, InputStream},
    mode::Mode,
};

pub(crate) mod internal {
    use std::fmt::{Debug, Display};

    use crate::{
        error::{MatcherRunError, error_handler::ErrorHandler},
        input::{Input, InputStream},
        matcher::{MatcherCombinator, runner::MatchRunner},
        mode::Mode,
    };

    /// Crate-private matching interface used by [`super::Matcher`].
    pub trait MatcherImpl<'src, Inp, MRes>: Debug + MatcherCombinator + Clone
    where
        Inp: Input<'src>,
    {
        /// `true` when matching can run directly into the active result buffer
        /// without temporary backtracking storage.
        const CAN_MATCH_DIRECTLY: bool;
        /// Same as `CAN_MATCH_DIRECTLY` but evaluated under the assumption that
        /// sub-matchers do not return `false`.
        const CAN_MATCH_DIRECTLY_ASSUMING_NO_FAIL: bool = Self::CAN_MATCH_DIRECTLY;
        /// `true` when this matcher may write properties/results to `MRes`.
        const HAS_PROPERTY: bool;
        /// `true` when this matcher can return `Ok(false)` on a normal match path.
        ///
        /// This constant models match absence and does not indicate whether
        /// `Err([`MatcherRunError`])` may be returned.
        const CAN_FAIL: bool;

        /// Run this matcher via `runner`, updating `pos` and possibly `MRes` on success.
        fn match_with_runner<'a, Runner, M: Mode>(
            &'a self,
            runner: &mut Runner,
            error_handler: &mut impl ErrorHandler,
            input: &mut InputStream<'src, Inp>,
        ) -> Result<bool, MatcherRunError>
        where
            Runner: MatchRunner<'a, 'src, Inp, MRes = MRes>,
            'src: 'a;

        #[inline]
        fn maybe_label(&self) -> Option<Box<dyn Display>> {
            None
        }
    }
}

/// Facade for matchers over `Token` that read and write match state into `MRes`.
///
/// `MRes` is usually the capture bucket type in [`crate::parser::capture::Capture`].
/// For how binds populate `MRes`, see [`crate::guide::capture_and_binds`].
///
/// Blanket-implemented for all types that implement the crate-private matcher implementation trait.
pub trait Matcher<'src, Inp: Input<'src>, MRes>: internal::MatcherImpl<'src, Inp, MRes> {}

/// Extension methods for matchers (labels, missing-token diagnostics, furthest-failure enrichment).
///
/// See [`crate::guide::errors_and_recovery`] for `try_insert_if_missing`, `unwanted`, and `if_error`.
pub trait MatcherCombinator {
    /// Wrap this matcher so that on furthest-failure, `error_parser` runs to attach diagnostics.
    #[cfg(feature = "parser-trace")]
    #[track_caller]
    fn add_error_info<Pars>(self, error_parser: Pars) -> ErrorContextualizer<Self, Pars>
    where
        Self: Sized,
    {
        ErrorContextualizer::new(self, error_parser)
    }

    /// Wrap this matcher so that on furthest-failure, `error_parser` runs to attach diagnostics.
    #[cfg(not(feature = "parser-trace"))]
    fn add_error_info<Pars>(self, error_parser: Pars) -> ErrorContextualizer<Self, Pars>
    where
        Self: Sized,
    {
        ErrorContextualizer::new(self, error_parser)
    }

    /// Emit an inline diagnostic if this matcher does not match and error handling is active.
    fn err_if_no_match<F>(self, factory: F) -> ErrIfNoMatchMatcher<Self, F>
    where
        Self: Sized,
    {
        ErrIfNoMatchMatcher::new(self, factory)
    }

    /// If the matcher does not match, record an [`crate::error::InlineError`] (missing syntax)
    /// when error handling is active — built on [`err_if_no_match`].
    fn try_insert_if_missing<M: Display>(
        self,
        message: M,
    ) -> ErrIfNoMatchMatcher<Self, MissingSyntax>
    where
        Self: Sized,
    {
        self.err_if_no_match(MissingSyntax(message.to_string()))
    }

    /// Emit an inline diagnostic when this matcher matches.
    fn err_if_matched<F>(self, factory: F) -> ErrIfMatchedMatcher<Self, F>
    where
        Self: Sized,
    {
        ErrIfMatchedMatcher::new(self, factory)
    }

    /// Emit an inline diagnostic when this matcher matches (unwanted syntax) — built on [`err_if_matched`].
    fn unwanted<M: Display>(self, message: M) -> ErrIfMatchedMatcher<Self, UnwantedSyntax>
    where
        Self: Sized,
    {
        self.err_if_matched(UnwantedSyntax(message.to_string()))
    }

    /// Convert this matcher into a parser that returns `output` when the matcher succeeds.
    ///
    /// This is a compact alternative to `capture!(matcher => output)` for grammar pieces
    /// where the matched text is not needed. The output is cloned on each successful parse.
    fn to<Output>(self, output: Output) -> ToParser<Self, Output>
    where
        Self: Sized,
    {
        ToParser::new(self, output)
    }
}

impl<'src, Inp: Input<'src>, MRes, M> Matcher<'src, Inp, MRes> for M where
    M: internal::MatcherImpl<'src, Inp, MRes>
{
}

impl<Inner> MatcherCombinator for &Inner where Inner: MatcherCombinator {}

impl<'src, Inp: Input<'src>, MRes, Inner> internal::MatcherImpl<'src, Inp, MRes> for &Inner
where
    Inner: Matcher<'src, Inp, MRes>,
{
    const CAN_MATCH_DIRECTLY: bool = Inner::CAN_MATCH_DIRECTLY;
    const HAS_PROPERTY: bool = Inner::HAS_PROPERTY;
    const CAN_FAIL: bool = Inner::CAN_FAIL;

    #[inline]
    fn match_with_runner<'a, Runner, M: Mode>(
        &'a self,
        runner: &mut Runner,
        error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, Inp>,
    ) -> Result<bool, MatcherRunError>
    where
        Runner: MatchRunner<'a, 'src, Inp, MRes = MRes>,
        'src: 'a,
    {
        (**self).match_with_runner::<Runner, M>(runner, error_handler, input)
    }

    #[inline]
    fn maybe_label(&self) -> Option<Box<dyn std::fmt::Display>> {
        (**self).maybe_label()
    }
}

impl<Inner> MatcherCombinator for Rc<Inner> where Inner: MatcherCombinator {}

impl<'src, Inp: Input<'src>, MRes, Inner> internal::MatcherImpl<'src, Inp, MRes> for Rc<Inner>
where
    Inner: Matcher<'src, Inp, MRes>,
{
    const CAN_MATCH_DIRECTLY: bool = Inner::CAN_MATCH_DIRECTLY;
    const HAS_PROPERTY: bool = Inner::HAS_PROPERTY;
    const CAN_FAIL: bool = Inner::CAN_FAIL;

    #[inline]
    fn match_with_runner<'a, Runner, M: Mode>(
        &'a self,
        runner: &mut Runner,
        error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, Inp>,
    ) -> Result<bool, MatcherRunError>
    where
        Runner: MatchRunner<'a, 'src, Inp, MRes = MRes>,
        'src: 'a,
    {
        self.deref()
            .match_with_runner::<Runner, M>(runner, error_handler, input)
    }

    #[inline]
    fn maybe_label(&self) -> Option<Box<dyn std::fmt::Display>> {
        self.deref().maybe_label()
    }
}

impl<Inner> MatcherCombinator for Box<Inner> where Inner: MatcherCombinator {}

impl<'src, Inp: Input<'src>, MRes, Inner> internal::MatcherImpl<'src, Inp, MRes> for Box<Inner>
where
    Inner: Matcher<'src, Inp, MRes>,
{
    const CAN_MATCH_DIRECTLY: bool = Inner::CAN_MATCH_DIRECTLY;
    const HAS_PROPERTY: bool = Inner::HAS_PROPERTY;
    const CAN_FAIL: bool = Inner::CAN_FAIL;

    #[inline]
    fn match_with_runner<'a, Runner, M: Mode>(
        &'a self,
        runner: &mut Runner,
        error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, Inp>,
    ) -> Result<bool, MatcherRunError>
    where
        Runner: MatchRunner<'a, 'src, Inp, MRes = MRes>,
        'src: 'a,
    {
        (**self).match_with_runner::<Runner, M>(runner, error_handler, input)
    }

    #[inline]
    fn maybe_label(&self) -> Option<Box<dyn std::fmt::Display>> {
        (**self).maybe_label()
    }
}

impl MatcherCombinator for () {}

impl<'src, Inp: Input<'src>, MRes> internal::MatcherImpl<'src, Inp, MRes> for () {
    const CAN_MATCH_DIRECTLY: bool = true;
    const HAS_PROPERTY: bool = false;
    const CAN_FAIL: bool = false;

    #[inline(always)]
    fn match_with_runner<'a, Runner, M: Mode>(
        &'a self,
        _runner: &mut Runner,
        _error_handler: &mut impl ErrorHandler,
        _input: &mut InputStream<'src, Inp>,
    ) -> Result<bool, MatcherRunError>
    where
        Runner: MatchRunner<'a, 'src, Inp, MRes = MRes>,
        'src: 'a,
    {
        Ok(true)
    }
}
