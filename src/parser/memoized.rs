//! Packrat-style memoization of parse results per `(parser_id, input_position)`.
//! Memoized parser outputs need to implement clone. You can achieve this for example by using
//! parser.map_output(Rc::new).memoized()
use core::fmt;
use std::fmt::Display;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::parser::ParserCombinator;
use crate::{
    context::ParserContext,
    error::{MatcherRunError, error_handler::ErrorHandler},
    input::{Input, InputStream},
    parser::Parser,
};

static NEXT_MEMO_ID: AtomicUsize = AtomicUsize::new(0);

/// Wraps parser `P` with output `POut`; successful outputs are cloned across repeated parses at the same position.
#[derive(Clone)]
pub struct Memoized<P, POut> {
    inner: P,
    id: usize,
    _marker: PhantomData<POut>,
}

impl<P, POut> fmt::Debug for Memoized<P, POut>
where
    P: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Memoized").field(&self.inner).finish()
    }
}

impl<P, POut> Memoized<P, POut> {
    /// Assigns a unique memo table id for this wrapper.
    pub fn new(inner: P) -> Self {
        Self {
            inner,
            id: NEXT_MEMO_ID.fetch_add(1, Ordering::Relaxed),
            _marker: PhantomData,
        }
    }
}

impl<P, POut> ParserCombinator for Memoized<P, POut> where P: ParserCombinator {}

impl<'src, Inp: Input<'src>, P, POut> super::internal::ParserImpl<'src, Inp> for Memoized<P, POut>
where
    P: Parser<'src, Inp, Output = POut>,
    Inp: Input<'src>,
    POut: Clone + 'src,
{
    type Output = POut;
    const CAN_FAIL: bool = P::CAN_FAIL;

    #[inline]
    fn parse(
        &self,
        context: &mut ParserContext<'src>,
        error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, Inp>,
    ) -> Result<Option<Self::Output>, MatcherRunError> {
        let pos = input.get_pos().into();

        // AtomicUsize ensures that this parser_id is unique to this parser. This means that this
        // parser_id is only ever used together with this parser and more importantly with the same output type POut.
        // this is why i think this is safe.

        let cached = unsafe { context.cache.get_entry::<Option<POut>>(self.id, pos) };

        if let Some(result) = cached {
            return Ok(result.clone());
        }

        let result = self.inner.parse(context, error_handler, input)?;
        Ok(unsafe {
            context
                .cache
                .set_entry::<Option<POut>>(self.id, pos, result)
                .clone()
        })
    }

    #[inline]
    fn maybe_label(&self) -> Option<Box<dyn Display>> {
        self.inner.maybe_label()
    }
}
