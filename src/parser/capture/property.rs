//! Capture “properties”: how each `bind!` slot maps into the live [`MatchResult`] and into its
//! [`MatchResult::Snapshot`](super::match_result::MatchResult::Snapshot).
//!
//! Each [`Property`] implementation carries two pieces of logic:
//! - **Direct** (`direct_setter`): read/write owned values in the aggregate result while matching.
//! - **Snapshot** (`snapshot_setter`): pick the correct tuple field inside a *snapshot* of one
//!   bucket (`Single` / `Multiple` / `Optional`) so deferred captures can store
//!   `&'a` references into that snapshot. That second path uses the `SingleSnapProj` /
//!   `MultipleSnapProj` / `OptionalSnapProj` traits; for tuple-shaped buckets the concrete
//!   projector is a zero-sized `SingleSnapProjAt` / `MultipleSnapProjAt` / `OptionalSnapProjAt`
//!   whose const parameter `N` is the tuple index. The [`Property::put_ref_in_snapshot`] implementations
//!   on [`SingleProperty`], [`MultipleProperty`], and [`OptionalProperty`] call `SingleSnapProj::project`
//!   (or the multiple/optional equivalents) on that value.

use super::match_result::{MatchResult, MatchResultMultiple, MatchResultOptional, MatchResultSingle};

/// Snapshot projector for a [`MatchResultSingle`] bucket: given a bucket-level snapshot, return
/// the slot corresponding to one specific bound value.
///
/// The method's data lifetime `'d` is bound at the *call site*, so the bucket's natural lifetime
/// (e.g. `'src` in `(Option<&'src str>,)`) constrains `'d` per call rather than forcing the
/// whole bucket to be `'static` (as it would under an HRTB fn-pointer bound).
///
/// Typical implementors are `SingleSnapProjAt`; their `SingleSnapProj::project` is invoked
/// from the [`Property::put_ref_in_snapshot`] implementation for [`SingleProperty`].
pub trait SingleSnapProj<Bucket, V>
where
    Bucket: MatchResultSingle,
{
    /// Returns a mutable reference to the `Option<&'d V>` field inside `snap` for this bind slot.
    fn project<'a, 'd>(
        &self,
        snap: &'a mut Bucket::Snapshot<'d>,
    ) -> &'a mut Option<&'d V>
    where
        Bucket: 'd;
}

/// Snapshot projector for a [`MatchResultMultiple`] bucket: select one `Vec` column in the
/// snapshot tuple so each capture can push a `&'d` reference.
///
/// Typical implementors are `MultipleSnapProjAt`; used from the [`Property::put_ref_in_snapshot`]
/// implementation for [`MultipleProperty`].
pub trait MultipleSnapProj<Bucket, V>
where
    Bucket: MatchResultMultiple,
{
    /// Returns a mutable reference to the `Vec<&'d V>` field inside `snap` for this bind slot.
    fn project<'a, 'd>(
        &self,
        snap: &'a mut Bucket::Snapshot<'d>,
    ) -> &'a mut Vec<&'d V>
    where
        Bucket: 'd;
}

/// Snapshot projector for a [`MatchResultOptional`] bucket: select one `Option` column in the
/// snapshot tuple for optional (`?`) binds.
///
/// Typical implementors are `OptionalSnapProjAt`; used from the [`Property::put_ref_in_snapshot`]
/// implementation for [`OptionalProperty`].
pub trait OptionalSnapProj<Bucket, V>
where
    Bucket: MatchResultOptional,
{
    /// Returns a mutable reference to the `Option<&'d V>` field inside `snap` for this bind slot.
    fn project<'a, 'd>(
        &self,
        snap: &'a mut Bucket::Snapshot<'d>,
    ) -> &'a mut Option<&'d V>
    where
        Bucket: 'd;
}

/// Zero-sized type (no fields) that means “project snapshot slot **N**” for the **single** bucket.
///
/// Tuple [`MatchResult`](super::match_result::MatchResult) implementations store one
/// [`SingleProperty`] per `bind!` column. The const generic `N` matches the tuple index (`0`, `1`,
/// …) of that column in [`MatchResultSingle::Snapshot`](MatchResultSingle::Snapshot). There is no
/// runtime index: `N` exists only in the type.
///
/// The actual `SingleSnapProj::project` implementation is emitted by the
/// `impl_match_results_for_tuple!` macro in `match_result.rs` (helper `__impl_snap_projs_recurse!`)
/// for each concrete tuple shape; it compiles to `&mut snap.N`.
///
/// Stored as the `snapshot_setter` field of [`SingleProperty`] (see [`SingleProperty::new`]) and used
/// when merging deferred captures via [`Property::put_ref_in_snapshot`].
#[derive(Clone, Copy)]
pub struct SingleSnapProjAt<const N: usize>;

