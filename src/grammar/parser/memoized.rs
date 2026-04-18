use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::grammar::{
    context::ParserContext,
    error::{FurthestFailError, error_handler::ErrorHandler},
    parser::Parser,
};

static NEXT_MEMO_ID: AtomicUsize = AtomicUsize::new(0);

pub struct Memoized<P> {
    inner: P,
    id: usize,
}

impl<P> Memoized<P> {
    pub fn new(inner: P) -> Self {
        Self {
            inner,
            id: NEXT_MEMO_ID.fetch_add(1, Ordering::Relaxed),
        }
    }
}

impl<Token, P> Parser<Token> for Memoized<P>
where
    P: Parser<Token>,
    P::Output: 'static,
{
    type Output = Rc<P::Output>;
    const CAN_FAIL: bool = P::CAN_FAIL;

    fn parse<'ctx>(
        &self,
        context: &mut ParserContext<'ctx, Token>,
        error_handler: &mut impl ErrorHandler,
        pos: &mut usize,
    ) -> Result<Option<Self::Output>, FurthestFailError> {
        let key = (self.id, *pos);

        if let Some(entry) = context.memo_table.get(&key) {
            return match entry
                .downcast_ref::<Option<(Rc<P::Output>, usize)>>()
                .expect("memo table entry type mismatch")
            {
                None => Ok(None),
                Some((rc, new_pos)) => {
                    *pos = *new_pos;
                    Ok(Some(Rc::clone(rc)))
                }
            };
        }

        match self.inner.parse(context, error_handler, pos) {
            Ok(None) => {
                context
                    .memo_table
                    .insert(key, Box::new(None::<(Rc<P::Output>, usize)>));
                Ok(None)
            }
            Ok(Some(output)) => {
                let rc = Rc::new(output);
                let new_pos = *pos;
                context
                    .memo_table
                    .insert(key, Box::new(Some((Rc::clone(&rc), new_pos))));
                Ok(Some(rc))
            }
            Err(e) => Err(e),
        }
    }
}
