use crate::context::ParserContext;
use crate::error::MatcherRunError;
use crate::error::error_handler::ErrorHandler;
use crate::input::{Input, InputStream};
use crate::parser::internal::ParserImpl;
use crate::parser::{Parser, ParserCombinator, ParserObjSafe};

pub struct Erased<'a, 'src, Inp, Out>
where
    Inp: Input<'src> + 'a,
    Out: 'a,
{
    inner: Box<dyn ParserObjSafe<'src, Inp, Out> + 'a>,
}

impl<'a, 'src, Inp, Out> Clone for Erased<'a, 'src, Inp, Out>
where
    'src: 'a,
    Inp: Input<'src> + 'a,
    Out: 'a,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone_boxed(),
        }
    }
}

impl<'a, 'src, Inp, Out> std::fmt::Debug for Erased<'a, 'src, Inp, Out>
where
    Inp: Input<'src> + 'a,
    Out: 'a,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Erased").finish()
    }
}
impl<'a, 'src, Inp, Out> ParserCombinator for Erased<'a, 'src, Inp, Out>
where
    Inp: Input<'src> + 'a,
    Out: 'a,
{
}
impl<'a, 'src, Inp, Out> ParserImpl<'src, Inp> for Erased<'a, 'src, Inp, Out>
where
    'src: 'a,
    Inp: Input<'src> + 'a,
    Out: 'a,
{
    type Output = Out;
    const CAN_FAIL: bool = true; // conservative; see note below
    fn parse(
        &self,
        context: &mut ParserContext<'src>,
        error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, Inp>,
    ) -> Result<Option<Self::Output>, MatcherRunError> {
        self.inner.parse(context, error_handler.to_choice(), input)
    }

    fn maybe_label(&self) -> Option<Box<dyn std::fmt::Display>> {
        self.inner.maybe_label()
    }
}
/// Boxes `p` behind a trait object whose **self** lifetime equals `'src`.
///
/// We tie `'a = 'src` on [`Erased`] so callers cannot instantiate an erased parser
/// with an unconstrained `'a` lifetime (rustc would otherwise happily infer
/// `'a = 'static`, which incorrectly forces `'src` to outlive `'static` whenever
/// the output type contains `'_`/`'src` placeholders).
pub fn erase<'src, Inp, Out, P>(p: P) -> Erased<'src, 'src, Inp, Out>
where
    Inp: Input<'src> + 'src,
    P: Parser<'src, Inp, Output = Out> + 'src,
{
    Erased { inner: Box::new(p) }
}