/// Same idea as `SingleSnapProjAt`, but for the **multiple** bucket (`bind!(…, *ident)`):
/// projects the `N`-th `Vec<…>` field of the snapshot tuple.
///
/// `MultipleSnapProj::project` impls are generated in the same macro block as `SingleSnapProjAt`
/// in `match_result.rs`; see [`MultipleProperty::new`] and [`Property::put_ref_in_snapshot`].
#[derive(Clone, Copy)]
pub struct MultipleSnapProjAt<const N: usize>;

/// Same idea as `SingleSnapProjAt`, but for the **optional** bucket (`bind!(…, ?ident)`):
/// projects the `N`-th optional snapshot field.
///
/// See [`OptionalProperty::new`] and [`Property::put_ref_in_snapshot`].
#[derive(Clone, Copy)]
pub struct OptionalSnapProjAt<const N: usize>;

/// Source location and name for debugging double-bind panics.
#[derive(Clone, Copy)]
pub struct BindDebugInfo {
    /// Name of the binding (from `bind!` / macro expansion).
    pub property_name: &'static str,
    /// File where the bind was created.
    pub file: &'static str,
    pub line: u32,
    pub column: u32,
}

/// Describes how to write a captured `Value` into the aggregate result `MRes`.
pub trait Property<Value, MRes>
where
    MRes: MatchResult,
{
    /// Store `value` into `result`, optionally using `debug` for panic messages on conflict.
    fn put_in_result(&self, result: &mut MRes, value: Value, debug: Option<BindDebugInfo>);
    /// Insert a reference to `value` into the corresponding slot of `snapshot`.
    ///
    /// The data lifetime of the inserted reference equals the lifetime of `value`, which
    /// must outlive the data lifetime of `snapshot`.
    fn put_ref_in_snapshot<'a>(&self, snapshot: &mut MRes::Snapshot<'a>, value: &'a Value);
    /// Remove `value` from `result`.
    fn remove_from_result(&self, result: &mut MRes, debug: Option<BindDebugInfo>);
    /// Wrap `value` with this property for deferred insertion into `MRes`.
    fn bind_result(&self, value: Value) -> super::bound::BoundValue<Value, Self>
    where
        Self: Sized,
        Self: Clone,
    {
        super::bound::BoundValue::new(value, self.clone(), None)
    }
    /// Like [`bind_result`](Self::bind_result) but attaches [`BindDebugInfo`] for diagnostics.
    fn bind_result_with_debug(
        &self,
        value: Value,
        debug: BindDebugInfo,
    ) -> super::bound::BoundValue<Value, Self>
    where
        Self: Sized,
        Self: Clone,
    {
        super::bound::BoundValue::new(value, self.clone(), Some(debug))
    }
}

/// Property that writes at most once into an `Option` field (panics if set twice).
///
/// `FDirect` is usually `fn(&mut (Option<T0>, …)) -> &mut Option<Ti>`; `FSnapshot` is usually
/// `SingleSnapProjAt` with the same const index `i` (for example `SingleSnapProjAt<1>`) so
/// snapshot projection matches the same tuple index.
#[derive(Clone, Copy)]
pub struct SingleProperty<FDirect, FSnapshot> {
    /// Selects this bind's `Option<V>` in the **live** single bucket (`MRes::single()`).
    direct_setter: FDirect,
    /// Selects the same slot in the **snapshot** single bucket for deferred `&` inserts.
    snapshot_setter: FSnapshot,
}

impl<FDirect, FSnapshot> SingleProperty<FDirect, FSnapshot> {
    /// `setter` selects the `Option` slot inside the “single” capture bucket.
    pub fn new(direct_setter: FDirect, snapshot_setter: FSnapshot) -> Self {
        Self {
            direct_setter,
            snapshot_setter,
        }
    }
}

impl<V, MRes, FDirect, FSnapshot> Property<V, MRes> for SingleProperty<FDirect, FSnapshot>
where
    MRes: MatchResult,
    FDirect: Fn(&mut MRes::Single) -> &mut Option<V>,
    FSnapshot: SingleSnapProj<MRes::Single, V>,
{
    fn put_in_result(&self, result: &mut MRes, value: V, debug: Option<BindDebugInfo>) {
        let property_slot = (self.direct_setter)(result.single());
        if property_slot.is_some() {
            if let Some(debug) = debug {
                panic!(
                    "SingleProperty '{}' already set (bind! at {}:{}:{})",
                    debug.property_name, debug.file, debug.line, debug.column
                );
            }
            panic!("SingleProperty already set");
        }
        *property_slot = Some(value);
    }

    fn put_ref_in_snapshot<'a>(&self, snapshot: &mut MRes::Snapshot<'a>, value: &'a V) {
        let bucket = MRes::project_single_snapshot_mut(snapshot);
        let property_slot = self.snapshot_setter.project(bucket);
        if property_slot.is_some() {
            panic!("SingleProperty already set in snapshot");
        }
        *property_slot = Some(value);
    }

    fn remove_from_result(&self, result: &mut MRes, _debug: Option<BindDebugInfo>) {
        let property_slot = (self.direct_setter)(result.single());
        if property_slot.is_some() {
            *property_slot = None;
        } else {
            panic!("Trying to remove a value that was not set");
        }
    }
}

