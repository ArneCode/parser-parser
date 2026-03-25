use std::{cell::RefCell, collections::HashMap, ops::Deref, rc::Rc};

use crate::grammar::{AstNode, Token};

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
