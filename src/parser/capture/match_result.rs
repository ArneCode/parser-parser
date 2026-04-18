use super::property::{MultipleProperty, OptionalProperty, SingleProperty};

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
        (match_result_single, match_result_multiple, match_result_optional)
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

impl MatchResultSingle for () {
    type Properties = ();
    type Output = ();
    fn new() -> Self {}
    fn new_properties() -> Self::Properties {}
    fn to_output(self) -> Self::Output {}
}

impl MatchResultMultiple for () {
    type Properties = ();
    fn new() -> Self {}
    fn new_properties() -> Self::Properties {}
}

impl MatchResultOptional for () {
    type Properties = ();
    fn new() -> Self {}
    fn new_properties() -> Self::Properties {}
}

fn unwrap_single<T>(option: Option<T>) -> T {
    option.expect("Expected single match result to be set, but it was not")
}

macro_rules! impl_match_results_for_tuple {
    ( $(($T:ident, $idx:tt)),+ ) => {

        impl<$($T),+> MatchResultSingle for ($(Option<$T>,)+)
        {
            type Properties = (
                $(SingleProperty<fn(&mut Self) -> &mut Option<$T>>,)+
            );
            type Output = ($($T,)+);

            fn new() -> Self {
                ($( { let _: std::marker::PhantomData<$T>; None },)+ )
            }

            fn new_properties() -> Self::Properties {
                ($(
                    SingleProperty::new(
                        (|s: &mut Self| -> &mut Option<$T> { &mut s.$idx })
                            as fn(&mut Self) -> &mut Option<$T>,
                    ),
                )+)
            }

            fn to_output(self) -> Self::Output {
                #[allow(non_snake_case)]
                let ($( $T, )+) = self;
                ($(unwrap_single($T),)+)
            }
        }

        impl<$($T),+> MatchResultMultiple for ($(Vec<$T>,)+) {
            type Properties = (
                $(MultipleProperty<fn(&mut Self) -> &mut Vec<$T>>,)+
            );

            fn new() -> Self {
                ($( { let _: std::marker::PhantomData<$T>; Vec::new() },)+ )
            }

            fn new_properties() -> Self::Properties {
                ($(
                    MultipleProperty::new(
                        (|s: &mut Self| -> &mut Vec<$T> { &mut s.$idx })
                            as fn(&mut Self) -> &mut Vec<$T>,
                    ),
                )+)
            }
        }

        impl<$($T),+> MatchResultOptional for ($(Option<$T>,)+) {
            type Properties = (
                $(OptionalProperty<fn(&mut Self) -> &mut Option<$T>>,)+
            );

            fn new() -> Self {
                ($( { let _: std::marker::PhantomData<$T>; None },)+ )
            }

            fn new_properties() -> Self::Properties {
                ($(
                    OptionalProperty::new(
                        (|s: &mut Self| -> &mut Option<$T> { &mut s.$idx })
                            as fn(&mut Self) -> &mut Option<$T>,
                    ),
                )+)
            }
        }
    };
}

impl_match_results_for_tuple!((T0, 0));
impl_match_results_for_tuple!((T0, 0), (T1, 1));
impl_match_results_for_tuple!((T0, 0), (T1, 1), (T2, 2));
impl_match_results_for_tuple!((T0, 0), (T1, 1), (T2, 2), (T3, 3));
impl_match_results_for_tuple!((T0, 0), (T1, 1), (T2, 2), (T3, 3), (T4, 4));
impl_match_results_for_tuple!((T0, 0), (T1, 1), (T2, 2), (T3, 3), (T4, 4), (T5, 5));
impl_match_results_for_tuple!(
    (T0, 0),
    (T1, 1),
    (T2, 2),
    (T3, 3),
    (T4, 4),
    (T5, 5),
    (T6, 6)
);
impl_match_results_for_tuple!(
    (T0, 0),
    (T1, 1),
    (T2, 2),
    (T3, 3),
    (T4, 4),
    (T5, 5),
    (T6, 6),
    (T7, 7)
);
impl_match_results_for_tuple!(
    (T0, 0),
    (T1, 1),
    (T2, 2),
    (T3, 3),
    (T4, 4),
    (T5, 5),
    (T6, 6),
    (T7, 7),
    (T8, 8)
);
impl_match_results_for_tuple!(
    (T0, 0),
    (T1, 1),
    (T2, 2),
    (T3, 3),
    (T4, 4),
    (T5, 5),
    (T6, 6),
    (T7, 7),
    (T8, 8),
    (T9, 9)
);
impl_match_results_for_tuple!(
    (T0, 0),
    (T1, 1),
    (T2, 2),
    (T3, 3),
    (T4, 4),
    (T5, 5),
    (T6, 6),
    (T7, 7),
    (T8, 8),
    (T9, 9),
    (T10, 10)
);
impl_match_results_for_tuple!(
    (T0, 0),
    (T1, 1),
    (T2, 2),
    (T3, 3),
    (T4, 4),
    (T5, 5),
    (T6, 6),
    (T7, 7),
    (T8, 8),
    (T9, 9),
    (T10, 10),
    (T11, 11)
);
