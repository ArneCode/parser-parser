use std::{cell::RefCell, collections::HashMap, ops::Deref, rc::Rc};

pub struct ParserContext<T> {
    pub tokens: Vec<T>,
    pub memo_table: RefCell<HashMap<(usize, usize), Option<usize>>>,
}

impl<T> ParserContext<T> {
    pub fn new(tokens: Vec<T>) -> Self {
        Self {
            tokens,
            memo_table: RefCell::new(HashMap::new()),
        }
    }
}

pub trait MatchResultSingle {
    type Properties;
    fn new() -> Self;
    fn new_properties() -> Self::Properties;
}
pub trait MatchResultMultiple {
    type Properties;
    fn new() -> Self;
    fn new_properties() -> Self::Properties;
}
pub trait MatchResultOptional {
    type Properties;
    fn new() -> Self;
    fn new_properties() -> Self::Properties;
}

pub struct MatcherContext<T, MResSingle, MResMultiple, MResOptional> {
    pub parser_context: Rc<ParserContext<T>>,
    pub match_result_single: MResSingle,
    pub match_result_multiple: MResMultiple,
    pub match_result_optional: MResOptional,
}

impl<T, MResSingle, MResMultiple, MResOptional> Deref
    for MatcherContext<T, MResSingle, MResMultiple, MResOptional>
where
    MResSingle: MatchResultSingle,
    MResMultiple: MatchResultMultiple,
    MResOptional: MatchResultOptional,
{
    type Target = ParserContext<T>;

    fn deref(&self) -> &Self::Target {
        &self.parser_context
    }
}

impl<T, MResSingle, MResMultiple, MResOptional>
    MatcherContext<T, MResSingle, MResMultiple, MResOptional>
where
    MResSingle: MatchResultSingle,
    MResMultiple: MatchResultMultiple,
    MResOptional: MatchResultOptional,
{
    pub fn new(
        parser_context: Rc<ParserContext<T>>,
        match_result_single: MResSingle,
        match_result_multiple: MResMultiple,
        match_result_optional: MResOptional,
    ) -> Self {
        Self {
            parser_context,
            match_result_single,
            match_result_multiple,
            match_result_optional,
        }
    }
}
