use std::any::Any;
use std::collections::HashMap;

use crate::error::ParserError;

pub struct ParserContext<'a, T> {
    pub tokens: &'a Vec<T>,
    pub memo_table: HashMap<(usize, usize), Box<dyn Any>>,
    pub error_sink: Vec<ParserError>,
}

impl<'a, T> ParserContext<'a, T> {
    pub fn new(tokens: &'a Vec<T>) -> Self {
        Self {
            tokens,
            memo_table: HashMap::new(),
            error_sink: Vec::new(),
        }
    }

    pub fn get_errors(self) -> Vec<ParserError> {
        self.error_sink
    }
}
