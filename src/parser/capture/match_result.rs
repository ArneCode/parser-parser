use super::property::{
    MultipleProperty, MultipleSnapProj, MultipleSnapProjAt, OptionalProperty, OptionalSnapProj,
    OptionalSnapProjAt, SingleProperty, SingleSnapProj, SingleSnapProjAt,
};

pub trait MatchResultSingle {
    type Snapshot<'a>
    where
        Self: 'a;
    type Properties;
    type Output;
    fn new() -> Self;
    fn new_properties() -> Self::Properties;
    fn to_output(self) -> Self::Output;
    fn subtract_from_result(&self, result: &mut Self);
    fn snapshot(&self) -> Self::Snapshot<'_>;
    fn new_empty_snapshot<'a>() -> Self::Snapshot<'a>
    where
        Self: 'a;
}

pub trait MatchResultMultiple {
    type Snapshot<'a>
    where
        Self: 'a;
    type Properties;
    fn new() -> Self;
    fn new_properties() -> Self::Properties;
    fn subtract_from_result(&self, result: &mut Self);
    fn snapshot(&self) -> Self::Snapshot<'_>;
    fn new_empty_snapshot<'a>() -> Self::Snapshot<'a>
    where
        Self: 'a;
}

pub trait MatchResultOptional {
    type Snapshot<'a>
    where
        Self: 'a;
    type Properties;
    fn new() -> Self;
    fn new_properties() -> Self::Properties;
    fn subtract_from_result(&self, result: &mut Self);
    fn snapshot(&self) -> Self::Snapshot<'_>;
    fn new_empty_snapshot<'a>() -> Self::Snapshot<'a>
    where
        Self: 'a;
}

pub trait MatchResult {
    type Snapshot<'a>
    where
        Self: 'a;
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
    fn snapshot(&self) -> Self::Snapshot<'_>;
    fn subtract_from_result(&self, result: &mut Self);

    /// Empty snapshot (all slots unset / empty vectors) for replaying bound captures.
    fn new_empty_snapshot<'a>() -> Self::Snapshot<'a>
    where
        Self: 'a;

    /// Project a top-level snapshot into its `Single` bucket sub-snapshot.
    fn project_single_snapshot_mut<'a, 'd>(
        snap: &'a mut Self::Snapshot<'d>,
    ) -> &'a mut <Self::Single as MatchResultSingle>::Snapshot<'d>
    where
        Self: 'd;
    /// Project a top-level snapshot into its `Multiple` bucket sub-snapshot.
    fn project_multiple_snapshot_mut<'a, 'd>(
        snap: &'a mut Self::Snapshot<'d>,
    ) -> &'a mut <Self::Multiple as MatchResultMultiple>::Snapshot<'d>
    where
        Self: 'd;
    /// Project a top-level snapshot into its `Optional` bucket sub-snapshot.
    fn project_optional_snapshot_mut<'a, 'd>(
        snap: &'a mut Self::Snapshot<'d>,
    ) -> &'a mut <Self::Optional as MatchResultOptional>::Snapshot<'d>
    where
        Self: 'd;
}

impl<MResSingle, MResMultiple, MResOptional> MatchResult
    for (MResSingle, MResMultiple, MResOptional)
where
    MResSingle: MatchResultSingle,
    MResMultiple: MatchResultMultiple,
    MResOptional: MatchResultOptional,
{
    type Snapshot<'a>
        = (
        MResSingle::Snapshot<'a>,
        MResMultiple::Snapshot<'a>,
        MResOptional::Snapshot<'a>,
    )
    where
        Self: 'a;

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

    fn snapshot(&self) -> Self::Snapshot<'_> {
        (self.0.snapshot(), self.1.snapshot(), self.2.snapshot())
    }

    fn subtract_from_result(&self, result: &mut Self) {
        self.0.subtract_from_result(result.single());
        self.1.subtract_from_result(result.multiple());
        self.2.subtract_from_result(result.optional());
    }

    fn project_single_snapshot_mut<'a, 'd>(
        snap: &'a mut Self::Snapshot<'d>,
    ) -> &'a mut MResSingle::Snapshot<'d>
    where
        Self: 'd,
    {
        &mut snap.0
    }

    fn project_multiple_snapshot_mut<'a, 'd>(
        snap: &'a mut Self::Snapshot<'d>,
    ) -> &'a mut MResMultiple::Snapshot<'d>
    where
        Self: 'd,
    {
        &mut snap.1
    }

    fn project_optional_snapshot_mut<'a, 'd>(
        snap: &'a mut Self::Snapshot<'d>,
    ) -> &'a mut MResOptional::Snapshot<'d>
    where
        Self: 'd,
    {
        &mut snap.2
    }

    fn new_empty_snapshot<'a>() -> Self::Snapshot<'a>
    where
        Self: 'a,
    {
        (
            MResSingle::new_empty_snapshot(),
            MResMultiple::new_empty_snapshot(),
            MResOptional::new_empty_snapshot(),
        )
    }
}

