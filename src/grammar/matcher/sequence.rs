use crate::grammar::{
    error_handler::{ErrorHandler, ParserError},
    matcher::{MatchRunner, Matcher},
};
// pub struct Sequence<Tuple> {
//     elements: Tuple,
// }
// impl<Tuple> Sequence<Tuple> {
//     fn new(elements: Tuple) -> Self {
//         Self { elements }
//     }
// }

// pub fn seq<Tuple>(elements: Tuple) -> Sequence<Tuple> {
//     Sequence::new(elements)
// }
macro_rules! impl_matcher_for_seq_tuples {
    () => {};
    ($head:ident $(,$tail:ident)*) => {
        impl<'a, 'ctx, Runner, $head, $($tail),*> Matcher<Runner> for ($head, $($tail,)*)
        where
            $head: Matcher<Runner>,
            $($tail: Matcher<Runner>,)*
            Runner: MatchRunner<'a, 'ctx>,
        {
            const CAN_MATCH_DIRECTLY: bool = {
                if !($head::CAN_MATCH_DIRECTLY $(&& $tail::CAN_MATCH_DIRECTLY)*) {
                    false
                } else {
                    let has_props = [$head::HAS_PROPERTY, $($tail::HAS_PROPERTY),*];
                    let can_fail = [$head::CAN_FAIL, $($tail::CAN_FAIL),*];

                    let mut i = 0;
                    // find first element with property
                    while i < has_props.len() {
                        if has_props[i] {
                            break;
                        }
                        i += 1;
                    }
                    // check that all elements after it can not fail
                    i += 1;
                    let mut can_fail_after_prop = false;
                    while i < can_fail.len() {
                        if can_fail[i] {
                            can_fail_after_prop = true;
                            break;
                        }
                        i += 1;
                    }
                    !can_fail_after_prop
                }
            };
            const HAS_PROPERTY: bool = $head::HAS_PROPERTY  $(|| $tail::HAS_PROPERTY)*;
            const CAN_FAIL: bool = $head::CAN_FAIL  $(|| $tail::CAN_FAIL)*;

            fn match_with_runner(&self, runner: &mut Runner, error_handler: &mut impl ErrorHandler, pos: &mut usize) -> Result<bool, ParserError> {

                #[allow(non_snake_case)]
                let ($head, $($tail,)*) = &self;

                if !runner.run_match($head, error_handler, pos)? {
                    return Ok(false);
                }

                $(
                    if !runner.run_match($tail, error_handler, pos)? {
                        return Ok(false);
                    }
                )*

                Ok(true)
            }
        }



        impl_matcher_for_seq_tuples!($($tail),*);
    };
}

impl_matcher_for_seq_tuples!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12);
