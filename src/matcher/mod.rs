//! Matchers: predicates and grammar fragments used inside [`crate::parser::capture::Capture`]
//! and related runners.
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
//! `CAN_FAIL` does **not** indicate whether `Err` with [`crate::error::FurthestFailError`] is possible.

pub mod any_token;
pub mod commit_matcher;
pub mod error_contextualizer;
pub mod ignore_result;
pub mod insert_on_error;
pub mod multiple;
pub mod negative_lookahead;
pub mod one_or_more;
pub mod optional;
pub mod parser_matcher;
pub mod positive_lookahead;
pub(crate) mod runner;
pub mod sequence;
pub mod string;
pub mod unwanted;

pub use any_token::AnyToken;
pub use commit_matcher::{CommitMatcher, commit_on};
pub use error_contextualizer::ErrorContextualizer;
pub use insert_on_error::InsertOnErrorMatcher;
pub use multiple::{Multiple, many};
pub use negative_lookahead::{NegativeLookahead, negative_lookahead};
pub use one_or_more::{OneOrMore, one_or_more};
pub use optional::{Optional, optional};
pub use parser_matcher::ParserMatcher;
pub use positive_lookahead::{PositiveLookahead, positive_lookahead};
pub(crate) use runner::{DirectMatchRunner, MatchRunner, NoMemoizeBacktrackingRunner};
pub use string::StringMatcher;

use std::{fmt::Display, ops::Deref, rc::Rc};

use crate::{
    error::{FurthestFailError, error_handler::ErrorHandler},
    input::{Input, InputStream},
};

pub(crate) mod internal {
    use std::fmt::Display;

    use crate::{
        error::{FurthestFailError, error_handler::ErrorHandler},
        input::{Input, InputStream},
        matcher::runner::MatchRunner,
    };

    /// Crate-private matching interface used by [`super::Matcher`].
    pub trait MatcherImpl<'src, Inp, MRes>
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
        /// `Err(FurthestFailError)` may be returned.
        const CAN_FAIL: bool;

        /// Run this matcher via `runner`, updating `pos` and possibly `MRes` on success.
        fn match_with_runner<'a, Runner>(
            &'a self,
            runner: &mut Runner,
            error_handler: &mut impl ErrorHandler,
            input: &mut InputStream<'src, Inp>,
        ) -> Result<bool, FurthestFailError>
        where
            Runner: MatchRunner<'a, 'src, Inp, MRes = MRes>,
            'src: 'a;

        fn maybe_label(&self) -> Option<Box<dyn Display>> {
            None
        }
    }
}

/// Facade for matchers over `Token` that read and write match state into `MRes`.
///
/// `MRes` is usually the capture bucket type in [`crate::parser::capture::Capture`].
/// Blanket-implemented for all types that implement the crate-private matcher implementation trait.
pub trait Matcher<'src, Inp: Input<'src>, MRes>: internal::MatcherImpl<'src, Inp, MRes> {}

pub trait MatcherCombinator {
    /// Wrap this matcher so that on furthest-failure, `error_parser` runs to attach diagnostics.
    fn add_error_info<Pars>(self, error_parser: Pars) -> ErrorContextualizer<Self, Pars>
    where
        Self: Sized,
    {
        ErrorContextualizer::new(self, error_parser)
    }

    /// If the matcher fails to extend the furthest error, insert `message` into that error.
    fn try_insert_if_missing<M: Display>(self, message: M) -> InsertOnErrorMatcher<Self>
    where
        Self: Sized,
    {
        InsertOnErrorMatcher {
            inner: self,
            message: message.to_string(),
        }
    }
}

impl<'src, Inp: Input<'src>, MRes, M> Matcher<'src, Inp, MRes> for M where
    M: internal::MatcherImpl<'src, Inp, MRes>
{
}

impl<'src, Inp: Input<'src>, MRes, Inner> internal::MatcherImpl<'src, Inp, MRes> for &Inner
where
    Inner: Matcher<'src, Inp, MRes>,
{
    const CAN_MATCH_DIRECTLY: bool = Inner::CAN_MATCH_DIRECTLY;
    const HAS_PROPERTY: bool = Inner::HAS_PROPERTY;
    const CAN_FAIL: bool = Inner::CAN_FAIL;

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
        (**self).match_with_runner(runner, error_handler, input)
    }
}

impl<'src, Inp: Input<'src>, MRes, Inner> internal::MatcherImpl<'src, Inp, MRes> for Rc<Inner>
where
    Inner: Matcher<'src, Inp, MRes>,
{
    const CAN_MATCH_DIRECTLY: bool = Inner::CAN_MATCH_DIRECTLY;
    const HAS_PROPERTY: bool = Inner::HAS_PROPERTY;
    const CAN_FAIL: bool = Inner::CAN_FAIL;

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
        self.deref().match_with_runner(runner, error_handler, input)
    }
}

impl<'src, Inp: Input<'src>, MRes, Inner> internal::MatcherImpl<'src, Inp, MRes> for Box<Inner>
where
    Inner: Matcher<'src, Inp, MRes>,
{
    const CAN_MATCH_DIRECTLY: bool = Inner::CAN_MATCH_DIRECTLY;
    const HAS_PROPERTY: bool = Inner::HAS_PROPERTY;
    const CAN_FAIL: bool = Inner::CAN_FAIL;

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
        (**self).match_with_runner(runner, error_handler, input)
    }
}

impl<'src, Inp: Input<'src>, MRes> internal::MatcherImpl<'src, Inp, MRes> for () {
    const CAN_MATCH_DIRECTLY: bool = true;
    const HAS_PROPERTY: bool = false;
    const CAN_FAIL: bool = false;

    fn match_with_runner<'a, Runner>(
        &'a self,
        _runner: &mut Runner,
        _error_handler: &mut impl ErrorHandler,
        _input: &mut InputStream<'src, Inp>,
    ) -> Result<bool, FurthestFailError>
    where
        Runner: MatchRunner<'a, 'src, Inp, MRes = MRes>,
        'src: 'a,
    {
        Ok(true)
    }
}
