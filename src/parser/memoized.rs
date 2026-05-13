//! Packrat-style memoization of parse results per `(parser_id, input_position)`.

use std::fmt::Display;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::parser::ParserCombinator;
use crate::{
    context::ParserContext,
    error::{MatcherRunError, error_handler::ErrorHandler},
    input::{Input, InputStream},
    parser::Parser,
};

static NEXT_MEMO_ID: AtomicUsize = AtomicUsize::new(0);

/// Wraps parser `P`; successful outputs are shared as [`Rc`] across repeated parses at the same position.
#[derive(Clone, Debug)]
pub struct Memoized<P> {
    inner: P,
    id: usize,
}

impl<P> Memoized<P> {
    /// Assigns a unique memo table id for this wrapper.
    pub fn new(inner: P) -> Self {
        Self {
            inner,
            id: NEXT_MEMO_ID.fetch_add(1, Ordering::Relaxed),
        }
    }
}

impl<P> ParserCombinator for Memoized<P> where P: ParserCombinator {}

impl<'src, Inp: Input<'src>, P, POut> super::internal::ParserImpl<'src, Inp> for Memoized<P>
where
    P: Parser<'src, Inp, Output = POut>,
    Inp: Input<'src>,
    POut: 'src,
{
    type Output = Rc<POut>;
    const CAN_FAIL: bool = P::CAN_FAIL;

    fn parse(
        &self,
        context: &mut ParserContext<'src>,
        error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, Inp>,
    ) -> Result<Option<Self::Output>, MatcherRunError> {
        let pos = input.get_pos();
        let key: usize = pos.clone().into();

        match context.memo_store.get_entry::<POut>(self.id, key) {
            None => {}
            Some(None) => return Ok(None),
            Some(Some((rc, new_pos))) => {
                while input.get_pos().into() < new_pos {
                    if input.next().is_none() {
                        break;
                    }
                }
                return Ok(Some(rc));
            }
        }

        match self.inner.parse(context, error_handler, input) {
            Ok(None) => {
                context
                    .memo_store
                    .table_mut::<POut>(self.id)
                    .insert(key, None);
                Ok(None)
            }
            Ok(Some(output)) => {
                let rc = Rc::new(output);
                let new_pos: usize = input.get_pos().into();
                context
                    .memo_store
                    .table_mut::<POut>(self.id)
                    .insert(key, Some((Rc::clone(&rc), new_pos)));
                Ok(Some(rc))
            }
            Err(e) => Err(e),
        }
    }

    fn maybe_label(&self) -> Option<Box<dyn Display>> {
        self.inner.maybe_label()
    }
}
