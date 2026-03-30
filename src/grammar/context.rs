use std::collections::HashMap;

use crate::grammar::error_handler::ErrorHandler;

pub struct ParserContext<'a, T, EHandler: ErrorHandler> {
    pub tokens: &'a mut Vec<T>,
    pub memo_table: HashMap<(usize, usize), Option<usize>>,
    pub match_start: usize,
    pub error_handler: &'a mut EHandler,
}

impl<'a, T, EHandler: ErrorHandler> ParserContext<'a, T, EHandler> {
    pub fn new(tokens: &'a mut Vec<T>, error_handler: &'a mut EHandler) -> Self {
        Self {
            tokens,
            memo_table: HashMap::new(),
            match_start: 0,
            error_handler,
        }
    }
}

pub trait MatchResultSingle {
    type Properties;
    type Output;
    fn new() -> Self;
    fn new_properties() -> Self::Properties;
    fn as_output(self) -> Self::Output;
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

// pub trait MatchResult {
//     type Single: MatchResultSingle;
//     type Multiple: MatchResultMultiple;
//     type Optional: MatchResultOptional;
//     fn new(
//         match_result_single: Self::Single,
//         match_result_multiple: Self::Multiple,
//         match_result_optional: Self::Optional,
//     ) -> Self;

//     fn single(&mut self) -> &mut Self::Single;
//     fn multiple(&mut self) -> &mut Self::Multiple;
//     fn optional(&mut self) -> &mut Self::Optional;
// }
// pub struct MatchResult<MResSingle, MResMultiple, MResOptional> {
//     pub single: MResSingle,
//     pub multiple: MResMultiple,
//     pub optional: MResOptional,
// }

// impl<MResSingle, MResMultiple, MResOptional> MatchResult<MResSingle, MResMultiple, MResOptional> {
//     pub fn new(
//         match_result_single: MResSingle,
//         match_result_multiple: MResMultiple,
//         match_result_optional: MResOptional,
//     ) -> Self {
//         Self {
//             single: match_result_single,
//             multiple: match_result_multiple,
//             optional: match_result_optional,
//         }
//     }
// }

// impl<MResSingle, MResMultiple, MResOptional> MatchResult
//     for (MResSingle, MResMultiple, MResOptional)
// where
//     MResSingle: MatchResultSingle,
//     MResMultiple: MatchResultMultiple,
//     MResOptional: MatchResultOptional,
// {
//     type Single = MResSingle;
//     type Multiple = MResMultiple;
//     type Optional = MResOptional;

//     fn new(
//         match_result_single: MResSingle,
//         match_result_multiple: MResMultiple,
//         match_result_optional: MResOptional,
//     ) -> Self {
//         (
//             match_result_single,
//             match_result_multiple,
//             match_result_optional,
//         )
//     }

//     fn single(&mut self) -> &mut MResSingle {
//         &mut self.0
//     }

//     fn multiple(&mut self) -> &mut MResMultiple {
//         &mut self.1
//     }

//     fn optional(&mut self) -> &mut MResOptional {
//         &mut self.2
//     }
// }

pub struct MatcherContext<'a, 'ctx, Token, MRes, EHandler: ErrorHandler> {
    pub parser_context: &'a mut ParserContext<'ctx, Token, EHandler>,
    pub match_result: MRes,
}

impl<'a, 'ctx, T, MResSingle, MResMultiple, MResOptional, EHandler: ErrorHandler>
    MatcherContext<'a, 'ctx, T, (MResSingle, MResMultiple, MResOptional), EHandler>
where
    MResSingle: MatchResultSingle,
    MResMultiple: MatchResultMultiple,
    MResOptional: MatchResultOptional,
{
    pub fn new(
        parser_context: &'a mut ParserContext<'ctx, T, EHandler>,
        match_result_single: MResSingle,
        match_result_multiple: MResMultiple,
        match_result_optional: MResOptional,
    ) -> Self {
        let match_result = (
            match_result_single,
            match_result_multiple,
            match_result_optional,
        );
        Self {
            parser_context,
            match_result,
        }
    }
}
