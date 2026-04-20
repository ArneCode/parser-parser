//! Packrat-style memoization of parse results per `(parser_id, input_position)`.

use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::{
    context::ParserContext,
    error::{FurthestFailError, error_handler::ErrorHandler},
    input::{Input, InputStream},
    parser::Parser,
};

static NEXT_MEMO_ID: AtomicUsize = AtomicUsize::new(0);

/// Wraps parser `P`; successful outputs are shared as [`Rc`] across repeated parses at the same position.
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

impl<'src, Inp: Input<'src>, P, POut> super::internal::ParserImpl<'src, Inp> for Memoized<P>
where
    P: Parser<'src, Inp, Output = POut>,
    Inp: Input<'src>,
    POut: 'static,
{
    type Output = Rc<POut>;
    const CAN_FAIL: bool = P::CAN_FAIL;

    fn parse(
        &self,
        context: &mut ParserContext,
        error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, Inp>,
    ) -> Result<Option<Self::Output>, FurthestFailError> {
        let pos = input.get_pos();
        let key = (self.id, pos.clone().into());

        if let Some(entry) = context.memo_table.get(&key) {
            return match entry
                .downcast_ref::<Option<(Rc<POut>, usize)>>()
                .expect("memo table entry type mismatch")
            {
                None => Ok(None),
                Some((rc, new_pos)) => {
                    while input.get_pos().into() < *new_pos {
                        if input.next().is_none() {
                            break;
                        }
                    }
                    Ok(Some(Rc::clone(rc)))
                }
            };
        }

        match self.inner.parse(context, error_handler, input) {
            Ok(None) => {
                context
                    .memo_table
                    .insert(key, Box::new(None::<(Rc<POut>, usize)>));
                Ok(None)
            }
            Ok(Some(output)) => {
                let rc = Rc::new(output);
                let new_pos: usize = input.get_pos().into();
                context
                    .memo_table
                    .insert(key, Box::new(Some((Rc::clone(&rc), new_pos))));
                Ok(Some(rc))
            }
            Err(e) => Err(e),
        }
    }
}