/// Property that appends each capture into a `Vec`.
///
/// `FSnapshot` is typically `MultipleSnapProjAt` with the same index as `FDirect`, e.g. `MultipleSnapProjAt<0>`.
#[derive(Clone, Copy)]
pub struct MultipleProperty<FDirect, FSnapshot> {
    /// Selects this bind's `Vec<V>` in the live multiple bucket.
    direct_setter: FDirect,
    /// Selects the same `Vec` column in the snapshot tuple for deferred references.
    snapshot_setter: FSnapshot,
}

impl<FDirect, FSnapshot> MultipleProperty<FDirect, FSnapshot> {
    /// `setter` selects the `Vec` field inside the “multiple” capture bucket.
    pub fn new(direct_setter: FDirect, snapshot_setter: FSnapshot) -> Self {
        Self {
            direct_setter,
            snapshot_setter,
        }
    }
}

impl<V, MRes, FDirect, FSnapshot> Property<V, MRes> for MultipleProperty<FDirect, FSnapshot>
where
    MRes: MatchResult,
    FDirect: Fn(&mut MRes::Multiple) -> &mut Vec<V>,
    FSnapshot: MultipleSnapProj<MRes::Multiple, V>,
{
    fn put_in_result(&self, result: &mut MRes, value: V, _debug: Option<BindDebugInfo>) {
        let property_slot = (self.direct_setter)(result.multiple());
        property_slot.push(value);
    }

    fn put_ref_in_snapshot<'a>(&self, snapshot: &mut MRes::Snapshot<'a>, value: &'a V) {
        let bucket = MRes::project_multiple_snapshot_mut(snapshot);
        let property_slot = self.snapshot_setter.project(bucket);
        property_slot.push(value);
    }

    fn remove_from_result(&self, result: &mut MRes, _debug: Option<BindDebugInfo>) {
        let property_slot = (self.direct_setter)(result.multiple());
        if property_slot.pop().is_none() {
            panic!("Trying to remove a value that was not set");
        }
    }
}

/// Property for an optional capture stored in `Option` (panics if set twice).
///
/// `FSnapshot` is typically `OptionalSnapProjAt` with the same index as `FDirect`, e.g. `OptionalSnapProjAt<0>`.
#[derive(Clone, Copy)]
pub struct OptionalProperty<FDirect, FSnapshot> {
    /// Selects this bind's `Option<V>` in the live optional bucket.
    direct_setter: FDirect,
    /// Selects the same slot in the snapshot optional bucket for deferred references.
    snapshot_setter: FSnapshot,
}

impl<FDirect, FSnapshot> OptionalProperty<FDirect, FSnapshot> {
    /// `setter` selects the `Option` slot inside the “optional” capture bucket.
    pub fn new(direct_setter: FDirect, snapshot_setter: FSnapshot) -> Self {
        Self {
            direct_setter,
            snapshot_setter,
        }
    }
}

impl<V, MRes, FDirect, FSnapshot> Property<V, MRes> for OptionalProperty<FDirect, FSnapshot>
where
    MRes: MatchResult,
    FDirect: Fn(&mut MRes::Optional) -> &mut Option<V>,
    FSnapshot: OptionalSnapProj<MRes::Optional, V>,
{
    fn put_in_result(&self, result: &mut MRes, value: V, debug: Option<BindDebugInfo>) {
        let property_slot = (self.direct_setter)(result.optional());
        if property_slot.is_some() {
            if let Some(debug) = debug {
                panic!(
                    "OptionalProperty '{}' already set (bind! at {}:{}:{})",
                    debug.property_name, debug.file, debug.line, debug.column
                );
            }
            panic!("OptionalProperty already set");
        }
        *property_slot = Some(value);
    }

    fn put_ref_in_snapshot<'a>(&self, snapshot: &mut MRes::Snapshot<'a>, value: &'a V) {
        let bucket = MRes::project_optional_snapshot_mut(snapshot);
        let property_slot = self.snapshot_setter.project(bucket);
        if property_slot.is_some() {
            panic!("OptionalProperty already set in snapshot");
        }
        *property_slot = Some(value);
    }

    fn remove_from_result(&self, result: &mut MRes, _debug: Option<BindDebugInfo>) {
        let property_slot = (self.direct_setter)(result.optional());
        if property_slot.is_some() {
            *property_slot = None;
        } else {
            panic!("Trying to remove a value that was not set");
        }
    }
}