impl MatchResultSingle for () {
    type Snapshot<'a>
        = ()
    where
        Self: 'a;
    type Properties = ();
    type Output = ();
    fn new() -> Self {}
    fn new_properties() -> Self::Properties {}
    fn to_output(self) -> Self::Output {}
    fn subtract_from_result(&self, _result: &mut Self) {}
    fn snapshot(&self) -> Self::Snapshot<'_> {}
    fn new_empty_snapshot<'a>() -> Self::Snapshot<'a>
    where
        Self: 'a,
    {
        
    }
}

impl MatchResultMultiple for () {
    type Snapshot<'a>
        = ()
    where
        Self: 'a;
    type Properties = ();
    fn new() -> Self {}
    fn new_properties() -> Self::Properties {}
    fn subtract_from_result(&self, _result: &mut Self) {}
    fn snapshot(&self) -> Self::Snapshot<'_> {}
    fn new_empty_snapshot<'a>() -> Self::Snapshot<'a>
    where
        Self: 'a,
    {
        
    }
}

impl MatchResultOptional for () {
    type Snapshot<'a>
        = ()
    where
        Self: 'a;
    type Properties = ();
    fn new() -> Self {}
    fn new_properties() -> Self::Properties {}
    fn subtract_from_result(&self, _result: &mut Self) {}
    fn snapshot(&self) -> Self::Snapshot<'_> {}
    fn new_empty_snapshot<'a>() -> Self::Snapshot<'a>
    where
        Self: 'a,
    {
        
    }
}

fn unwrap_single<T>(option: Option<T>) -> T {
    option.expect("Expected single match result to be set, but it was not")
}

// Emit `SingleSnapProj` / `MultipleSnapProj` / `OptionalSnapProj` impls for each slot.
//
// Has to be recursive (TT-munching) because `macro_rules!` doesn't allow nesting two `$(...)+`
// over the same metavariable: we need to mention the *full* tuple type inside a *per-slot*
// expansion, so we pre-expand the bucket types once at the top-level callsite, pass them in
// as `:ty` tokens, and walk the slot list one element at a time.
macro_rules! __impl_snap_projs_recurse {
    // Base case: no more slots to process.
    (
        $bucket_single:ty,
        $bucket_multiple:ty,
        $bucket_optional:ty,
        [$($Tall:ident),+],
        []
    ) => {};

    // Recursive case: handle one (T, idx) pair, recurse on the rest.
    (
        $bucket_single:ty,
        $bucket_multiple:ty,
        $bucket_optional:ty,
        [$($Tall:ident),+],
        [($Thead:ident, $idxhead:tt) $(, ($Trest:ident, $idxrest:tt))*]
    ) => {
        impl<$($Tall),+> SingleSnapProj<$bucket_single, $Thead> for SingleSnapProjAt<$idxhead> {
            fn project<'a, 'd>(
                &self,
                snap: &'a mut <$bucket_single as MatchResultSingle>::Snapshot<'d>,
            ) -> &'a mut Option<&'d $Thead>
            where
                $bucket_single: 'd,
            {
                &mut snap.$idxhead
            }
        }
        impl<$($Tall),+> MultipleSnapProj<$bucket_multiple, $Thead> for MultipleSnapProjAt<$idxhead> {
            fn project<'a, 'd>(
                &self,
                snap: &'a mut <$bucket_multiple as MatchResultMultiple>::Snapshot<'d>,
            ) -> &'a mut Vec<&'d $Thead>
            where
                $bucket_multiple: 'd,
            {
                &mut snap.$idxhead
            }
        }
        impl<$($Tall),+> OptionalSnapProj<$bucket_optional, $Thead> for OptionalSnapProjAt<$idxhead> {
            fn project<'a, 'd>(
                &self,
                snap: &'a mut <$bucket_optional as MatchResultOptional>::Snapshot<'d>,
            ) -> &'a mut Option<&'d $Thead>
            where
                $bucket_optional: 'd,
            {
                &mut snap.$idxhead
            }
        }
        __impl_snap_projs_recurse! {
            $bucket_single,
            $bucket_multiple,
            $bucket_optional,
            [$($Tall),+],
            [$(($Trest, $idxrest)),*]
        }
    };
}

