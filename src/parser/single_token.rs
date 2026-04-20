//! Exact single-token parsers: a literal value or a [`char`] literal.

use std::fmt::Debug;

use crate::{
    context::ParserContext,
    error::{FurthestFailError, error_handler::ErrorHandler},
    input::{InputFamily, InputStream},
};

/// Matches `token` and advances by one on success.
pub struct SingleTokenParser<Token> {
    token: Token,
}

impl<Token> SingleTokenParser<Token> {
    /// Parser for one occurrence of `token`.
    pub fn new(token: Token) -> Self {
        Self { token }
    }
}

// impl<Token> HasId for SingleTokenParser<Token> {
//     fn id(&self) -> usize {
//         self.id
//     }
// }

// impl<Token: PartialEq> IsCheckable<Token> for SingleTokenParser<Token> {
//     fn calc_check(
//         &self,
//         context: &mut ParserContext<Token, impl ErrorHandler>,
//         pos: &mut usize,
//     ) -> bool {
//         if *pos < context.tokens.len() {
//             let token = &context.tokens[*pos];
//             if token == &self.token {
//                 *pos += 1; // Advance position on success
//                 return true;
//             }
//         }
//         false
//     }
// }

// impl<Token: PartialEq + Clone + Debug> Parser<Token> for SingleTokenParser<Token> {
//     type Output = Token;

//     fn parse(
//         &self,
//         context: &mut ParserContext<Token, impl ErrorHandler>,
//         pos: &mut usize,
//     ) -> Result<Self::Output, String> {
//         if self.calc_check(context, pos) {
//             Ok(self.token.clone())
//         } else {
//             Err(format!(
//                 "Expected token {:?} at position {}, but found {:?}",
//                 self.token,
//                 *pos,
//                 context.tokens.get(*pos)
//             ))
//         }
//     }
// }
impl<InpFam, Token: PartialEq + Clone + Debug> super::internal::ParserImpl<InpFam>
    for SingleTokenParser<Token>
where
    InpFam: InputFamily + ?Sized,
    for<'src> InpFam::In<'src>: crate::input::Input<'src, Token = Token>,
{
    type Output<'src> = Token;
    const CAN_FAIL: bool = true;
    fn parse<'src>(
        &self,
        _context: &mut ParserContext,
        _error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, InpFam::In<'src>>,
    ) -> Result<Option<Self::Output<'src>>, FurthestFailError> {
        let start = input.get_pos();
        if let Some(token) = input.next()
            && token == self.token
        {
            return Ok(Some(self.token.clone()));
        }
        input.set_pos(start);
        Ok(None)
    }
}

// impl<Token, Label> MaybeLabel<Label> for SingleTokenParser<Token> {}

impl<InpFam> super::internal::ParserImpl<InpFam> for char
where
    InpFam: InputFamily + ?Sized,
    for<'src> InpFam::In<'src>: crate::input::Input<'src, Token = char>,
{
    type Output<'src> = char;
    const CAN_FAIL: bool = true;

    fn parse<'src>(
        &self,
        _context: &mut ParserContext,
        _error_handler: &mut impl ErrorHandler,
        input: &mut InputStream<'src, InpFam::In<'src>>,
    ) -> Result<Option<Self::Output<'src>>, FurthestFailError> {
        let start = input.get_pos();
        if let Some(token) = input.next()
            && token == *self
        {
            return Ok(Some(*self));
        }
        input.set_pos(start);
        Ok(None)
    }
}
