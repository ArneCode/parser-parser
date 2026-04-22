use std::any::Any;
use std::collections::{HashMap, HashSet};

use crate::error::ParserError;

pub struct ParserContext {
    pub memo_table: HashMap<(usize, usize), Box<dyn Any>>,
    pub error_sink: Vec<ParserError>,
    pub registered_error_set: HashSet<(usize, usize)>,
}

impl ParserContext {
    pub fn new() -> Self {
        Self {
            memo_table: HashMap::new(),
            error_sink: Vec::new(),
            registered_error_set: HashSet::new(),
        }
    }

    pub fn get_errors(self) -> Vec<ParserError> {
        self.error_sink
    }
}