macro_rules! impl_match_results_for_tuple {
    ( $(($T:ident, $idx:tt)),+ ) => {

        impl<$($T),+> MatchResultSingle for ($(Option<$T>,)+)
        {
            type Snapshot<'a>
                = ($(Option<&'a $T>,)+)
            where
                Self: 'a;
            type Properties = (
                $(SingleProperty<
                    fn(&mut Self) -> &mut Option<$T>,
                    SingleSnapProjAt<$idx>,
                >,)+
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
                        SingleSnapProjAt::<$idx>,
                    ),
                )+)
            }

            fn to_output(self) -> Self::Output {
                #[allow(non_snake_case)]
                let ($( $T, )+) = self;
                ($(unwrap_single($T),)+)
            }

            fn subtract_from_result(&self, result: &mut Self) {
                // check if the result is set, then remove it if it is
                $(if self.$idx.is_some() {
                    result.$idx = None;
                })+;

            }

            fn snapshot(&self) -> Self::Snapshot<'_> {
                ($(self.$idx.as_ref(),)+)
            }

            fn new_empty_snapshot<'a>() -> Self::Snapshot<'a>
            where
                Self: 'a,
            {
                ($(None::<&'a $T>,)+)
            }
        }

        impl<$($T),+> MatchResultMultiple for ($(Vec<$T>,)+) {
            type Snapshot<'a>
                = ($(Vec<&'a $T>,)+)
            where
                Self: 'a;
            type Properties = (
                $(MultipleProperty<
                    fn(&mut Self) -> &mut Vec<$T>,
                    MultipleSnapProjAt<$idx>,
                >,)+
            );

            fn new() -> Self {
                ($( { let _: std::marker::PhantomData<$T>; Vec::new() },)+ )
            }

            fn new_properties() -> Self::Properties {
                ($(
                    MultipleProperty::new(
                        (|s: &mut Self| -> &mut Vec<$T> { &mut s.$idx })
                            as fn(&mut Self) -> &mut Vec<$T>,
                        MultipleSnapProjAt::<$idx>,
                    ),
                )+)
            }

            fn subtract_from_result(&self, result: &mut Self) {
                $(
                    {
                        let n = self.$idx.len();
                        // remove the last n elements from the result
                        result.$idx.truncate(result.$idx.len() - n);
                    }
                )+;
            }

            fn snapshot(&self) -> Self::Snapshot<'_> {
                ($(self.$idx.iter().collect::<Vec<&$T>>(),)+)
            }

            fn new_empty_snapshot<'a>() -> Self::Snapshot<'a>
            where
                Self: 'a,
            {
                ($(Vec::<&'a $T>::new(),)+)
            }

        }

        impl<$($T),+> MatchResultOptional for ($(Option<$T>,)+) {
            type Snapshot<'a>
                = ($(Option<&'a $T>,)+)
            where
                Self: 'a;
            type Properties = (
                $(OptionalProperty<
                    fn(&mut Self) -> &mut Option<$T>,
                    OptionalSnapProjAt<$idx>,
                >,)+
            );

            fn new() -> Self {
                ($( { let _: std::marker::PhantomData<$T>; None },)+ )
            }

            fn new_properties() -> Self::Properties {
                ($(
                    OptionalProperty::new(
                        (|s: &mut Self| -> &mut Option<$T> { &mut s.$idx })
                            as fn(&mut Self) -> &mut Option<$T>,
                        OptionalSnapProjAt::<$idx>,
                    ),
                )+)
            }

            fn subtract_from_result(&self, result: &mut Self) {
                $(if self.$idx.is_some() {
                    result.$idx = None;
                })+;
            }

            fn snapshot(&self) -> Self::Snapshot<'_> {
                ($(self.$idx.as_ref(),)+)
            }

            fn new_empty_snapshot<'a>() -> Self::Snapshot<'a>
            where
                Self: 'a,
            {
                ($(None::<&'a $T>,)+)
            }
        }

        __impl_snap_projs_recurse! {
            ($(Option<$T>,)+),
            ($(Vec<$T>,)+),
            ($(Option<$T>,)+),
            [$($T),+],
            [$(($T, $idx)),+]
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
