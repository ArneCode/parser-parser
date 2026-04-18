use std::any::Any;
use std::collections::HashMap;

use crate::grammar::{
    error::{FurthestFailError, ParserError},
    parser::Parser,
};

pub struct ParserContext<'a, T> {
    pub tokens: &'a Vec<T>,
    pub memo_table: HashMap<(usize, usize), Box<dyn Any>>,
    pub match_start: usize,
    pub error_sink: Vec<ParserError>,
}

impl<'a, T> ParserContext<'a, T> {
    pub fn new(tokens: &'a Vec<T>) -> Self {
        Self {
            tokens,
            memo_table: HashMap::new(),
            match_start: 0,
            error_sink: Vec::new(),
        }
    }
    pub fn get_errors(self) -> Vec<ParserError> {
        self.error_sink
    }
}

pub trait MatchResultSingle {
    type Properties;
    type Output;
    fn new() -> Self;
    fn new_properties() -> Self::Properties;
    fn to_output(self) -> Self::Output;
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

pub trait MatchResult {
    type Single: MatchResultSingle;
    type Multiple: MatchResultMultiple;
    type Optional: MatchResultOptional;
    fn new(
        match_result_single: Self::Single,
        match_result_multiple: Self::Multiple,
        match_result_optional: Self::Optional,
    ) -> Self;
    fn new_empty() -> Self
    where
        Self: Sized,
    {
        Self::new(
            Self::Single::new(),
            Self::Multiple::new(),
            Self::Optional::new(),
        )
    }

    fn single(&mut self) -> &mut Self::Single;
    fn multiple(&mut self) -> &mut Self::Multiple;
    fn optional(&mut self) -> &mut Self::Optional;
}

impl<MResSingle, MResMultiple, MResOptional> MatchResult
    for (MResSingle, MResMultiple, MResOptional)
where
    MResSingle: MatchResultSingle,
    MResMultiple: MatchResultMultiple,
    MResOptional: MatchResultOptional,
{
    type Single = MResSingle;
    type Multiple = MResMultiple;
    type Optional = MResOptional;

    fn new(
        match_result_single: MResSingle,
        match_result_multiple: MResMultiple,
        match_result_optional: MResOptional,
    ) -> Self {
        (
            match_result_single,
            match_result_multiple,
            match_result_optional,
        )
    }

    fn single(&mut self) -> &mut MResSingle {
        &mut self.0
    }

    fn multiple(&mut self) -> &mut MResMultiple {
        &mut self.1
    }

    fn optional(&mut self) -> &mut MResOptional {
        &mut self.2
    }
}
