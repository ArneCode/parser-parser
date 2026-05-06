use std::any::Any;
use std::collections::{HashMap, HashSet};

use crate::error::ParserError;
#[cfg(feature = "parser-trace")]
mod trace_impl;
#[cfg(feature = "parser-trace")]
use trace_impl::TraceState;

pub struct ParserContext {
    pub memo_table: HashMap<(usize, usize), Box<dyn Any>>,
    pub error_sink: Vec<ParserError>,
    pub registered_error_set: HashSet<(usize, usize)>,
    pub error_stack: Vec<ParserError>,
    #[cfg(feature = "parser-trace")]
    trace: Option<TraceState>,
}

impl ParserContext {
    pub fn new() -> Self {
        Self {
            memo_table: HashMap::new(),
            error_sink: Vec::new(),
            registered_error_set: HashSet::new(),
            error_stack: Vec::new(),
            #[cfg(feature = "parser-trace")]
            trace: None,
        }
    }

    pub fn get_errors(mut self) -> Vec<ParserError> {
        // return combined errors from error_sink and error_stack
        self.error_sink.extend(self.error_stack);
        self.error_sink
    }

    pub fn push_stack_error(&mut self, error: ParserError) {
        self.error_stack.push(error);
    }
}
